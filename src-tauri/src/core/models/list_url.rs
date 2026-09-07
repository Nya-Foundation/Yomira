//! Port of `client/data/ListUrlBuilder.java` + `client/data/FavListUrlBuilder.java`
//! (+ `wrapTagKeyword` from `GalleryListScene.java` for the `$` exact-match
//! formatting of tag searches).

use crate::core::client::url::{self, Site};

use super::category;

/// Advanced-search bitflags, ported from `widget/AdvanceSearchTable.java`.
pub mod adv {
    pub const SNAME: i32 = 0x1; // f_sname — search gallery names
    pub const STAGS: i32 = 0x2; // f_stags — search gallery tags
    pub const SDESC: i32 = 0x4; // f_sdesc — search gallery descriptions
    pub const STORR: i32 = 0x8; // f_storr — search torrent filenames
    pub const STO: i32 = 0x10; // f_sto — use the tag repository (on = no)
    pub const SDT1: i32 = 0x20; // f_sdt1 — search low-power tags
    pub const SDT2: i32 = 0x40; // f_sdt2 — search downvoted tags
    pub const SH: i32 = 0x80; // f_sh — search expunged galleries
    pub const SFL: i32 = 0x100; // f_sfl — only show galleries with torrents
    pub const SFU: i32 = 0x200; // f_sfu — search uploader names/comments
    pub const SFT: i32 = 0x400; // f_sft — search uploader names only
    pub const DEFAULT: i32 = SNAME | STAGS;
}

/// Which list a [`ListUrlBuilder`] points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMode {
    /// Front-page style search (MODE_NORMAL).
    Normal,
    /// `/uploader/{name}[/{page}]` (MODE_UPLOADER).
    Uploader,
    /// `/tag/{name}[/{page}]` (MODE_TAG).
    Tag,
    /// `/popular` (MODE_WHATS_HOT).
    Popular,
    /// `/watched` (MODE_SUBSCRIPTION).
    Watched,
    /// `toplist.php?{tl_arg}[&p=N]` (MODE_TOP_LIST).
    TopList,
    /// `favorites.php[?favcat=N...]`.
    Favorites,
}

/// `advsearch` section of the search request.
///
/// Serialized exactly like the original: `advsearch=1`, then one `f_xxx=on`
/// per enabled flag, then `f_sr=on&f_srdd=N` when `min_rating != -1`, then
/// `f_sp=on&f_spf=A&f_spt=B` when a page range is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedSearch {
    pub search_name: bool,
    pub search_tags: bool,
    pub search_descr: bool,
    pub search_torrent: bool,
    /// `f_sto` — "off" checkbox in the original UI; `true` means skip the
    /// tag repository.
    pub use_tag_repo: bool,
    /// `f_sdt1` — the original UI calls this "search low-power tags".
    pub search_lowpower_tags: bool,
    /// `f_sdt2` — search downvoted tags.
    pub search_downvoted_tags: bool,
    /// `f_sh` — search expunged galleries.
    pub search_expunged: bool,
    /// -1 = disabled (f_srdd).
    pub min_rating: i32,
    /// -1 = disabled (f_spf).
    pub page_from: i32,
    /// -1 = disabled (f_spt).
    pub page_to: i32,
}

impl Default for AdvancedSearch {
    fn default() -> Self {
        Self {
            search_name: true,
            search_tags: true,
            search_descr: false,
            search_torrent: false,
            use_tag_repo: false,
            search_lowpower_tags: false,
            search_downvoted_tags: false,
            search_expunged: false,
            min_rating: -1,
            page_from: -1,
            page_to: -1,
        }
    }
}

/// One term of a tag-set (`f_search`) query.
///
/// Formatting rules (ported from `GalleryListScene.wrapTagKeyword` plus the
/// documented e-hentai search syntax):
/// * plain: `name` or `namespace:name`
/// * exact: `name$`, `namespace:"name$"` — `$` marks exact matching
/// * excluded: `-name` / `-namespace:name`
/// * weak excluded: `~-name` / `~-namespace:name` (down-ranks instead of
///   excluding; prefix is a documented assumption, see the project report)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTerm {
    pub namespace: Option<String>,
    pub name: String,
    pub exact: bool,
    pub excluded: bool,
    pub weak_excluded: bool,
}

