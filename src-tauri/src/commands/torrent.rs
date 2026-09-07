// Portions adapted from EhViewer / Ehviewer_CN_SXJ.
// Copyright 2014-2019 Hippo Seven and upstream contributors.
// Original portions: Apache-2.0; see licenses/Apache-2.0.txt and NOTICE.
// Modified for Yomira: Rust/Tauri port, async I/O and application integration.

//! Torrent + archiver commands — the `src/lib/api.ts` contract.
//!
//! All mechanics ported from `EhEngine.getTorrentList` / `getArchiver` /
//! `downloadArchiver` (see `core/client/mod.rs`): the torrent list is GET
//! `gallerytorrents.php?gid&t`, the archiver dialog page is GET
//! `archiver.php?gid&token`, and the archive download is the form POST +
//! `document.location` + "Click Here To Start Downloading" chain. Files land
//! in the downloads engine's root (`get_download_root`) so everything the
//! user saves sits under the one directory they can relocate.

use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;
use crate::core::error::EhError;
use crate::core::models::TorrentEntry;
use crate::core::EhCookieStore;

/// `types.ts` torrent list entry (camelCase).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentDto {
    pub name: String,
    /// Human readable size exactly as shown on the torrents page.
    pub size: String,
    pub seeds: i32,
    pub peers: i32,
    pub downloads: i32,
    /// The torrent file link (verbatim `href` from the page; may point at
    /// another host such as ehtracker.org).
    pub url: String,
}

impl From<&TorrentEntry> for TorrentDto {
    fn from(entry: &TorrentEntry) -> Self {
        TorrentDto {
            name: entry.name.clone(),
            size: entry.size.clone(),
            seeds: entry.seeds,
            peers: entry.peers,
            downloads: entry.downloads,
            url: entry.url.clone(),
        }
    }
}

/// GET `gallerytorrents.php?gid&t` and return the parsed torrent list
/// (port of `EhEngine.getTorrentList`).
#[tauri::command]
pub async fn get_torrent_list(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
) -> Result<Vec<TorrentDto>, EhError> {
    let entries = state.client.get_torrent_list(gid, &token).await?;
    Ok(entries.iter().map(TorrentDto::from).collect())
}

/// Download one `.torrent` file (`url` is the href returned by
/// `get_torrent_list`, resolved against the site when relative) into
/// `<download_root>/torrents/` and return the saved absolute path.
#[tauri::command]
pub async fn download_torrent(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    url: String,
) -> Result<String, EhError> {
    let site = state.client.site();
    let absolute = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!(
            "{}/{}",
            crate::core::client::url::referer(site),
            url.trim_start_matches('/')
        )
    };
    let file_name = torrent_file_name(&absolute);
    let dir = state.downloads.root().join("torrents");
    std::fs::create_dir_all(&dir)
        .map_err(|e| EhError::Eh(format!("create {}: {e}", dir.display())))?;
    let dest = dir.join(&file_name);
    let referer = crate::core::client::url::gallery_detail_url(site, gid, &token, 0, false);
    state
        .client
        .download_file(&absolute, Some(&referer), &dest)
        .await?;
    Ok(dest.to_string_lossy().into_owned())
}

/// Last URL path segment as a safe file name (always `.torrent`).
fn torrent_file_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let segment = path.rsplit('/').next().unwrap_or("torrent");
    let safe: String = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim();
    if safe.is_empty() || safe == "." || safe == ".." {
        return "gallery.torrent".to_string();
    }
    if safe.ends_with(".torrent") {
        safe.to_string()
    } else {
        format!("{safe}.torrent")
    }
}

/// `types.ts` archiver dialog info: the original ("org") form's POST target
/// and hidden fields, plus the GP/credit cost text.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiverDto {
    /// Form `action` — the `archiver.php?...&or=...` URL the original POSTs.
    pub action: String,
    /// Hidden form fields (`dltype`, `dlcheck`) the original posts back.
    pub params: Vec<(String, String)>,
    /// Cost text for both resolutions ("Original: Free! · Resample: Free!").
    pub cost_text: String,
}

/// GET the `archiver.php` page and extract the download form action, hidden
/// fields and cost text (port of `EhEngine.getArchiver` +
/// `ArchiveParser.parseArchiver`). `download_archive` re-resolves the form
/// for the chosen resolution, so this is informational for the dialog.
#[tauri::command]
pub async fn get_archiver_info(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
) -> Result<ArchiverDto, EhError> {
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    let page = state.client.get_archiver_page(gid, &token).await?;
    let info = crate::core::parser::parse_archiver(&page);
    let form = info
        .original
        .as_ref()
        .or(info.resample.as_ref())
        .ok_or_else(|| EhError::Eh("archiver page has no download form".to_string()))?;
    let params = [
        ("dltype".to_string(), form.dltype.clone()),
        ("dlcheck".to_string(), form.dlcheck.clone()),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .collect();
    let cost_text = [
        info.original
            .as_ref()
            .map(|f| ("Original", f.cost.as_str())),
        info.resample
            .as_ref()
            .map(|f| ("Resample", f.cost.as_str())),
    ]
    .into_iter()
    .flatten()
    .filter(|(_, cost)| !cost.is_empty())
    .map(|(label, cost)| format!("{label}: {cost}"))
    .collect::<Vec<_>>()
    .join(" · ");
    Ok(ArchiverDto {
        action: form.action.clone(),
        params,
        cost_text,
    })
}

/// Download the gallery archive zip for one resolution ("res" or "org"):
/// the full `EhEngine.downloadArchiver` chain (archiver page → form POST →
/// `document.location` → download link), streamed into
/// `<download_root>/archives/<gid>_<resolution>.zip`. Returns the saved
/// absolute path.
#[tauri::command]
pub async fn download_archive(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    resolution: String,
) -> Result<String, EhError> {
    if resolution != "res" && resolution != "org" {
        return Err(EhError::Eh(format!(
            "invalid archive resolution {resolution:?} (expected \"res\" or \"org\")"
        )));
    }
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    let url = state
        .client
        .resolve_archive_download_url(gid, &token, &resolution)
        .await?;
    let dir = state.downloads.root().join("archives");
    std::fs::create_dir_all(&dir)
        .map_err(|e| EhError::Eh(format!("create {}: {e}", dir.display())))?;
    let dest = dir.join(format!("{gid}_{resolution}.zip"));
    let referer =
        crate::core::client::url::gallery_detail_url(state.client.site(), gid, &token, 0, false);
    state
        .client
        .download_file(&url, Some(&referer), &dest)
        .await?;
    Ok(dest.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torrent_file_name_uses_last_segment() {
        assert_eq!(
            torrent_file_name(
                "https://ehtracker.org/get/4165398/d8354b9e0e562bdca5fd3d2ec8e77348c01fc1fd.torrent"
            ),
            "d8354b9e0e562bdca5fd3d2ec8e77348c01fc1fd.torrent"
        );
        // Query strings and unsafe characters are dropped/sanitized.
        assert_eq!(
            torrent_file_name("https://exhentai.org/torrent/1/abc.torrent?p=1"),
            "abc.torrent"
        );
        assert_eq!(
            torrent_file_name("https://x/get/a%2fb.torrent"),
            "a_2fb.torrent"
        );
        // Extension is enforced.
        assert_eq!(torrent_file_name("https://x/get/abc"), "abc.torrent");
        // Degenerate inputs fall back to a stable name.
        assert_eq!(torrent_file_name("https://x/"), "gallery.torrent");
        assert_eq!(torrent_file_name("https://x/.."), "gallery.torrent");
    }
}
