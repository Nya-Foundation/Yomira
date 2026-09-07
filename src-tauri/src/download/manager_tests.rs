use super::*;
use crate::download::spider::FetchedImage;
use crate::download::store::DownloadStore;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Network-free page source: canned bytes per page, injectable failures.
struct MockSource {
    fail_at: Option<i32>,
    error: Option<EhError>,
    block: Arc<AtomicBool>,
    delay: Duration,
    calls: Mutex<Vec<i32>>,
}

impl MockSource {
    fn new() -> Self {
        Self {
            fail_at: None,
            error: None,
            block: Arc::new(AtomicBool::new(false)),
            delay: Duration::ZERO,
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl PageSource for MockSource {
    fn resolve_page<'a>(
        &'a self,
        _gid: u64,
        _gtoken: &'a str,
        index: i32,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<crate::download::spider::ResolvedPage, EhError>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(crate::download::spider::ResolvedPage {
                image_url: format!("mock://{index}"),
                referer: format!("mock-ref-{index}"),
                ptoken: Some(format!("pt{index}")),
            })
        })
    }

    fn fetch_image<'a>(
        &'a self,
        image_url: &'a str,
        _referer: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<FetchedImage, EhError>> + Send + 'a>> {
        Box::pin(async move {
            while self.block.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            tokio::time::sleep(self.delay).await;
            // The mock URL carries the page index.
            let index: i32 = image_url.trim_start_matches("mock://").parse().unwrap();
            self.calls.lock().unwrap().push(index);
            if self.fail_at == Some(index) {
                return Err(self
                    .error
                    .clone()
                    .unwrap_or_else(|| EhError::Eh("mock failure".into())));
            }
            Ok(FetchedImage {
                bytes: format!("page {index}").into_bytes(),
                extension: "jpg".into(),
            })
        })
    }
}

fn temp_dir(name: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "exviewer-manager-test-{}-{}-{unique}",
        name,
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn info(gid: u64, pages: i32) -> GalleryDownloadInfo {
    GalleryDownloadInfo {
        gid,
        token: format!("tok{gid}"),
        title: format!("Title {gid}"),
        title_jpn: None,
        thumb: "https://ehgt.org/t.jpg".into(),
        category: 2,
        pages,
        preview_pages: 1,
        preview_ptokens: vec![(0, "pv0".into()), (1, "pv1".into())],
    }
}

/// A manager over a fresh temp DB + download root.
fn test_manager(name: &str, source: Arc<dyn PageSource>) -> (PathBuf, DownloadManager) {
    let dir = temp_dir(name);
    let root = dir.join("downloads");
    let manager = DownloadManager::init(&dir.join("exviewer.db"), root.clone(), source).unwrap();
    (dir, manager)
}

async fn wait_for(mut check: impl FnMut() -> bool) {
    for _ in 0..500 {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("condition not met within the timeout");
}

#[test]
fn enqueue_seeds_dir_metadata_and_persists() {
    let (_dir, manager) = test_manager("enqueue", Arc::new(MockSource::new()));
    let item = manager.enqueue(info(42, 10), Some("fav".into())).unwrap();

    assert_eq!(item.state, DownloadState::Queued);
    assert_eq!(item.label.as_deref(), Some("fav"));
    assert_eq!(item.dirname, "42-Title 42");
    let dir = item.dir_path(&manager.root());
    assert!(dir.is_dir());
    // Metadata seeded with the preview ptokens.
    let spider_info = SpiderInfo::read(&dir.join(SPIDER_INFO_FILENAME))
        .unwrap()
        .expect("metadata present");
    assert_eq!(spider_info.gid, 42);
    assert_eq!(spider_info.pages, 10);
    assert_eq!(spider_info.ptoken_map.len(), 2);
    // Persisted to the downloads table.
    let rows = manager.list();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].gid, 42);
    assert_eq!(rows[0].state, DownloadState::Queued);
}

