//! Ported from `client/parser/GalleryDetailParser.java` +
//! `client/parser/GalleryDetailUrlParser.java`.
//!
//! Covers: `gd` rows (title, cover, category, uploader, posted/parent/visible/
//! language/size/length/favorited from `gdd`), `grating`, favorite state from
//! `gdf`/`fvd`, tag groups from `tagmenu` (`taglist`), comments from `cdiv`
//! with the reusable `cs`/`c7` spans, torrent count + URL, archiver key from
//! the popUp onclick, rating count, and preview sets via the normal/small/
//! large regex families (mirroring `NormalPreviewSet`/`LargePreviewSet`).

use std::sync::LazyLock;

use chrono::{TimeZone, Utc};
use regex::Regex;
use scraper::{ElementRef, Html};

use crate::core::error::EhError;
use crate::core::models::{
    category, Comment, GalleryDetail, GalleryInfo, PreviewItem, PreviewSet, TagGroup,
};

use super::gallery_list::parse_posted;
use super::{
    child_element, first_by_class, first_by_tag, own_text, parse_error, parse_i64, parse_int,
    selector, unescape_xml,
};

static PATTERN_ERROR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<div class="d">\s*<p>([^<]+)</p>"#).unwrap());
static PATTERN_DETAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)var gid = (\d+);.+?var token = "([a-f0-9]+)";.+?var apiuid = ([-\d]+);.+?var apikey = "([a-f0-9]+)""#,
    )
    .unwrap()
});
static PATTERN_TORRENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a[^<>]*onclick="return popUp\('([^']+)'[^)]+\)">Torrent Download \((\d+)\)</a>"#)
        .unwrap()
});
static PATTERN_ARCHIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a[^<>]*onclick="return popUp\('([^']+)'[^)]+\)">Archive Download</a>"#).unwrap()
});
static PATTERN_COVER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"width:(\d+)px; height:(\d+)px.+?url\((.+?)\)").unwrap());
static PATTERN_PREVIEW_PAGES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<td[^>]+><a[^>]+>([\d,]+)</a></td><td[^>]+>(?:<a[^>]+>)?&gt;(?:</a>)?</td>"#)
        .unwrap()
});
static PATTERN_DETAIL_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)/([0-9a-f]{10})(?:[^0-9a-f]|$)").unwrap());
static PATTERN_ARCHIVER_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[?&]or=([^&'"]*)"#).unwrap());
static PATTERN_PAGE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/s/([0-9a-f]{10})/(\d+)-(\d+)").unwrap());

// Normal (medium) / small preview families. The original has four patterns
// (small/normal x plain/label) whose capture groups assume the attribute
// order of the raw HTML. The desktop port matches against
// scraper-reserialized markup (attributes are reordered), so a single
// order-agnostic "anchor + title div" pattern locates each preview and the
// geometry/url fields are extracted from the matched tag with the same
// sub-shapes the original patterns used. `(?:[^<>]|<div>)*?` also covers the
// "with label" wrapper `<a><div><div title=...>`.
static PATTERN_PREVIEW_ANCHOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a href="(.+?)">(?:[^<>]|<div>)*?<div[^<>]*title="Page (\d+):[^<>]*>"#).unwrap()
});
static PATTERN_TAG_WIDTH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"width:(\d+)").unwrap());
static PATTERN_TAG_HEIGHT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"height:(\d+)").unwrap());
static PATTERN_TAG_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((.+?)\)").unwrap());
static PATTERN_TAG_X_OFFSET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-(\d+)px").unwrap());
static PATTERN_NORMAL_PREVIEW_LEGACY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"<div class="gdtm"[^<>]*><div[^<>]*width:(\d+)[^<>]*height:(\d+)[^<>]*\((.+?)\)[^<>]*-(\d+)px[^<>]*><a[^<>]*href="(.+?)"[^<>]*><img alt="([\d,]+)""#,
    )
    .unwrap()
});
// Large preview families: full-image URL, sprite position `0 0`, no
// width/height geometry in the tag (see the note above about attribute
// order; `parse_large_preview_set` reuses PATTERN_PREVIEW_ANCHOR).
static PATTERN_LARGE_PREVIEW_LEGACY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<div class="gdtl".+?<a href="(.+?)"><img alt="([\d,]+)".+?src="(.+?)""#).unwrap()
});

