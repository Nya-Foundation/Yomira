//! Fixture-driven tests for the parsers. The fixtures under
//! `src-tauri/fixtures/` are synthetic but model the real EH DOM shapes the
//! ported patterns expect (verified against the upstream Java sources).

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

use super::testutil::fixture;
use super::*;
use crate::core::models::category;

fn ts(secs: &str) -> DateTime<Utc> {
    let naive = NaiveDateTime::parse_from_str(secs, "%Y-%m-%d %H:%M").unwrap();
    Utc.from_utc_datetime(&naive)
}

fn millis(day: &str) -> i64 {
    ts(day).timestamp_millis()
}

// ---------------------------------------------------------------------
// Gallery list
// ---------------------------------------------------------------------

#[test]
fn gallery_list_extended_layout() {
    let result = parse_gallery_list(&fixture("gallery_list_extended.html")).unwrap();
    assert_eq!(result.pages, 2);
    assert_eq!(result.next_page, 1);
    // Keep the complete continuation even for legacy numbered pagers.
    assert_eq!(result.next_href.as_deref(), Some("?page=1&f_search=touhou"));
    assert_eq!(result.galleries.len(), 2);

    let g1 = &result.galleries[0];
    assert_eq!(g1.gid, 618395);
    assert_eq!(g1.token, "0439fa3666");
    assert_eq!(g1.title, "(C99) [Artist] Test Title One [English]");
    assert_eq!(g1.category, category::DOUJINSHI);
    assert_eq!(g1.uploader.as_deref(), Some("SomeOne"));
    assert_eq!(g1.posted, ts("2021-01-01 12:34"));
    assert!((g1.rating - 4.0).abs() < f32::EPSILON);
    assert_eq!(g1.rated, 1);
    assert_eq!(g1.pages, 20);
    assert_eq!(g1.favorite_slot, 1);
    assert_eq!(
        g1.thumb,
        "https://ehgt.org/t/aa/bb/aabbcc112233-445566-250-354-jpg_250.jpg"
    );
    assert_eq!(g1.thumb_width, 250);
    assert_eq!(g1.thumb_height, 354);
    assert!(g1.tags.is_empty());

    let g2 = &result.galleries[1];
    assert_eq!(g2.gid, 618400);
    assert_eq!(g2.token, "aaaabbbb11");
    assert_eq!(g2.title, "Test Title Two");
    assert_eq!(g2.category, category::MANGA);
    assert_eq!(g2.uploader.as_deref(), Some("OtherOne"));
    assert_eq!(g2.posted, ts("2021-01-02 00:00"));
    assert!((g2.rating - 1.5).abs() < f32::EPSILON);
    assert_eq!(g2.rated, 0);
    assert_eq!(g2.pages, 15);
    assert_eq!(g2.favorite_slot, -2);
    // data-src is preferred over src when present
    assert_eq!(
        g2.thumb,
        "https://ehgt.org/t/cc/dd/ccddee334455-778899-250-300-jpg_250.jpg"
    );
    assert_eq!(g2.thumb_width, 250);
    assert_eq!(g2.thumb_height, 300);
}

#[test]
fn gallery_list_thumb_layout() {
    let result = parse_gallery_list(&fixture("gallery_list_thumb.html")).unwrap();
    assert_eq!(result.pages, 2);
    assert_eq!(result.next_page, 1);
    assert_eq!(result.galleries.len(), 2);

    let g1 = &result.galleries[0];
    assert_eq!(g1.gid, 777111);
    assert_eq!(g1.token, "abcabc1234");
    assert_eq!(g1.title, "Thumb Title One");
    assert_eq!(g1.category, category::NON_H);
    assert_eq!(g1.uploader, None);
    assert_eq!(g1.pages, 35);
    assert_eq!(g1.favorite_slot, -2);
    assert!((g1.rating - 5.0).abs() < f32::EPSILON);
    assert_eq!(g1.thumb_width, 250);
    assert_eq!(g1.thumb_height, 354);

    let g2 = &result.galleries[1];
    assert_eq!(g2.gid, 777222);
    assert_eq!(g2.token, "dedede5678");
    assert_eq!(g2.category, category::WESTERN);
    assert_eq!(g2.pages, 8);
    assert!((g2.rating - 0.5).abs() < f32::EPSILON);
}

