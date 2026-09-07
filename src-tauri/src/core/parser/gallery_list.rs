//! Ported from `client/parser/GalleryListParser.java`.
//!
//! Handles the extended (`gltc` table rows), thumbnail (`gl1t` divs) and
//! compact layouts, last-page detection via `ptt`, result counts via
//! `searchnav`, rating from the `ir` star sprite offsets, favorite slots from
//! the `posted_` span background color and tags from `gl3t`/`gt`/`gtl` links.

use std::sync::LazyLock;

use chrono::{TimeZone, Utc};
use regex::Regex;
use scraper::{ElementRef, Html};

use crate::core::error::EhError;
use crate::core::models::{category, GalleryInfo, TagGroup};

use super::gallery_detail::parse_tag_group_rows;
use super::{
    child_element, first_by_class, first_by_tag, parse_error, parse_f32, parse_int, selector,
};

static PATTERN_RATING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+px").unwrap());
static PATTERN_THUMB_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"height:(\d+)px;width:(\d+)px").unwrap());
static PATTERN_FAVORITE_SLOT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"background-color:rgba\((\d+),(\d+),(\d+),").unwrap());
static PATTERN_PAGES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+) page").unwrap());
static PATTERN_NEXT_PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"page=(\d+)").unwrap());
static PATTERN_NEXT_EX_PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"next=(\d+)").unwrap());
static PATTERN_RESULT_COUNT_PAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Found .* results").unwrap());

/// Favorite slot background colors, port of FAVORITE_SLOT_RGB.
const FAVORITE_SLOT_RGB: [[&str; 3]; 10] = [
    ["0", "0", "0"],
    ["240", "0", "0"],
    ["240", "160", "0"],
    ["208", "208", "0"],
    ["0", "128", "0"],
    ["144", "240", "64"],
    ["64", "176", "240"],
    ["0", "0", "240"],
    ["80", "0", "128"],
    ["224", "128", "224"],
];

/// Port of `GalleryListParser.Result`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GalleryListResult {
    /// Total number of pages (-1 = unknown, 0 = no hits, i32::MAX = error fallback).
    pub pages: i32,
    pub next_page: i32,
    pub result_count: String,
    pub first_href: Option<String>,
    pub prev_href: Option<String>,
    pub next_href: Option<String>,
    pub last_href: Option<String>,
    pub no_watched_tags: bool,
    pub galleries: Vec<GalleryInfo>,
}

impl GalleryListResult {
    /// The URL to fetch the next page, if any (original: `nextHref`, falling
    /// back to rebuilding from `page=N`).
    pub fn next_page_url(&self) -> Option<String> {
        self.next_href.clone()
    }
}

/// Parse a gallery list document (port of `GalleryListParser.parse`).
pub fn parse_gallery_list(body: &str) -> Result<GalleryListResult, EhError> {
    let doc = Html::parse_document(body);
    let mut result = GalleryListResult::default();

    // Port of the original's first try/catch: pager errors (missing ptt,
    // malformed pager, missing searchnav) enter the fallback handling instead
    // of failing the parse.
    if parse_pager(&doc, &mut result).is_err() {
        result.no_watched_tags = body.contains("<p>You do not have any watched tags");
        if body.contains("No hits found</p>") {
            result.pages = 0;
            return Ok(result);
        }
        result.pages = if doc.select(&selector(".ptt")).next().is_none() {
            1
        } else {
            i32::MAX
        };
    }

    let itg = doc
        .select(&selector(".itg"))
        .next()
        .ok_or_else(|| parse_error("Can't parse gallery list", body))?;
    let rows: Vec<ElementRef> = if itg.value().name().eq_ignore_ascii_case("table") {
        // gltc table layout: tbody holds the rows, first row is the header.
        itg.child_elements()
            .next()
            .map(|tbody| tbody.child_elements().collect())
            .unwrap_or_default()
    } else {
        // gl1t thumbnail layout: divs directly under .itg.
        itg.child_elements().collect()
    };

    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(gi) = parse_gallery_info(row) {
            list.push(gi);
        }
    }

    if list.is_empty() {
        // Port of the "No gallery" check: an `itg gltc` whose second row's
        // first cell carries a message (e.g. a login notice) may be empty;
        // anything else is a parse error, like the original.
        let has_message = doc
            .select(&selector(".itg.gltc"))
            .next()
            .and_then(|e| child_element(e, 0))
            .and_then(|e| child_element(e, 1))
            .and_then(|e| child_element(e, 0))
            .is_some_and(|td| !td.text().collect::<String>().trim().is_empty());
        if !has_message {
            return Err(parse_error("No gallery", body));
        }
    }
    result.galleries = list;
    Ok(result)
}

