//! Ported from `client/parser/FavoritesParser.java`.
//!
//! Parses the favorite category counts from the `fp` divs (11 of them; the
//! last is the "fav fps" all-total), the currently selected ordering from the
//! `searchnav` select, and reuses [`super::gallery_list`] for the galleries.

use scraper::Html;

use crate::core::error::EhError;
use crate::core::models::GalleryInfo;

use super::gallery_list::parse_gallery_list;
use super::{first_by_class, first_by_tag, parse_error, parse_int, selector};

/// Port of `FavoritesParser.Result`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FavoritesResult {
    /// Favorite category display names (10 entries, "Favorites 0"...).
    pub cat_array: Vec<String>,
    /// Gallery counts per category (10 entries).
    pub count_array: Vec<i32>,
    pub pages: i32,
    pub next_page: i32,
    pub result_count: String,
    pub first_href: Option<String>,
    pub prev_href: Option<String>,
    pub next_href: Option<String>,
    pub last_href: Option<String>,
    /// Currently selected ordering (`favcat`-sort select value).
    pub fav_order: Option<String>,
    pub galleries: Vec<GalleryInfo>,
}

impl FavoritesResult {
    pub fn next_page_url(&self) -> Option<String> {
        self.next_href.clone()
    }
}

/// Parse a `favorites.php` page (port of `FavoritesParser.parse`).
pub fn parse_favorites(body: &str) -> Result<FavoritesResult, EhError> {
    if body.contains("This page requires you to log on.</p>") {
        return Err(EhError::NotSignedIn);
    }

    let document = Html::parse_document(body);
    let ido = first_by_class(document.root_element(), "ido")
        .ok_or_else(|| parse_error("Parse favorites error (no ido)", body))?;
    let fps: Vec<_> = ido.select(&selector(".fp")).collect();
    if fps.len() != 11 {
        return Err(parse_error(
            &format!("Parse favorites error ({} fp divs, want 11)", fps.len()),
            body,
        ));
    }

    let mut cat_array = Vec::with_capacity(10);
    let mut count_array = Vec::with_capacity(10);
    for fp in fps.iter().take(10) {
        let count = fp
            .child_elements()
            .next()
            .map(|c| parse_int(&c.text().collect::<String>(), 0))
            .unwrap_or(0);
        let name = fp
            .child_elements()
            .nth(2)
            .map(|c| c.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        count_array.push(count);
        cat_array.push(name);
    }

    // Selected ordering from the searchnav select.
    let fav_order = document
        .select(&selector(".searchnav"))
        .next()
        .and_then(|nav| first_by_tag(nav, "select"))
        .and_then(|sel| {
            sel.select(&selector("[selected]"))
                .next()
                .and_then(|o| o.attr("value"))
                .map(str::to_string)
        });

    let list = parse_gallery_list(body)?;
    Ok(FavoritesResult {
        cat_array,
        count_array,
        pages: list.pages,
        next_page: list.next_page,
        result_count: list.result_count,
        first_href: list.first_href,
        prev_href: list.prev_href,
        next_href: list.next_href,
        last_href: list.last_href,
        fav_order,
        galleries: list.galleries,
    })
}
