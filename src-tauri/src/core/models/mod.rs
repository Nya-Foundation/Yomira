//! Serde DTOs and URL builders ported from the original `client/data` package.

pub mod category;
pub mod gallery;
pub mod list_url;

pub use category::{
    category_color, category_name, get_category, ALL_CATEGORY, ARTIST_CG, ASIAN_PORN, COSPLAY,
    DOUJINSHI, GAME_CG, IMAGE_SET, MANGA, MISC, NONE, NON_H, UNKNOWN, WESTERN,
};
pub use gallery::{
    Comment, GalleryDetail, GalleryInfo, GalleryPageData, NewsItem, PreviewItem, PreviewSet, Quota,
    TagGroup, TopListDetail, TopListInfo, TopListItem, TopListKind, TorrentEntry, TorrentInfo,
};
pub use list_url::{
    format_tag_set, java_url_encode, AdvancedSearch, ListMode, ListUrlBuilder, TagTerm, UrlBuilder,
};