#[test]
fn enqueue_requires_durable_offline_metadata() {
    let (_dir, manager) = test_manager("metadata-failure", Arc::new(MockSource::new()));
    let den = GalleryDen::open(&manager.root(), "42-Title 42").unwrap();
    // A directory at the metadata destination makes the atomic rename fail.
    std::fs::create_dir(den.path().join(SPIDER_INFO_FILENAME)).unwrap();
    assert!(manager.enqueue(info(42, 3), None).is_err());
    assert!(manager.list().is_empty());
}

#[test]
fn reenqueue_refreshes_and_moves_to_back() {
    let (_dir, manager) = test_manager("reenqueue", Arc::new(MockSource::new()));
    manager.enqueue(info(1, 5), None).unwrap();
    manager.enqueue(info(2, 5), None).unwrap();

    let mut refreshed = info(1, 7);
    refreshed.title = "Renamed".into();
    let item = manager.enqueue(refreshed, None).unwrap();
    assert_eq!(item.title, "Renamed");
    assert_eq!(item.pages, 7);

    let order: Vec<u64> = manager.list().iter().map(|i| i.gid).collect();
    assert_eq!(
        order,
        vec![2, 1],
        "re-enqueue moves the gallery to the back"
    );
}

#[test]
fn start_pause_and_remove_transitions() {
    let (_dir, manager) = test_manager("transitions", Arc::new(MockSource::new()));
    let item = manager.enqueue(info(1, 5), None).unwrap();
    assert_eq!(item.state, DownloadState::Queued);

    manager.pause(1).unwrap();
    assert_eq!(manager.get(1).unwrap().state, DownloadState::Paused);
    manager.pause(1).unwrap(); // pausing twice is a no-op

    manager.start(1).unwrap();
    assert_eq!(manager.get(1).unwrap().state, DownloadState::Queued);
    manager.start(1).unwrap(); // starting a queued download is a no-op

    assert!(manager
        .start(999)
        .unwrap_err()
        .to_string()
        .contains("no such download"));

    // remove without deleting files keeps the dir
    let dir = manager.get(1).unwrap().dir_path(&manager.root());
    manager.remove(1, false).unwrap();
    assert!(manager.get(1).is_none());
    assert!(dir.is_dir());
    manager.remove(1, true).unwrap(); // unknown gid is a no-op

    // remove with delete_files wipes the dir
    let item = manager.enqueue(info(2, 5), None).unwrap();
    let dir = item.dir_path(&manager.root());
    manager.remove(2, true).unwrap();
    assert!(!dir.exists());
    assert!(manager.list().is_empty());
}

#[test]
fn failed_download_can_be_requeued() {
    let (_dir, manager) = test_manager("requeue", Arc::new(MockSource::new()));
    let item = manager.enqueue(info(1, 5), None).unwrap();
    let mut failed = item;
    failed.state = DownloadState::Failed;
    failed.error = Some("boom".into());
    {
        let mut items = manager.items_lock();
        items[0] = failed.clone();
    }
    manager
        .inner
        .store
        .update_state(1, DownloadState::Failed.to_db())
        .unwrap();

    manager.start(1).unwrap();
    let item = manager.get(1).unwrap();
    assert_eq!(item.state, DownloadState::Queued);
    assert_eq!(item.error, None);
}

#[test]
fn labels_crud_and_detach_on_delete() {
    let (_dir, manager) = test_manager("labels", Arc::new(MockSource::new()));
    manager.add_label("miku").unwrap();
    manager.add_label("miku").unwrap();
    manager.add_label("other").unwrap();
    assert!(manager.add_label("  ").is_err());
    assert_eq!(manager.labels().unwrap(), vec!["miku", "other"]);

    manager.enqueue(info(1, 5), None).unwrap();
    manager.set_label(1, Some("miku".into())).unwrap();
    assert_eq!(manager.get(1).unwrap().label.as_deref(), Some("miku"));
    manager.set_label(1, None).unwrap();
    assert_eq!(manager.get(1).unwrap().label, None);
    assert!(manager.set_label(999, Some("x".into())).is_err());

    manager.set_label(1, Some("miku".into())).unwrap();
    manager.delete_label("miku").unwrap();
    assert_eq!(manager.labels().unwrap(), vec!["other"]);
    assert_eq!(
        manager.get(1).unwrap().label,
        None,
        "delete detaches the label"
    );
}