/// Port of the pager half of the original parse: the `ptt` table, or the
/// `searchnav` + `searchtext` watched-tags layout. `Err(())` mirrors the
/// original's first catch block being entered.
fn parse_pager(doc: &Html, result: &mut GalleryListResult) -> Result<(), ()> {
    let Some(ptt) = doc.select(&selector(".ptt")).next() else {
        // Watched/subscription layout with searchnav instead of ptt.
        result.pages = -1;
        result.next_page = -1;
        let Some(search_nav) = doc.select(&selector(".searchnav")).next() else {
            return Err(());
        };
        for (id, slot) in [
            ("uFirst", &mut result.first_href),
            ("uprev", &mut result.prev_href),
            ("unext", &mut result.next_href),
            ("ulast", &mut result.last_href),
        ] {
            if let Some(el) = search_nav.select(&selector(&format!("#{id}"))).next() {
                *slot = el.attr("href").map(str::to_string);
            }
        }
        // The watched-tags pager links use `next=N` instead of `page=N`; the
        // original reads the number from the unext href.
        if let Some(href) = result.next_href.as_deref() {
            if let Some(m) = PATTERN_NEXT_EX_PAGE.captures(href) {
                result.next_page = parse_int(&m[1], 0);
            }
        }
        if let Some(search_text) = doc.select(&selector(".searchtext")).next() {
            let text = search_text.text().collect::<String>();
            if let Some(m) = PATTERN_RESULT_COUNT_PAGE.find(&text) {
                result.result_count = format_result_count(m.as_str());
            }
        } else {
            result.result_count = String::new();
        }
        return Ok(());
    };

    let tds: Vec<ElementRef> = ptt
        .select(&selector("tr"))
        .next()
        .map(|tr| tr.child_elements().collect())
        .unwrap_or_default();
    if tds.len() < 2 {
        return Err(());
    }
    result.pages = tds[tds.len() - 2]
        .text()
        .collect::<String>()
        .trim()
        .parse()
        .map_err(|_| ())?;
    // Original: `es.get(size-1).children().first()` — the <a> is the first
    // (usually only) child element of the last pager cell.
    if let Some(a) = tds[tds.len() - 1].child_elements().next() {
        if let Some(href) = a.attr("href") {
            result.next_href = Some(href.to_string());
            if let Some(m) = PATTERN_NEXT_PAGE.captures(href) {
                result.next_page = parse_int(&m[1], 0);
            } else if let Some(m) = PATTERN_NEXT_EX_PAGE.captures(href) {
                result.next_page = parse_int(&m[1], 0);
            }
        }
    }
    Ok(())
}

/// "Found 1,234 results" → "1,234"; port of the resultCount switch.
fn format_result_count(found: &str) -> String {
    let parts: Vec<&str> = found.split(' ').collect();
    if parts.len() > 3 {
        match parts[1] {
            "thousands" => "1,000+".to_string(),
            "about" => format!("{}+", parts[2]),
            _ => parts[1..parts.len() - 1].concat(),
        }
    } else if parts.len() == 3 {
        parts[1].to_string()
    } else {
        String::new()
    }
}

