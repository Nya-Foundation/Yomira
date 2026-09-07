//! Direct rusqlite access to the downloads tables.
//!
//! The schema is owned by `migrations/0001_init.sql` (columns ported from the
//! original greenDAO `DownloadsDao` / `DownloadLabelDao`); the SQL is embedded
//! here so this module stays independent of `crate::db` and can open its own
//! connection to the same SQLite file (WAL mode makes the two connections
//! coexist with the application database connection).
//!
//! Only the columns the download engine needs are read/written; the rest
//! (`posted`, `uploader`, `rating`, `language`, `legacy`, `archive_uri`) stay
//! reserved for later phases.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::error::EhError;

/// DDL for the two tables this module owns the access path of. Must stay
/// identical to the statements in `migrations/0001_init.sql` (a test asserts
/// that); `CREATE TABLE IF NOT EXISTS` makes re-applying harmless.
pub const DOWNLOADS_DDL: &str = "\
CREATE TABLE IF NOT EXISTS downloads (
    gid         INTEGER PRIMARY KEY NOT NULL,
    token       TEXT,
    title       TEXT,
    title_jpn   TEXT,
    thumb       TEXT,
    category    INTEGER NOT NULL DEFAULT 0,
    posted      TEXT,
    uploader    TEXT,
    rating      REAL NOT NULL DEFAULT -1,
    language    TEXT,
    state       INTEGER NOT NULL DEFAULT 0,
    legacy      INTEGER NOT NULL DEFAULT 0,
    time        INTEGER NOT NULL DEFAULT 0,
    label       TEXT,
    dirname     TEXT,
    archive_uri TEXT
);

