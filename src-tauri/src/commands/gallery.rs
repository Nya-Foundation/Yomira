//! Gallery-facing commands: lists, detail, previews, reader pages, comments,
//! favorites, rating — the `src/lib/api.ts` contract.

use std::sync::{Arc, LazyLock};

use regex::Regex;
use tauri::State;

use crate::app_state::AppState;
use crate::commands::dto::{
    comment_dto, favorite_item_dto, gallery_detail_dto, gallery_info_dto, CommentDto,
    FavoriteItemDto, GalleryDetailDto, GalleryListResponseDto, ListPage, ListParamsDto,
    PageDataDto, PreviewPageDto, PreviewSetDto,
};
use crate::core::error::EhError;
use crate::core::models::{category, AdvancedSearch, GalleryInfo, ListMode, ListUrlBuilder};
use crate::core::parser;
use crate::core::EhCookieStore;
use crate::db::FilterEntry;

/// Toplist timespan → `tl` argument (port of `EhTopListAdapter.urlFollow`):
/// yesterday=tl=15, past month=tl=13, past year=tl=12, all time=tl=11.
fn toplist_arg(timespan: Option<&str>) -> &'static str {
    match timespan.unwrap_or("month") {
        "yesterday" => "tl=15",
        "year" => "tl=12",
        "all" => "tl=11",
        _ => "tl=13",
    }
}

/// Map the frontend's [`ListParamsDto`] onto the core [`ListUrlBuilder`].
pub fn list_params_to_builder(
    params: &ListParamsDto,
    mode: &str,
    timespan: Option<&str>,
) -> Result<ListUrlBuilder, EhError> {
    if params
        .min_rating
        .is_some_and(|rating| !rating.is_finite() || !(2.0..=5.0).contains(&rating))
    {
        return Err(EhError::Eh(
            "minimum rating must be between 2 and 5 stars".into(),
        ));
    }
    let mut builder = ListUrlBuilder::new();
    builder.mode = match mode {
        "home" | "normal" | "" => ListMode::Normal,
        "popular" => ListMode::Popular,
        "watched" => ListMode::Watched,
        "toplist" => ListMode::TopList,
        other => return Err(EhError::Eh(format!("unknown list mode: {other}"))),
    };
    builder.page = params.page.max(0);
    if builder.mode == ListMode::TopList {
        builder.toplist_arg = toplist_arg(timespan).to_string();
    }

    // Categories: the frontend sends display names ("Doujinshi", ...); the
    // builder wants the OR-ed bitflags (NONE = show everything).
    if let Some(cats) = &params.categories {
        let mut bits = 0;
        for name in cats {
            let bit = category::get_category(name);
            if bit == category::UNKNOWN {
                return Err(EhError::Eh(format!("unknown category: {name}")));
            }
            bits |= bit;
        }
        builder.category = if bits == 0 { category::NONE } else { bits };
    }

    let keyword = params.search.trim();
    if !keyword.is_empty() {
        builder.keyword = Some(keyword.to_string());
    }

    // Advanced search is enabled when ANY knob differs from its default —
    // rating/page filters, the title/tag toggles (f_sname/f_stags), or any
    // of the extra flags (f_sdesc/f_storr/f_sdt1/f_sdt2/f_sh).
    let advanced_enabled = params.min_rating.is_some()
        || params.min_pages.is_some()
        || params.max_pages.is_some()
        || !params.enable_uploader
        || !params.enable_tags
        || params.search_desc
        || params.search_torrent
        || params.search_lowpower_tags
        || params.search_downvoted_tags
        || params.search_expunged;
    if advanced_enabled {
        builder.advanced = Some(AdvancedSearch {
            search_name: params.enable_uploader,
            search_tags: params.enable_tags,
            search_descr: params.search_desc,
            search_torrent: params.search_torrent,
            search_lowpower_tags: params.search_lowpower_tags,
            search_downvoted_tags: params.search_downvoted_tags,
            search_expunged: params.search_expunged,
            // EH accepts whole-star thresholds only. Keep the requested
            // fraction for the local pass below; never round it upwards.
            min_rating: params.min_rating.map_or(-1, |rating| rating.floor() as i32),
            page_from: params.min_pages.unwrap_or(-1),
            page_to: params.max_pages.unwrap_or(-1),
            ..AdvancedSearch::default()
        });
    }
    Ok(builder)
}

