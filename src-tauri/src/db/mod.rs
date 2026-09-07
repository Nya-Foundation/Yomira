//! SQLite layer.
//!
//! The app data DB (`exviewer.db` in the Tauri app-config dir) is owned by
//! rusqlite. Frontend access goes through Tauri commands. The schema lives
//! in `src-tauri/migrations` and is applied idempotently at startup.
//!
//! Cookies are intentionally NOT stored here — the live cookie jar is
//! persisted as JSON (`cookies.json`) by `crate::cookies`, matching how the
//! in-memory store works. A SQLite cookie store can replace it later behind
//! the same `EhCookieStore` trait.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::core::error::EhError;
use crate::core::models::GalleryInfo;

/// Initial schema, embedded from `migrations/0001_init.sql`.
pub const MIGRATION_SQL: &str = include_str!("../../migrations/0001_init.sql");

/// Search-history schema, embedded from `migrations/0002_search_history.sql`.
pub const MIGRATION_2_SQL: &str = include_str!("../../migrations/0002_search_history.sql");

/// One row of the `filter` table (EhFilter entries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterEntry {
    pub mode: String,
    pub text: String,
}

/// One row of the `blacklist` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlacklistEntry {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub time: i64,
}

/// One row of the `history` table (newest first when listed).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub thumb: String,
    pub category: i32,
    pub time: i64,
    pub pages: i32,
}

/// One row of the `quick_search` table.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickSearchEntry {
    pub id: i64,
    pub name: String,
    pub keyword: String,
    pub categories: Option<String>,
    pub min_rating: i32,
    pub min_pages: i32,
    pub max_pages: i32,
    pub time: i64,
}