/// Port of `parseRating` — the star sprite position encodes the rating:
/// `background-position:0px -21px` style, two `\d+px` numbers.
fn parse_rating(rating_style: &str) -> Option<String> {
    let mut iter = PATTERN_RATING.captures_iter(rating_style);
    let num1: i32 = iter
        .next()
        .and_then(|c| c.get(0))
        .map(|m| m.as_str().replace("px", "").parse().unwrap_or(i32::MIN))
        .unwrap_or(i32::MIN);
    let num2: i32 = iter
        .next()
        .and_then(|c| c.get(0))
        .map(|m| m.as_str().replace("px", "").parse().unwrap_or(i32::MIN))
        .unwrap_or(i32::MIN);
    if num1 == i32::MIN || num2 == i32::MIN {
        return None;
    }
    let mut rate = 5 - num1 / 16;
    if num2 == 21 {
        rate -= 1;
        Some(format!("{rate}.5"))
    } else {
        Some(rate.to_string())
    }
}

/// Port of `parseFavoriteSlot` — `background-color:rgba(r,g,b,` matched
/// against FAVORITE_SLOT_RGB. Returns -2 when unknown.
fn parse_favorite_slot(style: &str) -> i32 {
    if let Some(m) = PATTERN_FAVORITE_SLOT.captures(style) {
        for (slot, rgb) in FAVORITE_SLOT_RGB.iter().enumerate() {
            if &m[1] == rgb[0] && &m[2] == rgb[1] && &m[3] == rgb[2] {
                return slot as i32;
            }
        }
    }
    -2
}

