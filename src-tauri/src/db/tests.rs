use super::*;
use std::path::PathBuf;

#[test]
fn embedded_migration_matches_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations/0001_init.sql");
    let on_disk = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        on_disk.trim_end(),
        MIGRATION_SQL.trim_end(),
        "migrations/0001_init.sql and db::MIGRATION_SQL drifted apart"
    );

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations/0002_search_history.sql");
    let on_disk = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        on_disk.trim_end(),
        MIGRATION_2_SQL.trim_end(),
        "migrations/0002_search_history.sql and db::MIGRATION_2_SQL drifted apart"
    );
}

#[test]
fn migration_applies_cleanly() {
    let db = Db::open_in_memory().unwrap();
    // Second init on the same connection is a no-op (idempotent).
    let conn = db.lock().unwrap();
    conn.execute_batch(MIGRATION_SQL).unwrap();
    drop(conn);

    let tables: Vec<String> = db
        .lock()
        .unwrap()
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for expected in [
        "history",
        "local_favorites",
        "quick_search",
        "blacklist",
        "filter",
        "read_position",
        "settings_kv",
        "downloads",
        "download_labels",
        "search_history",
    ] {
        assert!(
            tables.iter().any(|t| t == expected),
            "missing table {expected}"
        );
    }
}

#[test]
fn settings_roundtrip() {
    let db = Db::open_in_memory().unwrap();
    assert_eq!(db.get_setting("site").unwrap(), None);
    db.set_setting("site", "ex").unwrap();
    db.set_setting("site", "e").unwrap();
    assert_eq!(db.get_setting("site").unwrap().as_deref(), Some("e"));
}

#[test]
fn history_records_and_pages() {
    let db = Db::open_in_memory().unwrap();
    let mk = |gid, title: &str| GalleryInfo {
        gid,
        token: "tok".into(),
        title: title.into(),
        ..Default::default()
    };
    db.record_history(&mk(1, "one")).unwrap();
    // Millisecond timestamps can tie within one test run; space the
    // inserts so the recency ordering is deterministic.
    std::thread::sleep(std::time::Duration::from_millis(2));
    db.record_history(&mk(2, "two")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    db.record_history(&mk(1, "one again")).unwrap();

    let page = db.history(0, 10).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].gid, 1);
    assert_eq!(page[0].title, "one again");

    assert_eq!(db.history(1, 1).unwrap().len(), 1);
    assert!(db.history(2, 1).unwrap().is_empty());

    db.delete_history(1).unwrap();
    let rest = db.history(0, 10).unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].gid, 2);
    db.delete_history(999).unwrap(); // unknown gid is a no-op

    db.clear_history().unwrap();
    assert!(db.history(0, 10).unwrap().is_empty());
}

#[test]
fn quick_search_crud() {
    let db = Db::open_in_memory().unwrap();
    let id = db
        .add_quick_search("miku", "hatsune miku", Some("[]"), 3, -1, -1)
        .unwrap();
    db.add_quick_search("other", "x", None, -1, -1, -1).unwrap();
    let all = db.quick_searches().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "other"); // newest first
    db.delete_quick_search(id).unwrap();
    assert_eq!(db.quick_searches().unwrap().len(), 1);
}

#[test]
fn search_history_records_dedupes_and_prunes() {
    let db = Db::open_in_memory().unwrap();
    // Empty and whitespace-only terms are ignored.
    db.record_search("").unwrap();
    db.record_search("   ").unwrap();
    assert!(db.search_history().unwrap().is_empty());

    db.record_search("full color").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    db.record_search("touhou").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    // Repeat: refreshes time, no duplicate row.
    db.record_search("full color").unwrap();

    let terms = db.search_history().unwrap();
    assert_eq!(terms, vec!["full color", "touhou"]);

    db.delete_search_term("touhou").unwrap();
    assert_eq!(db.search_history().unwrap(), vec!["full color"]);
    db.delete_search_term("missing").unwrap(); // no-op

    // Only the 20 most recent are kept.
    db.clear_search_history().unwrap();
    for i in 0..25 {
        db.record_search(&format!("term {i:02}")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let terms = db.search_history().unwrap();
    assert_eq!(terms.len(), 20);
    assert_eq!(terms[0], "term 24");
    assert_eq!(terms[19], "term 05");

    db.clear_search_history().unwrap();
    assert!(db.search_history().unwrap().is_empty());
}

#[test]
fn blacklist_and_filters_union() {
    let db = Db::open_in_memory().unwrap();
    db.add_blacklist("tag", "scat").unwrap();
    db.add_blacklist("uploader", "spammy").unwrap();
    db.add_filter("title", "decensored").unwrap();

    let blacklist = db.blacklist().unwrap();
    assert_eq!(blacklist.len(), 2);

    let filters = db.filters().unwrap();
    assert_eq!(filters.len(), 3);
    assert!(filters
        .iter()
        .any(|f| f.mode == "title" && f.text == "decensored"));
    assert!(filters.iter().any(|f| f.mode == "tag" && f.text == "scat"));
    assert!(filters
        .iter()
        .any(|f| f.mode == "uploader" && f.text == "spammy"));
}