#[test]
fn set_root_persists_across_restarts() {
    let (dir, manager) = test_manager("root", Arc::new(MockSource::new()));
    assert_eq!(manager.root(), dir.join("downloads"));

    let new_root = dir.join("elsewhere").join("dl");
    manager.set_root(new_root.clone()).unwrap();
    assert_eq!(manager.root(), new_root);
    assert!(new_root.is_dir());

    // A new manager over the same DB picks the root up.
    let reopened = DownloadManager::init(
        &dir.join("exviewer.db"),
        dir.join("downloads"),
        Arc::new(MockSource::new()),
    )
    .unwrap();
    assert_eq!(reopened.root(), new_root);
}

#[test]
fn restore_scan_counts_pages_and_reconciles_states() {
    let dir = temp_dir("restore");
    let root = dir.join("downloads");
    let db = dir.join("exviewer.db");

    // Row 1: dir fully populated, was "downloading" -> Done.
    let den = GalleryDen::open(&root, "1-Full").unwrap();
    for index in 0..3 {
        den.write_page(index, "jpg", b"x").unwrap();
    }
    // Row 2: dir half-populated, was "downloading" -> Paused(1).
    let den = GalleryDen::open(&root, "2-Partial").unwrap();
    den.write_page(0, "jpg", b"x").unwrap();
    // Row 3: dir missing, was "queued" -> Paused.
    // Row 4: finished download whose dir vanished -> stays Done.
    let store = DownloadStore::open(&db).unwrap();
    for (gid, title, state) in [
        (1, "Full", 2),
        (2, "Partial", 2),
        (3, "Missing", 1),
        (4, "Gone", 4),
    ] {
        store
            .upsert(&DownloadRow {
                gid,
                token: "tok".into(),
                title: title.into(),
                title_jpn: None,
                thumb: String::new(),
                category: 0,
                state,
                time: gid,
                label: None,
                dirname: Some(format!("{gid}-{title}")),
            })
            .unwrap();
    }
    // The downloads table has no pages column (same as the original
    // schema); the totals live in the `.ehviewer` headers.
    let full_info = SpiderInfo::new(1, "tok", 3, 1, 40);
    full_info.write_to_dir(&root.join("1-Full")).unwrap();
    let mut partial_info = SpiderInfo::new(2, "tok", 3, 1, 40);
    partial_info.start_page = 1;
    partial_info.write_to_dir(&root.join("2-Partial")).unwrap();

    let manager = DownloadManager::init(&db, root, Arc::new(MockSource::new())).unwrap();
    let items = manager.list();
    assert_eq!(items.len(), 4);

    let full = manager.get(1).unwrap();
    assert_eq!(full.state, DownloadState::Done);
    assert_eq!(full.finished_pages, 3);

    let partial = manager.get(2).unwrap();
    assert_eq!(partial.state, DownloadState::Paused);
    assert_eq!(partial.finished_pages, 1);

    let missing = manager.get(3).unwrap();
    assert_eq!(missing.state, DownloadState::Paused);
    assert_eq!(missing.finished_pages, 0);

    let gone = manager.get(4).unwrap();
    assert_eq!(gone.state, DownloadState::Paused);
}

#[tokio::test]
async fn worker_downloads_a_gallery_to_done() {
    let (dir, manager) = test_manager("done", Arc::new(MockSource::new()));
    manager.enqueue(info(7, 4), None).unwrap();
    manager.run_workers();

    wait_for(|| manager.get(7).map(|i| i.state) == Some(DownloadState::Done)).await;
    let item = manager.get(7).unwrap();
    assert_eq!(item.finished_pages, 4);
    assert_eq!(item.error, None);

    let den = GalleryDen::at(item.dir_path(&manager.root()));
    assert_eq!(den.count_pages(), 4);
    assert_eq!(std::fs::read(den.find_page(3).unwrap()).unwrap(), b"page 3");
    // Progress made it into the metadata (durable resume point).
    let spider_info = SpiderInfo::read(&den.path().join(SPIDER_INFO_FILENAME))
        .unwrap()
        .unwrap();
    assert_eq!(spider_info.start_page, 4);
    assert_eq!(spider_info.ptoken_map.len(), 4);

    // And into the DB state column.
    let store = DownloadStore::open(&dir.join("exviewer.db")).unwrap();
    assert_eq!(store.list().unwrap()[0].state, DownloadState::Done.to_db());
}