/// Port of `EhFilter` — drop galleries matching any filter/blacklist entry.
/// Title matches are case-insensitive substring (checked against both title
/// and titleJpn, like the original), uploader matches are case-insensitive
/// equality, tag entries match either the full `namespace:name` form or a
/// bare tag name.
fn is_blocked(info: &GalleryInfo, filters: &[FilterEntry]) -> bool {
    let title = info.title.to_lowercase();
    let title_jpn = info.title_jpn.clone().unwrap_or_default().to_lowercase();
    let tags: Vec<String> = info
        .tags
        .iter()
        .flat_map(|g| {
            g.tags
                .iter()
                .map(|t| format!("{}:{}", g.namespace, t).to_lowercase())
                .collect::<Vec<_>>()
                .into_iter()
                .chain(g.tags.iter().map(|t| t.to_lowercase()))
        })
        .collect();

    filters.iter().any(|f| {
        let text = f.text.trim().to_lowercase();
        if text.is_empty() {
            return false;
        }
        match f.mode.as_str() {
            "title" => title.contains(&text) || title_jpn.contains(&text),
            "uploader" => info
                .uploader
                .as_deref()
                .is_some_and(|u| u.eq_ignore_ascii_case(f.text.trim())),
            "tag" | "tag_namespace" => {
                if text.contains(':') {
                    tags.iter().any(|full| full == &text)
                } else {
                    tags.iter()
                        .any(|t| t == &text || t.rsplit(':').next() == Some(text.as_str()))
                }
            }
            _ => false,
        }
    })
}

fn apply_filters(galleries: &mut Vec<GalleryInfo>, filters: &[FilterEntry]) {
    if filters.is_empty() {
        return;
    }
    galleries.retain(|gi| !is_blocked(gi, filters));
}

fn apply_min_rating(galleries: &mut Vec<GalleryInfo>, min_rating: Option<f32>) {
    if let Some(minimum) = min_rating {
        galleries.retain(|gallery| gallery.rating >= minimum);
    }
}

/// Extract the numeric total from a "Found 24,582 results" style string.
fn parse_result_count(result_count: &str) -> i64 {
    let digits: String = result_count
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

/// `next_page` → DTO `nextPage`: the ptt pager fills only the number (no
/// href), so a positive number is the only signal; 0/-1 mean no next page.
pub(crate) fn dto_next_page(next_page: i32) -> Option<i32> {
    (next_page > 0).then_some(next_page)
}

/// Continuations are constrained to this listing's origin and path. Preserve
/// the whole query: `next` is a gallery cursor, not an ordinal page number.
fn list_request_url(base: &str, cursor: Option<&str>) -> Result<String, EhError> {
    let Some(cursor) = cursor else {
        return Ok(base.to_string());
    };
    let base = tauri::Url::parse(base).map_err(|e| EhError::Eh(e.to_string()))?;
    let mut next = base.join(cursor).map_err(|e| EhError::Eh(e.to_string()))?;
    if next.origin() != base.origin()
        || next.path() != base.path()
        || !next.username().is_empty()
        || next.password().is_some()
    {
        return Err(EhError::Eh("invalid listing continuation".into()));
    }
    next.set_fragment(None);
    Ok(next.to_string())
}

pub(crate) fn list_next_page(
    next_href: Option<&str>,
    next_page: i32,
    current: &str,
) -> Option<ListPage> {
    if let Some(href) = next_href.filter(|s| !s.trim().is_empty()) {
        return list_request_url(current, Some(href))
            .ok()
            .filter(|next| next != current)
            .map(ListPage::Url);
    }
    dto_next_page(next_page).map(ListPage::Number)
}

/// Parser `pages` (total page count) → DTO `pages`: 0 when unknown (-1) or
/// bogus (the i32::MAX error fallback).
pub(crate) fn dto_pages(pages: i32) -> u32 {
    match pages {
        p if p > 0 && p != i32::MAX => p as u32,
        _ => 0,
    }
}

#[tauri::command]
pub async fn get_gallery_list(
    state: State<'_, AppState>,
    params: ListParamsDto,
    mode: String,
    timespan: Option<String>,
) -> Result<GalleryListResponseDto, EhError> {
    let builder = list_params_to_builder(&params, &mode, timespan.as_deref())?;
    if builder.mode == ListMode::Watched && !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    let url = list_request_url(
        &builder.build(state.client.site()),
        params.cursor.as_deref(),
    )?;
    eprintln!("[yomira] get_gallery_list mode={mode} url={url}");
    let mut result = match state.client.get_gallery_list(&url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[yomira] get_gallery_list FAILED: kind={} msg={}",
                e.kind(),
                e
            );
            return Err(e);
        }
    };

    let filters = tokio::task::spawn_blocking({
        let db = Arc::clone(&state.db);
        move || db.filters()
    })
    .await
    .map_err(|e| EhError::Db(format!("filter load failed: {e}")))??;
    apply_filters(&mut result.galleries, &filters);

    apply_min_rating(&mut result.galleries, params.min_rating);

    let next_page = list_next_page(result.next_href.as_deref(), result.next_page, &url);
    Ok(GalleryListResponseDto {
        galleries: result.galleries.iter().map(gallery_info_dto).collect(),
        next_page,
        pages: dto_pages(result.pages),
        total: parse_result_count(&result.result_count),
    })
}