#[test]
fn gallery_list_last_page_has_no_next() {
    let result = parse_gallery_list(&fixture("gallery_list_lastpage.html")).unwrap();
    assert_eq!(result.pages, 2);
    assert_eq!(result.next_page, 0);
    assert_eq!(result.galleries.len(), 1);
    assert_eq!(result.galleries[0].gid, 888111);
    assert_eq!(result.galleries[0].token, "fafa123456");
    assert_eq!(result.galleries[0].title, "Last Page Item");
    assert_eq!(result.galleries[0].category, category::MISC);
}

#[test]
fn gallery_list_searchnav_layout() {
    let result = parse_gallery_list(&fixture("gallery_list_searchnav.html")).unwrap();
    assert_eq!(result.pages, -1);
    assert_eq!(result.next_page, -1);
    assert_eq!(result.result_count, "12,345");
    assert_eq!(
        result.first_href.as_deref(),
        Some("?f_search=touhou&page=0")
    );
    assert_eq!(result.prev_href.as_deref(), Some("?f_search=touhou&page=0"));
    // Spans carry no href.
    assert_eq!(result.next_href, None);
    assert_eq!(result.last_href, None);
    assert_eq!(result.galleries.len(), 1);
    let g = &result.galleries[0];
    assert_eq!(g.gid, 999111);
    assert_eq!(g.token, "0f0f0f0f0f");
    assert_eq!(g.title, "Watched Item");
    assert_eq!(g.category, category::MANGA);
    assert!((g.rating - -1.0).abs() < f32::EPSILON);
}

#[test]
fn gallery_list_searchnav_parses_next_page_from_unext_href() {
    // Watched tags pages link with next=N instead of page=N.
    let body = fixture("gallery_list_searchnav.html").replace(
        r#"<span id="unext">Next</span>"#,
        r#"<a id="unext" href="?f_search=touhou&next=12345">Next</a>"#,
    );
    let result = parse_gallery_list(&body).unwrap();
    assert_eq!(result.next_page, 12345);
    assert_eq!(
        result.next_href.as_deref(),
        Some("?f_search=touhou&next=12345")
    );
}

#[test]
fn gallery_list_no_hits() {
    let result = parse_gallery_list(&fixture("gallery_list_nohits.html")).unwrap();
    assert_eq!(result.pages, 0);
    assert!(result.galleries.is_empty());
}

// ---------------------------------------------------------------------
// Gallery detail
// ---------------------------------------------------------------------

