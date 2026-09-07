//! Port of `client/EhUrl.java` — every base URL and path builder the app uses.
//!
//! The original switches on `Settings.getGallerySite()`; here the site is an
//! explicit [`Site`] argument so one `EhClient` can serve both e-hentai and
//! exhentai.

/// Which EH deployment to talk to (SITE_E / SITE_EX in the original).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Site {
    /// e-hentai.org
    #[default]
    E,
    /// exhentai.org
    Ex,
}

impl Site {
    pub fn domain(self) -> &'static str {
        match self {
            Site::E => DOMAIN_E,
            Site::Ex => DOMAIN_EX,
        }
    }
}

pub const SITE_E: Site = Site::E;
pub const SITE_EX: Site = Site::Ex;

pub const DOMAIN_EX: &str = "exhentai.org";
pub const DOMAIN_E: &str = "e-hentai.org";
pub const DOMAIN_LOFI: &str = "lofi.e-hentai.org";
pub const DOMAIN_FORUMS: &str = "forums.e-hentai.org";

pub const REFERER_EX: &str = "https://exhentai.org";
pub const REFERER_E: &str = "https://e-hentai.org";

pub const HOST_EX: &str = "https://exhentai.org/";
pub const HOST_E: &str = "https://e-hentai.org/";

pub const API_SIGN_IN: &str = "https://forums.e-hentai.org/index.php?act=Login&CODE=01";
pub const URL_SIGN_IN: &str = "https://forums.e-hentai.org/index.php?act=Login";
pub const URL_REGISTER: &str = "https://forums.e-hentai.org/index.php?act=Reg&CODE=00";
pub const URL_FORUMS: &str = "https://forums.e-hentai.org/";

pub const API_E: &str = "https://e-hentai.org/api.php";
pub const API_EX: &str = "https://exhentai.org/api.php";

pub const HOME_E: &str = "https://e-hentai.org/home.php";
pub const HOME_EX: &str = "https://exhentai.org/home.php";

pub const URL_NEWS_E: &str = "https://e-hentai.org/news.php";

pub const URL_POPULAR_E: &str = "https://e-hentai.org/popular";
pub const URL_POPULAR_EX: &str = "https://exhentai.org/popular";

pub const URL_TOP_LIST_E: &str = "https://e-hentai.org/toplist.php";
pub const URL_TOP_LIST_EX: &str = "https://exhentai.org/toplist.php";

pub const URL_IMAGE_SEARCH_E: &str = "https://upld.e-hentai.org/image_lookup.php";
pub const URL_IMAGE_SEARCH_EX: &str = "https://upld.exhentai.org/upld/image_lookup.php";

pub const URL_FAVORITES_E: &str = "https://e-hentai.org/favorites.php";
pub const URL_FAVORITES_EX: &str = "https://exhentai.org/favorites.php";

pub const ORIGIN_EX: &str = REFERER_EX;
pub const ORIGIN_E: &str = REFERER_E;

pub const URL_UCONFIG_E: &str = "https://e-hentai.org/uconfig.php";
pub const URL_UCONFIG_EX: &str = "https://exhentai.org/uconfig.php";

pub const URL_MY_TAGS_E: &str = "https://e-hentai.org/mytags";
pub const URL_MY_TAGS_EX: &str = "https://exhentai.org/mytags";

pub const URL_WATCHED_E: &str = "https://e-hentai.org/watched";
pub const URL_WATCHED_EX: &str = "https://exhentai.org/watched";

pub const URL_PREFIX_THUMB_E: &str = "https://ehgt.org/";
pub const URL_PREFIX_THUMB_EX: &str = "https://exhentai.org/t/";

pub fn referer(site: Site) -> &'static str {
    match site {
        Site::E => REFERER_E,
        Site::Ex => REFERER_EX,
    }
}

pub fn origin(site: Site) -> &'static str {
    referer(site)
}

pub fn host(site: Site) -> &'static str {
    match site {
        Site::E => HOST_E,
        Site::Ex => HOST_EX,
    }
}

pub fn home_url(_site: Site) -> &'static str {
    // The original always returns HOME_E.
    HOME_E
}

pub fn api_url(site: Site) -> &'static str {
    match site {
        Site::E => API_E,
        Site::Ex => API_EX,
    }
}

pub fn favorites_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_FAVORITES_E,
        Site::Ex => URL_FAVORITES_EX,
    }
}

pub fn popular_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_POPULAR_E,
        Site::Ex => URL_POPULAR_EX,
    }
}

/// Note: the original hard-returns `URL_TOP_LIST_E` — exhentai has no top list.
pub fn toplist_url(_site: Site) -> &'static str {
    URL_TOP_LIST_E
}

pub fn news_url(_site: Site) -> &'static str {
    URL_NEWS_E
}

pub fn image_search_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_IMAGE_SEARCH_E,
        Site::Ex => URL_IMAGE_SEARCH_EX,
    }
}

pub fn watched_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_WATCHED_E,
        Site::Ex => URL_WATCHED_EX,
    }
}

pub fn uconfig_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_UCONFIG_E,
        Site::Ex => URL_UCONFIG_EX,
    }
}

