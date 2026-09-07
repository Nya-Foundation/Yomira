// Portions adapted from EhViewer / Ehviewer_CN_SXJ.
// Copyright 2014-2019 Hippo Seven and upstream contributors.
// Original portions: Apache-2.0; see licenses/Apache-2.0.txt and NOTICE.
// Modified for Yomira: Rust/Tauri port, async I/O and application integration.

//! Image search, news, quota, favorite notes and search history commands.

use std::sync::{Arc, LazyLock};

use base64::Engine as _;
use regex::Regex;
use tauri::State;

use crate::app_state::AppState;
use crate::commands::dto::{gallery_info_dto, GalleryListResponseDto};
use crate::commands::gallery::{dto_pages, list_next_page};
use crate::core::error::EhError;
use crate::core::models::Quota;
use crate::core::EhCookieStore;

// ---------------------------------------------------------------------------
// Image lookup (port of EhEngine.imageSearch / image_lookup.php)
// ---------------------------------------------------------------------------

/// Extract the numeric total from a "Found 24,582 results" style string.
fn parse_result_count(result_count: &str) -> i64 {
    let digits: String = result_count
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

/// Upload an image to `upld.e-hentai.org/image_lookup.php` (multipart POST,
/// file field `sfile`, toggles `fs_similar`/`fs_covers`/`fs_exp`,
/// `f_sfile=File Search`) and return the result list parsed like a gallery
/// list. `data_base64` is the file content, base64-encoded.
#[tauri::command]
pub async fn image_lookup(
    state: State<'_, AppState>,
    data_base64: String,
    filename: String,
    similar: bool,
    covers_only: bool,
    expunged: bool,
) -> Result<GalleryListResponseDto, EhError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| EhError::Eh(format!("invalid base64 image data: {e}")))?;
    let (final_url, result) = state
        .client
        .image_lookup(&filename, bytes, similar, covers_only, expunged)
        .await?;
    // NOTE: the EhFilter list filters are not applied here — `is_blocked`
    // lives in commands/gallery.rs. Thread the
    // lookup result through it in a later pass if desired.
    let next_page = list_next_page(result.next_href.as_deref(), result.next_page, &final_url);
    Ok(GalleryListResponseDto {
        galleries: result.galleries.iter().map(gallery_info_dto).collect(),
        next_page,
        pages: dto_pages(result.pages),
        total: parse_result_count(&result.result_count),
    })
}

// ---------------------------------------------------------------------------
// News (port of EhEngine.getEhNews + news.php parsing)
// ---------------------------------------------------------------------------

/// `types.ts` news item wire shape.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItemDto {
    pub title: String,
    pub date: String,
    /// Sanitized plain text (`<br>`/`<p>` → newlines, tags stripped).
    pub body: String,
    pub link: String,
}