const OFFENSIVE_STRING: &str =
    "<p>(And if you choose to ignore this warning, you lose all rights to complain about it in the future.)</p>";
const PINING_STRING: &str = "<p>This gallery is pining for the fjords.</p>";
const UNAVAILABLE_STRING: &str = "This gallery is unavailable";

/// `dd MMMMM yyyy, HH:mm` in UTC — port of WEB_COMMENT_DATE_FORMAT.
const COMMENT_DATE_FORMAT: &str = "%d %B %Y, %H:%M";

/// gid/token pair parsed out of a gallery URL, port of
/// `GalleryDetailUrlParser.Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryDetailUrl {
    pub gid: u64,
    pub token: String,
}

/// Parse a gallery detail URL (`https://host/g|mpv/{gid}/{token}`), port of
/// `GalleryDetailUrlParser.parse(url, strict=false)`.
pub fn parse_gallery_detail_url(url: &str) -> Option<GalleryDetailUrl> {
    let caps = PATTERN_DETAIL_URL.captures(url)?;
    let gid: u64 = caps.get(1)?.as_str().parse().ok()?;
    if gid == 0 {
        return None;
    }
    Some(GalleryDetailUrl {
        gid,
        token: caps.get(2)?.as_str().to_string(),
    })
}

/// Parse a full gallery detail page (port of `GalleryDetailParser.parse`).
pub fn parse_gallery_detail(body: &str) -> Result<GalleryDetail, EhError> {
    if body.contains(OFFENSIVE_STRING) {
        return Err(EhError::Eh("offensive content warning".to_string()));
    }
    if body.contains(PINING_STRING) {
        return Err(EhError::Eh(
            "this gallery is pining for the fjords".to_string(),
        ));
    }
    if body.contains(UNAVAILABLE_STRING) {
        return Err(EhError::Eh("this gallery is unavailable".to_string()));
    }
    if let Some(m) = PATTERN_ERROR.captures(body) {
        return Err(EhError::Eh(m[1].trim().to_string()));
    }

    let document = Html::parse_document(body);
    let mut detail = GalleryDetail::default();

    // var gid / token / apiuid / apikey
    let caps = PATTERN_DETAIL
        .captures(body)
        .ok_or_else(|| parse_error("Can't parse gallery detail (no gid/token)", body))?;
    detail.info.gid = parse_i64(&caps[1], -1) as u64;
    detail.info.token = caps[2].to_string();
    if detail.info.gid == 0 {
        return Err(parse_error("Can't parse gallery detail (bad gid)", body));
    }

    // Torrent count + URL
    if let Some(m) = PATTERN_TORRENT.captures(body) {
        detail.torrent_url = unescape_xml(m[1].trim());
        detail.torrent_count = parse_int(&m[2], 0);
    }

    // Archive popUp URL → archiver key (`or` parameter)
    if let Some(m) = PATTERN_ARCHIVE.captures(body) {
        detail.archive_url = unescape_xml(m[1].trim());
        detail.archiver_key = PATTERN_ARCHIVER_KEY
            .captures(&detail.archive_url)
            .map(|c| c[1].to_string())
            .filter(|s| !s.is_empty());
    }

    parse_detail_block(&mut detail, &document, body);
    detail.info.tags = parse_tag_groups(&document);
    let (comments, has_more) = parse_comments(&document);
    detail.comments = comments;
    detail.has_more_comments = has_more;
    detail.preview_pages = parse_preview_pages(&document, body);
    detail.preview_sets = vec![parse_preview_set(body, 0)];
    Ok(detail)
}

