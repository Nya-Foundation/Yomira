// Portions adapted from EhViewer / Ehviewer_CN_SXJ.
// Copyright 2014-2019 Hippo Seven and upstream contributors.
// Original portions: Apache-2.0; see licenses/Apache-2.0.txt and NOTICE.
// Modified for Yomira: Rust/Tauri port, async I/O and application integration.

//! Core EH client logic: errors, DTOs, HTTP client and HTML parsers.
//!
//! This is a port of the original Android app's `client` package
//! (github.com/xiaojieonly/Ehviewer_CN_SXJ, branch BiLi_PC_Gamer) to async
//! Rust for the Tauri desktop app.

pub mod client;
pub mod error;
pub mod models;
pub mod parser;

pub use client::{
    built_in_hosts, ClientConfig, DnsResolver, DohResolver, EhClient, EhCookieStore, EhResolver,
    HostsResolver, InMemoryCookieStore, Site, SystemResolver, DEFAULT_DOH_ENDPOINT,
    DEFAULT_USER_AGENT,
};
pub use error::EhError;
pub use models::{
    category_color, category_name, format_tag_set, get_category, AdvancedSearch, Comment,
    GalleryDetail, GalleryInfo, GalleryPageData, ListMode, ListUrlBuilder, NewsItem, PreviewItem,
    PreviewSet, Quota, TagGroup, TagTerm, TopListDetail, TopListInfo, TopListItem, TopListKind,
    TorrentEntry, TorrentInfo, ALL_CATEGORY, ARTIST_CG, ASIAN_PORN, COSPLAY, DOUJINSHI, GAME_CG,
    IMAGE_SET, MANGA, MISC, NONE, NON_H, UNKNOWN, WESTERN,
};
pub use parser::{
    detect_signed_in, parse_favorites, parse_gallery_detail, parse_gallery_list,
    parse_gallery_page, parse_news, parse_preview_set, parse_quota, parse_sign_in, parse_toplist,
    parse_torrent_page, FavoritesResult, GalleryListResult,
};
