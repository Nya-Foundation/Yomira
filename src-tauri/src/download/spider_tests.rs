use super::*;
use std::sync::Mutex;

/// Records download order and serves canned results; failures and the
/// block flag apply to the image download stage.
struct MockSource {
    calls: Mutex<Vec<i32>>,
    fail_at: Option<i32>,
    block: Arc<AtomicBool>,
}

impl MockSource {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
            block: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PageSource for MockSource {
    fn resolve_page<'a>(
        &'a self,
        _gid: u64,
        _gtoken: &'a str,
        index: i32,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedPage, EhError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(ResolvedPage {
                image_url: format!("mock://{index}"),
                referer: format!("mock-ref-{index}"),
                ptoken: Some(format!("ptoken-{index}")),
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
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
            // The mock URL carries the page index.
            let index: i32 = image_url.trim_start_matches("mock://").parse().unwrap();
            self.calls.lock().unwrap().push(index);
            if self.fail_at == Some(index) {
                return Err(EhError::Eh("mock failure".into()));
            }
            Ok(FetchedImage {
                bytes: format!("page {index}").into_bytes(),
                extension: "jpg".into(),
            })
        })
    }
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "exviewer-spider-test-{}-{}-{unique}",
        name,
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn downloads_all_pages_in_order() {
    let root = temp_dir("ordered");
    let source = Arc::new(MockSource::new());
    let den = GalleryDen::open(&root, "gid-title").unwrap();
    let mut info = SpiderInfo::new(7, "tok", 5, 1, 40);
    let progress = std::sync::Mutex::new(Vec::new());

    run_gallery(
        source.clone(),
        &den,
        7,
        "tok",
        5,
        0,
        &mut info,
        Arc::new(AtomicBool::new(false)),
        &|n| progress.lock().unwrap().push(n),
    )
    .await
    .unwrap();

    assert_eq!(den.count_pages(), 5);
    // Resolution order is recorded on completion, so calls stay in
    // gallery order even with the one-page lookahead.
    let calls = source.calls.lock().unwrap().clone();
    assert_eq!(calls, vec![0, 1, 2, 3, 4], "lookahead broke in-order fetch");
    assert_eq!(
        progress.into_inner().unwrap(),
        vec![1, 2, 3, 4, 5],
        "progress reports finished-page counts"
    );
    // Metadata carries the learned ptokens and final start page.
    assert_eq!(info.start_page, 5);
    assert_eq!(info.ptoken_map.len(), 5);
    assert_eq!(
        info.ptoken_map.get(&3).map(String::as_str),
        Some("ptoken-3")
    );
    assert_eq!(std::fs::read(den.find_page(2).unwrap()).unwrap(), b"page 2");

    // A second run resumes from the file count and is a no-op.
    let mut info2 = SpiderInfo::new(7, "tok", 5, 1, 40);
    run_gallery(
        source.clone(),
        &den,
        7,
        "tok",
        5,
        den.count_pages(),
        &mut info2,
        Arc::new(AtomicBool::new(false)),
        &|_| {},
    )
    .await
    .unwrap();
    assert_eq!(source.calls.lock().unwrap().len(), 5);
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn failure_propagates_and_stops_the_run() {
    let root = temp_dir("failed");
    let mut source = MockSource::new();
    source.fail_at = Some(2);
    let source = Arc::new(source);
    let den = GalleryDen::open(&root, "gid-title").unwrap();
    let mut info = SpiderInfo::new(7, "tok", 5, 1, 40);

    let result = run_gallery(
        source,
        &den,
        7,
        "tok",
        5,
        0,
        &mut info,
        Arc::new(AtomicBool::new(false)),
        &|_| {},
    )
    .await;
    assert!(matches!(result, Err(EhError::Eh(_))));

    // Pages before the failure are on disk and counted; the failing and
    // later pages are not.
    assert_eq!(den.count_pages(), 2);
    assert_eq!(info.start_page, 2);
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn cancel_aborts_between_pages() {
    let root = temp_dir("cancel");
    let source = Arc::new(MockSource::new());
    let den = GalleryDen::open(&root, "gid-title").unwrap();
    let mut info = SpiderInfo::new(7, "tok", 50, 1, 40);
    let cancel = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(std::sync::atomic::AtomicI32::new(0));
    {
        let cancel = cancel.clone();
        let seen = seen.clone();
        std::thread::spawn(move || loop {
            if seen.load(Ordering::Relaxed) >= 3 {
                cancel.store(true, Ordering::Relaxed);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        });
    }

    let result = run_gallery(source, &den, 7, "tok", 50, 0, &mut info, cancel, &|n| {
        seen.store(n, Ordering::Relaxed)
    })
    .await;
    assert!(matches!(result, Err(EhError::Cancelled)));
    assert!(seen.load(Ordering::Relaxed) < 50);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn content_type_maps_to_extension() {
    assert_eq!(extension_from_content_type("image/jpeg"), "jpg");
    assert_eq!(
        extension_from_content_type("image/jpeg; charset=binary"),
        "jpg"
    );
    assert_eq!(extension_from_content_type("image/png"), "png");
    assert_eq!(extension_from_content_type("image/gif"), "gif");
    assert_eq!(extension_from_content_type("image/webp"), "webp");
    assert_eq!(
        extension_from_content_type("application/octet-stream"),
        "jpg"
    );
    assert_eq!(extension_from_content_type(""), "jpg");
}