/// The `gm` block: cover, titles, category, uploader, gdd rows, rating.
fn parse_detail_block(detail: &mut GalleryDetail, d: &Html, body: &str) {
    let Some(gm) = first_by_class(d.root_element(), "gm") else {
        return;
    };

    // Thumb url from #gd1 style
    if let Some(gd1) = gm
        .select(&selector("#gd1"))
        .next()
        .and_then(|e| child_element(e, 0))
    {
        let style = gd1.attr("style").unwrap_or_default();
        if let Some(m) = PATTERN_COVER.captures(style) {
            detail.info.thumb_width = parse_int(&m[1], 0);
            detail.info.thumb_height = parse_int(&m[2], 0);
            detail.info.thumb = m[3].trim().to_string();
        }
    }

    // Titles
    if let Some(gn) = gm.select(&selector("#gn")).next() {
        detail.info.title = gn.text().collect::<String>().trim().to_string();
    }
    if let Some(gj) = gm.select(&selector("#gj")).next() {
        let t = gj.text().collect::<String>().trim().to_string();
        detail.info.title_jpn = if t.is_empty() { None } else { Some(t) };
    }

    // Category
    if let Some(gdc) = gm.select(&selector("#gdc")).next() {
        let ce = first_by_class(gdc, "cn").or_else(|| first_by_class(gdc, "cs"));
        if let Some(ce) = ce {
            detail.info.category = category::get_category(ce.text().collect::<String>().trim());
        }
    }

    // Uploader
    if let Some(gdn) = gm.select(&selector("#gdn")).next() {
        let t = gdn.text().collect::<String>().trim().to_string();
        detail.info.uploader = if t.is_empty() { None } else { Some(t) };
    }

    // gdd rows: Posted / Parent / Visible / Language / File Size / Length / Favorited
    if let Some(gdd) = gm.select(&selector("#gdd")).next() {
        if let Some(tbody) = gdd.select(&selector("tbody")).next() {
            for row in tbody.child_elements() {
                parse_detail_info(detail, row);
            }
        }
    }

    // Rating
    if let Some(rating_count) = gm.select(&selector("#rating_count")).next() {
        detail.rating_count = parse_i64(&rating_count.text().collect::<String>(), 0);
    }
    if let Some(rating_label) = gm.select(&selector("#rating_label")).next() {
        let text = rating_label.text().collect::<String>().trim().to_string();
        if text == "Not Yet Rated" {
            detail.average_rating = -1.0;
        } else if let Some(idx) = text.find(' ') {
            detail.average_rating = text[idx + 1..].trim().parse().unwrap_or(0.0);
        } else {
            detail.average_rating = 0.0;
        }
    } else {
        detail.average_rating = -1.0;
    }

    // Favorited state + slot from the gdf form (#fvd input)
    if let Some(gdf) = gm.select(&selector("#gdf")).next() {
        let text = gdf.text().collect::<String>().trim().to_string();
        detail.favorited = text != "Add to Favorites";
        detail.favorite_name = if detail.favorited { Some(text) } else { None };
        if let Some(fvd) = gm.select(&selector("#fvd")).next() {
            if let Some(value) = fvd.attr("value") {
                detail.info.favorite_slot = parse_int(value, -1);
            }
        }
    }
    let _ = body;
}