CREATE TABLE IF NOT EXISTS download_labels (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL DEFAULT '',
    time INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings_kv (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT
);
";

/// One row of the `downloads` table (the subset the engine uses).
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadRow {
    pub gid: i64,
    pub token: String,
    pub title: String,
    pub title_jpn: Option<String>,
    pub thumb: String,
    pub category: i32,
    /// Engine state as stored in the DB; see `manager::DownloadState::to_db`.
    pub state: i32,
    pub time: i64,
    pub label: Option<String>,
    pub dirname: Option<String>,
}

/// Thread-safe handle over the app DB's download tables. Same locking
/// discipline as `db::Db`: every op takes the connection lock briefly; call
/// from `spawn_blocking` when on a tokio worker thread.
pub struct DownloadStore {
    conn: Mutex<Connection>,
}

fn db_err(e: rusqlite::Error) -> EhError {
    EhError::Db(e.to_string())
}

impl DownloadStore {
    /// Open the app DB at `path`, enabling WAL and ensuring the download
    /// tables exist (idempotent).
    pub fn open(path: &Path) -> Result<Self, EhError> {
        let conn = Connection::open(path).map_err(db_err)?;
        Self::init(conn)
    }

    /// In-memory store, used by tests.
    pub fn in_memory() -> Result<Self, EhError> {
        Self::init(Connection::open_in_memory().map_err(db_err)?)
    }

    fn init(conn: Connection) -> Result<Self, EhError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        conn.execute_batch(DOWNLOADS_DDL).map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, EhError> {
        self.conn
            .lock()
            .map_err(|_| EhError::Db("download store lock poisoned".into()))
    }

    /// All downloads in queue (insertion) order: oldest enqueue time first,
    /// which is the engine's global priority order.
    pub fn list(&self) -> Result<Vec<DownloadRow>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT gid, token, title, title_jpn, thumb, category, state, time, label, dirname \
                 FROM downloads ORDER BY time ASC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(DownloadRow {
                    gid: r.get(0)?,
                    token: r.get(1)?,
                    title: r.get(2)?,
                    title_jpn: r.get(3)?,
                    thumb: r.get(4)?,
                    category: r.get(5)?,
                    state: r.get(6)?,
                    time: r.get(7)?,
                    label: r.get(8)?,
                    dirname: r.get(9)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// Insert a download or refresh its mutable columns
    /// (port of `DownloadsDao.insert`, which is an upsert on gid).
    pub fn upsert(&self, row: &DownloadRow) -> Result<(), EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO downloads (gid, token, title, title_jpn, thumb, category, state, time, label, dirname) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(gid) DO UPDATE SET \
                 token = excluded.token, title = excluded.title, title_jpn = excluded.title_jpn, \
                 thumb = excluded.thumb, category = excluded.category, state = excluded.state, \
                 time = excluded.time, label = excluded.label, dirname = excluded.dirname",
            params![
                row.gid,
                row.token,
                row.title,
                row.title_jpn,
                row.thumb,
                row.category,
                row.state,
                row.time,
                row.label,
                row.dirname,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    pub fn delete(&self, gid: i64) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM downloads WHERE gid = ?", params![gid])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn update_state(&self, gid: i64, state: i32) -> Result<(), EhError> {
        self.lock()?
            .execute(
                "UPDATE downloads SET state = ? WHERE gid = ?",
                params![state, gid],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn update_label(&self, gid: i64, label: Option<&str>) -> Result<(), EhError> {
        self.lock()?
            .execute(
                "UPDATE downloads SET label = ? WHERE gid = ?",
                params![label, gid],
            )
            .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // download_labels
    // ------------------------------------------------------------------

    /// All label names, oldest first (queue order).
    pub fn labels(&self) -> Result<Vec<String>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT name FROM download_labels ORDER BY time ASC")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn add_label(&self, name: &str) -> Result<(), EhError> {
        let conn = self.lock()?;
        let exists: Option<i64> = conn
            .query_row(
                "SELECT id FROM download_labels WHERE name = ?",
                params![name],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?;
        if exists.is_some() {
            return Ok(());
        }
        conn.execute(
            "INSERT INTO download_labels (name, time) VALUES (?, ?)",
            params![name, chrono::Utc::now().timestamp_millis()],
        )
        .map_err(db_err)?;
        Ok(())
    }

    /// Delete the label and detach it from every download carrying it
    /// (the original `DownloadLabelDao` does both).
    pub fn delete_label(&self, name: &str) -> Result<(), EhError> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM download_labels WHERE name = ?", params![name])
            .map_err(db_err)?;
        conn.execute(
            "UPDATE downloads SET label = NULL WHERE label = ?",
            params![name],
        )
        .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // settings_kv (only the download_root key)
    // ------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, EhError> {
        self.lock()?
            .query_row(
                "SELECT value FROM settings_kv WHERE key = ?",
                params![key],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), EhError> {
        self.lock()?
            .execute(
                "INSERT INTO settings_kv (key, value) VALUES (?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(gid: i64, title: &str) -> DownloadRow {
        DownloadRow {
            gid,
            token: "tok".into(),
            title: title.into(),
            title_jpn: None,
            thumb: "https://ehgt.org/t.jpg".into(),
            category: 2,
            state: 1,
            time: gid, // keep ordering deterministic without sleeping
            label: None,
            dirname: Some(format!("{gid}-dir")),
        }
    }

    #[test]
    fn embedded_ddl_matches_the_migration_file() {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations/0001_init.sql");
        let migration = std::fs::read_to_string(path).unwrap();
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let migration = normalize(&migration);
        for statement in DOWNLOADS_DDL.split(';').filter(|s| !s.trim().is_empty()) {
            assert!(
                migration.contains(&normalize(statement)),
                "download store DDL drifted from migrations/0001_init.sql:\n{statement}"
            );
        }
    }

    #[test]
    fn upsert_list_and_delete_roundtrip() {
        let store = DownloadStore::in_memory().unwrap();
        assert!(store.list().unwrap().is_empty());

        store.upsert(&row(1, "one")).unwrap();
        store.upsert(&row(2, "two")).unwrap();
        let all = store.list().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].gid, 1); // oldest enqueue time first

        // Upsert refreshes mutable columns instead of duplicating.
        let mut refreshed = row(1, "one again");
        refreshed.label = Some("fav".into());
        store.upsert(&refreshed).unwrap();
        let all = store.list().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].title, "one again");
        assert_eq!(all[0].label.as_deref(), Some("fav"));

        store.delete(1).unwrap();
        store.delete(999).unwrap(); // unknown gid is a no-op
        let rest = store.list().unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].gid, 2);
    }

    #[test]
    fn state_and_label_updates() {
        let store = DownloadStore::in_memory().unwrap();
        store.upsert(&row(1, "one")).unwrap();
        store.update_state(1, 4).unwrap();
        store.update_label(1, Some("label")).unwrap();
        let item = &store.list().unwrap()[0];
        assert_eq!(item.state, 4);
        assert_eq!(item.label.as_deref(), Some("label"));

        store.update_label(1, None).unwrap();
        assert_eq!(store.list().unwrap()[0].label, None);
    }

    #[test]
    fn labels_crud_and_detach_on_delete() {
        let store = DownloadStore::in_memory().unwrap();
        store.add_label("miku").unwrap();
        store.add_label("miku").unwrap(); // duplicate is a no-op
        store.add_label("other").unwrap();
        assert_eq!(store.labels().unwrap(), vec!["miku", "other"]);

        store.upsert(&row(1, "one")).unwrap();
        store.update_label(1, Some("miku")).unwrap();

        store.delete_label("miku").unwrap();
        assert_eq!(store.labels().unwrap(), vec!["other"]);
        // Deleting the label detaches it from downloads.
        assert_eq!(store.list().unwrap()[0].label, None);
    }

    #[test]
    fn settings_roundtrip() {
        let store = DownloadStore::in_memory().unwrap();
        assert_eq!(store.get_setting("download_root").unwrap(), None);
        store.set_setting("download_root", "/tmp/dl").unwrap();
        store.set_setting("download_root", "/tmp/dl2").unwrap();
        assert_eq!(
            store.get_setting("download_root").unwrap().as_deref(),
            Some("/tmp/dl2")
        );
    }
}