#[test]
fn gallery_detail_full() {
    let detail = parse_gallery_detail(&fixture("gallery_detail.html")).unwrap();
    assert_eq!(detail.info.gid, 618395);
    assert_eq!(detail.info.token, "0439fa3666");
    assert_eq!(detail.info.title, "(C99) [Artist] Test Title One [English]");
    assert_eq!(
        detail.info.title_jpn.as_deref(),
        Some("(C99) [アーティスト] テストタイトル1 [英訳]")
    );
    assert_eq!(detail.info.category, category::DOUJINSHI);
    assert_eq!(detail.info.uploader.as_deref(), Some("SomeOne"));
    assert_eq!(detail.info.posted, ts("2021-01-01 12:34"));
    assert_eq!(detail.parent_gid, Some(618390));
    assert_eq!(detail.parent_token.as_deref(), Some("deadbeef12"));
    assert!(detail.visible);
    assert_eq!(detail.language.as_deref(), Some("English"));
    assert_eq!(detail.size, "12.34 MiB");
    assert_eq!(detail.size_bytes, 12_939_427);
    assert_eq!(detail.info.pages, 20);
    assert_eq!(detail.favorite_count, 42);
    assert_eq!(detail.info.favorites, 42);
    assert!(detail.favorited);
    assert_eq!(detail.favorite_name.as_deref(), Some("Favorites 3"));
    assert_eq!(detail.info.favorite_slot, 3);
    assert_eq!(detail.rating_count, 42);
    assert!((detail.average_rating - 4.5).abs() < f32::EPSILON);
    assert!((detail.user_rating - -1.0).abs() < f32::EPSILON);
    assert_eq!(
        detail.info.thumb,
        "https://ehgt.org/t/fe/1f/fe1fcfa9bf8fba2f03982eda0aa347cc9d6a6372-145921-1050-1492-jpg_250.jpg"
    );
    assert_eq!(detail.info.thumb_width, 250);
    assert_eq!(detail.info.thumb_height, 356);

    // Torrent popUp URL is XML-unescaped, like the original.
    assert_eq!(
        detail.torrent_url,
        "https://exhentai.org/gallerytorrents.php?gid=618395&t=0439fa3666"
    );
    assert_eq!(detail.torrent_count, 2);
    assert_eq!(
        detail.archive_url,
        "https://exhentai.org/archiver.php?gid=618395&token=0439fa3666&or=somearchkey"
    );
    assert_eq!(detail.archiver_key.as_deref(), Some("somearchkey"));

    // Tag groups; the parody tag loses its "| translation" part.
    assert_eq!(detail.info.tags.len(), 3);
    assert_eq!(detail.info.tags[0].namespace, "female");
    assert_eq!(detail.info.tags[0].tags, ["sole female", "big breasts"]);
    assert_eq!(detail.info.tags[1].namespace, "parody");
    assert_eq!(detail.info.tags[1].tags, ["touhou project"]);
    assert_eq!(detail.info.tags[2].namespace, "language");
    assert_eq!(detail.info.tags[2].tags, ["english", "translated"]);

    // Comments.
    assert!(detail.has_more_comments);
    assert_eq!(detail.comments.len(), 2);
    let c1 = &detail.comments[0];
    assert_eq!(c1.id, 1001);
    assert_eq!(c1.user, "CommenterOne");
    assert_eq!(c1.score, 5);
    assert!(c1.vote_up_able && c1.vote_down_able && c1.editable);
    assert_eq!(c1.time, millis("2021-01-02 13:37"));
    assert!(c1.comment.contains("Great stuff,"));
    // Bug-compatible with the original: lastEdited copies the post time.
    assert_eq!(c1.last_edited, c1.time);
    let c2 = &detail.comments[1];
    assert_eq!(c2.id, 1002);
    assert_eq!(c2.user, "CommenterTwo");
    assert_eq!(c2.score, 0);
    assert!(!c2.vote_up_able && !c2.vote_down_able && !c2.editable);
    assert_eq!(c2.time, millis("2021-01-03 08:09"));
    assert_eq!(c2.last_edited, 0);

    // Previews.
    assert_eq!(detail.preview_pages, 2);
    assert_eq!(detail.preview_sets.len(), 1);
    let set = &detail.preview_sets[0];
    assert_eq!(set.pages, 0);
    assert_eq!(set.rows.len(), 2);
    assert_eq!(set.rows[0].position, 0);
    assert_eq!(set.rows[0].gid, 618395);
    assert_eq!(set.rows[0].token, "aaaa1111bb");
    assert_eq!(set.rows[0].xres, 100);
    assert_eq!(set.rows[0].yres, 125);
    assert_eq!(set.rows[0].xoffset, 0);
    assert_eq!(set.rows[1].position, 1);
    assert_eq!(set.rows[1].token, "cccc3333dd");
}

#[test]
fn preview_set_small_layout() {
    let set = parse_preview_set(&fixture("gallery_previews_small.html"), 0);
    assert_eq!(set.rows.len(), 2);
    assert_eq!(set.rows[0].position, 0);
    assert_eq!(set.rows[0].xres, 100);
    assert_eq!(set.rows[0].yres, 127);
    assert_eq!(set.rows[0].xoffset, 100);
    assert_eq!(set.rows[0].gid, 618395);
    assert_eq!(set.rows[0].token, "bbbb2222cc");
    assert_eq!(
        set.rows[0].url,
        "https://ehgt.org/t/33/44/3344def-456-100-127-jpg.jpg"
    );
    assert_eq!(set.rows[1].xoffset, 200);
    assert_eq!(set.rows[1].token, "dddd4444ee");
}

#[test]
fn preview_set_large_layout() {
    let set = parse_preview_set(&fixture("gallery_previews_large.html"), 3);
    assert_eq!(set.pages, 3);
    assert_eq!(set.rows.len(), 2);
    assert_eq!(set.rows[0].position, 0);
    // Large previews carry the full image and no sprite geometry.
    assert_eq!(set.rows[0].xres, 0);
    assert_eq!(set.rows[0].yres, 0);
    assert_eq!(set.rows[0].xoffset, 0);
    assert_eq!(
        set.rows[0].url,
        "https://ehgt.org/t/fe/1f/fe1fcfa9bf8fba2f03982eda0aa347cc9d6a6372-145921-1050-1492-jpg_250.jpg"
    );
    assert_eq!(set.rows[0].token, "eeee5555ff");
    assert_eq!(set.rows[1].token, "0f0f6666aa");
}

