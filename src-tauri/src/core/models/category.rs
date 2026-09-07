//! Category bitflags, names, colors and parsing.
//!
//! Ported from `client/EhConfig.java` (constants) and `client/EhUtils.kt`
//! (`getCategory(String)`, `getCategory(int)`, `getCategoryColor(int)`).

/// Unknown category (0x400). Used when the category cell cannot be parsed.
pub const UNKNOWN: i32 = 0x400;
/// Sentinel for "no category filter" (homepage). Not a real category.
pub const NONE: i32 = -1;
/// All-category mask.
pub const ALL_CATEGORY: i32 = 0x3ff;

pub const MISC: i32 = 0x1;
pub const DOUJINSHI: i32 = 0x2;
pub const MANGA: i32 = 0x4;
pub const ARTIST_CG: i32 = 0x8;
pub const GAME_CG: i32 = 0x10;
pub const IMAGE_SET: i32 = 0x20;
pub const COSPLAY: i32 = 0x40;
pub const ASIAN_PORN: i32 = 0x80;
pub const NON_H: i32 = 0x100;
pub const WESTERN: i32 = 0x200;

/// (bitflag, parsing aliases, display name) — mirrors CATEGORY_VALUES /
/// CATEGORY_STRINGS / CATEGORY_NAMES in EhUtils.kt.
const CATEGORIES: &[(i32, &[&str], &str)] = &[
    (MISC, &["misc"], "Misc"),
    (DOUJINSHI, &["doujinshi"], "Doujinshi"),
    (MANGA, &["manga"], "Manga"),
    (
        ARTIST_CG,
        &["artistcg", "Artist CG Sets", "Artist CG"],
        "Artist CG",
    ),
    (GAME_CG, &["gamecg", "Game CG Sets", "Game CG"], "Game CG"),
    (
        IMAGE_SET,
        &["imageset", "Image Sets", "Image Set"],
        "Image Set",
    ),
    (COSPLAY, &["cosplay"], "Cosplay"),
    (ASIAN_PORN, &["asianporn", "Asian Porn"], "Asian Porn"),
    (NON_H, &["non-h"], "Non-H"),
    (WESTERN, &["western"], "Western"),
];

/// Background color hints, ported 1:1 from `BG_COLOR_*` in EhUtils.kt
/// (Android `Color.argb(255, r, g, b)` packed as u32).
pub const COLOR_DOUJINSHI: u32 = 0xFFF44436;
pub const COLOR_MANGA: u32 = 0xFFFFF800;
pub const COLOR_ARTIST_CG: u32 = 0xFFBC022D;
pub const COLOR_GAME_CG: u32 = 0xFF4CAF50;
pub const COLOR_WESTERN: u32 = 0xFF8BC34A;
pub const COLOR_NON_H: u32 = 0xFF2196F3;
pub const COLOR_IMAGE_SET: u32 = 0xFF3F51B5;
pub const COLOR_COSPLAY: u32 = 0xFF9C27B0;
pub const COLOR_ASIAN_PORN: u32 = 0xFF9575CD;
pub const COLOR_MISC: u32 = 0xFFF06292;
pub const COLOR_UNKNOWN: u32 = 0xFF000000;

/// Port of `EhUtils.getCategory(String)` — resolve a category cell text to its
/// bitflag. Returns [`UNKNOWN`] when nothing matches.
pub fn get_category<S: AsRef<str>>(name: S) -> i32 {
    let name = name.as_ref();
    for &(value, aliases, _) in CATEGORIES {
        if aliases.iter().any(|a| a.eq_ignore_ascii_case(name)) {
            return value;
        }
    }
    UNKNOWN
}

/// Display name for a category bitflag, exactly like
/// `categoryName()` in the original (Doujinshi/Manga/Artist CG/...).
/// Returns `"Unknown"` for unrecognized values.
pub fn category_name(category: i32) -> &'static str {
    for &(value, _, display) in CATEGORIES {
        if value == category {
            return display;
        }
    }
    "Unknown"
}

/// Color hint for a category bitflag (port of `EhUtils.getCategoryColor`).
pub fn category_color(category: i32) -> u32 {
    match category {
        DOUJINSHI => COLOR_DOUJINSHI,
        MANGA => COLOR_MANGA,
        ARTIST_CG => COLOR_ARTIST_CG,
        GAME_CG => COLOR_GAME_CG,
        WESTERN => COLOR_WESTERN,
        NON_H => COLOR_NON_H,
        IMAGE_SET => COLOR_IMAGE_SET,
        COSPLAY => COLOR_COSPLAY,
        ASIAN_PORN => COLOR_ASIAN_PORN,
        MISC => COLOR_MISC,
        _ => COLOR_UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_roundtrip() {
        for &(value, aliases, display) in CATEGORIES {
            assert_eq!(get_category(aliases[0]), value);
            assert_eq!(get_category(display), value, "display name {display}");
            assert_eq!(category_name(value), display);
        }
        assert_eq!(get_category("whatever"), UNKNOWN);
        assert_eq!(category_name(0x9999), "Unknown");
    }

    #[test]
    fn color_hints_cover_all_categories() {
        for &(value, _, _) in CATEGORIES {
            assert_ne!(category_color(value), COLOR_UNKNOWN);
        }
        assert_eq!(category_color(UNKNOWN), COLOR_UNKNOWN);
    }
}