#[tauri::command]
pub async fn get_gallery_detail(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
) -> Result<GalleryDetailDto, EhError> {
    let detail = state.client.get_gallery_detail(gid, &token).await?;
    // Fire-and-forget history record, like the original's HistoryScene.
    let info = detail.info.clone();
    let db = Arc::clone(&state.db);
    let _ = tokio::task::spawn_blocking(move || db.record_history(&info)).await;
    Ok(gallery_detail_dto(&detail))
}

#[tauri::command]
pub async fn get_preview_set(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    index: i32,
) -> Result<PreviewSetDto, EhError> {
    // The preview-set page itself doesn't carry the total page count, so
    // fetch the detail first (same two-step the original reader performs).
    let detail = state.client.get_gallery_detail(gid, &token).await?;
    let set = state.client.get_preview_set(gid, &token, index).await?;
    Ok(PreviewSetDto {
        gid,
        previews: set
            .rows
            .iter()
            .map(|row| PreviewPageDto {
                index: row.position,
                thumb: row.url.clone(),
                width: row.xres,
                height: row.yres,
            })
            .collect(),
        total: detail.preview_pages,
    })
}

#[tauri::command]
pub async fn get_page_data(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    index: i32,
) -> Result<PageDataDto, EhError> {
    let manager = state.downloads.clone();
    let local = state.local_images.clone();
    let saved_token = token.clone();
    let port = state.image_server_port;
    if let Ok(url) = tokio::task::spawn_blocking(move || {
        crate::download::offline::page_url(&manager, &local, port, gid, &saved_token, index)
    })
    .await?
    {
        return Ok(PageDataDto {
            index,
            url,
            width: 0,
            height: 0,
        });
    }
    // The frontend passes the GALLERY token; the page endpoint needs the
    // page token (ptoken). The original resolved it through the gtoken API,
    // which now answers "Invalid page." to everything, so ptokens are
    // harvested from the gallery preview pages instead (API kept as
    // fallback inside resolve_ptoken).
    let ptoken = state.client.resolve_ptoken(gid, &token, index).await?;
    let mut data = state.client.get_page_data(gid, &ptoken, index).await?;

    // H@H redirect pages carry no <img>; follow the JS redirect once and
    // re-parse (the original does this through nl(skipHathKey)).
    if data.image_url.is_none() {
        if let Some(redirect) = data.redirect_url.clone() {
            let referer = crate::core::client::url::gallery_detail_url(
                state.client.site(),
                gid,
                &token,
                0,
                false,
            );
            let body = state.client.get_string(&redirect, Some(&referer)).await?;
            data = parser::parse_gallery_page(&body, &ptoken)?;
        }
    }

    let image_url = data
        .image_url
        .ok_or_else(|| EhError::Parse("gallery page has no image URL".into()))?;
    let url = crate::image_cache::get_image(
        &state.client,
        &state.image_cache,
        state.image_server_port,
        &image_url,
    )
    .await?;
    Ok(PageDataDto {
        index,
        url,
        width: 0,
        height: 0,
    })
}

