//! Wire DTOs — the exact shapes `src/lib/types.ts` declares.
//!
//! The core models are a faithful port of the original Java beans (bitflag
//! categories, `Option<String>` uploaders, chrono timestamps, ...); the
//! frontend instead wants display-ready camelCase JSON: category as display
//! name, epoch-second timestamps, zero-filled list-only fields. Mapping
//! happens here, at the command boundary.

use serde::{Deserialize, Serialize};

use crate::core::models::{category, Comment, GalleryDetail, GalleryInfo, PreviewSet};

/// `types.ts` `GalleryInfo`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryInfoDto {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub title_jpn: Option<String>,
    pub category: String,
    pub uploader: String,
    /// Epoch seconds.
    pub posted: i64,
    pub pages: i32,
    /// 0.0-5.0 (the core uses -1.0 for "not yet rated"; clamp to 0 here).
    pub rating: f32,
    pub rated_count: i64,
    /// Number of users who favorited the gallery (0 on lists).
    pub favorited: i64,
    pub language: String,
    pub thumb: String,
    /// Bytes (0 on lists).
    pub size: u64,
    pub torrent_count: i32,
    pub file_count: i32,
}

/// Map a core [`GalleryInfo`] with the list-page defaults for the
/// detail-only fields.
pub fn gallery_info_dto(info: &GalleryInfo) -> GalleryInfoDto {
    GalleryInfoDto {
        gid: info.gid,
        token: info.token.clone(),
        title: info.title.clone(),
        title_jpn: info.title_jpn.clone(),
        category: category::category_name(info.category).to_string(),
        uploader: info.uploader.clone().unwrap_or_default(),
        posted: info.posted.timestamp(),
        pages: info.pages,
        rating: info.rating.max(0.0),
        rated_count: 0,
        favorited: info.favorites,
        language: String::new(),
        thumb: info.thumb.clone(),
        size: 0,
        torrent_count: 0,
        file_count: 0,
    }
}

/// `types.ts` `Comment` (time as epoch seconds, body kept as the original
/// raw HTML like `GalleryComment.comment`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: i64,
    pub author: String,
    pub score: i64,
    pub time: i64,
    /// Epoch seconds; 0 = never edited (the parser copies post time for
    /// edited-looking comments, exactly like the original).
    pub last_edited: i64,
    pub text: String,
}

pub fn comment_dto(c: &Comment) -> CommentDto {
    CommentDto {
        id: c.id,
        author: c.user.clone(),
        score: c.score,
        time: c.time / 1000,
        last_edited: c.last_edited / 1000,
        text: c.comment.clone(),
    }
}

/// `types.ts` `PreviewPage`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPageDto {
    pub index: i32,
    pub thumb: String,
    pub width: i32,
    pub height: i32,
}

/// `types.ts` `PreviewSet`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSetDto {
    pub gid: u64,
    pub previews: Vec<PreviewPageDto>,
    pub total: i32,
}

/// `types.ts` `GalleryDetail`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryDetailDto {
    #[serde(flatten)]
    pub info: GalleryInfoDto,
    pub tags: Vec<TagGroupDto>,
    pub comments: Vec<CommentDto>,
    pub preview_set: PreviewSetDto,
    pub is_favorited: bool,
    /// -1 = not favorited.
    pub favorite_slot: i32,
    pub my_rating: f32,
    /// Total comment count from the detail header (may exceed the parsed
    /// comment list when there are more pages).
    pub comment_count: i32,
    /// True when the gallery has more comments than were parsed.
    pub has_more_comments: bool,
}

/// `types.ts` `TagGroup`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroupDto {
    pub namespace: String,
    pub tags: Vec<String>,
}

