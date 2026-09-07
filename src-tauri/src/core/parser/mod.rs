//! HTML parsers, ported 1:1 from the original `client/parser` package
//! (jsoup selectors → `scraper` css selectors, regexes → `regex`).
//!
//! One submodule per original Java file:
//! * [`gallery_list`] ← `GalleryListParser.java`
//! * [`gallery_detail`] ← `GalleryDetailParser.java` (+ `GalleryDetailUrlParser.java`)
//! * [`gallery_page`] ← `GalleryPageParser.java`
//! * [`favorites`] ← `FavoritesParser.java`
//! * [`toplist`] ← `TopListParser.java`
//! * [`sign_in`] ← `SignInParser.java`
//! * [`torrent`] ← `TorrentParser.java`
//! * [`news`] ← `EhNewsDetail` (news.php → structured items)
//! * [`quota`] ← `EhHomeParser.java` (home.php quota/limits)

pub mod archiver;
pub mod favorites;
pub mod gallery_detail;
pub mod gallery_list;
pub mod gallery_page;
pub mod news;
pub mod quota;
pub mod sign_in;
pub mod toplist;
pub mod torrent;

#[cfg(test)]
mod tests;

use scraper::{ElementRef, Selector};

use crate::core::error::EhError;

pub use archiver::{parse_archiver, parse_archiver_download_link, ArchiverForm, ArchiverInfo};
pub use favorites::{parse_favorites, FavoritesResult};
pub use gallery_detail::{
    parse_comment_error, parse_gallery_detail, parse_preview_set, GalleryDetailUrl,
};
pub use gallery_list::{parse_gallery_list, GalleryListResult};
pub use gallery_page::{is_509_image, parse_gallery_page};
pub use news::parse_news;
pub use quota::parse_quota;
pub use sign_in::{detect_signed_in, parse_sign_in};
pub use toplist::parse_toplist;
pub use torrent::{parse_torrent_entries, parse_torrent_page};

/// `JsoupUtils.getElementByClass` — first descendant with the class.
pub(crate) fn first_by_class<'a>(root: ElementRef<'a>, class: &str) -> Option<ElementRef<'a>> {
    root.select(&selector(&format!(".{class}"))).next()
}

/// `JsoupUtils.getElementByTag` — first descendant with the tag name.
pub(crate) fn first_by_tag<'a>(root: ElementRef<'a>, tag: &str) -> Option<ElementRef<'a>> {
    root.select(&selector(tag)).next()
}

/// Compile a CSS selector, panicking only on programmer error (static
/// selector strings).
pub(crate) fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("static selector must compile")
}

/// jsoup `Element.ownText()` — text directly inside the element.
pub(crate) fn own_text(el: ElementRef) -> String {
    el.children()
        .filter_map(|n| n.value().as_text().map(|t| t.to_string()))
        .collect()
}

/// jsoup `Element.child(i)` restricted to element children.
pub(crate) fn child_element(el: ElementRef, index: usize) -> Option<ElementRef> {
    el.child_elements().nth(index)
}

/// Port of `ParserUtils.parseInt`/`NumberUtils.parseIntSafely`.
pub(crate) fn parse_int(s: &str, default: i32) -> i32 {
    let s = s.trim().replace(',', "");
    s.parse().unwrap_or(default)
}

pub(crate) fn parse_i64(s: &str, default: i64) -> i64 {
    let s = s.trim().replace(',', "");
    s.parse().unwrap_or(default)
}

pub(crate) fn parse_f32(s: &str, default: f32) -> f32 {
    s.trim().parse().unwrap_or(default)
}

/// Minimal `StringUtils.unescapeXml` port (the ampersand entities EH URLs
/// and image links can contain).
pub(crate) fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Build an [`EhError::Parse`] with a trimmed body snippet.
pub(crate) fn parse_error(context: &str, body: &str) -> EhError {
    EhError::Parse(format!("{context}: {}", snippet(body, 120)))
}

fn snippet(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

/// Format helper shared by the parsers' unit tests.
#[cfg(test)]
pub(crate) mod testutil {
    pub fn fixture(name: &str) -> String {
        let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
    }
}
