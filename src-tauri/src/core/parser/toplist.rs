//! Ported from `client/parser/TopListParser.java`.
//!
//! Parses the seven top lists (`GalleryList`, `Uploader`, `Tagging`,
//! `Hentai@Home`, `EHTracker`, `Cleanup`, `Rating+Reviewing`) from
//! `toplist.php`, each with All Time / Past Year / Past Month / Yesterday
//! tables of 10 `tun` rows.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{ElementRef, Html};

use crate::core::error::EhError;
use crate::core::models::{TopListDetail, TopListInfo, TopListItem, TopListKind};

use super::{child_element, parse_error, selector};

static PATTERN_ERROR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<div class="d">\s*<p>([^<]+)</p>"#).unwrap());

const OFFENSIVE_STRING: &str =
    "<p>(And if you choose to ignore this warning, you lose all rights to complain about it in the future.)</p>";
const PINING_STRING: &str = "<p>This gallery is pining for the fjords.</p>";

/// Parse `toplist.php` (port of `TopListParser.parse`).
pub fn parse_toplist(body: &str) -> Result<TopListDetail, EhError> {
    if body.contains(OFFENSIVE_STRING) {
        return Err(EhError::Eh("offensive content warning".to_string()));
    }
    if body.contains(PINING_STRING) {
        return Err(EhError::Eh(
            "this gallery is pining for the fjords".to_string(),
        ));
    }
    if let Some(m) = PATTERN_ERROR.captures(body) {
        return Err(EhError::Eh(m[1].trim().to_string()));
    }

    let document = Html::parse_document(body);
    let ido = document
        .select(&selector(".ido"))
        .next()
        .ok_or_else(|| parse_error("Can't parse toplist (no ido)", body))?;
    let children: Vec<ElementRef> = ido.child_elements().collect();
    if children.len() < 14 {
        return Err(parse_error("Can't parse toplist (short ido)", body));
    }

    // children: 0 title, 1 table, 2 spacer, 3 table, ...
    let title = children[0].text().collect::<String>().trim().to_string();
    let kinds = [
        (1, TopListKind::Gallery),
        (3, TopListKind::Uploader),
        (5, TopListKind::Tagging),
        (7, TopListKind::HentaiHome),
        (9, TopListKind::EhTracker),
        (11, TopListKind::CleanUp),
        (13, TopListKind::RatingAndReviewing),
    ];
    let mut detail = TopListDetail {
        title,
        gallery: TopListInfo {
            kind: TopListKind::Gallery,
            ..TopListInfo::default()
        },
        uploader: TopListInfo {
            kind: TopListKind::Uploader,
            ..TopListInfo::default()
        },
        tagging: TopListInfo {
            kind: TopListKind::Tagging,
            ..TopListInfo::default()
        },
        hentai_home: TopListInfo {
            kind: TopListKind::HentaiHome,
            ..TopListInfo::default()
        },
        eh_tracker: TopListInfo {
            kind: TopListKind::EhTracker,
            ..TopListInfo::default()
        },
        clean_up: TopListInfo {
            kind: TopListKind::CleanUp,
            ..TopListInfo::default()
        },
        rating_and_reviewing: TopListInfo {
            kind: TopListKind::RatingAndReviewing,
            ..TopListInfo::default()
        },
    };
    for (index, kind) in kinds {
        let info = parse_info(children[index], kind);
        match kind {
            TopListKind::Gallery => detail.gallery = info,
            TopListKind::Uploader => detail.uploader = info,
            TopListKind::Tagging => detail.tagging = info,
            TopListKind::HentaiHome => detail.hentai_home = info,
            TopListKind::EhTracker => detail.eh_tracker = info,
            TopListKind::CleanUp => detail.clean_up = info,
            TopListKind::RatingAndReviewing => detail.rating_and_reviewing = info,
        }
    }
    Ok(detail)
}

/// Port of `parseInfo` — the list div's children are: 0 header, 1 all-time,
/// 2 past-year, 3 past-month, 4 yesterday tables. The original walks
/// `table.child(1).child(0)` (tbody, then tr, given the whitespace text nodes
/// in the real markup); here that is the robust element-only equivalent.
fn parse_info(element: ElementRef, kind: TopListKind) -> TopListInfo {
    let rows: Vec<ElementRef> = element.child_elements().collect();
    let mut info = TopListInfo {
        kind,
        ..TopListInfo::default()
    };
    let sections = [
        (1, &mut info.all_time),
        (2, &mut info.past_year),
        (3, &mut info.past_month),
        (4, &mut info.yesterday),
    ];
    for (index, target) in sections {
        if let Some(tr) = rows
            .get(index)
            .and_then(|table| child_element(*table, 0))
            .and_then(|tbody| child_element(tbody, 0))
        {
            *target = parse_array(tr);
        }
    }
    info
}

/// Port of `parseArray` — the 10 `.tun` rows.
fn parse_array(ele: ElementRef) -> Vec<TopListItem> {
    ele.select(&selector(".tun"))
        .enumerate()
        .filter_map(|(i, tun)| {
            tun.child_elements().next().map(|item| TopListItem {
                value: item.text().collect::<String>().trim().to_string(),
                href: item.attr("href").unwrap_or_default().to_string(),
                rank: i as i32 + 1,
            })
        })
        .collect()
}
