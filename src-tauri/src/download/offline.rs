//! Disk-only reader access. No client or PageSource is consulted here.
use super::den::{GalleryDen, SpiderInfo, SPIDER_INFO_FILENAME};
use super::manager::{DownloadItem, DownloadManager};
use crate::core::error::EhError;
use crate::http_images::{http_url, LocalImages};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedGallery {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub pages: i32,
    pub available_pages: Vec<i32>,
}

fn saved_gallery(
    manager: &DownloadManager,
    gid: u64,
    token: &str,
) -> Option<(DownloadItem, GalleryDen, i32)> {
    let item = manager.get(gid)?;
    if item.token != token {
        return None;
    }
    let root = manager.root().canonicalize().ok()?;
    let dir = item.dir_path(&root).canonicalize().ok()?;
    if dir.parent()? != root {
        return None;
    }
    let info = SpiderInfo::read_header(&dir.join(SPIDER_INFO_FILENAME)).ok()??;
    if info.gid != gid as i64 || info.token != token || info.pages <= 0 {
        return None;
    }
    Some((item, GalleryDen::at(dir), info.pages))
}

pub fn gallery(manager: &DownloadManager, gid: u64, token: &str) -> Option<DownloadedGallery> {
    let (item, den, pages) = saved_gallery(manager, gid, token)?;
    Some(DownloadedGallery {
        gid,
        token: item.token,
        title: item.title,
        pages,
        available_pages: den.available_pages(pages),
    })
}

pub fn page_url(
    manager: &DownloadManager,
    local: &LocalImages,
    port: u16,
    gid: u64,
    token: &str,
    index: i32,
) -> Result<String, EhError> {
    let (_, den, pages) = saved_gallery(manager, gid, token).ok_or(EhError::PageUnavailable)?;
    if index < 0 || index >= pages {
        return Err(EhError::PageUnavailable);
    }
    let path = den.find_page(index).ok_or(EhError::PageUnavailable)?;
    let name = local
        .register(den.path(), &path)
        .ok_or(EhError::PageUnavailable)?;
    Ok(http_url(port, &name))
}

/// Prefer the first page without scanning every file on every queue event.
pub fn thumbnail_url(
    manager: &DownloadManager,
    local: &LocalImages,
    port: u16,
    gid: u64,
    token: &str,
) -> Option<String> {
    let (_, den, _) = saved_gallery(manager, gid, token)?;
    let path = den.find_page(0)?;
    let name = local.register(den.path(), &path)?;
    Some(http_url(port, &name))
}