impl TagTerm {
    pub fn tag(name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            name: name.into(),
            exact: false,
            excluded: false,
            weak_excluded: false,
        }
    }

    pub fn namespaced(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            ..Self::tag(name)
        }
    }

    pub fn exact(mut self) -> Self {
        self.exact = true;
        self
    }

    pub fn excluded(mut self) -> Self {
        self.excluded = true;
        self
    }

    pub fn weak_excluded(mut self) -> Self {
        self.weak_excluded = true;
        self
    }

    fn write_into(&self, out: &mut String) {
        if self.weak_excluded {
            // `~` weakens the exclusion that follows it.
            out.push('~');
        }
        if self.excluded || self.weak_excluded {
            out.push('-');
        }
        if let Some(ns) = &self.namespace {
            out.push_str(ns);
            out.push(':');
        }
        if self.exact {
            // wrapTagKeyword: namespace:"the rest$"
            out.push('"');
            out.push_str(&self.name);
            out.push('$');
            out.push('"');
        } else {
            out.push_str(&self.name);
        }
    }
}

/// Format a tag-set into the `f_search` value (space separated).
pub fn format_tag_set(terms: &[TagTerm]) -> String {
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        t.write_into(&mut out);
    }
    out
}

/// Minimal ordered port of `com.hippo.network.UrlBuilder`. Unlike the
/// original (a Java HashMap) the query parameter order is deterministic and
/// follows insertion order.
#[derive(Debug, Default)]
pub struct UrlBuilder {
    root: String,
    queries: Vec<(String, String)>,
}

impl UrlBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            queries: Vec::new(),
        }
    }

    pub fn add_query(&mut self, key: impl Into<String>, value: impl ToString) {
        self.queries.push((key.into(), value.to_string()));
    }

    pub fn build(&self) -> String {
        if self.queries.is_empty() {
            return self.root.clone();
        }
        let mut sb = String::with_capacity(self.root.len() + self.queries.len() * 8 + 1);
        sb.push_str(&self.root);
        sb.push('?');
        for (i, (k, v)) in self.queries.iter().enumerate() {
            if i > 0 {
                sb.push('&');
            }
            sb.push_str(k);
            sb.push('=');
            sb.push_str(v);
        }
        sb
    }
}

/// Java `URLEncoder.encode(s, "UTF-8")` equivalent: unreserved chars
/// `[a-zA-Z0-9.\-*_]`, space as `+`, everything else percent-escaped UTF-8
/// bytes (e-hentai decodes `+` as space in f_search, so this matters).
pub fn java_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0xf) as usize] as char);
            }
        }
    }
    out
}

/// Port of `ListUrlBuilder`/`FavListUrlBuilder` — builds gallery-list URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUrlBuilder {
    pub mode: ListMode,
    pub page: i32,
    /// Category bitflags or `category::NONE`.
    pub category: i32,
    /// `f_search` keyword (already formatted, or a raw string for tag sets).
    pub keyword: Option<String>,
    /// Tag-set mode keyword (formatted via [`format_tag_set`]).
    pub tag_set: Vec<TagTerm>,
    /// Advanced search; `None` = `advsearch` disabled.
    pub advanced: Option<AdvancedSearch>,
    /// Favorites mode: -1 = all categories, 0-9 = a single category.
    pub fav_cat: i32,
    /// Favorites mode: cloud sort order from the `favcat`-sort dropdown.
    /// "f" = favorited time, "p" = posted time (port of
    /// `EhConfig.ORDER_BY_FAV_TIME`/`ORDER_BY_PUB_TIME` and the
    /// `FavoriteListSortDialog` → `FavoritesScene.updateSort` flow); sent as
    /// `inline_set=fs_f` / `inline_set=fs_p` like the original appends it.
    pub fav_sort: Option<String>,
    /// Top-list mode: the `tl=xx` argument (e.g. "tl=11").
    pub toplist_arg: String,
}