pub fn gallery_detail_dto(detail: &GalleryDetail) -> GalleryDetailDto {
    let mut info = gallery_info_dto(&detail.info);
    info.rated_count = detail.rating_count;
    info.favorited = detail.favorite_count;
    info.language = detail.language.clone().unwrap_or_default();
    info.size = detail.size_bytes;
    info.torrent_count = detail.torrent_count;
    // The original detail page shows "filecount", which equals the page
    // count for galleries.
    info.file_count = detail.info.pages;

    // Merge every parsed preview set into one flat list (the detail parser
    // returns one set per preview page; the homepage layout has one).
    let previews: Vec<PreviewPageDto> = detail
        .preview_sets
        .iter()
        .flat_map(|set: &PreviewSet| {
            set.rows.iter().map(|row| PreviewPageDto {
                index: row.position,
                thumb: row.url.clone(),
                width: row.xres,
                height: row.yres,
            })
        })
        .collect();

    GalleryDetailDto {
        info,
        tags: detail
            .info
            .tags
            .iter()
            .map(|g| TagGroupDto {
                namespace: g.namespace.clone(),
                tags: g.tags.clone(),
            })
            .collect(),
        comments: detail.comments.iter().map(comment_dto).collect(),
        preview_set: PreviewSetDto {
            gid: detail.info.gid,
            previews,
            total: detail.preview_pages,
        },
        is_favorited: (0..=9).contains(&detail.info.favorite_slot),
        favorite_slot: detail.info.favorite_slot,
        my_rating: detail.user_rating.max(0.0),
        comment_count: detail.comment_count as i32,
        has_more_comments: detail.has_more_comments,
    }
}

/// `types.ts` `FavoriteItem`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItemDto {
    #[serde(flatten)]
    pub info: GalleryInfoDto,
    pub favorite_slot: i32,
    pub favorite_note: String,
    /// Epoch seconds; the favorites page does not expose a time, so 0.
    pub favorited_time: i64,
}

pub fn favorite_item_dto(info: &GalleryInfo) -> FavoriteItemDto {
    let mut dto = gallery_info_dto(info);
    // The favorites parser fills `favorites` (count); FavoriteItem wants the
    // favorited marker semantics, which for list rows is just the count.
    dto.favorited = info.favorites;
    FavoriteItemDto {
        favorite_slot: if (0..=9).contains(&info.favorite_slot) {
            info.favorite_slot
        } else {
            -1
        },
        favorite_note: String::new(),
        favorited_time: 0,
        info: dto,
    }
}

/// A numeric legacy page or the site's opaque continuation link.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ListPage {
    Number(i32),
    Url(String),
}

/// `types.ts` `GalleryListResponse`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryListResponseDto {
    pub galleries: Vec<GalleryInfoDto>,
    pub next_page: Option<ListPage>,
    /// Total page count for the current query (0 when unknown, e.g. watched).
    pub pages: u32,
    pub total: i64,
}

/// `types.ts` `ListParams` (verbatim mirror; camelCase in, camelCase out is
/// handled by serde).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListParamsDto {
    pub page: i32,
    pub cursor: Option<String>,
    #[serde(default)]
    pub search: String,
    pub categories: Option<Vec<String>>,
    pub min_rating: Option<f32>,
    pub min_pages: Option<i32>,
    pub max_pages: Option<i32>,
    #[serde(default = "default_true")]
    pub enable_uploader: bool,
    #[serde(default = "default_true")]
    pub enable_tags: bool,
    #[serde(default)]
    pub search_desc: bool,
    #[serde(default)]
    pub search_torrent: bool,
    #[serde(default)]
    pub search_lowpower_tags: bool,
    #[serde(default)]
    pub search_downvoted_tags: bool,
    #[serde(default)]
    pub search_expunged: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ListParamsDto {
    fn default() -> Self {
        Self {
            page: 0,
            cursor: None,
            search: String::new(),
            categories: None,
            min_rating: None,
            min_pages: None,
            max_pages: None,
            enable_uploader: true,
            enable_tags: true,
            search_desc: false,
            search_torrent: false,
            search_lowpower_tags: false,
            search_downvoted_tags: false,
            search_expunged: false,
        }
    }
}

/// `types.ts` `PageData` — `url` is the loopback image-server URL the reader
/// drops into `<img src>`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDataDto {
    pub index: i32,
    pub url: String,
    pub width: i32,
    pub height: i32,
}
