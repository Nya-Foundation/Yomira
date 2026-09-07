//! Ported from `client/parser/GalleryPageParser.java` (+ the 509/bandwidth
//! page handling used by `EhEngine.getGalleryPage`).
//!
//! Parses the image page (`s/{ptoken}/{gid}-{page}`): full image URL from the
//! `i3`/`i7` `<img>` tag, the H@H skip key (`onclick="return nl('...')"`),
//! the original-image link, and `var showkey = "..."`. Server time/offset are
//! read from the page JS when present. Redirect pages (a JS
//! `document.location` assignment with no image) are reported via
//! [`GalleryPageData::redirect_url`]. The "You have exceeded your image
//! viewing limits" error page maps to [`EhError::Image509`].

use std::sync::LazyLock;

use regex::Regex;

use crate::core::error::EhError;
use crate::core::models::GalleryPageData;

use super::unescape_xml;

static PATTERN_IMAGE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<img[^>]*src="([^"]+)" style"#).unwrap());
static PATTERN_SKIP_HATH_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"onclick="return nl\('([^\)]+)'\)""#).unwrap());
static PATTERN_ORIGIN_IMAGE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a href="([^"]+)fullimg([^"]+)">"#).unwrap());
static PATTERN_SHOW_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"var showkey\s*=\s*"([0-9a-z]+)""#).unwrap());
static PATTERN_SERVER_TIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"var server_time\s*=\s*(\d+)"#).unwrap());
static PATTERN_SERVER_OFFSET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"var server_offset\s*=\s*(-?\d+)"#).unwrap());
static PATTERN_JS_REDIRECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:document|window)\.location(?:\.href)?\s*=\s*["']([^"']+)["']"#).unwrap()
});

/// Parse a gallery page body (port of `GalleryPageParser.parse`).
///
/// `ptoken` is the page token from the request URL; it is echoed back on the
/// result so callers can chain pages without re-parsing URLs.
pub fn parse_gallery_page(body: &str, ptoken: &str) -> Result<GalleryPageData, EhError> {
    // 509 detection — the original EhViewer shows the image-limit page here.
    if body.contains("You have exceeded your image viewing limits") {
        return Err(EhError::Image509);
    }
    if body.contains("exceeded your bandwidth limits") {
        return Err(EhError::BandwidthExceeded);
    }

    let mut result = GalleryPageData {
        ptoken: ptoken.to_string(),
        ..GalleryPageData::default()
    };

    if let Some(m) = PATTERN_IMAGE_URL.captures(body) {
        result.image_url = Some(unescape_xml(m.get(1).unwrap().as_str().trim()));
    }
    if let Some(m) = PATTERN_SKIP_HATH_KEY.captures(body) {
        result.skip_hath_key = Some(unescape_xml(m.get(1).unwrap().as_str().trim()));
    }
    if let Some(m) = PATTERN_ORIGIN_IMAGE_URL.captures(body) {
        result.origin_image_url = Some(format!(
            "{}fullimg{}",
            unescape_xml(m.get(1).unwrap().as_str()),
            unescape_xml(m.get(2).unwrap().as_str())
        ));
    }
    if let Some(m) = PATTERN_SHOW_KEY.captures(body) {
        result.show_key = Some(m.get(1).unwrap().as_str().to_string());
    }
    if let Some(m) = PATTERN_SERVER_TIME.captures(body) {
        result.server_time = m.get(1).unwrap().as_str().parse().unwrap_or(0);
    }
    if let Some(m) = PATTERN_SERVER_OFFSET.captures(body) {
        result.server_offset = m.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if result.image_url.is_none() || result.show_key.is_none() {
        // Not a plain image page — maybe a JS redirect (expired key).
        if let Some(m) = PATTERN_JS_REDIRECT.captures(body) {
            result.redirect_url = Some(m.get(1).unwrap().as_str().to_string());
            return Ok(result);
        }
        return Err(EhError::Parse("Parse image url and show error".to_string()));
    }
    Ok(result)
}

/// Heuristic 509-image detection for image byte responses: the EH limit
/// placeholder is a tiny GIF (a few hundred bytes). Kept as a separate,
/// testable function shared with image fetching.
pub fn is_509_image(content_type: &str, bytes: &[u8]) -> bool {
    content_type.starts_with("image/gif") && bytes.len() < 1024
}