// ---------------------------------------------------------------------
// Gallery page (reader)
// ---------------------------------------------------------------------

#[test]
fn gallery_page_ok() {
    let data = parse_gallery_page(&fixture("gallery_page.html"), "aaaa1111bb").unwrap();
    assert_eq!(
        data.image_url.as_deref(),
        Some("https://mm1.exhentai.org:8355/h0123/abcdef0123456789/1.jpg")
    );
    assert_eq!(data.skip_hath_key.as_deref(), Some("skip-hath-key-123"));
    assert_eq!(
        data.origin_image_url.as_deref(),
        Some("https://exhentai.org/fullimg/618395/1/aaaaaaaaaa/1000px-618395_p1.jpg")
    );
    assert_eq!(data.show_key.as_deref(), Some("abc123def456"));
    assert_eq!(data.server_time, 1_609_503_840);
    assert_eq!(data.server_offset, -18_000);
    assert_eq!(data.ptoken, "aaaa1111bb");
    assert_eq!(data.redirect_url, None);
}

#[test]
fn gallery_page_509() {
    let err = parse_gallery_page(&fixture("gallery_page_509.html"), "tok").unwrap_err();
    assert_eq!(err.kind(), "image509");
}

#[test]
fn gallery_page_js_redirect() {
    let data = parse_gallery_page(&fixture("gallery_page_redirect.html"), "aaaa1111bb").unwrap();
    assert_eq!(data.image_url, None);
    assert_eq!(data.show_key, None);
    assert_eq!(
        data.redirect_url.as_deref(),
        Some("https://exhentai.org/s/aaaa1111bb/618395-5?nl=skip-hath-key-123")
    );
}

#[test]
fn image_509_heuristic() {
    assert!(is_509_image("image/gif", &[0u8; 512]));
    assert!(!is_509_image("image/gif", &[0u8; 2048]));
    assert!(!is_509_image("image/jpeg", &[0u8; 512]));
}

// ---------------------------------------------------------------------
// Favorites
// ---------------------------------------------------------------------