/// Fetch `news.php` (page N via `?p=N`, 0 = first) and return the parsed
/// news posts, newest first.
#[tauri::command]
pub async fn get_news(state: State<'_, AppState>, page: i32) -> Result<Vec<NewsItemDto>, EhError> {
    let items = state.client.get_news(page).await?;
    Ok(items
        .iter()
        .map(|item| NewsItemDto {
            title: item.title.clone(),
            date: item.date.clone(),
            body: item.body.clone(),
            link: item.link.clone(),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Quota (port of EhEngine.getHomeDetail + EhHomeParser)
// ---------------------------------------------------------------------------

/// Quota/rate-limit wire shape; `-1` = unknown, absent Optionals = the
/// homepage did not expose them (anonymous or markup drift).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaDto {
    pub images_used: i64,
    pub images_total: i64,
    pub image_reset_seconds: Option<i64>,
    pub searches_used: Option<i64>,
    pub searches_total: Option<i64>,
    pub search_reset_seconds: Option<i64>,
}

fn quota_dto(quota: &Quota) -> QuotaDto {
    QuotaDto {
        images_used: quota.images_used,
        images_total: quota.images_total,
        image_reset_seconds: quota.image_reset_seconds,
        searches_used: quota.searches_used,
        searches_total: quota.searches_total,
        search_reset_seconds: quota.search_reset_seconds,
    }
}

/// Fetch `home.php` and parse the image/search quota numbers. Never fails
/// when the limits table is absent (anonymous session / markup drift).
#[tauri::command]
pub async fn get_quota(state: State<'_, AppState>) -> Result<QuotaDto, EhError> {
    let quota = state.client.get_quota().await?;
    Ok(quota_dto(&quota))
}

// ---------------------------------------------------------------------------
// Favorite note (port of the addfav-popup note flow in EhEngine.addFavorites)
// ---------------------------------------------------------------------------

/// Popup radio `value` attribute, tolerant of attribute order (same patterns
/// as the toggle_favorite popup reader in commands/gallery.rs).
static FAVCAT_RADIO_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<input[^>]*name="favcat"[^>]*>"#).unwrap());
static VALUE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new("value=\"(\\d+)\"").unwrap());

/// GET the addfav popup and read the currently checked category, if any.
async fn current_favorite_slot(
    client: &crate::core::EhClient,
    gid: u64,
    token: &str,
) -> Option<i32> {
    let url = crate::core::client::url::add_favorites_url(client.site(), gid, token);
    let body = client.get_string(&url, Some(&url)).await.ok()?;
    for radio in FAVCAT_RADIO_PATTERN.captures_iter(&body) {
        let tag = &radio[0];
        if !tag.contains("checked") {
            continue;
        }
        if let Some(m) = VALUE_PATTERN.captures(tag) {
            return m[1].parse().ok();
        }
    }
    None
}

/// Update the favorite note of an already-favorited gallery: read the
/// current category from the addfav popup, then POST it back with the new
/// `favnote` (port of `EhEngine.addFavorites` with `submit=Apply Changes`,
/// `update=1` — the same popup-GET-then-POST flow as `toggle_favorite`).
#[tauri::command]
pub async fn modify_favorite_note(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    note: String,
) -> Result<(), EhError> {
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    let slot = current_favorite_slot(&state.client, gid, &token)
        .await
        .ok_or_else(|| EhError::Eh("gallery is not favorited".to_string()))?;
    let url = crate::core::client::url::add_favorites_url(state.client.site(), gid, &token);
    let slot_string = slot.to_string();
    state
        .client
        .post_form(
            &url,
            Some(&url),
            &[
                ("favcat", slot_string.as_str()),
                ("favnote", note.as_str()),
                ("submit", "Apply Changes"),
                ("update", "1"),
            ],
        )
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Search history (suggestions for the search bar)
// ---------------------------------------------------------------------------

/// The 20 most recent search terms, newest first.
#[tauri::command]
pub async fn search_history(state: State<'_, AppState>) -> Result<Vec<String>, EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.search_history()).await?
}

/// Record a search term (trimmed; empty terms ignored, repeats refresh the
/// timestamp so they move to the top).
#[tauri::command]
pub async fn record_search(state: State<'_, AppState>, term: String) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.record_search(&term)).await?
}

#[tauri::command]
pub async fn clear_search_history(state: State<'_, AppState>) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.clear_search_history()).await?
}

#[tauri::command]
pub async fn delete_search_term(state: State<'_, AppState>, term: String) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.delete_search_term(&term)).await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_count_parses_digits() {
        assert_eq!(parse_result_count("Found 24,582 results"), 24582);
        assert_eq!(parse_result_count(""), 0);
    }

    #[test]
    fn quota_dto_maps_model() {
        let dto = quota_dto(&Quota {
            images_used: 10,
            images_total: 5000,
            image_reset_seconds: Some(3600),
            searches_used: Some(2),
            searches_total: None,
            search_reset_seconds: None,
        });
        assert_eq!(dto.images_used, 10);
        assert_eq!(dto.image_reset_seconds, Some(3600));
        assert!(dto.searches_total.is_none());
    }
}