#[tokio::test]
async fn offline_reader_survives_restart_and_streams_without_network_or_cache() {
    use crate::download::offline;
    use crate::http_images::{spawn_server, LocalImages};
    let (dir, manager) = test_manager("offline-restart", Arc::new(MockSource::new()));
    manager.enqueue(info(71, 4), None).unwrap();
    manager.run_workers();
    wait_for(|| manager.get(71).map(|i| i.state) == Some(DownloadState::Done)).await;
    let gallery_dir = manager.get(71).unwrap().dir_path(&manager.root());
    drop(manager);

    // Simulate a missing middle page and a legacy EhViewer filename.
    std::fs::remove_file(gallery_dir.join("0002.jpg")).unwrap();
    std::fs::rename(
        gallery_dir.join("0003.jpg"),
        gallery_dir.join("00000003.jpg"),
    )
    .unwrap();
    std::fs::rename(gallery_dir.join("0004.jpg"), gallery_dir.join("0004.avif")).unwrap();
    // Empty, out-of-range files and directories must not count as saved pages.
    std::fs::write(gallery_dir.join("0002.png"), []).unwrap();
    std::fs::write(gallery_dir.join("9999.jpg"), b"outside page range").unwrap();
    std::fs::create_dir(gallery_dir.join("0002.webp")).unwrap();
    let source = Arc::new(MockSource::new());
    let reopened = DownloadManager::init(
        &dir.join("exviewer.db"),
        dir.join("downloads"),
        source.clone(),
    )
    .unwrap();
    let saved = offline::gallery(&reopened, 71, "tok71").unwrap();
    assert_eq!(saved.title, "Title 71");
    assert_eq!(saved.pages, 4);
    assert_eq!(saved.available_pages, vec![0, 2, 3]);
    assert_eq!(reopened.get(71).unwrap().state, DownloadState::Paused);
    assert!(offline::gallery(&reopened, 71, "wrong-token").is_none());
    assert!(offline::gallery(&reopened, 999, "tok999").is_none());

    // Fresh registry/server with an empty image cache, as after restart.
    let local = LocalImages::default();
    let cache_dir = dir.join("empty-cache");
    std::fs::create_dir(&cache_dir).unwrap();
    let port = spawn_server(cache_dir.clone(), local.clone()).unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    for index in [0, 2, 3] {
        let url = offline::page_url(&reopened, &local, port, 71, "tok71", index).unwrap();
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            response.headers()["content-type"],
            if index == 3 {
                "image/avif"
            } else {
                "image/jpeg"
            }
        );
        assert_eq!(response.text().await.unwrap(), format!("page {index}"));
    }
    for index in [-1, 1, 4, i32::MAX] {
        assert!(matches!(
            offline::page_url(&reopened, &local, port, 71, "tok71", index),
            Err(EhError::PageUnavailable)
        ));
    }
    assert_eq!(std::fs::read_dir(cache_dir).unwrap().count(), 0);
    assert!(
        source.calls.lock().unwrap().is_empty(),
        "offline reads must not fetch images"
    );

    let url = offline::page_url(&reopened, &local, port, 71, "tok71", 0).unwrap();
    std::fs::remove_file(gallery_dir.join("0001.jpg")).unwrap();
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(response.headers()["cache-control"], "no-store");
    // A previously missing page becomes available on retry, without restart.
    std::fs::write(gallery_dir.join("0002.jpg"), b"restored page").unwrap();
    let url = offline::page_url(&reopened, &local, port, 71, "tok71", 1).unwrap();
    assert_eq!(
        client.get(url).send().await.unwrap().text().await.unwrap(),
        "restored page"
    );
    assert!(source.calls.lock().unwrap().is_empty());
    // Explicitly resuming repairs the remaining hole only, preserving later files.
    reopened.start(71).unwrap();
    reopened.run_workers();
    wait_for(|| reopened.get(71).map(|i| i.state) == Some(DownloadState::Done)).await;
    assert_eq!(*source.calls.lock().unwrap(), vec![0]);
    assert_eq!(
        offline::gallery(&reopened, 71, "tok71")
            .unwrap()
            .available_pages,
        vec![0, 1, 2, 3]
    );
    reopened.remove(71, false).unwrap();
    assert!(offline::gallery(&reopened, 71, "tok71").is_none());
}