#[test]
fn favorites_page() {
    let result = parse_favorites(&fixture("favorites.html")).unwrap();
    assert_eq!(result.cat_array.len(), 10);
    assert_eq!(result.cat_array[0], "Favorites 0");
    assert_eq!(result.cat_array[9], "Favorites 9");
    assert_eq!(result.count_array, [10, 0, 3, 7, 0, 0, 0, 0, 0, 0]);
    assert_eq!(result.fav_order.as_deref(), Some("fa"));
    assert_eq!(result.pages, 1);
    assert_eq!(result.next_page, 1);
    assert_eq!(result.galleries.len(), 1);
    let g = &result.galleries[0];
    assert_eq!(g.gid, 555000);
    assert_eq!(g.token, "1234567890");
    assert_eq!(g.title, "Favorited Gallery");
    assert_eq!(g.uploader.as_deref(), Some("FavUp"));
    assert_eq!(g.pages, 50);
    assert!((g.rating - 3.0).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------
// Toplist
// ---------------------------------------------------------------------

#[test]
fn toplist_page() {
    use crate::core::models::TopListKind;

    let detail = parse_toplist(&fixture("toplist.html")).unwrap();
    assert_eq!(detail.title, "E-Hentai Toplists");

    let gallery = &detail.gallery;
    assert_eq!(gallery.kind, TopListKind::Gallery);
    assert_eq!(gallery.all_time.len(), 10);
    assert_eq!(gallery.all_time[0].value, "Gallery One");
    assert_eq!(gallery.all_time[0].href, "?tl=11&p=1");
    assert_eq!(gallery.all_time[0].rank, 1);
    assert_eq!(gallery.all_time[9].rank, 10);
    assert_eq!(gallery.past_year.len(), 2);
    assert_eq!(gallery.past_year[0].value, "Year One");
    assert_eq!(gallery.past_month.len(), 1);
    assert_eq!(gallery.yesterday.len(), 3);
    assert_eq!(gallery.yesterday[2].value, "Yesterday Three");

    assert_eq!(detail.uploader.kind, TopListKind::Uploader);
    assert_eq!(detail.uploader.all_time.len(), 1);
    assert_eq!(detail.uploader.all_time[0].value, "Uploader One");
    assert_eq!(detail.uploader.all_time[0].href, "?tl=21&p=1");

    // Header-only list divs parse as empty lists.
    assert!(detail.tagging.all_time.is_empty());
    assert!(detail.hentai_home.all_time.is_empty());
    assert!(detail.eh_tracker.all_time.is_empty());
    assert!(detail.clean_up.all_time.is_empty());
    assert!(detail.rating_and_reviewing.all_time.is_empty());
}

// ---------------------------------------------------------------------
// Torrents
// ---------------------------------------------------------------------

#[test]
fn torrent_page() {
    let torrents = parse_torrent_page(&fixture("torrents.html"));
    assert_eq!(torrents.len(), 2);
    assert_eq!(
        torrents[0].url,
        "https://exhentai.org/torrent/618395/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.torrent"
    );
    assert_eq!(torrents[0].name, "Test Title One [English] [C99]");
    assert_eq!(torrents[0].posted, "2021-01-01 12:30");
    assert_eq!(
        torrents[1].url,
        "https://exhentai.org/torrent/618395/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.torrent"
    );
    assert_eq!(torrents[1].name, "Test Title One [English] [C99] [2]");
    assert_eq!(torrents[1].posted, "2021-01-02 10:00");
}

// ---------------------------------------------------------------------
// Sign in
// ---------------------------------------------------------------------

#[test]
fn sign_in_success() {
    let name = parse_sign_in(&fixture("sign_in.html")).unwrap();
    assert_eq!(name, "TestUser");
}

#[test]
fn sign_in_error() {
    let err = parse_sign_in(&fixture("sign_in_error.html")).unwrap_err();
    assert_eq!(err.kind(), "eh");
    assert!(err.message().contains("incorrect"));
}

#[test]
fn detect_signed_in_markers() {
    assert!(detect_signed_in(&fixture("forum_member.html")));
    assert!(!detect_signed_in(&fixture("forum_anon.html")));
    // A gallery detail page has no session markers.
    assert!(!detect_signed_in(&fixture("gallery_detail.html")));
}

// ---------------------------------------------------------------------
// News (news.php)
// ---------------------------------------------------------------------

#[test]
fn news_items_from_fixture() {
    let items = parse_news(&fixture("news.html"));
    assert_eq!(items.len(), 2);

    let first = &items[0];
    assert_eq!(first.title, "New Search Engine");
    assert_eq!(
        first.link,
        "https://forums.e-hentai.org/index.php?showtopic=261743"
    );
    assert_eq!(first.id, Some(261743));
    assert_eq!(first.date, "Wednesday, 2nd of November 2022, 10:09 UTC");
    // <b> stripped, <br> → newline, <p> → blank line, list → bullets,
    // entities unescaped.
    assert!(first
        .body
        .starts_with("If you want to know more, start by reading"));
    assert!(first.body.contains("this post"));
    assert!(first
        .body
        .contains("\n\nSecond paragraph with a list:\n\n• first item\n• second item"));
    assert!(!first.body.contains('<'));

    let second = &items[1];
    assert_eq!(second.title, "Minor Maintenance Window");
    assert_eq!(second.id, None); // no showtopic in the link
    assert_eq!(second.date, "Friday, 21st of October 2022, 06:12 UTC");
    assert!(second
        .body
        .contains("Thanks for your patience & understanding."));
}

#[test]
fn news_parse_never_fails() {
    assert!(parse_news("").is_empty());
    assert!(parse_news("<html><body>Please log on.</body></html>").is_empty());
}

// ---------------------------------------------------------------------
// Quota (home.php limits)
// ---------------------------------------------------------------------

#[test]
fn quota_from_home_fixture() {
    let quota = parse_quota(&fixture("home_quota.html"));
    assert_eq!(quota.images_used, 1234);
    assert_eq!(quota.images_total, 5000);
    assert_eq!(quota.image_reset_seconds, Some(9000)); // 2 h 30 min
    assert_eq!(quota.searches_used, Some(3));
    assert_eq!(quota.searches_total, Some(9));
    assert_eq!(quota.search_reset_seconds, Some(2700)); // 45 min
}

#[test]
fn quota_absent_markup_yields_default() {
    // Anonymous visitors get the login redirect; drifted markup just lacks
    // the homeboxes. Either way: no error, empty quota.
    let quota = parse_quota("<html><body>Please log on to continue.</body></html>");
    assert_eq!(quota.images_used, -1);
    assert_eq!(quota.images_total, -1);
    assert!(quota.image_reset_seconds.is_none());
    assert!(quota.searches_used.is_none());
    assert!(quota.searches_total.is_none());
    assert!(quota.search_reset_seconds.is_none());
}
