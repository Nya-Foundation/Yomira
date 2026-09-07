use super::*;
use crate::core::models::TagGroup;

#[test]
fn half_star_requests_deserialize_and_use_supported_server_thresholds() {
    for rating in [2.0_f32, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0] {
        let params: ListParamsDto =
            serde_json::from_value(serde_json::json!({ "page": 0, "minRating": rating })).unwrap();
        let url = list_params_to_builder(&params, "home", None)
            .unwrap()
            .build(crate::core::Site::E);
        let url = tauri::Url::parse(&url).unwrap();
        let threshold = url
            .query_pairs()
            .find(|(key, _)| key == "f_srdd")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(threshold, (rating.floor() as i32).to_string());
        let mut galleries: Vec<_> = [2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0]
            .into_iter()
            .map(|rating| GalleryInfo {
                rating,
                ..Default::default()
            })
            .collect();
        apply_min_rating(&mut galleries, params.min_rating);
        assert_eq!(galleries.first().unwrap().rating, rating);
        assert!(galleries.iter().all(|gallery| gallery.rating >= rating));
    }
}

#[test]
fn rating_filter_can_empty_a_page_without_losing_continuation() {
    let mut galleries = vec![GalleryInfo {
        rating: 4.0,
        ..Default::default()
    }];
    apply_min_rating(&mut galleries, Some(4.5));
    assert!(galleries.is_empty());
    assert!(matches!(
        list_next_page(
            Some("?f_srdd=4&next=900"),
            -1,
            "https://e-hentai.org/?f_srdd=4"
        ),
        Some(ListPage::Url(_))
    ));
    for rating in [1.0, 5.5, f32::NAN, f32::INFINITY] {
        assert!(list_params_to_builder(
            &ListParamsDto {
                min_rating: Some(rating),
                ..Default::default()
            },
            "home",
            None
        )
        .is_err());
    }
}

#[test]
fn maps_home_params_with_categories() {
    let params = ListParamsDto {
        page: 2,
        search: "full color".into(),
        categories: Some(vec!["Doujinshi".into(), "Manga".into()]),
        min_rating: Some(4.0),
        min_pages: None,
        max_pages: Some(50),
        enable_uploader: true,
        enable_tags: false,
        ..Default::default()
    };
    let builder = list_params_to_builder(&params, "home", None).unwrap();
    assert_eq!(builder.mode, ListMode::Normal);
    assert_eq!(builder.page, 2);
    assert_eq!(builder.category, category::DOUJINSHI | category::MANGA);
    assert_eq!(builder.keyword.as_deref(), Some("full color"));
    let url = builder.build(crate::core::Site::E);
    let adv = builder.advanced.as_ref().unwrap();
    assert_eq!(adv.min_rating, 4);
    assert_eq!(adv.page_to, 50);
    assert!(!adv.search_tags);
    assert!(url.contains("f_cats=1017"), "{url}");
    assert!(url.contains("f_srdd=4"), "{url}");
    assert!(url.contains("f_spt=50"), "{url}");
}

#[test]
fn advanced_search_enables_on_any_knob() {
    // Every default is a no-advanced-search pass-through.
    let builder = list_params_to_builder(&ListParamsDto::default(), "home", None).unwrap();
    assert!(builder.advanced.is_none());

    // A single boolean flag is enough to turn advanced search on.
    let params = ListParamsDto {
        search_expunged: true,
        ..Default::default()
    };
    let builder = list_params_to_builder(&params, "home", None).unwrap();
    let url = builder.build(crate::core::Site::E);
    assert_eq!(
        builder.advanced.as_ref().unwrap(),
        &crate::core::models::AdvancedSearch {
            search_expunged: true,
            ..Default::default()
        }
    );
    assert!(url.contains("advsearch=1"), "{url}");
    assert!(url.contains("f_sh=on"), "{url}");

    // Toggling the default-on switches off also enables it (f_sname off).
    let params = ListParamsDto {
        enable_uploader: false,
        ..Default::default()
    };
    let builder = list_params_to_builder(&params, "home", None).unwrap();
    let url = builder.build(crate::core::Site::E);
    assert!(url.contains("advsearch=1"), "{url}");
    assert!(!url.contains("f_sname"), "{url}");
    assert!(url.contains("f_stags=on"), "{url}");
}

