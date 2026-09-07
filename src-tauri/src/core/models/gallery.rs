//! Serde DTOs ported from the original `client/data` package:
//! `GalleryInfo.java`, `GalleryDetail.java`, `GalleryComment.java`,
//! `TorrentInfo.java`, `NormalPreviewSet.java`, `LargePreviewSet.java` and
//! `topList/TopListItem.java`.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::category;

/// Serialize a [`DateTime<Utc>`] as a unix timestamp (seconds) so the wire
/// format matches the "posted as timestamp" requirement without needing
/// chrono's `serde` feature.
pub mod ts_seconds {
    use super::*;

    pub fn serialize<S: serde::Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i64(dt.timestamp())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let secs = i64::deserialize(d)?;
        Utc.timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| serde::de::Error::custom("invalid unix timestamp"))
    }
}

/// A group of tags sharing one namespace (port of `GalleryTagGroup`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroup {
    pub namespace: String,
    pub tags: Vec<String>,
}

/// Port of `GalleryComment.java`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: i64,
    pub score: i64,
    pub editable: bool,
    pub vote_up_able: bool,
    pub vote_up_ed: bool,
    pub vote_down_able: bool,
    pub vote_down_ed: bool,
    pub vote_state: Option<String>,
    /// Epoch milliseconds, like the original `GalleryComment.time`.
    pub time: i64,
    pub user: String,
    /// Raw HTML of the comment body (`c6` inner HTML), like the original.
    pub comment: String,
    pub last_edited: i64,
}

/// Port of `TorrentInfo.java`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentInfo {
    pub url: String,
    pub name: String,
    pub posted: String,
}

/// One torrent entry of the `gallerytorrents.php` list (the current EH
/// markup: one bordered div per torrent with Posted/Size/Seeds/Peers/
/// Downloads cells and the `.torrent` link).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentEntry {
    /// Verbatim `href` of the torrent link.
    pub url: String,
    pub name: String,
    /// Human readable size exactly as shown ("2.20 MiB").
    pub size: String,
    pub seeds: i32,
    pub peers: i32,
    pub downloads: i32,
}

/// One row of a preview set (large/medium/small layouts unified).
///
/// Mirrors `NormalPreviewSet.addItem(position, imageUrl, xOffset, yOffset,
/// width, height, pageUrl)` and `LargePreviewSet.addItem(index, imageUrl,
/// pageUrl)`; `gid`/`token` are derived from the page URL
/// (`https://host/s/{ptoken}/{gid}-{page}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    /// 0-based page index inside the gallery.
    pub position: i32,
    /// Image URL (thumbnail or full image for large previews).
    pub url: String,
    /// Gallery id, parsed from the page URL.
    pub gid: u64,
    /// Page token (ptoken), parsed from the page URL.
    pub token: String,
    pub xres: i32,
    pub yres: i32,
    pub xoffset: i32,
    pub yoffset: i32,
}

/// A set of preview rows parsed from one preview page of the detail gallery.
///
/// `pages` is the 0-based index of the preview page this set was parsed from
/// (each preview page covers one HTML page of the detail gallery).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSet {
    pub pages: i32,
    pub rows: Vec<PreviewItem>,
}