impl Default for ListUrlBuilder {
    fn default() -> Self {
        Self {
            mode: ListMode::Normal,
            page: 0,
            category: category::NONE,
            keyword: None,
            tag_set: Vec::new(),
            advanced: None,
            fav_cat: -1,
            fav_sort: None,
            toplist_arg: String::new(),
        }
    }
}

impl ListUrlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Make the builder point to the homepage again (port of `reset()`).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Build the absolute URL for the current state (port of `build()`).
    pub fn build(&self, site: Site) -> String {
        match self.mode {
            ListMode::Normal | ListMode::Watched => {
                let root = match self.mode {
                    ListMode::Watched => url::watched_url(site),
                    _ => url::host(site),
                };
                let mut ub = UrlBuilder::new(root);
                if self.category != category::NONE {
                    ub.add_query("f_cats", (!self.category) & category::ALL_CATEGORY);
                }
                let keyword = self.formatted_keyword();
                if let Some(kw) = keyword {
                    let kw = kw.trim();
                    if !kw.is_empty() {
                        ub.add_query("f_search", java_url_encode(kw));
                    }
                }
                if self.page != 0 {
                    ub.add_query("page", self.page);
                }
                if let Some(adv) = &self.advanced {
                    ub.add_query("advsearch", "1");
                    if adv.search_name {
                        ub.add_query("f_sname", "on");
                    }
                    if adv.search_tags {
                        ub.add_query("f_stags", "on");
                    }
                    if adv.search_descr {
                        ub.add_query("f_sdesc", "on");
                    }
                    if adv.search_torrent {
                        ub.add_query("f_storr", "on");
                    }
                    if adv.use_tag_repo {
                        ub.add_query("f_sto", "on");
                    }
                    if adv.search_lowpower_tags {
                        ub.add_query("f_sdt1", "on");
                    }
                    if adv.search_downvoted_tags {
                        ub.add_query("f_sdt2", "on");
                    }
                    if adv.search_expunged {
                        ub.add_query("f_sh", "on");
                    }
                    if adv.min_rating != -1 {
                        ub.add_query("f_sr", "on");
                        ub.add_query("f_srdd", adv.min_rating);
                    }
                    if adv.page_from != -1 || adv.page_to != -1 {
                        ub.add_query("f_sp", "on");
                        ub.add_query(
                            "f_spf",
                            if adv.page_from != -1 {
                                adv.page_from.to_string()
                            } else {
                                String::new()
                            },
                        );
                        ub.add_query(
                            "f_spt",
                            if adv.page_to != -1 {
                                adv.page_to.to_string()
                            } else {
                                String::new()
                            },
                        );
                    }
                }
                ub.build()
            }
            ListMode::Uploader | ListMode::Tag => {
                let mut sb = String::from(url::host(site));
                sb.push_str(match self.mode {
                    ListMode::Uploader => "uploader/",
                    _ => "tag/",
                });
                sb.push_str(&java_url_encode(
                    self.keyword.as_deref().unwrap_or_default(),
                ));
                if self.page != 0 {
                    sb.push('/');
                    sb.push_str(&self.page.to_string());
                }
                sb
            }
            ListMode::Popular => url::popular_url(site).to_string(),
            ListMode::TopList => {
                let mut sb = String::from(url::toplist_url(site));
                sb.push('?');
                sb.push_str(&self.toplist_arg);
                if self.page > 0 && self.page < 200 {
                    sb.push_str("&p=");
                    sb.push_str(&self.page.to_string());
                }
                sb
            }
            ListMode::Favorites => {
                // Port of FavListUrlBuilder.build()
                let mut ub = UrlBuilder::new(url::favorites_url(site));
                if (0..=9).contains(&self.fav_cat) {
                    ub.add_query("favcat", self.fav_cat);
                }
                let keyword = self.formatted_keyword();
                if let Some(kw) = keyword.filter(|k| !k.is_empty()) {
                    ub.add_query("f_search", java_url_encode(&kw));
                    ub.add_query("sn", "on");
                    ub.add_query("st", "on");
                    ub.add_query("sf", "on");
                }
                if self.page > 0 {
                    ub.add_query("page", self.page);
                }
                // FavoriteListSortDialog → FavoritesScene.updateSort appends
                // "inline_set=fs_f" (favorited time) or "inline_set=fs_p"
                // (posted time) to the favorites.php URL.
                if let Some(sort) = &self.fav_sort {
                    if sort == "f" || sort == "p" {
                        ub.add_query("inline_set", format!("fs_{sort}"));
                    }
                }
                ub.build()
            }
        }
    }

    fn formatted_keyword(&self) -> Option<String> {
        if !self.tag_set.is_empty() {
            Some(format_tag_set(&self.tag_set))
        } else {
            self.keyword.clone()
        }
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn normal_keyword_and_categories() {
        let mut b = ListUrlBuilder::new();
        b.category = category::DOUJINSHI | category::MANGA;
        b.keyword = Some("full color".to_string());
        let url = b.build(Site::E);
        assert_eq!(url, "https://e-hentai.org/?f_cats=1017&f_search=full+color");
    }

    #[test]
    fn normal_paginated_no_category() {
        let mut b = ListUrlBuilder::new();
        b.page = 3;
        let url = b.build(Site::Ex);
        assert_eq!(url, "https://exhentai.org/?page=3");
    }

    #[test]
    fn advanced_search_flags() {
        let mut b = ListUrlBuilder::new();
        b.keyword = Some("touhou".to_string());
        b.advanced = Some(AdvancedSearch {
            search_name: true,
            search_tags: false,
            search_descr: true,
            search_torrent: true,
            use_tag_repo: true,
            search_lowpower_tags: true,
            search_downvoted_tags: true,
            search_expunged: true,
            min_rating: 4,
            page_from: 10,
            page_to: 20,
        });
        let url = b.build(Site::E);
        assert_eq!(
            url,
            "https://e-hentai.org/?f_search=touhou&advsearch=1&f_sname=on&f_sdesc=on&f_storr=on&f_sto=on&f_sdt1=on&f_sdt2=on&f_sh=on&f_sr=on&f_srdd=4&f_sp=on&f_spf=10&f_spt=20"
        );
    }

    #[test]
    fn advanced_search_downvoted_and_expunged_flags() {
        let mut b = ListUrlBuilder::new();
        b.advanced = Some(AdvancedSearch {
            search_downvoted_tags: true,
            ..AdvancedSearch::default()
        });
        assert_eq!(
            b.build(Site::Ex),
            "https://exhentai.org/?advsearch=1&f_sname=on&f_stags=on&f_sdt2=on"
        );

        let mut b = ListUrlBuilder::new();
        b.advanced = Some(AdvancedSearch {
            search_expunged: true,
            ..AdvancedSearch::default()
        });
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/?advsearch=1&f_sname=on&f_stags=on&f_sh=on"
        );

        // Defaults stay off.
        let b = ListUrlBuilder {
            advanced: Some(AdvancedSearch::default()),
            ..Default::default()
        };
        let url = b.build(Site::E);
        assert!(!url.contains("f_sdt2"), "{url}");
        assert!(!url.contains("f_sh"), "{url}");
    }

    #[test]
    fn advanced_search_open_ended_page_range() {
        let mut b = ListUrlBuilder::new();
        b.advanced = Some(AdvancedSearch {
            page_from: 5,
            page_to: -1,
            ..AdvancedSearch::default()
        });
        let url = b.build(Site::E);
        assert_eq!(
            url,
            "https://e-hentai.org/?advsearch=1&f_sname=on&f_stags=on&f_sp=on&f_spf=5&f_spt="
        );
    }

    #[test]
    fn tag_set_formatting() {
        let terms = vec![
            TagTerm::namespaced("female", "sole female"),
            TagTerm::namespaced("parody", "touhou project").exact(),
            TagTerm::tag("ffm threesome").exact(),
            TagTerm::tag("males only").excluded(),
            TagTerm::namespaced("artist", "someone").weak_excluded(),
        ];
        let kw = format_tag_set(&terms);
        assert_eq!(
            kw,
            "female:sole female parody:\"touhou project$\" \"ffm threesome$\" -males only ~-artist:someone"
        );
    }

    #[test]
    fn tag_set_url_uses_f_search() {
        let mut b = ListUrlBuilder::new();
        b.tag_set = vec![
            TagTerm::namespaced("female", "sole female"),
            TagTerm::tag("big breasts").excluded(),
        ];
        let url = b.build(Site::Ex);
        assert_eq!(
            url,
            "https://exhentai.org/?f_search=female%3Asole+female+-big+breasts"
        );
    }

    #[test]
    fn top_list_mode() {
        let mut b = ListUrlBuilder::new();
        b.mode = ListMode::TopList;
        b.toplist_arg = "tl=11".to_string();
        b.page = 2;
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/toplist.php?tl=11&p=2"
        );
        b.page = 0;
        assert_eq!(b.build(Site::E), "https://e-hentai.org/toplist.php?tl=11");
    }

    #[test]
    fn favorites_mode() {
        let mut b = ListUrlBuilder::new();
        b.mode = ListMode::Favorites;
        b.fav_cat = 3;
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/favorites.php?favcat=3"
        );

        b.keyword = Some("miku".to_string());
        b.page = 1;
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/favorites.php?favcat=3&f_search=miku&sn=on&st=on&sf=on&page=1"
        );

        b.fav_cat = -1;
        let url = b.build(Site::E);
        assert!(!url.contains("favcat"), "{url}");
    }

    #[test]
    fn favorites_sort_orders() {
        // Port of FavoriteListSortDialog: sort by favorite time appends
        // "inline_set=fs_f", by posted time "inline_set=fs_p"
        // (EhConfig.ORDER_BY_FAV_TIME = "f", ORDER_BY_PUB_TIME = "p").
        for (sort, expect) in [("f", "inline_set=fs_f"), ("p", "inline_set=fs_p")] {
            let mut b = ListUrlBuilder::new();
            b.mode = ListMode::Favorites;
            b.fav_cat = 2;
            b.page = 1;
            b.fav_sort = Some(sort.to_string());
            let url = b.build(Site::E);
            assert_eq!(
                url,
                format!("https://e-hentai.org/favorites.php?favcat=2&page=1&{expect}")
            );
        }

        // Unknown sort codes are ignored (the favorites parser only ever
        // reports "f"/"p" from the dropdown's selected option).
        let mut b = ListUrlBuilder::new();
        b.mode = ListMode::Favorites;
        b.fav_sort = Some("bogus".to_string());
        assert_eq!(b.build(Site::E), "https://e-hentai.org/favorites.php");

        // No sort → no param, back-compat with the current frontend call.
        let b = ListUrlBuilder {
            mode: ListMode::Favorites,
            ..Default::default()
        };
        assert_eq!(b.build(Site::E), "https://e-hentai.org/favorites.php");
    }

    #[test]
    fn popular_and_watched() {
        let b = ListUrlBuilder {
            mode: ListMode::Popular,
            ..Default::default()
        };
        assert_eq!(b.build(Site::E), "https://e-hentai.org/popular");
        assert_eq!(b.build(Site::Ex), "https://exhentai.org/popular");

        let b = ListUrlBuilder {
            mode: ListMode::Watched,
            page: 1,
            ..Default::default()
        };
        assert_eq!(b.build(Site::E), "https://e-hentai.org/watched?page=1");
    }

    #[test]
    fn uploader_and_tag_modes() {
        let mut b = ListUrlBuilder::new();
        b.mode = ListMode::Uploader;
        b.keyword = Some("some uploader".to_string());
        b.page = 2;
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/uploader/some+uploader/2"
        );

        b.mode = ListMode::Tag;
        b.keyword = Some("female:sole female".to_string());
        b.page = 0;
        assert_eq!(
            b.build(Site::E),
            "https://e-hentai.org/tag/female%3Asole+female"
        );
    }

    #[test]
    fn url_builder_order_and_encoding() {
        let mut ub = UrlBuilder::new("https://example.com/");
        ub.add_query("a", 1);
        ub.add_query("b", "x y");
        assert_eq!(ub.build(), "https://example.com/?a=1&b=x y");
        assert_eq!(java_url_encode("a b+c/漢"), "a+b%2Bc%2F%E6%BC%A2");
    }
}