#[test]
fn advanced_search_maps_every_flag() {
    let params = ListParamsDto {
        search: "touhou".into(),
        min_rating: Some(3.0),
        enable_uploader: false,
        enable_tags: true,
        search_desc: true,
        search_torrent: true,
        search_lowpower_tags: true,
        search_downvoted_tags: true,
        search_expunged: true,
        ..Default::default()
    };
    let builder = list_params_to_builder(&params, "home", None).unwrap();
    let url = builder.build(crate::core::Site::E);
    let adv = builder.advanced.as_ref().unwrap();
    assert!(!adv.search_name);
    assert!(adv.search_tags);
    assert!(adv.search_descr);
    assert!(adv.search_torrent);
    assert!(adv.search_lowpower_tags);
    assert!(adv.search_downvoted_tags);
    assert!(adv.search_expunged);
    assert!(!adv.use_tag_repo);
    assert_eq!(adv.min_rating, 3);
    for flag in [
        "f_sdesc=on",
        "f_storr=on",
        "f_sdt1=on",
        "f_sdt2=on",
        "f_sh=on",
        "f_srdd=3",
    ] {
        assert!(url.contains(flag), "{url} missing {flag}");
    }
    assert!(!url.contains("f_sname"), "{url}");
    assert!(!url.contains("f_sto=on"), "{url}");
}

#[test]
fn maps_toplist_timespans() {
    for (timespan, tl) in [
        ("yesterday", "tl=15"),
        ("month", "tl=13"),
        ("year", "tl=12"),
        ("all", "tl=11"),
    ] {
        let builder =
            list_params_to_builder(&ListParamsDto::default(), "toplist", Some(timespan)).unwrap();
        assert_eq!(builder.mode, ListMode::TopList);
        assert!(builder.build(crate::core::Site::E).contains(tl));
    }
    // Default (no timespan) is the past-month list.
    let builder = list_params_to_builder(&ListParamsDto::default(), "toplist", None).unwrap();
    assert!(builder.build(crate::core::Site::E).contains("tl=13"));
}

#[test]
fn rejects_unknown_mode_and_category() {
    assert!(list_params_to_builder(&ListParamsDto::default(), "nope", None).is_err());
    let bad = ListParamsDto {
        categories: Some(vec!["Bogus".into()]),
        ..Default::default()
    };
    assert!(list_params_to_builder(&bad, "home", None).is_err());
}

