//! Parses the `gallerypopups.php?act=addfav` form that the original
//! `EhEngine.addFavorites` GETs before posting changes back.
//!
//! The form carries both pieces of per-gallery favorite state the site will
//! accept on the following POST: the checked `favcat` radio and the current
//! `favnote` text. Reading them together means one request answers "which
//! collection is this in" and "what note does it carry".

use scraper::Html;

use super::selector;

/// Current favorite state of one gallery as the addfav popup renders it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FavoritePopup {
    /// Checked `favcat` collection slot. `None` when the gallery is not
    /// favorited, or when the checked radio is the `favdel` pseudo-slot.
    pub slot: Option<i32>,
    /// Contents of the `favnote` field; empty when the field is absent.
    pub note: String,
}

/// Parse an addfav popup body. Markup drift yields an empty result rather
/// than an error: callers treat "no state" and "not favorited" alike.
pub fn parse_favorite_popup(body: &str) -> FavoritePopup {
    let document = Html::parse_document(body);

    let slot = document
        .select(&selector(r#"input[name="favcat"][checked]"#))
        .next()
        .and_then(|input| input.attr("value"))
        .and_then(|value| value.parse().ok());

    let note = document
        .select(&selector(r#"textarea[name="favnote"]"#))
        .next()
        .map(|area| area.text().collect::<String>())
        .unwrap_or_default();

    FavoritePopup { slot, note }
}
