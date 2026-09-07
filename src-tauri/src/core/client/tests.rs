use super::*;

#[test]
fn body_errors_detect_ip_ban() {
    let ban = "This IP address has been temporarily banned due to an excessive request rate. This probably means you are using automated mirroring/harvesting software or share the IP address with someone who does. The ban expires in 2 minutes and 59 seconds";
    let err = check_body_errors(ban).unwrap_err();
    assert!(err.to_string().contains("expires in"), "{err}");
    assert!(check_body_errors("<div id='itg'>fine</div>").is_ok());
}

#[test]
fn fix_thumb_resolution() {
    let client = EhClient::with_defaults(ClientConfig::default()).unwrap();
    let url = "https://ehgt.org/a/b/abcdef-1-2-3-jpg_250.jpg";
    assert_eq!(client.fix_thumb(url), url);

    let client = EhClient::with_defaults(ClientConfig {
        thumb_resolution: 2,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        client.fix_thumb(url),
        "https://ehgt.org/a/b/abcdef-1-2-3-jpg_300.jpg"
    );
    // No underscore before the extension → untouched.
    assert_eq!(client.fix_thumb("https://x/y.jpg"), "https://x/y.jpg");
}

#[test]
fn image_lookup_form_fields_match_the_original() {
    // All toggles on: file part aside, the original EhEngine.imageSearch
    // adds fs_similar/fs_covers/fs_exp ("on") then f_sfile=File Search.
    let fields = EhClient::image_lookup_form_fields(true, true, true);
    assert_eq!(
        fields,
        vec![
            ("fs_similar", "on"),
            ("fs_covers", "on"),
            ("fs_exp", "on"),
            ("f_sfile", "File Search"),
        ]
    );
    // All off: only the submit button stays.
    let fields = EhClient::image_lookup_form_fields(false, false, false);
    assert_eq!(fields, vec![("f_sfile", "File Search")]);
    // Individual toggles.
    let fields = EhClient::image_lookup_form_fields(true, false, true);
    assert_eq!(
        fields,
        vec![
            ("fs_similar", "on"),
            ("fs_exp", "on"),
            ("f_sfile", "File Search")
        ]
    );
}

#[test]
fn image_lookup_form_has_file_part_and_fields() {
    // Mock-free smoke test: the assembled multipart body must build and
    // reqwest must be able to serialize it into a request.
    let form =
        EhClient::build_image_lookup_form("cover.jpg", vec![1, 2, 3], true, false, true).unwrap();
    let request = reqwest::Client::new()
        .post("https://upld.e-hentai.org/image_lookup.php")
        .multipart(form)
        .build()
        .unwrap();
    let ct = request
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(ct.starts_with("multipart/form-data; boundary="), "{ct}");
}

#[test]
fn image_resolution_cookie_preserves_auth_and_other_preferences() {
    use reqwest::cookie::CookieStore;
    let jar = Arc::new(InMemoryCookieStore::new());
    let url = reqwest::Url::parse("https://e-hentai.org/s/token/1-1").unwrap();
    let headers = [
        HeaderValue::from_static("ipb_member_id=42; Path=/"),
        HeaderValue::from_static("uconfig=dm_t-xr_780-ts_m; Path=/"),
    ];
    jar.set_cookies(&mut headers.iter(), &url);
    let configured = CookieStoreHandle(jar.clone(), "1600".into());
    let header = configured
        .cookies(&url)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(header.contains("ipb_member_id=42"));
    assert!(header.contains("uconfig=dm_t-ts_m-xr_1600"));
    assert!(!header.contains("xr_780"));
    assert_eq!(
        jar.get_cookie_value(&url, "uconfig").as_deref(),
        Some("dm_t-xr_780-ts_m")
    );
    assert!(configured
        .cookies(&reqwest::Url::parse("https://example.org/").unwrap())
        .is_none());
    jar.clear().unwrap();
    assert_eq!(configured.cookies(&url).unwrap(), "uconfig=xr_1600");
}