/// One row of the `gdd` table (port of `parseDetailInfo`).
fn parse_detail_info(detail: &mut GalleryDetail, row: ElementRef) {
    let cells: Vec<ElementRef> = row.child_elements().collect();
    if cells.len() < 2 {
        return;
    }
    let key = cells[0].text().collect::<String>().trim().to_string();
    let value = own_text(cells[1]).trim().to_string();
    if key.starts_with("Posted") {
        detail.info.posted = parse_posted(&value);
    } else if key.starts_with("Parent") {
        if let Some(a) = first_by_tag(cells[1], "a") {
            if let Some(href) = a.attr("href") {
                if let Some(url) = parse_gallery_detail_url(href) {
                    detail.parent_gid = Some(url.gid);
                    detail.parent_token = Some(url.token);
                }
            }
        }
    } else if key.starts_with("Visible") {
        detail.visible = !matches!(value.as_str(), "No" | "no");
    } else if key.starts_with("Language") {
        detail.language = if value.is_empty() { None } else { Some(value) };
    } else if key.starts_with("File Size") {
        detail.size = value.clone();
        detail.size_bytes = parse_size_bytes(&value);
    } else if key.starts_with("Length") {
        let num: String = value
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ',')
            .collect();
        detail.info.pages = parse_int(&num, 1).max(1);
    } else if key.starts_with("Favorited") {
        detail.favorite_count = match value.as_str() {
            "Never" => 0,
            "Once" => 1,
            _ => value
                .find(' ')
                .map(|i| parse_i64(&value[..i], 0))
                .unwrap_or(0),
        };
        detail.info.favorites = detail.favorite_count;
    }
}

/// "12.34 MiB" / "1.2 GB" → bytes (binary units, like the original size text).
pub(crate) fn parse_size_bytes(size: &str) -> u64 {
    let mut parts = size.split_whitespace();
    let num: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let unit = parts.next().unwrap_or_default().to_ascii_lowercase();
    let mult = match unit.trim_end_matches('b').trim_end_matches('i') {
        "k" => 1u64 << 10,
        "m" => 1u64 << 20,
        "g" => 1u64 << 30,
        "t" => 1u64 << 40,
        _ => 1,
    };
    (num * mult as f64) as u64
}

// ---------------------------------------------------------------------
// Tag groups (port of parseTagGroup / parseTagGroups)
// ---------------------------------------------------------------------

/// Parse tag group rows (`<tr>` with namespace in the first cell and tag divs
/// in the second) — used by both the detail page and the extended list.
pub(crate) fn parse_tag_group_rows<'a>(
    rows: impl Iterator<Item = ElementRef<'a>>,
) -> Vec<TagGroup> {
    rows.filter_map(parse_tag_group).collect()
}

fn parse_tag_group(row: ElementRef) -> Option<TagGroup> {
    let ns_cell = child_element(row, 0)?;
    let tags_cell = child_element(row, 1)?;
    let mut namespace = ns_cell.text().collect::<String>().trim().to_string();
    // Remove trailing ':'
    if namespace.ends_with(':') {
        namespace.pop();
    }
    let mut tags = Vec::new();
    for tag_el in tags_cell.child_elements() {
        let mut tag = tag_el.text().collect::<String>().trim().to_string();
        // Sometimes parody tags are followed by '|' and a translation.
        if let Some(idx) = tag.find('|') {
            tag = tag[..idx].trim().to_string();
        }
        if !tag.is_empty() {
            tags.push(tag);
        }
    }
    if tags.is_empty() {
        None
    } else {
        Some(TagGroup { namespace, tags })
    }
}