/// Thread-safe handle over the app data DB. Every operation takes the
/// connection lock briefly; call from `spawn_blocking` when on a tokio
/// worker thread.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Open (creating if needed) the DB at `path` and apply migrations.
    pub fn open(path: &Path) -> Result<Self, EhError> {
        let conn = Connection::open(path).map_err(db_err)?;
        Self::init(conn)
    }

    /// In-memory DB, used by tests.
    pub fn open_in_memory() -> Result<Self, EhError> {
        Self::init(Connection::open_in_memory().map_err(db_err)?)
    }

    fn init(conn: Connection) -> Result<Self, EhError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(db_err)?;
        conn.execute_batch(MIGRATION_SQL).map_err(db_err)?;
        conn.execute_batch(MIGRATION_2_SQL).map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, EhError> {
        self.conn
            .lock()
            .map_err(|_| EhError::Db("db lock poisoned".into()))
    }

    // ------------------------------------------------------------------
    // settings_kv
    // ------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, EhError> {
        let conn = self.lock()?;
        conn.query_row("SELECT value FROM settings_kv WHERE key = ?", [key], |r| {
            r.get(0)
        })
        .optional()
        .map_err(db_err)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO settings_kv (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // history
    // ------------------------------------------------------------------

    /// Insert or refresh a history row (port of `HistoryDao.insert`).
    pub fn record_history(&self, info: &GalleryInfo) -> Result<(), EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO history (gid, token, title, thumb, category, time, pages) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(gid) DO UPDATE SET \
                 token = excluded.token, title = excluded.title, thumb = excluded.thumb, \
                 category = excluded.category, time = excluded.time, pages = excluded.pages",
            params![
                info.gid as i64,
                info.token,
                info.title,
                info.thumb,
                info.category,
                chrono::Utc::now().timestamp_millis(),
                info.pages,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    /// One page of history, newest first.
    pub fn history(&self, page: i64, limit: i64) -> Result<Vec<HistoryEntry>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT gid, token, title, thumb, category, time, pages FROM history \
                 ORDER BY time DESC LIMIT ? OFFSET ?",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![limit, page.saturating_mul(limit)], |r| {
                Ok(HistoryEntry {
                    gid: r.get::<_, i64>(0)? as u64,
                    token: r.get(1)?,
                    title: r.get(2)?,
                    thumb: r.get(3)?,
                    category: r.get(4)?,
                    time: r.get(5)?,
                    pages: r.get(6)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn clear_history(&self) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM history", [])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn delete_history(&self, gid: i64) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM history WHERE gid = ?", params![gid])
            .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // quick search CRUD
    // ------------------------------------------------------------------

    pub fn quick_searches(&self) -> Result<Vec<QuickSearchEntry>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, keyword, categories, min_rating, min_pages, max_pages, time \
                 FROM quick_search ORDER BY time DESC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(QuickSearchEntry {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    keyword: r.get(2)?,
                    categories: r.get(3)?,
                    min_rating: r.get(4)?,
                    min_pages: r.get(5)?,
                    max_pages: r.get(6)?,
                    time: r.get(7)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn add_quick_search(
        &self,
        name: &str,
        keyword: &str,
        categories: Option<&str>,
        min_rating: i32,
        min_pages: i32,
        max_pages: i32,
    ) -> Result<i64, EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO quick_search (name, keyword, categories, min_rating, min_pages, max_pages, time) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                name,
                keyword,
                categories,
                min_rating,
                min_pages,
                max_pages,
                chrono::Utc::now().timestamp_millis(),
            ],
        )
        .map_err(db_err)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_quick_search(&self, id: i64) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM quick_search WHERE id = ?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // search history (search-term suggestions)
    // ------------------------------------------------------------------

    /// The 20 most recent search terms, newest first.
    pub fn search_history(&self) -> Result<Vec<String>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT term FROM search_history ORDER BY time DESC LIMIT 20")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// Record a search term: empty/whitespace terms are ignored, repeats
    /// refresh `time` so the term moves to the top (upsert on the PK).
    pub fn record_search(&self, term: &str) -> Result<(), EhError> {
        let term = term.trim();
        if term.is_empty() {
            return Ok(());
        }
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO search_history (term, time) VALUES (?, ?) \
             ON CONFLICT(term) DO UPDATE SET time = excluded.time",
            params![term, chrono::Utc::now().timestamp_millis()],
        )
        .map_err(db_err)?;
        Ok(())
    }

    pub fn clear_search_history(&self) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM search_history", [])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn delete_search_term(&self, term: &str) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM search_history WHERE term = ?", params![term])
            .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // blacklist CRUD
    // ------------------------------------------------------------------

    pub fn blacklist(&self) -> Result<Vec<BlacklistEntry>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, type, content, time FROM blacklist ORDER BY time DESC")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(BlacklistEntry {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    content: r.get(2)?,
                    time: r.get(3)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn add_blacklist(&self, kind: &str, content: &str) -> Result<i64, EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO blacklist (type, content, time) VALUES (?, ?, ?)",
            params![kind, content, chrono::Utc::now().timestamp_millis()],
        )
        .map_err(db_err)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_blacklist(&self, id: i64) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM blacklist WHERE id = ?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // filters (EhFilter) — read by the list pipeline
    // ------------------------------------------------------------------

    /// All `filter` rows followed by all `blacklist` rows (as title/uploader/
    /// tag entries), the union the original EhFilter applies to list results.
    pub fn filters(&self) -> Result<Vec<FilterEntry>, EhError> {
        let conn = self.lock()?;
        let mut entries = Vec::new();
        let mut stmt = conn
            .prepare("SELECT mode, text FROM filter WHERE text <> ''")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(FilterEntry {
                    mode: r.get(0)?,
                    text: r.get(1)?,
                })
            })
            .map_err(db_err)?;
        entries.extend(rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?);

        let mut stmt = conn
            .prepare("SELECT type, content FROM blacklist WHERE content <> ''")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(FilterEntry {
                    mode: r.get(0)?,
                    text: r.get(1)?,
                })
            })
            .map_err(db_err)?;
        entries.extend(rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?);
        Ok(entries)
    }

    /// `filter` table rows with ids (for CRUD); use [`Db::filters`] for the
    /// union with the blacklist that list filtering consumes.
    pub fn filter_table(&self) -> Result<Vec<(i64, String, String)>, EhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, mode, text FROM filter ORDER BY id")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn add_filter(&self, mode: &str, text: &str) -> Result<i64, EhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO filter (mode, text) VALUES (?, ?)",
            params![mode, text],
        )
        .map_err(db_err)?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_filter(&self, id: i64) -> Result<(), EhError> {
        self.lock()?
            .execute("DELETE FROM filter WHERE id = ?", [id])
            .map_err(db_err)?;
        Ok(())
    }
}

fn db_err(e: rusqlite::Error) -> EhError {
    EhError::Db(e.to_string())
}

#[cfg(test)]
mod tests;