impl PreviewSet {
    pub fn new(pages: i32) -> Self {
        Self {
            pages,
            rows: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Port of `GalleryInfo.java`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryInfo {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub title_jpn: Option<String>,
    pub thumb: String,
    /// Category bitflag, see [`crate::core::models::category`].
    pub category: i32,
    pub uploader: Option<String>,
    /// Post time as a unix timestamp (UTC).
    #[serde(with = "ts_seconds")]
    pub posted: DateTime<Utc>,
    /// Average rating (0.0 - 5.0). The original uses -1.0f for "not yet rated".
    pub rating: f32,
    /// 1 if the current user has rated this gallery, else 0
    /// (the original stores a plain boolean `rated`).
    pub rated: i64,
    pub pages: i32,
    /// Number of users who favorited the gallery (detail pages only; 0 on lists).
    pub favorites: i64,
    /// 0-9 = favorite slot, -1 = checked and not favorited, -2 = unknown
    /// (same semantics as the original `favoriteSlot`).
    pub favorite_slot: i32,
    pub tags: Vec<TagGroup>,
    /// Thumbnail dimensions in pixels.
    pub thumb_width: i32,
    pub thumb_height: i32,
}

impl Default for GalleryInfo {
    fn default() -> Self {
        Self {
            gid: 0,
            token: String::new(),
            title: String::new(),
            title_jpn: None,
            thumb: String::new(),
            category: category::UNKNOWN,
            uploader: None,
            posted: Utc.timestamp_opt(0, 0).unwrap(),
            rating: -1.0,
            rated: 0,
            pages: 0,
            favorites: 0,
            favorite_slot: -2,
            tags: Vec::new(),
            thumb_width: 0,
            thumb_height: 0,
        }
    }
}

/// Port of `GalleryDetail.java` (all fields from the original plus the ones
/// the desktop UI needs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryDetail {
    #[serde(flatten)]
    pub info: GalleryInfo,
    pub torrent_count: i32,
    pub torrents: Vec<TorrentInfo>,
    /// Page URL of the torrent list (parsed from the "Torrent Download (N)"
    /// popUp link; fetched separately via `parse_torrent_page`).
    pub torrent_url: String,
    /// Page URL of the archiver popUp (parsed from the Archive Download link).
    pub archive_url: String,
    /// `archiver_key` parsed out of the Archive Download popUp URL
    /// (`archiver.php?gid=..&token=..&or={key}`).
    pub archiver_key: Option<String>,
    /// Display name of the favorite slot when favorited ("Add to Favorites"
    /// otherwise), port of `GalleryDetail.favoriteName`.
    pub favorite_name: Option<String>,
    pub parent_gid: Option<u64>,
    pub parent_token: Option<String>,
    pub visible: bool,
    pub language: Option<String>,
    /// Human readable size, e.g. "12.34 MiB".
    pub size: String,
    /// Size in bytes parsed from `size` (0 when unparseable).
    pub size_bytes: u64,
    pub favorite_count: i64,
    pub favorited: bool,
    pub rating_count: i64,
    pub average_rating: f32,
    pub user_rating: f32,
    pub comment_count: i64,
    pub has_more_comments: bool,
    pub comments: Vec<Comment>,
    /// Number of preview pages of the gallery.
    pub preview_pages: i32,
    pub preview_sets: Vec<PreviewSet>,
}

impl Default for GalleryDetail {
    fn default() -> Self {
        Self {
            info: GalleryInfo::default(),
            torrent_count: 0,
            torrents: Vec::new(),
            torrent_url: String::new(),
            archive_url: String::new(),
            archiver_key: None,
            favorite_name: None,
            parent_gid: None,
            parent_token: None,
            visible: true,
            language: None,
            size: String::new(),
            size_bytes: 0,
            favorite_count: 0,
            favorited: false,
            rating_count: 0,
            average_rating: -1.0,
            user_rating: -1.0,
            comment_count: 0,
            has_more_comments: false,
            comments: Vec::new(),
            preview_pages: 0,
            preview_sets: Vec::new(),
        }
    }
}

/// Port of `topList/TopListItem.java`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopListItem {
    pub value: String,
    pub href: String,
    pub rank: i32,
}

/// The seven top lists on `toplist.php`, port of `EhTopListDetail`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopListDetail {
    pub title: String,
    pub gallery: TopListInfo,
    pub uploader: TopListInfo,
    pub tagging: TopListInfo,
    pub hentai_home: TopListInfo,
    pub eh_tracker: TopListInfo,
    pub clean_up: TopListInfo,
    pub rating_and_reviewing: TopListInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TopListKind {
    #[default]
    Gallery,
    Uploader,
    Tagging,
    HentaiHome,
    EhTracker,
    CleanUp,
    RatingAndReviewing,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopListInfo {
    pub kind: TopListKind,
    pub all_time: Vec<TopListItem>,
    pub past_year: Vec<TopListItem>,
    pub past_month: Vec<TopListItem>,
    pub yesterday: Vec<TopListItem>,
}

/// Result of parsing a gallery page (`s/{ptoken}/{gid}-{page}`), port of
/// `GalleryPageParser.Result` plus the server time/offset and redirect
/// handling the reader pipeline needs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryPageData {
    /// Full image URL from the `i3`/`i7` `<img>` tag, if present.
    pub image_url: Option<String>,
    /// H@H skip key (`onclick="return nl('...')"`).
    pub skip_hath_key: Option<String>,
    /// Original (full-res) image URL when the "click to see original" link exists.
    pub origin_image_url: Option<String>,
    /// The showkey used by the `showpage` API call.
    pub show_key: Option<String>,
    /// The ptoken of the parsed page (from the request URL).
    pub ptoken: String,
    /// Server time from the page JS (`var server_time = ...`), epoch seconds.
    pub server_time: i64,
    /// Server timezone offset from the page JS, seconds.
    pub server_offset: i64,
    /// Set when the page is a JS redirect instead of an image page.
    pub redirect_url: Option<String>,
}

/// One news post from `news.php` (the `.newstable` entries; the original's
/// `EhNewsDetail` keeps the raw page, the desktop port exposes structured
/// items with the body sanitized to plain text).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    /// Stable id when one can be derived from the post link
    /// (`showtopic=<id>` forum topic), else `None`.
    pub id: Option<i64>,
    pub title: String,
    /// The `.newsdate` text without the "Posted " prefix.
    pub date: String,
    /// Sanitized body: tags stripped, `<br>`/`<p>` turned into newlines.
    pub body: String,
    pub link: String,
}

/// Quota / rate-limit numbers parsed from `home.php` (the "Image Limits"
/// homebox paragraph plus the Current/Peak/Regen limits table, matching
/// `EhHomeParser.java` / `HomeParser`). Every field is best-effort: the
/// homepage markup drifts and anonymous sessions get the login redirect, so
/// absent data stays at the `-1`/`None` defaults instead of erroring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quota {
    /// Image quota used (`-1` = unknown).
    pub images_used: i64,
    /// Image quota cap (`-1` = unknown, `0` = "No restrictions").
    pub images_total: i64,
    /// Seconds until the image quota resets, when the page says.
    pub image_reset_seconds: Option<i64>,
    /// Search quota used, from the limits table (`None` = absent).
    pub searches_used: Option<i64>,
    /// Search quota peak/cap, from the limits table (`None` = absent).
    pub searches_total: Option<i64>,
    /// Seconds until the search quota resets, when the page says.
    pub search_reset_seconds: Option<i64>,
}

impl Default for Quota {
    fn default() -> Self {
        Self {
            images_used: -1,
            images_total: -1,
            image_reset_seconds: None,
            searches_used: None,
            searches_total: None,
            search_reset_seconds: None,
        }
    }
}