fn gallery_with(title: &str, uploader: &str, tags: &[(&str, &[&str])]) -> GalleryInfo {
    GalleryInfo {
        title: title.into(),
        uploader: if uploader.is_empty() {
            None
        } else {
            Some(uploader.into())
        },
        tags: tags
            .iter()
            .map(|(ns, list)| TagGroup {
                namespace: ns.to_string(),
                tags: list.iter().map(|s| s.to_string()).collect(),
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn filters_block_by_title_uploader_and_tag() {
    let mut galleries = vec![
        gallery_with(
            "A Nice Title",
            "gooduploader",
            &[("female", &["sole female"])],
        ),
        gallery_with("Scat Warning", "gooduploader", &[]),
        gallery_with("Something", "Spammy Guy", &[("male", &["males only"])]),
        gallery_with("Other", "gooduploader", &[("male", &["males only"])]),
    ];
    let filters = vec![
        FilterEntry {
            mode: "title".into(),
            text: "scat".into(),
        },
        FilterEntry {
            mode: "uploader".into(),
            text: "spammy guy".into(),
        },
        FilterEntry {
            mode: "tag".into(),
            text: "male:males only".into(),
        },
    ];
    apply_filters(&mut galleries, &filters);
    let titles: Vec<_> = galleries.iter().map(|g| g.title.as_str()).collect();
    assert_eq!(titles, vec!["A Nice Title"]);
}

#[test]
fn bare_tag_filter_matches_any_namespace() {
    let mut galleries = vec![
        gallery_with("x", "u", &[("female", &["big breasts"])]),
        gallery_with("y", "u", &[("male", &["sole male"])]),
    ];
    let filters = vec![FilterEntry {
        mode: "tag".into(),
        text: "big breasts".into(),
    }];
    apply_filters(&mut galleries, &filters);
    assert_eq!(galleries.len(), 1);
    assert_eq!(galleries[0].title, "y");
}

#[test]
fn title_filter_checks_japanese_title_too() {
    let mut gi = gallery_with("English", "u", &[]);
    gi.title_jpn = Some("闇の中".into());
    let mut galleries = vec![gi];
    let filters = vec![FilterEntry {
        mode: "title".into(),
        text: "闇".into(),
    }];
    apply_filters(&mut galleries, &filters);
    assert!(galleries.is_empty());
}

#[test]
fn result_count_parses_digits() {
    assert_eq!(parse_result_count("Found 24,582 results"), 24582);
    assert_eq!(parse_result_count(""), 0);
}

#[test]
fn next_page_dto_ignores_href_gate() {
    // ptt-paginated lists (home/search/toplist) fill next_page without a
    // next_href — a positive number alone means "has next page".
    assert_eq!(dto_next_page(1), Some(1));
    assert_eq!(dto_next_page(42), Some(42));
    assert_eq!(dto_next_page(0), None); // last page
    assert_eq!(dto_next_page(-1), None); // watched: no unext href
}

#[test]
fn pages_dto_maps_unknown_to_zero() {
    assert_eq!(dto_pages(25), 25);
    assert_eq!(dto_pages(1), 1);
    assert_eq!(dto_pages(0), 0); // "no hits" sentinel
    assert_eq!(dto_pages(-1), 0); // unknown (watched searchnav)
    assert_eq!(dto_pages(i32::MAX), 0); // parse-error fallback
}

#[test]
fn filters_come_from_the_db() {
    let db = crate::db::Db::open_in_memory().unwrap();
    db.add_blacklist("uploader", "blocked user").unwrap();
    db.add_filter("title", "forbidden").unwrap();

    let mut galleries = vec![
        gallery_with("Keep me", "blocked user", &[]),
        gallery_with("forbidden fruit", "ok", &[]),
        gallery_with("Keep me too", "ok", &[]),
    ];
    let filters = db.filters().unwrap();
    apply_filters(&mut galleries, &filters);
    let titles: Vec<_> = galleries.iter().map(|g| g.title.as_str()).collect();
    assert_eq!(titles, vec!["Keep me too"]);
}

#[test]
fn continuation_keeps_the_cursor_and_search_parameters() {
    let base = "https://e-hentai.org/?f_search=artist%3Aexample";
    let next = "?next=4123456&f_search=artist%3Aexample&seek=2026-09-01";
    let url = list_request_url(base, Some(next)).unwrap();
    let parsed = tauri::Url::parse(&url).unwrap();
    assert!(parsed
        .query_pairs()
        .any(|(key, value)| key == "next" && value == "4123456"));
    assert!(!parsed.query_pairs().any(|(key, _)| key == "page"));
    assert!(matches!(
        list_next_page(Some(next), 4123456, base),
        Some(ListPage::Url(_))
    ));
    assert!(list_next_page(Some(&url), 4123456, &url).is_none());
    assert!(matches!(
        list_next_page(None, 2, base),
        Some(ListPage::Number(2))
    ));
}

#[test]
fn continuations_cannot_change_origin_or_endpoint() {
    for cursor in [
        "https://elsewhere.test/?next=1",
        "//elsewhere.test/?next=1",
        "http://e-hentai.org/?next=1",
        "/g/1/token/",
        "https://user@e-hentai.org/?next=1",
    ] {
        assert!(
            list_request_url("https://e-hentai.org/", Some(cursor)).is_err(),
            "{cursor}"
        );
    }
    assert!(list_request_url(
        "https://e-hentai.org/favorites.php?favcat=2",
        Some("?next=10&favcat=2")
    )
    .is_ok());
}
