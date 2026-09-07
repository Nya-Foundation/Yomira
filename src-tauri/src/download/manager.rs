//! `DownloadManager` — the Phase-2 port of `download/DownloadManager.java`
//! without the original's lock soup.
//!
//! * The queue lives in memory (`Vec<DownloadItem>` in queue position order),
//!   mirrored to the `downloads` table on every change. Loading it back on
//!   startup restores the queue.
//! * A fixed pool of worker tasks ([`WORKER_COUNT`], port of the original's
//!   `mDownloadThread` pool default of 3) claims the *first* queued gallery
//!   and runs it through [`spider::run_gallery`]; one gallery is processed by
//!   exactly one worker, pages in order, so at most `WORKER_COUNT` galleries
//!   download concurrently and queue position defines global priority.
//! * Every state change is announced on the Tauri event
//!   [`CHANGED_EVENT`] (`download://changed`) so the frontend refetches.
//!
//! ## State machine (port of `DownloadInfo` NONE/WAIT/DOWNLOAD/FINISH/FAILED)
//!
//! ```text
//!                 enqueue / start
//!   None ───────────────────────────────┐
//!     ▲                                 ▼
//!     │            ┌──────────────▶ Queued ─────────────┐
//!     │            │                    │ claimed by a worker
//!     │            │                    ▼
//!     │   pause    │              Downloading ──pause/cancel──┐
//!     │  (cancel   │                    │ ok                  │
//!     │   flag)    │        error       ▼                     ▼
//!     │            └─────────────   Failed   509        Paused (partial,
//!     │              non-509        (error    (error     resumable from
//!     │                             kept)     kept)      finished pages)
//!     │                              │ start/             │ start
//!     └──────────────────────────────┴──enqueue──────────┘
//!
//!   any state ──all pages found on disk──▶ Done
//! ```
//!
//! `None` marks a listed-but-never-started download; `Done` is also set by
//! the startup scan when the folder already holds every page.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::Notify;

use crate::core::error::EhError;

use super::den::{suitable_dirname, GalleryDen, SpiderInfo, SPIDER_INFO_FILENAME};
use super::spider::{self, PageSource};
use super::store::{DownloadRow, DownloadStore};

/// Default number of galleries downloaded concurrently (the original
/// `DownloadManager` starts 3 `DownloadThread`s).
pub const WORKER_COUNT: usize = 3;

/// Tauri event emitted after every queue/state/progress change; the
/// frontend refetches `list_downloads` when it fires.
pub const CHANGED_EVENT: &str = "download://changed";

/// settings_kv key holding the download root override.
const ROOT_SETTING: &str = "download_root";

/// Lifecycle of one download (port of `DownloadInfo` states). The DB mapping
/// matches the original greenDAO enum: NONE=0, WAIT=1, DOWNLOAD=2, PAUSE=3,
/// FINISH=4, FAILED=5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    /// Listed but never started (DownloadInfo.STATE_NONE).
    None,
    /// Waiting for a worker (STATE_WAIT).
    Queued,
    /// Claimed by a worker, pages coming in (STATE_DOWNLOAD).
    Downloading,
    /// Stopped by the user or by the app shutting down mid-download;
    /// resumable from `finished_pages` (STATE_PAUSE).
    Paused,
    /// Every page is on disk (STATE_FINISH).
    Done,
    /// Stopped by an error; `error` holds the message (STATE_FAILED).
    Failed,
}

impl DownloadState {
    pub fn to_db(self) -> i32 {
        match self {
            DownloadState::None => 0,
            DownloadState::Queued => 1,
            DownloadState::Downloading => 2,
            DownloadState::Paused => 3,
            DownloadState::Done => 4,
            DownloadState::Failed => 5,
        }
    }

    pub fn from_db(value: i32) -> Self {
        match value {
            1 => DownloadState::Queued,
            2 => DownloadState::Downloading,
            3 => DownloadState::Paused,
            4 => DownloadState::Done,
            5 => DownloadState::Failed,
            _ => DownloadState::None,
        }
    }
}

/// One entry of the download queue (runtime form of a `downloads` row; the
/// `downloads` table has no finished/error columns — progress lives on disk
/// in the `.ehviewer` file + page files and is recomputed by scanning).
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadItem {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub title_jpn: Option<String>,
    pub thumb: String,
    pub category: i32,
    /// Total pages of the gallery.
    pub pages: i32,
    pub state: DownloadState,
    pub label: Option<String>,
    /// Folder name under the download root.
    pub dirname: String,
    /// Enqueue time (unix epoch millis); doubles as persistent queue order.
    pub time: i64,
    /// Pages present on disk / confirmed downloaded (runtime only).
    pub finished_pages: i32,
    /// Last error message for `Failed` / `Paused(509)` states (runtime only).
    pub error: Option<String>,
}