#[tokio::test]
async fn worker_marks_failures_and_keeps_partial_pages() {
    let mut source = MockSource::new();
    source.fail_at = Some(1);
    let (_dir, manager) = test_manager("failed", Arc::new(source));
    manager.enqueue(info(7, 4), None).unwrap();
    manager.run_workers();

    wait_for(|| {
        manager
            .get(7)
            .map(|i| i.state == DownloadState::Failed)
            .unwrap_or(false)
    })
    .await;
    let item = manager.get(7).unwrap();
    assert_eq!(item.finished_pages, 1);
    assert!(item.error.unwrap().contains("mock failure"));
}

#[tokio::test]
async fn worker_pauses_on_509_and_surfaces_the_error() {
    let mut source = MockSource::new();
    source.fail_at = Some(0);
    source.error = Some(EhError::Image509);
    let (_dir, manager) = test_manager("509", Arc::new(source));
    manager.enqueue(info(7, 4), None).unwrap();
    manager.run_workers();

    wait_for(|| {
        manager
            .get(7)
            .map(|i| i.state == DownloadState::Paused && i.error.is_some())
            .unwrap_or(false)
    })
    .await;
    let item = manager.get(7).unwrap();
    assert!(
        item.error.unwrap().contains("509"),
        "509 must surface as the error"
    );
    assert_eq!(item.finished_pages, 0);
}

#[tokio::test]
async fn pause_mid_download_stops_between_pages() {
    let mut source = MockSource::new();
    source.block = Arc::new(AtomicBool::new(true));
    source.delay = Duration::from_millis(10);
    let block = source.block.clone();
    let (_dir, manager) = test_manager("midpause", Arc::new(source));
    manager.enqueue(info(7, 50), None).unwrap();
    manager.run_workers();

    wait_for(|| {
        manager
            .get(7)
            .map(|i| i.state == DownloadState::Downloading)
            .unwrap_or(false)
    })
    .await;
    manager.pause(7).unwrap();
    assert_eq!(manager.get(7).unwrap().state, DownloadState::Paused);
    block.store(false, Ordering::Relaxed);

    wait_for(|| {
        manager
            .get(7)
            .map(|i| i.state == DownloadState::Paused && i.finished_pages < 50)
            .unwrap_or(false)
    })
    .await;
    let item = manager.get(7).unwrap();
    assert!(item.finished_pages < 50);
    let den = GalleryDen::at(item.dir_path(&manager.root()));
    assert_eq!(den.count_pages(), item.finished_pages);
}

#[tokio::test]
async fn remove_cancels_an_active_download() {
    let mut source = MockSource::new();
    source.block = Arc::new(AtomicBool::new(true));
    let block = source.block.clone();
    let (_dir, manager) = test_manager("rmactive", Arc::new(source));
    manager.enqueue(info(7, 10), None).unwrap();
    manager.run_workers();
    wait_for(|| {
        manager
            .get(7)
            .map(|i| i.state == DownloadState::Downloading)
            .unwrap_or(false)
    })
    .await;

    manager.remove(7, true).unwrap();
    assert!(manager.get(7).is_none());
    // The in-flight worker is parked mid-fetch; let it finish its
    // current page so it observes the cancel and unregisters.
    block.store(false, Ordering::Relaxed);
    wait_for(|| !manager.cancels_lock().contains_key(&7)).await;
}
