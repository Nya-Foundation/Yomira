//! Yomira — Tauri shell around the Ehviewer core port.
//!
//! Wiring done here:
//! * `AppState` (client, cookie store, hosts, DB, image cache) built in
//!   `setup()` from the app config/data directories.
//! * Cookie persistence: `cookies.json` restored at startup, re-saved every
//!   60 s and on `RunEvent::ExitRequested`.
//! * App data lives in `app_config_dir()/exviewer.db`, owned by rusqlite.
//!   Frontend access goes through typed commands; migrations run in `Db::open`.

pub mod app_state;
pub mod commands;
pub mod cookies;
pub mod core;
mod db;
pub mod download;
mod http_images;
mod image_cache;
pub mod tags;

use std::sync::{Arc, Mutex, RwLock};

use tauri::Manager;

use app_state::{AppState, NetworkConfig};
use core::{ClientConfig, EhClient, EhCookieStore, InMemoryCookieStore};
use download::{DownloadManager, EhPageSource, PageSource};

fn build_state(app: &tauri::App) -> Result<AppState, Box<dyn std::error::Error>> {
    let config_dir = app.path().app_config_dir()?;
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&config_dir)?;
    std::fs::create_dir_all(&data_dir)?;

    let db = Arc::new(db::Db::open(&config_dir.join("exviewer.db"))?);

    // Network settings seed the client config.
    let network = NetworkConfig::load(&db)?;
    let hosts = Arc::new(RwLock::new(network.parsed_hosts()));
    let client_config: ClientConfig = network.to_client_config()?;

    let cookie_store = Arc::new(InMemoryCookieStore::new());
    // Restore the previous session, if any.
    let cookies_path = data_dir.join("cookies.json");
    let dump = cookies::load_persisted(&cookies_path);
    if !dump.is_empty() {
        cookie_store.restore(dump)?;
    }

    let client = EhClient::new(
        client_config,
        cookie_store.clone() as Arc<dyn EhCookieStore>,
        hosts.clone(),
    )?;

    // Loopback image server: the webview can't load Tauri's `asset://`
    // scheme (WebKit2GTK blocks custom schemes from the dev origin), so
    // cached images are served over 127.0.0.1 instead. Runs on its own
    // thread + runtime, independent of the app's.
    let cache_dir = data_dir.join("image_cache");
    let local_images = http_images::LocalImages::default();
    let image_server_port = http_images::spawn_server(cache_dir.clone(), local_images.clone())?;

    let image_cache = image_cache::ImageCache::open(
        cache_dir,
        network.image_cache_size_mb.saturating_mul(1024 * 1024),
    )?;

    // Tag-translation database: load any usable on-disk copy now; the
    // weekly re-check runs fire-and-forget below.
    let tag_db_dir = data_dir.join("tag_db");
    std::fs::create_dir_all(&tag_db_dir)?;
    let tag_db = Arc::new(tags::TagDb::load(tag_db_dir));

    // Downloads engine. The engine runs its own EhClient instance (the
    // reader's client lives in AppState and cannot be aliased into the
    // worker pool); it shares the app's cookie store and hosts table, so
    // sessions and DNS overrides apply to downloads too. `init` restores
    // the queue from the downloads table and scans the gallery dirs
    // (resume). Network-config changes can be pushed through via
    // `DownloadManager::apply_client_config`.
    let download_source: Arc<dyn PageSource> = Arc::new(EhPageSource::new(EhClient::new(
        network.to_client_config()?,
        cookie_store.clone() as Arc<dyn EhCookieStore>,
        hosts.clone(),
    )?));
    let downloads = DownloadManager::init(
        &config_dir.join("exviewer.db"),
        data_dir.join("downloads"),
        download_source,
    )?;

    Ok(AppState {
        client,
        cookie_store,
        hosts,
        db,
        image_cache,
        image_server_port,
        local_images,
        cookies_path,
        tag_db,
        downloads,
        signin_tx: Mutex::new(None),
        session_check: Mutex::new(None),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build());
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    builder
        .setup(|app| {
            let state = build_state(app)?;
            // Periodic cookie persistence (shutdown persistence hooks into
            // RunEvent::ExitRequested below). Tauri's own runtime is used
            // because `setup` runs outside Tokio's reactor context.
            tauri::async_runtime::spawn(cookies::persist_loop(
                state.cookie_store.clone(),
                state.cookies_path.clone(),
            ));
            // Tag DB weekly re-check: fire-and-forget, never fatal.
            tauri::async_runtime::spawn(tags::maintain(state.tag_db.clone()));
            // Downloads engine: announce state changes to the webview and
            // start the worker pool (tauri runtime, never tokio::spawn here).
            state.downloads.attach_app(app.handle().clone());
            state.downloads.run_workers();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(commands::all())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app.state::<AppState>();
                if let Err(e) = cookies::persist(&state.cookie_store, &state.cookies_path) {
                    eprintln!("[cookies] final persistence failed: {e}");
                }
            }
        });
}