impl DownloadItem {
    /// Absolute path of the gallery folder under `root`.
    pub fn dir_path(&self, root: &Path) -> PathBuf {
        root.join(&self.dirname)
    }

    fn from_row(row: &DownloadRow) -> DownloadItem {
        let title_jpn = row.title_jpn.clone().filter(|t| !t.trim().is_empty());
        let dirname = row
            .dirname
            .clone()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| suitable_dirname(row.gid as u64, &row.title, title_jpn.as_deref()));
        DownloadItem {
            gid: row.gid as u64,
            token: row.token.clone(),
            title: row.title.clone(),
            title_jpn,
            thumb: row.thumb.clone(),
            category: row.category,
            // The downloads table (like the original greenDAO schema) has no
            // pages column; the total comes from the `.ehviewer` header
            // written at enqueue time (see `init`'s restore scan).
            pages: 0,
            state: DownloadState::from_db(row.state),
            label: row.label.clone(),
            dirname,
            time: row.time,
            finished_pages: 0,
            error: None,
        }
    }

    fn to_row(&self) -> DownloadRow {
        DownloadRow {
            gid: self.gid as i64,
            token: self.token.clone(),
            title: self.title.clone(),
            title_jpn: self.title_jpn.clone(),
            thumb: self.thumb.clone(),
            category: self.category,
            state: self.state.to_db(),
            time: self.time,
            label: self.label.clone(),
            dirname: Some(self.dirname.clone()),
        }
    }
}

/// Gallery facts needed to enqueue a download. Built by the
/// `add_download` command from the fetched `GalleryDetail`.
#[derive(Debug, Clone)]
pub struct GalleryDownloadInfo {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub title_jpn: Option<String>,
    pub thumb: String,
    pub category: i32,
    pub pages: i32,
    pub preview_pages: i32,
    /// (page index, ptoken) pairs parsed from the detail preview sets.
    pub preview_ptokens: Vec<(i32, String)>,
}

struct Inner {
    store: DownloadStore,
    root: RwLock<PathBuf>,
    /// The queue; position in the Vec is the queue position.
    items: Mutex<Vec<DownloadItem>>,
    /// Wakes worker tasks when new work is claimable.
    notify: Notify,
    /// Cancel flags for in-flight galleries (gid → flag).
    cancels: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    source: Arc<dyn PageSource>,
    /// Set at startup so state changes can reach the webview.
    app: RwLock<Option<tauri::AppHandle>>,
}

impl Inner {
    /// Announce a queue/state/progress change to the webview.
    fn emit_changed(&self) {
        let app = self.app.read().map(|slot| slot.clone()).unwrap_or(None);
        if let Some(app) = app {
            let _ = app.emit(CHANGED_EVENT, ());
        }
    }
}

/// Shared handle to the downloads engine (cheap to clone, safe to store in
/// `AppState`).
#[derive(Clone)]
pub struct DownloadManager {
    inner: Arc<Inner>,
}

impl DownloadManager {
    /// Load the queue from the DB and reconcile it with the filesystem
    /// (resume scan): finished pages are counted per gallery dir and
    /// galleries whose folder is complete become `Done`; downloads that were
    /// in flight when the app last exited become `Paused` (resume needs an
    /// explicit `start_download`).
    pub fn init(
        db_path: &Path,
        default_root: PathBuf,
        source: Arc<dyn PageSource>,
    ) -> Result<Self, EhError> {
        let store = DownloadStore::open(db_path)?;
        let root = store
            .get_setting(ROOT_SETTING)?
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(default_root);
        std::fs::create_dir_all(&root).map_err(|e| {
            EhError::Eh(format!(
                "cannot create download root {}: {e}",
                root.display()
            ))
        })?;

        let rows = store.list()?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let mut item = DownloadItem::from_row(&row);
            let dir = item.dir_path(&root);
            if dir.is_dir() {
                // Total page count: the downloads table has no pages column
                // (same as the original schema); it lives in the `.ehviewer`
                // header written at enqueue time.
                let header = SpiderInfo::read_header(&dir.join(SPIDER_INFO_FILENAME))
                    .unwrap_or(None)
                    .filter(|info| info.gid == row.gid && info.token == item.token);
                if let Some(info) = header {
                    item.pages = info.pages;
                }
                let finished = GalleryDen::at(dir).available_pages(item.pages).len() as i32;
                item.finished_pages = finished.min(item.pages.max(0));
                if item.pages > 0 && finished >= item.pages {
                    item.state = DownloadState::Done;
                    item.error = None;
                } else if matches!(
                    item.state,
                    DownloadState::Queued | DownloadState::Downloading | DownloadState::Done
                ) {
                    item.state = DownloadState::Paused;
                }
            } else if matches!(
                item.state,
                DownloadState::Queued | DownloadState::Downloading | DownloadState::Done
            ) {
                item.state = DownloadState::Paused;
            }
            items.push(item);
        }

