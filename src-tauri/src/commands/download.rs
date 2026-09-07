//! Download-facing commands — the `src/lib/api.ts` contract for the
//! downloads engine. Every mutation triggers a `download://changed` event
//! (emitted by the manager) so the frontend refetches `list_downloads`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::error::EhError;
use crate::download::manager::{DownloadItem, DownloadManager, DownloadState};
use crate::download::GalleryDownloadInfo;

/// Wire form of [`DownloadItem`] (camelCase, matching `api.ts` conventions).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItemDto {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub thumb: String,
    pub local_thumb: Option<String>,
    pub category: String,
    pub pages: i32,
    pub state: DownloadState,
    pub label: Option<String>,
    pub dirname: String,
    pub finished_pages: i32,
    pub dir_path: String,
    pub error: Option<String>,
    pub time: i64,
}

fn item_dto(item: &DownloadItem, manager: &DownloadManager) -> DownloadItemDto {
    DownloadItemDto {
        gid: item.gid,
        token: item.token.clone(),
        title: item.title.clone(),
        thumb: item.thumb.clone(),
        local_thumb: None,
        category: crate::core::models::category::category_name(item.category).to_string(),
        pages: item.pages,
        state: item.state,
        label: item.label.clone(),
        dirname: item.dirname.clone(),
        finished_pages: item.finished_pages,
        dir_path: item
            .dir_path(&manager.root())
            .to_string_lossy()
            .into_owned(),
        error: item.error.clone(),
        time: item.time,
    }
}

#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadItemDto>, EhError> {
    let manager = state.downloads.clone();
    let local = state.local_images.clone();
    let port = state.image_server_port;
    tokio::task::spawn_blocking(move || {
        manager
            .list()
            .iter()
            .map(|item| {
                let mut dto = item_dto(item, &manager);
                dto.local_thumb = crate::download::offline::thumbnail_url(
                    &manager,
                    &local,
                    port,
                    item.gid,
                    &item.token,
                );
                dto
            })
            .collect()
    })
    .await
    .map_err(EhError::from)
}

/// Fetch the gallery detail (title/thumb/pages/previews) and enqueue the
/// download, optionally under a label.
#[tauri::command]
pub async fn add_download(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    label: Option<String>,
) -> Result<DownloadItemDto, EhError> {
    let detail = state.client.get_gallery_detail(gid, &token).await?;
    let info = GalleryDownloadInfo {
        gid: detail.info.gid,
        token: detail.info.token.clone(),
        title: detail.info.title.clone(),
        title_jpn: detail.info.title_jpn.clone(),
        thumb: detail.info.thumb.clone(),
        category: detail.info.category,
        pages: detail.info.pages,
        preview_pages: detail.preview_pages,
        preview_ptokens: detail
            .preview_sets
            .iter()
            .flat_map(|set| set.rows.iter().map(|row| (row.position, row.token.clone())))
            .collect(),
    };
    let manager = state.downloads.clone();
    let item = tokio::task::spawn_blocking(move || manager.enqueue(info, label))
        .await
        .map_err(|e| EhError::Db(format!("add_download failed: {e}")))??;
    let manager = state.downloads.clone();
    Ok(item_dto(&item, &manager))
}

#[tauri::command]
pub async fn remove_download(
    state: State<'_, AppState>,
    gid: u64,
    delete_files: bool,
) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.remove(gid, delete_files))
        .await
        .map_err(|e| EhError::Db(format!("remove_download failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn start_download(state: State<'_, AppState>, gid: u64) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.start(gid))
        .await
        .map_err(|e| EhError::Db(format!("start_download failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn pause_download(state: State<'_, AppState>, gid: u64) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.pause(gid))
        .await
        .map_err(|e| EhError::Db(format!("pause_download failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn download_labels(state: State<'_, AppState>) -> Result<Vec<String>, EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.labels())
        .await
        .map_err(|e| EhError::Db(format!("download_labels failed: {e}")))?
}

#[tauri::command]
pub async fn add_download_label(state: State<'_, AppState>, name: String) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.add_label(&name))
        .await
        .map_err(|e| EhError::Db(format!("add_download_label failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn delete_download_label(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.delete_label(&name))
        .await
        .map_err(|e| EhError::Db(format!("delete_download_label failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn set_download_label(
    state: State<'_, AppState>,
    gid: u64,
    label: Option<String>,
) -> Result<(), EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.set_label(gid, label))
        .await
        .map_err(|e| EhError::Db(format!("set_download_label failed: {e}")))??;
    Ok(())
}

#[tauri::command]
pub async fn get_download_root(state: State<'_, AppState>) -> Result<String, EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.root().to_string_lossy().into_owned())
        .await
        .map_err(|e| EhError::Db(format!("get_download_root failed: {e}")))
}

/// Point the download root at `path`: must be non-empty and creatable.
/// Existing downloads are not moved.
#[tauri::command]
pub async fn set_download_root(state: State<'_, AppState>, path: String) -> Result<(), EhError> {
    if path.trim().is_empty() {
        return Err(EhError::Eh("download root must not be empty".into()));
    }
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || manager.set_root(std::path::PathBuf::from(path)))
        .await
        .map_err(|e| EhError::Db(format!("set_download_root failed: {e}")))??;
    Ok(())
}

/// Read the persisted gallery header and scan actual files, without a request.
#[tauri::command]
pub async fn get_downloaded_gallery(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
) -> Result<Option<crate::download::offline::DownloadedGallery>, EhError> {
    let manager = state.downloads.clone();
    tokio::task::spawn_blocking(move || crate::download::offline::gallery(&manager, gid, &token))
        .await
        .map_err(EhError::from)
}

#[tauri::command]
pub async fn get_downloaded_page(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    index: i32,
) -> Result<super::dto::PageDataDto, EhError> {
    let manager = state.downloads.clone();
    let local = state.local_images.clone();
    let port = state.image_server_port;
    let url = tokio::task::spawn_blocking(move || {
        crate::download::offline::page_url(&manager, &local, port, gid, &token, index)
    })
    .await??;
    Ok(super::dto::PageDataDto {
        index,
        url,
        width: 0,
        height: 0,
    })
}