/// Parse the `taglist`/`tagmenu` table of the detail page.
pub(crate) fn parse_tag_groups(document: &Html) -> Vec<TagGroup> {
    document
        .select(&selector("#taglist"))
        .next()
        .and_then(|taglist| taglist.select(&selector("tbody")).next())
        .map(|tbody| parse_tag_group_rows(tbody.child_elements()))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------
// Comments (port of parseComment / parseComments)
// ---------------------------------------------------------------------

/// Parse comments from the `cdiv` block; returns `(comments, has_more)`.
pub fn parse_comments(document: &Html) -> (Vec<Comment>, bool) {
    let Some(cdiv) = document.select(&selector("#cdiv")).next() else {
        return (Vec::new(), false);
    };
    let mut list = Vec::new();
    for c1 in cdiv.select(&selector(".c1")) {
        if let Some(comment) = parse_comment(c1) {
            list.push(comment);
        }
    }
    // "click to show all" in the chd header means more comments exist.
    let has_more = cdiv
        .select(&selector("#chd"))
        .next()
        .is_some_and(|chd| chd.text().any(|t| t.trim() == "click to show all"));
    (list, has_more)
}

/// Previous sibling element (jsoup `previousElementSibling`).
fn previous_element_sibling(el: ElementRef) -> Option<ElementRef> {
    let mut node = el.prev_sibling();
    while let Some(n) = node {
        if let Some(e) = ElementRef::wrap(n) {
            return Some(e);
        }
        node = n.prev_sibling();
    }
    None
}

fn parse_comment(element: ElementRef) -> Option<Comment> {
    let mut comment = Comment::default();

    // Id from the named anchor before the comment div (name="c123").
    if let Some(prev) = previous_element_sibling(element) {
        if let Some(name) = prev.attr("name") {
            comment.id = name
                .trim()
                .strip_prefix('c')
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    // Editable / vote up / vote down from c4
    let mut c4_text = String::new();
    if let Some(c4) = first_by_class(element, "c4") {
        c4_text = c4.text().collect::<String>().trim().to_string();
        for e in c4.child_elements() {
            match e.text().collect::<String>().trim() {
                "Vote+" => {
                    comment.vote_up_able = true;
                    comment.vote_up_ed = !e.attr("style").unwrap_or_default().trim().is_empty();
                }
                "Vote-" => {
                    comment.vote_down_able = true;
                    comment.vote_down_ed = !e.attr("style").unwrap_or_default().trim().is_empty();
                }
                "Edit" => comment.editable = true,
                _ => {}
            }
        }
    }

    // Vote state from c7
    if let Some(c7) = first_by_class(element, "c7") {
        let t = c7.text().collect::<String>().trim().to_string();
        comment.vote_state = if t.is_empty() { None } else { Some(t) };
    }

    // Score from c5
    if let Some(c5) = first_by_class(element, "c5") {
        if let Some(first) = c5.child_elements().next() {
            comment.score = parse_i64(&first.text().collect::<String>(), 0);
        }
    }

    // Time + user from c3
    let c3 = first_by_class(element, "c3")?;
    let mut temp = own_text(c3);
    // The original cuts the " by:" trailer with a fixed-length substring and
    // relies on DateFormat ignoring the trailing junk; truncating at " by:"
    // is the robust equivalent.
    if let Some(idx) = temp.find(" by:") {
        temp.truncate(idx);
    }
    let temp = temp.trim_start_matches("Posted on ").trim().to_string();
    comment.time = parse_comment_date(&temp);
    if let Some(first) = c3.child_elements().next() {
        comment.user = first.text().collect::<String>().trim().to_string();
    } else {
        comment.user = c4_text;
    }

    // Body html from c6
    if let Some(c6) = first_by_class(element, "c6") {
        comment.comment = c6.inner_html();
    }

    // Last edited marker from c8 (the original copies the post time here).
    if let Some(c8) = first_by_class(element, "c8") {
        if c8.child_elements().next().is_some() {
            comment.last_edited = comment.time;
        }
    }

    Some(comment)
}

fn parse_comment_date(text: &str) -> i64 {
    for fmt in [COMMENT_DATE_FORMAT, "%Y-%m-%d %H:%M"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(text.trim(), fmt) {
            return Utc.from_utc_datetime(&dt).timestamp_millis();
        }
    }
    0
}

// ---------------------------------------------------------------------
// Preview pages & preview set (port of parsePreviewPages / parsePreviewSet)
// ---------------------------------------------------------------------

fn parse_preview_pages(document: &Html, body: &str) -> i32 {
    if let Some(ptt) = document.select(&selector(".ptt")).next() {
        let tds: Vec<ElementRef> = ptt
            .select(&selector("tr"))
            .next()
            .map(|tr| tr.child_elements().collect())
            .unwrap_or_default();
        if tds.len() >= 2 {
            return parse_int(&tds[tds.len() - 2].text().collect::<String>(), 0);
        }
    }
    if let Some(m) = PATTERN_PREVIEW_PAGES.captures(body) {
        return parse_int(&m[1], 0);
    }
    0
}

/// Parse the preview set of one detail/preview page. `preview_page` is the
/// 0-based index of the preview page (goes into [`PreviewSet::pages`]).
pub fn parse_preview_set(body: &str, preview_page: i32) -> PreviewSet {
    let document = Html::parse_document(body);
    let gt_html = document
        .select(&selector(".gt200"))
        .next()
        .or_else(|| document.select(&selector(".gt100")).next())
        .map(|e| e.inner_html());
    let target: &str = match &gt_html {
        Some(html) if !html.is_empty() => html,
        _ => body,
    };

    let set = parse_normal_preview_set(target, preview_page);
    if set.is_empty() {
        parse_large_preview_set(target, preview_page)
    } else {
        set
    }
}

/// Port of `parseNormalPreviewSet` — the four original "new layout" patterns
/// collapsed into one order-agnostic pass (see the note on
/// [`PATTERN_PREVIEW_ANCHOR`]), then the legacy `gdtm` pattern when nothing
/// matched.
fn parse_normal_preview_set(body: &str, preview_page: i32) -> PreviewSet {
    let mut set = PreviewSet::new(preview_page);
    for caps in PATTERN_PREVIEW_ANCHOR.captures_iter(body) {
        let position = parse_int(caps.get(2).map(|m| m.as_str()).unwrap_or("0"), 0) - 1;
        if position < 0 {
            continue;
        }
        let tag = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        let width = PATTERN_TAG_WIDTH
            .captures(tag)
            .map(|m| parse_int(&m[1], 0))
            .unwrap_or(0);
        let height = PATTERN_TAG_HEIGHT
            .captures(tag)
            .map(|m| parse_int(&m[1], 0))
            .unwrap_or(0);
        // Large previews have no width/height in the tag; they are handled by
        // the large family (tried after this one), like the original.
        if width <= 0 || height <= 0 {
            continue;
        }
        let image_url = PATTERN_TAG_URL
            .captures(tag)
            .map(|m| m[1].trim().to_string())
            .unwrap_or_default();
        let x_offset = PATTERN_TAG_X_OFFSET
            .captures(tag)
            .map(|m| parse_int(&m[1], 0))
            .unwrap_or(0);
        let page_url = caps.get(1).map(|m| m.as_str().trim()).unwrap_or_default();
        set.push_preview(position, &image_url, page_url, width, height, x_offset);
    }
    if set.is_empty() {
        for caps in PATTERN_NORMAL_PREVIEW_LEGACY.captures_iter(body) {
            let position = parse_int(caps.get(6).map(|m| m.as_str()).unwrap_or("0"), 0) - 1;
            if position < 0 {
                continue;
            }
            let image_url = caps.get(3).map(|m| m.as_str().trim()).unwrap_or_default();
            let x_offset = parse_int(caps.get(4).map(|m| m.as_str()).unwrap_or("0"), 0);
            let width = parse_int(caps.get(1).map(|m| m.as_str()).unwrap_or("0"), 0);
            let height = parse_int(caps.get(2).map(|m| m.as_str()).unwrap_or("0"), 0);
            if width <= 0 || height <= 0 {
                continue;
            }
            let page_url = caps.get(5).map(|m| m.as_str().trim()).unwrap_or_default();
            set.push_preview(position, image_url, page_url, width, height, x_offset);
        }
    }
    set
}

/// Port of `parseLargePreviewSet` (new layout first, then legacy `gdtl`).
fn parse_large_preview_set(body: &str, preview_page: i32) -> PreviewSet {
    let mut set = PreviewSet::new(preview_page);
    for caps in PATTERN_PREVIEW_ANCHOR.captures_iter(body) {
        let tag = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        // New-layout large previews are the ones with a `0 0` sprite position
        // and no width/height geometry in the tag.
        if !tag.contains("0 0") || PATTERN_TAG_WIDTH.captures(tag).is_some() {
            continue;
        }
        let position = parse_int(caps.get(2).map(|m| m.as_str()).unwrap_or("0"), 0) - 1;
        if position < 0 {
            continue;
        }
        let image_url = PATTERN_TAG_URL
            .captures(tag)
            .map(|m| m[1].trim().to_string())
            .unwrap_or_default();
        let page_url = caps.get(1).map(|m| m.as_str().trim()).unwrap_or_default();
        set.push_preview(position, &image_url, page_url, 0, 0, 0);
    }
    if set.is_empty() {
        for caps in PATTERN_LARGE_PREVIEW_LEGACY.captures_iter(body) {
            let position = parse_int(caps.get(2).map(|m| m.as_str()).unwrap_or("0"), 0) - 1;
            if position < 0 {
                continue;
            }
            let image_url = caps.get(3).map(|m| m.as_str().trim()).unwrap_or_default();
            let page_url = caps.get(1).map(|m| m.as_str().trim()).unwrap_or_default();
            set.push_preview(position, image_url, page_url, 0, 0, 0);
        }
    }
    set
}

impl PreviewSet {
    /// Build a [`PreviewItem`], deriving gid/ptoken from the page URL
    /// (`.../s/{ptoken}/{gid}-{page}`), and append it — port of
    /// `NormalPreviewSet.addItem` / `LargePreviewSet.addItem`. `yoffset` is
    /// always 0 in the original parsers and is kept at 0 here.
    fn push_preview(
        &mut self,
        position: i32,
        image_url: &str,
        page_url: &str,
        xres: i32,
        yres: i32,
        xoffset: i32,
    ) {
        let (gid, token) = parse_page_url(page_url).unwrap_or((0, String::new()));
        self.rows.push(PreviewItem {
            position,
            url: image_url.to_string(),
            gid,
            token,
            xres,
            yres,
            xoffset,
            yoffset: 0,
        });
    }
}

/// Extract `(gid, ptoken)` from a gallery page URL.
pub(crate) fn parse_page_url(page_url: &str) -> Option<(u64, String)> {
    let caps = PATTERN_PAGE_URL.captures(page_url)?;
    let ptoken = caps.get(1)?.as_str().to_string();
    let gid: u64 = caps.get(2)?.as_str().parse().ok()?;
    Some((gid, ptoken))
}

/// Copy detail fields back into a [`GalleryInfo`] (list→detail enrichment,
/// mirrors how the original fills a list item from the detail page).
pub fn fill_info_from_detail(info: &mut GalleryInfo, detail: &GalleryDetail) {
    info.title = detail.info.title.clone();
    info.title_jpn = detail.info.title_jpn.clone();
    info.thumb = detail.info.thumb.clone();
    info.category = detail.info.category;
    info.uploader = detail.info.uploader.clone();
    info.posted = detail.info.posted;
    info.rating = detail.average_rating;
    info.pages = detail.info.pages;
    info.favorites = detail.favorite_count;
    info.favorite_slot = detail.info.favorite_slot;
    info.tags = detail.info.tags.clone();
}

/// Error notice on a failed comment POST (the original checks jsoup's
/// `#chd + p` selector in `EhEngine.commentGallery` and throws the text as
/// an EhException). `None` = no error notice.
pub fn parse_comment_error(body: &str) -> Option<String> {
    static CHD_P: LazyLock<scraper::Selector> = LazyLock::new(|| selector("#chd + p"));
    let document = Html::parse_document(body);
    document
        .select(&CHD_P)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}