        Ok(Self {
            inner: Arc::new(Inner {
                store,
                root: RwLock::new(root),
                items: Mutex::new(items),
                notify: Notify::new(),
                cancels: Mutex::new(HashMap::new()),
                source,
                app: RwLock::new(None),
            }),
        })
    }

    /// Attach the app handle used for `download://changed` events.
    pub fn attach_app(&self, app: tauri::AppHandle) {
        if let Ok(mut slot) = self.inner.app.write() {
            *slot = Some(app);
        }
    }

    /// Spawn the fixed worker pool on Tauri's runtime (never `tokio::spawn`
    /// from sync contexts — setup() runs outside Tokio's reactor).
    pub fn run_workers(&self) {
        for _ in 0..WORKER_COUNT {
            let manager = self.clone();
            tauri::async_runtime::spawn(async move { manager.worker_loop().await });
        }
    }

    /// Apply network-config changes to the download engine's client.
    /// (The `set_network_config` command should call this; see the module
    /// docs in `lib.rs`.)
    pub fn apply_client_config(&self, config: &crate::core::ClientConfig) -> Result<(), EhError> {
        self.inner.source.update_config(config)
    }

    // ------------------------------------------------------------------
    // Queue queries
    // ------------------------------------------------------------------

    pub fn list(&self) -> Vec<DownloadItem> {
        self.items_lock().clone()
    }

    pub fn get(&self, gid: u64) -> Option<DownloadItem> {
        self.items_lock().iter().find(|i| i.gid == gid).cloned()
    }

    pub fn root(&self) -> PathBuf {
        self.inner
            .root
            .read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Change (and persist) the download root, creating it when missing.
    /// Existing gallery dirs are not migrated.
    pub fn set_root(&self, path: PathBuf) -> Result<(), EhError> {
        std::fs::create_dir_all(&path)
            .map_err(|e| EhError::Eh(format!("invalid download root {}: {e}", path.display())))?;
        *self.inner.root.write().map_err(|_| EhError::Cancelled)? = path.clone();
        self.inner
            .store
            .set_setting(ROOT_SETTING, &path.to_string_lossy())?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Queue mutations
    // ------------------------------------------------------------------

    /// Add (or refresh + requeue) a download and seed its on-disk storage
    /// with the `.ehviewer` metadata (ptokens included).
    pub fn enqueue(
        &self,
        info: GalleryDownloadInfo,
        label: Option<String>,
    ) -> Result<DownloadItem, EhError> {
        let root = self.root();
        let now = chrono::Utc::now().timestamp_millis();
        let mut items = self.items_lock();

        if let Some(existing) = items.iter_mut().find(|i| i.gid == info.gid) {
            existing.token = info.token.clone();
            existing.title = info.title.clone();
            existing.title_jpn = info.title_jpn.clone();
            existing.thumb = info.thumb.clone();
            existing.category = info.category;
            existing.pages = info.pages;
            existing.time = now;
            if label.is_some() {
                existing.label = label;
            }
            existing.error = None;
            if !matches!(
                existing.state,
                DownloadState::Done | DownloadState::Downloading | DownloadState::Queued
            ) {
                existing.state = DownloadState::Queued;
            }
            let item = existing.clone();
            // Re-enqueue moves the gallery to the back of the queue.
            items.retain(|i| i.gid != item.gid);
            items.push(item.clone());
            self.inner.store.upsert(&item.to_row())?;
            drop(items);
            self.inner.notify.notify_one();
            self.emit_changed();
            return Ok(item);
        }

        let dirname = suitable_dirname(info.gid, &info.title, info.title_jpn.as_deref());
        let den = GalleryDen::open(&root, &dirname)
            .map_err(|e| EhError::Eh(format!("cannot create download dir: {e}")))?;
        let mut spider_info = SpiderInfo::new(
            info.gid as i64,
            info.token.clone(),
            info.pages,
            info.preview_pages,
            info.preview_ptokens.len() as i32,
        );
        for (index, ptoken) in &info.preview_ptokens {
            spider_info.ptoken_map.insert(*index, ptoken.clone());
        }
        // The offline reader needs this header after restart. Surface storage
        // failures before accepting a new download into the queue.
        spider_info.write_to_dir(den.path())?;

        let finished = den.available_pages(info.pages).len() as i32;
        let state = if info.pages > 0 && finished >= info.pages {
            DownloadState::Done
        } else {
            DownloadState::Queued
        };
        let item = DownloadItem {
            gid: info.gid,
            token: info.token,
            title: info.title,
            title_jpn: info.title_jpn,
            thumb: info.thumb,
            category: info.category,
            pages: info.pages,
            state,
            label,
            dirname,
            time: now,
            finished_pages: finished,
            error: None,
        };
        self.inner.store.upsert(&item.to_row())?;
        items.push(item.clone());
        drop(items);
        self.inner.notify.notify_one();
        self.emit_changed();
        Ok(item)
    }

    /// Remove a download from the queue; optionally delete its folder.
    pub fn remove(&self, gid: u64, delete_files: bool) -> Result<(), EhError> {
        if let Some(cancel) = self.cancels_lock().get(&gid) {
            cancel.store(true, Ordering::Relaxed);
        }
        let mut items = self.items_lock();
        let Some(pos) = items.iter().position(|i| i.gid == gid) else {
            return Ok(()); // unknown gid is a no-op
        };
        let item = items.remove(pos);
        self.inner.store.delete(gid as i64)?;
        drop(items);
        if delete_files {
            let den = GalleryDen::at(item.dir_path(&self.root()));
            den.remove_dir()
                .map_err(|e| EhError::Eh(format!("cannot delete download dir: {e}")))?;
        }
        self.emit_changed();
        Ok(())
    }

    /// (Re)start a download: `None`/`Paused`/`Failed` → `Queued`.
    pub fn start(&self, gid: u64) -> Result<(), EhError> {
        let mut items = self.items_lock();
        let Some(item) = items.iter_mut().find(|i| i.gid == gid) else {
            return Err(EhError::Eh(format!("no such download: {gid}")));
        };
        match item.state {
            DownloadState::Done | DownloadState::Queued | DownloadState::Downloading => {}
            DownloadState::None | DownloadState::Paused | DownloadState::Failed => {
                item.state = DownloadState::Queued;
                item.error = None;
                self.inner
                    .store
                    .update_state(gid as i64, item.state.to_db())?;
            }
        }
        drop(items);
        self.inner.notify.notify_one();
        self.emit_changed();
        Ok(())
    }

    /// Pause a download: `Queued` → `Paused` immediately; `Downloading` gets
    /// its cancel flag set and turns `Paused` between pages.
    pub fn pause(&self, gid: u64) -> Result<(), EhError> {
        let mut items = self.items_lock();
        let Some(item) = items.iter_mut().find(|i| i.gid == gid) else {
            return Err(EhError::Eh(format!("no such download: {gid}")));
        };
        match item.state {
            DownloadState::Queued => {
                item.state = DownloadState::Paused;
                self.inner
                    .store
                    .update_state(gid as i64, item.state.to_db())?;
            }
            DownloadState::Downloading => {
                item.state = DownloadState::Paused;
                if let Some(cancel) = self.cancels_lock().get(&gid) {
                    cancel.store(true, Ordering::Relaxed);
                }
                self.inner
                    .store
                    .update_state(gid as i64, item.state.to_db())?;
            }
            _ => {}
        }
        drop(items);
        self.emit_changed();
        Ok(())
    }

    /// Attach a label to a download (port of `DownloadManager.setLabel`;
    /// the label list is managed through the label commands).
    pub fn set_label(&self, gid: u64, label: Option<String>) -> Result<(), EhError> {
        let mut items = self.items_lock();
        let Some(item) = items.iter_mut().find(|i| i.gid == gid) else {
            return Err(EhError::Eh(format!("no such download: {gid}")));
        };
        item.label = label.clone();
        self.inner
            .store
            .update_label(gid as i64, label.as_deref())?;
        drop(items);
        self.emit_changed();
        Ok(())
    }

    // ------------------------------------------------------------------
    // Labels
    // ------------------------------------------------------------------

    pub fn labels(&self) -> Result<Vec<String>, EhError> {
        self.inner.store.labels()
    }

    pub fn add_label(&self, name: &str) -> Result<(), EhError> {
        if name.trim().is_empty() {
            return Err(EhError::Eh("label name is empty".into()));
        }
        self.inner.store.add_label(name)?;
        self.emit_changed();
        Ok(())
    }

    /// Delete a label and detach it from every download carrying it.
    pub fn delete_label(&self, name: &str) -> Result<(), EhError> {
        self.inner.store.delete_label(name)?;
        let mut items = self.items_lock();
        let detached: Vec<u64> = items
            .iter()
            .filter(|i| i.label.as_deref() == Some(name))
            .map(|i| i.gid)
            .collect();
        for item in items.iter_mut() {
            if item.label.as_deref() == Some(name) {
                item.label = None;
            }
        }
        drop(items);
        for gid in detached {
            let _ = self.inner.store.update_label(gid as i64, None);
        }
        self.emit_changed();
        Ok(())
    }

    // ------------------------------------------------------------------
    // Internals
    // ------------------------------------------------------------------

    fn items_lock(&self) -> MutexGuard<'_, Vec<DownloadItem>> {
        self.inner.items.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn cancels_lock(&self) -> MutexGuard<'_, HashMap<u64, Arc<AtomicBool>>> {
        self.inner.cancels.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit_changed(&self) {
        self.inner.emit_changed();
    }

    /// Claim the first queued gallery, marking it `Downloading`.
    fn claim(&self) -> Option<(DownloadItem, Arc<AtomicBool>)> {
        let mut items = self.items_lock();
        let idx = items
            .iter()
            .position(|i| i.state == DownloadState::Queued)?;
        items[idx].state = DownloadState::Downloading;
        let item = items[idx].clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancels_lock().insert(item.gid, cancel.clone());
        let _ = self
            .inner
            .store
            .update_state(item.gid as i64, DownloadState::Downloading.to_db());
        drop(items);
        self.emit_changed();
        Some((item, cancel))
    }

    async fn worker_loop(&self) {
        loop {
            // Pin the notification before claiming so an enqueue that lands
            // between claim() and .await is not lost.
            let notified = self.inner.notify.notified();
            match self.claim() {
                Some((item, cancel)) => self.process(item, cancel).await,
                None => {
                    notified.await;
                }
            }
        }
    }

    /// Run one gallery to a terminal state and reconcile the queue.
    async fn process(&self, item: DownloadItem, cancel: Arc<AtomicBool>) {
        let gid = item.gid;
        let result = self.download_item(&item, cancel.clone()).await;
        self.cancels_lock().remove(&gid);

        let (state, error) = match &result {
            Ok(()) => (DownloadState::Done, None),
            Err(EhError::Cancelled) => (DownloadState::Paused, None),
            // 509: pause instead of fail so the user can resume when the
            // image limit resets (port of SpiderQueen's 509 stop).
            Err(e @ EhError::Image509) => (DownloadState::Paused, Some(e.message())),
            Err(e) => (DownloadState::Failed, Some(e.message())),
        };

        let mut items = self.items_lock();
        let Some(slot) = items.iter_mut().find(|i| i.gid == gid) else {
            return; // removed while downloading
        };
        // A concurrent pause already moved it out of Downloading; keep that.
        if slot.state != DownloadState::Downloading {
            return;
        }
        slot.state = state;
        slot.error = error;
        if state == DownloadState::Done {
            slot.finished_pages = slot.pages;
        }
        drop(items);
        let _ = self.inner.store.update_state(gid as i64, state.to_db());
        self.emit_changed();
    }

    async fn download_item(
        &self,
        item: &DownloadItem,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), EhError> {
        let root = self.root();
        let den = GalleryDen::open(&root, &item.dirname)
            .map_err(|e| EhError::Eh(format!("cannot create download dir: {e}")))?;

        let info_path = den.path().join(SPIDER_INFO_FILENAME);
        let mut spider_info = SpiderInfo::read(&info_path)
            .unwrap_or(None)
            .filter(|info| info.gid == item.gid as i64)
            .unwrap_or_else(|| {
                SpiderInfo::new(item.gid as i64, item.token.clone(), item.pages, 0, 0)
            });

        // Resume from what is already on disk.
        let available = den.available_pages(item.pages);
        let start = (0..item.pages)
            .find(|index| available.binary_search(index).is_err())
            .unwrap_or(item.pages);
        if item.pages > 0 && start >= item.pages {
            let mut items = self.items_lock();
            if let Some(it) = items.iter_mut().find(|i| i.gid == item.gid) {
                it.finished_pages = item.pages;
            }
            return Ok(());
        }

        let inner = self.inner.clone();
        let gid = item.gid;
        let on_progress = move |finished: i32| {
            let mut items = inner.items.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(it) = items.iter_mut().find(|i| i.gid == gid) {
                it.finished_pages = finished;
            }
            drop(items);
            inner.emit_changed();
        };

        spider::run_gallery(
            self.inner.source.clone(),
            &den,
            item.gid,
            &item.token,
            item.pages,
            start,
            &mut spider_info,
            cancel,
            &on_progress,
        )
        .await
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