pub fn my_tags_url(site: Site) -> &'static str {
    match site {
        Site::E => URL_MY_TAGS_E,
        Site::Ex => URL_MY_TAGS_EX,
    }
}

pub fn thumb_url_prefix() -> &'static str {
    // The original always uses the ehgt.org prefix now.
    URL_PREFIX_THUMB_E
}

/// `https://host/g/{gid}/{token}/[?p=N][&hc=1]` (port of `getGalleryDetailUrl`).
pub fn gallery_detail_url(
    site: Site,
    gid: u64,
    token: &str,
    index: i32,
    all_comment: bool,
) -> String {
    let mut url = format!("{}g/{}/{}/", host(site), gid, token);
    let mut has_query = false;
    if index != 0 {
        url.push_str("?p=");
        url.push_str(&index.to_string());
        has_query = true;
    }
    if all_comment {
        url.push_str(if has_query { "&hc=1" } else { "?hc=1" });
    }
    url
}

/// `https://host/s/{ptoken}/{gid}-{index+1}` (port of `getPageUrl`).
pub fn page_url(site: Site, gid: u64, index: i32, ptoken: &str) -> String {
    format!("{}s/{}/{}-{}", host(site), ptoken, gid, index + 1)
}

/// `gallerypopups.php?gid=..&t=..&act=addfav` (port of `getAddFavorites`).
pub fn add_favorites_url(site: Site, gid: u64, token: &str) -> String {
    format!(
        "{}gallerypopups.php?gid={}&t={}&act=addfav",
        host(site),
        gid,
        token
    )
}

/// `archiver.php?gid=..&token=..[&or=..]` (port of `getDownloadArchive`).
pub fn download_archive_url(site: Site, gid: u64, token: &str, or: &str) -> String {
    if or.is_empty() {
        format!("{}archiver.php?gid={}&token={}", host(site), gid, token)
    } else {
        format!(
            "{}archiver.php?gid={}&token={}&or={}",
            host(site),
            gid,
            token,
            or
        )
    }
}

pub fn tag_definition_url(tag: &str) -> String {
    format!("https://ehwiki.org/wiki/{}", tag.replace(' ', "_"))
}

/// Port of `EhUrl.getFixedPreviewThumbUrl` — rewrite
/// `https://exhentai.org/t/ab/cd/abcdef..._250.jpg` to
/// `https://ehgt.org/ab/cd/abcdef..._250.jpg` when the path segments line up.
pub fn fixed_preview_thumb_url(origin_url: &str) -> String {
    let Ok(url) = reqwest::Url::parse(origin_url) else {
        return origin_url.to_string();
    };
    let segs: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if segs.len() < 3 {
        return origin_url.to_string();
    }
    let n = segs.len();
    let (last, second_last, third_last) = (segs[n - 1], segs[n - 2], segs[n - 3]);
    if last.starts_with(third_last)
        && last
            .get(third_last.len()..)
            .is_some_and(|rest| rest.starts_with(second_last))
    {
        format!(
            "{}{}/{}/{}",
            thumb_url_prefix(),
            third_last,
            second_last,
            last
        )
    } else {
        origin_url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_detail_url_variants() {
        assert_eq!(
            gallery_detail_url(Site::E, 1234567, "a1b2c3d4e5", 0, false),
            "https://e-hentai.org/g/1234567/a1b2c3d4e5/"
        );
        assert_eq!(
            gallery_detail_url(Site::Ex, 1234567, "a1b2c3d4e5", 2, true),
            "https://exhentai.org/g/1234567/a1b2c3d4e5/?p=2&hc=1"
        );
    }

    #[test]
    fn page_url_format() {
        assert_eq!(
            page_url(Site::E, 1234567, 0, "abc123def4"),
            "https://e-hentai.org/s/abc123def4/1234567-1"
        );
        assert_eq!(
            page_url(Site::E, 1234567, 41, "abc123def4"),
            "https://e-hentai.org/s/abc123def4/1234567-42"
        );
    }

    #[test]
    fn fixed_thumb_rewrite() {
        assert_eq!(
            fixed_preview_thumb_url(
                "https://exhentai.org/t/fe/1f/fe1fcfa9bf8fba2f03982eda0aa347cc9d6a6372-145921-1050-1492-jpg_250.jpg"
            ),
            "https://ehgt.org/fe/1f/fe1fcfa9bf8fba2f03982eda0aa347cc9d6a6372-145921-1050-1492-jpg_250.jpg"
        );
        // Segments don't line up — returned unchanged.
        assert_eq!(
            fixed_preview_thumb_url("https://exhentai.org/t/aa/bb/xyz.jpg"),
            "https://exhentai.org/t/aa/bb/xyz.jpg"
        );
    }

    #[test]
    fn site_selectors() {
        assert_eq!(referer(Site::Ex), REFERER_EX);
        assert_eq!(host(Site::E), HOST_E);
        assert_eq!(api_url(Site::Ex), API_EX);
        assert_eq!(toplist_url(Site::Ex), URL_TOP_LIST_E);
    }
}