#[tauri::command]
pub async fn get_comments(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    page: i32,
) -> Result<Vec<CommentDto>, EhError> {
    let comments = if page <= 1 {
        state.client.get_comments(gid, &token).await?.0
    } else {
        // The original loads all remaining comments by re-fetching the
        // detail with hc=1 ("all comments on one page").
        let site = state.client.site();
        let url = crate::core::client::url::gallery_detail_url(site, gid, &token, 0, true);
        let referer = crate::core::client::url::referer(site);
        let body = state.client.get_string(&url, Some(referer)).await?;
        parser::parse_gallery_detail(&body)?.comments
    };
    Ok(comments.iter().map(comment_dto).collect())
}

#[tauri::command]
pub async fn get_favorites(
    state: State<'_, AppState>,
    page: i32,
    slot: Option<i32>,
    sort: Option<String>,
    cursor: Option<String>,
) -> Result<FavoritesResponseDto, EhError> {
    let mut builder = ListUrlBuilder::new();
    builder.mode = ListMode::Favorites;
    builder.page = page.max(0);
    builder.fav_cat = slot.unwrap_or(-1);
    builder.fav_sort = sort;
    let url = list_request_url(&builder.build(state.client.site()), cursor.as_deref())?;
    let result = state.client.get_favorites(&url).await?;
    let next_page = list_next_page(result.next_href.as_deref(), result.next_page, &url);
    Ok(FavoritesResponseDto {
        items: result.galleries.iter().map(favorite_item_dto).collect(),
        next_page,
        pages: dto_pages(result.pages),
    })
}

/// `api.ts` getFavorites return: `{ items, nextPage, pages }`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesResponseDto {
    pub items: Vec<FavoriteItemDto>,
    pub next_page: Option<ListPage>,
    /// Total page count (0 when unknown).
    pub pages: u32,
}

/// Popup radio `value` attribute, tolerant of attribute order.
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

#[tauri::command]
pub async fn toggle_favorite(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    slot: i32,
) -> Result<ToggleFavoriteDto, EhError> {
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    let url = crate::core::client::url::add_favorites_url(state.client.site(), gid, &token);
    // The frontend calls this with the *current* slot when the gallery is
    // already favorited; compare against the live popup state to decide
    // between add/move and delete (favdel), like the original's dstCat=-1.
    let current = current_favorite_slot(&state.client, gid, &token).await;
    let remove = current.is_some() && current == Some(slot);
    let slot_string = slot.to_string();
    let cat: &str = if remove { "favdel" } else { &slot_string };
    state
        .client
        .post_form(
            &url,
            Some(&url),
            &[
                ("favcat", cat),
                ("favnote", ""),
                ("submit", "Apply Changes"),
                ("update", "1"),
            ],
        )
        .await?;
    Ok(ToggleFavoriteDto {
        is_favorited: !remove,
        slot: if remove { -1 } else { slot },
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleFavoriteDto {
    pub is_favorited: bool,
    pub slot: i32,
}

/// Port of `EhEngine.rateGallery` — POSTs `method=rategallery` to `api.php`
/// with the `apiuid`/`apikey` cookie pair. `rating` is the star value
/// 0.5-5.0 (the original's 1-10 "even steps" vote is `ceil(rating * 2)`,
/// so any half-star lands on the even 2-10 scale the site accepts).
#[tauri::command]
pub async fn rate_gallery(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    rating: f64,
) -> Result<RateGalleryDto, EhError> {
    let rating = state.client.rate_gallery(gid, &token, rating).await?;
    Ok(RateGalleryDto { rating })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateGalleryDto {
    pub rating: f32,
}

/// Vote a comment up or down (`vote` = 1 or -1); returns the comment's
/// updated score (port of `EhEngine.voteComment`, the api.php `votecomment`
/// method).
#[tauri::command]
pub async fn vote_comment(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    comment_id: i64,
    vote: i64,
) -> Result<i64, EhError> {
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    state
        .client
        .vote_comment(gid, &token, comment_id, vote)
        .await
}

/// Post a new comment to a gallery (form POST `commenttext_new` to the
/// gallery page, port of `EhEngine.commentGallery`).
#[tauri::command]
pub async fn send_comment(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    text: String,
) -> Result<(), EhError> {
    if !state.cookie_store.has_signed_in() {
        return Err(EhError::NotSignedIn);
    }
    state.client.send_comment(gid, &token, &text).await
}

#[cfg(test)]
#[path = "gallery_tests.rs"]
mod tests;
