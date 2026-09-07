use super::*;

fn temp_dir(name: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "exviewer-den-test-{}-{}-{unique}",
        name,
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn sanitize_replaces_filesystem_hostiles() {
    assert_eq!(
        sanitize_filename(r#"a:b\c/d*e?f|g<h>i"j"#),
        "a_b_c_d_e_f_g_h_i_j"
    );
    assert_eq!(sanitize_filename("ctrl\u{1f}\u{0}char"), "ctrl__char");
    assert_eq!(
        sanitize_filename("normal title (2024) [digital]"),
        "normal title (2024) [digital]"
    );
    // Windows-hostile trailing dot/space is trimmed (documented deviation).
    assert_eq!(sanitize_filename("name. "), "name");
    assert_eq!(sanitize_filename(""), "");
}

#[test]
fn dirname_uses_japanese_title_when_present() {
    assert_eq!(suitable_dirname(1234, "English", None), "1234-English");
    assert_eq!(
        suitable_dirname(1234, "English", Some("日本語")),
        "1234-日本語"
    );
    assert_eq!(
        suitable_dirname(1234, "English", Some("   ")),
        "1234-English"
    );
    assert_eq!(suitable_dirname(1234, "a/b", Some("x:y")), "1234-x_y");
}

#[test]
fn page_filenames_are_four_digit_with_original_extension() {
    assert_eq!(page_filename(0, "jpg"), "0001.jpg");
    assert_eq!(page_filename(41, "png"), "0042.png");
    assert_eq!(page_filename(999, ".webp"), "1000.webp");
}

#[test]
fn page_index_recognizes_four_and_eight_digit_names() {
    assert_eq!(page_index_of("0001.jpg"), Some(0));
    assert_eq!(page_index_of("0042.png"), Some(41));
    assert_eq!(page_index_of("00000042.png"), Some(41));
    assert_eq!(page_index_of(".ehviewer"), None);
    assert_eq!(page_index_of("cover.jpg"), None); // not numeric
    assert_eq!(page_index_of("0001.txt"), None); // unsupported ext
    assert_eq!(page_index_of(".tmp-1-0001.jpg"), None);
}

#[test]
fn write_count_find_and_remove_pages() {
    let root = temp_dir("pages");
    let den = GalleryDen::open(&root, "42-gallery").unwrap();
    assert_eq!(den.count_pages(), 0);

    den.write_page(0, "jpg", b"first").unwrap();
    den.write_page(2, "png", b"third").unwrap();
    assert_eq!(den.count_pages(), 2);
    assert_eq!(den.find_page(0).unwrap().file_name().unwrap(), "0001.jpg");
    assert_eq!(den.find_page(1), None);
    assert_eq!(den.find_page(2).unwrap().file_name().unwrap(), "0003.png");

    // Overwriting a page does not double-count it.
    den.write_page(0, "jpg", b"first v2").unwrap();
    assert_eq!(den.count_pages(), 2);

    den.remove_dir().unwrap();
    assert!(!root.join("42-gallery").exists());
    den.remove_dir().unwrap(); // deleting twice is fine
    fs::remove_dir_all(&root).ok();
}

#[test]
fn spider_info_write_read_roundtrip() {
    let root = temp_dir("info");
    let mut info = SpiderInfo::new(987654321, "abcdef0123", 3, 1, 40);
    info.ptoken_map.insert(0, "aaaa".into());
    info.ptoken_map.insert(2, "cccc".into());
    info.ptoken_map.insert(1, "failed".into()); // skipped on write, like the original
    let path = info.write_to_dir(&root).unwrap();
    assert_eq!(path, root.join(SPIDER_INFO_FILENAME));

    let read = SpiderInfo::read(&path).unwrap().expect("readable");
    // "failed" tokens are skipped on write, like the original.
    let mut expected = info.clone();
    expected.ptoken_map.remove(&1);
    assert_eq!(read, expected);
    assert_eq!(read.ptoken_map.len(), 2);

    // Header-only parse: same counts, no ptokens.
    let header = SpiderInfo::read_header(&path).unwrap().expect("readable");
    assert_eq!(header.gid, info.gid);
    assert_eq!(header.pages, 3);
    assert!(header.ptoken_map.is_empty());

    // The on-disk layout is byte-compatible with the original format.
    let text = fs::read_to_string(&path).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("VERSION2"));
    assert_eq!(lines.next(), Some("00000000"));
    assert_eq!(lines.next(), Some("987654321"));
    assert_eq!(lines.next(), Some("abcdef0123"));
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("40"));
    assert_eq!(lines.next(), Some("3"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn spider_info_tracks_resume_progress_in_start_page() {
    let root = temp_dir("progress");
    let mut info = SpiderInfo::new(1, "tok", 10, 1, 40);
    info.start_page = 7;
    let path = info.write_to_dir(&root).unwrap();
    let read = SpiderInfo::read_header(&path).unwrap().unwrap();
    assert_eq!(read.start_page, 7);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn spider_info_reads_legacy_v1_files() {
    let root = temp_dir("v1");
    fs::write(
        root.join(SPIDER_INFO_FILENAME),
        "00000003\n42\ntok\n1\n2\n12\n0 aaaa\n1 bbbb\n",
    )
    .unwrap();
    let path = root.join(SPIDER_INFO_FILENAME);
    // v1 has no preview-per-page line; "12" is the page count.
    let info = SpiderInfo::read(&path).unwrap().expect("v1 readable");
    assert_eq!(info.start_page, 3);
    assert_eq!(info.gid, 42);
    assert_eq!(info.token, "tok");
    assert_eq!(info.preview_pages, 2);
    assert_eq!(info.preview_per_page, 0);
    assert_eq!(info.pages, 12);
    assert_eq!(info.ptoken_map.get(&1).map(String::as_str), Some("bbbb"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn spider_info_rejects_garbage() {
    let root = temp_dir("garbage");
    assert!(SpiderInfo::read(&root.join("missing")).unwrap().is_none());
    fs::write(root.join(SPIDER_INFO_FILENAME), "").unwrap();
    assert_eq!(
        SpiderInfo::read(&root.join(SPIDER_INFO_FILENAME)).unwrap(),
        None
    );
    fs::write(
        root.join(SPIDER_INFO_FILENAME),
        "VERSION2\n0\n1\ntok\n1\n1\n1\n0\n",
    )
    .unwrap();
    assert_eq!(
        SpiderInfo::read(&root.join(SPIDER_INFO_FILENAME)).unwrap(),
        None
    ); // pages = 0
    fs::write(
        root.join(SPIDER_INFO_FILENAME),
        "VERSION9\n0\n1\ntok\n1\n1\n1\n1\n",
    )
    .unwrap();
    assert_eq!(
        SpiderInfo::read(&root.join(SPIDER_INFO_FILENAME)).unwrap(),
        None
    ); // unknown version
    fs::remove_dir_all(&root).ok();
}

#[test]
fn header_read_does_not_decode_the_page_token_tail() {
    let root = temp_dir("header-only");
    let info = SpiderInfo::new(1, "token", 1000, 1, 40);
    let path = info.write_to_dir(&root).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    // The header remains useful even when an irrelevant page-token tail is corrupt.
    std::io::Write::write_all(&mut file, &[0xff; 32_000]).unwrap();
    assert_eq!(SpiderInfo::read_header(&path).unwrap().unwrap().pages, 1000);
    assert!(SpiderInfo::read(&path).unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
}