/// Parse one gallery row (port of `parseGalleryInfo`).
fn parse_gallery_info(e: ElementRef) -> Option<GalleryInfo> {
    let mut gi = GalleryInfo {
        favorite_slot: -2,
        ..GalleryInfo::default()
    };

    // Title, gid, token (required), tags
    let glname = first_by_class(e, "glname")?;
    {
        let mut a = first_by_tag(glname, "a");
        if a.is_none() {
            if let Some(parent) = glname.parent().and_then(ElementRef::wrap) {
                if parent.value().name().eq_ignore_ascii_case("a") {
                    a = Some(parent);
                }
            }
        }
        if let Some(a) = a {
            if let Some(href) = a.attr("href") {
                if let Some(url) = super::gallery_detail::parse_gallery_detail_url(href) {
                    gi.gid = url.gid;
                    gi.token = url.token;
                }
            }
        }

        // Deepest first-descendant chain gives the title text.
        let mut child = glname;
        while let Some(first) = child.child_elements().next() {
            child = first;
        }
        gi.title = child.text().collect::<String>().trim().to_string();

        if let Some(tbody) = first_by_tag(glname, "tbody") {
            let groups: Vec<TagGroup> = parse_tag_group_rows(tbody.child_elements());
            gi.tags = groups
                .into_iter()
                .flat_map(|g| {
                    let ns = g.namespace.clone();
                    g.tags.into_iter().map(move |t| TagGroup {
                        namespace: ns.clone(),
                        tags: vec![t],
                    })
                })
                .collect();
        }
    }
    if gi.title.is_empty() {
        return None;
    }

    // Category
    let ce = first_by_class(e, "cn").or_else(|| first_by_class(e, "cs"));
    if let Some(ce) = ce {
        gi.category = category::get_category(ce.text().collect::<String>().trim());
    }

    // Thumb (extended layout)
    let mut thumb_found = false;
    if let Some(gl_thumb) = first_by_class(e, "glthumb") {
        if let Some(img) = gl_thumb.select(&selector("div:nth-child(1)>img")).next() {
            thumb_found = true;
            if let Some(m) = PATTERN_THUMB_SIZE.captures(img.attr("style").unwrap_or_default()) {
                gi.thumb_height = parse_int(&m[1], 0);
                gi.thumb_width = parse_int(&m[2], 0);
            }
            let url = img
                .attr("data-src")
                .filter(|s| !s.is_empty())
                .or_else(|| img.attr("src"))
                .unwrap_or_default();
            gi.thumb = url.to_string();
        }
        // Pages: div:nth-child(2)>div:nth-child(2)>div:nth-child(2)
        if let Some(div) = gl_thumb
            .select(&selector(
                "div:nth-child(2)>div:nth-child(2)>div:nth-child(2)",
            ))
            .next()
        {
            let text = div.text().collect::<String>();
            if let Some(m) = PATTERN_PAGES.captures(&text) {
                gi.pages = parse_int(&m[1], 0);
            }
        }
    }
    // Thumbnail layout fallback
    if !thumb_found || gi.thumb.is_empty() {
        let gl = first_by_class(e, "gl1e").or_else(|| first_by_class(e, "gl3t"));
        if let Some(gl) = gl {
            if let Some(img) = first_by_tag(gl, "img") {
                if let Some(m) = PATTERN_THUMB_SIZE.captures(img.attr("style").unwrap_or_default())
                {
                    gi.thumb_height = parse_int(&m[1], 0);
                    gi.thumb_width = parse_int(&m[2], 0);
                }
                gi.thumb = img.attr("src").unwrap_or_default().to_string();
            }
        }
    }

    // Posted + favorite slot
    if let Some(posted) = e.select(&selector(&format!("#posted_{}", gi.gid))).next() {
        let text = posted.text().collect::<String>();
        gi.posted = parse_posted(text.trim());
        gi.favorite_slot = parse_favorite_slot(posted.attr("style").unwrap_or_default());
    }
    // NOTE: the original falls back to a local-DB favorites lookup here; the
    // A local-favorites feature would apply that after parsing the list.

    // Rating
    if let Some(ir) = first_by_class(e, "ir") {
        if let Some(style) = ir.attr("style") {
            gi.rating = parse_rating(style).map_or(-1.0, |r| parse_f32(&r, -1.0));
        }
        gi.rated =
            (ir.value()
                .has_class("irr", scraper::CaseSensitivity::CaseSensitive)
                || ir
                    .value()
                    .has_class("irg", scraper::CaseSensitivity::CaseSensitive)
                || ir
                    .value()
                    .has_class("irb", scraper::CaseSensitivity::CaseSensitive)) as i64;
    }

    // Uploader and pages (extended layout)
    let mut gl = first_by_class(e, "glhide");
    let (mut uploader_index, mut pages_index) = (0usize, 1usize);
    if gl.is_none() {
        gl = first_by_class(e, "gl3e");
        uploader_index = 3;
        pages_index = 4;
    }
    if let Some(gl) = gl {
        let children: Vec<ElementRef> = gl.child_elements().collect();
        if children.len() > uploader_index {
            if let Some(a) = child_element(children[uploader_index], 0) {
                gi.uploader = Some(a.text().collect::<String>().trim().to_string());
            }
        }
        if children.len() > pages_index {
            let text = children[pages_index].text().collect::<String>();
            if let Some(m) = PATTERN_PAGES.captures(&text) {
                gi.pages = parse_int(&m[1], 0);
            }
        }
    }
    // Pages (thumbnail layout)
    if let Some(gl5t) = first_by_class(e, "gl5t") {
        if let Some(div) = gl5t
            .select(&selector("div:nth-child(2)>div:nth-child(2)"))
            .next()
        {
            let text = div.text().collect::<String>();
            if let Some(m) = PATTERN_PAGES.captures(&text) {
                gi.pages = parse_int(&m[1], 0);
            }
        }
    }

    Some(gi)
}

/// "2021-01-01 12:34" (EH list `posted` format) → UTC timestamp.
pub(crate) fn parse_posted(text: &str) -> chrono::DateTime<Utc> {
    for fmt in ["%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(text, fmt) {
            return Utc.from_utc_datetime(&dt);
        }
    }
    Utc.timestamp_opt(0, 0).unwrap()
}
