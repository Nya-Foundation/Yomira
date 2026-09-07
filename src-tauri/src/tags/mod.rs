//! Tag-translation database: shared state, maintenance and suggestion API.
//!
//! See `db.rs` for the packed format / matching port and `download.rs` for
//! the fetch logic. The [`TagDb`] handle lives in `AppState`; commands read
//! it through it, and startup kicks a fire-and-forget weekly maintenance
//! task that never propagates errors.

mod db;
mod download;
mod sha1;

pub use db::{namespace_to_prefix, Suggestion, TagDatabase, TagEntry, MAX_SUGGESTIONS};

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

/// Handle to the tag DB shared through `AppState`.
pub struct TagDb {
    dir: PathBuf,
    db: Arc<RwLock<Option<TagDatabase>>>,
    /// sha1 of the packed artifact currently in memory ("" when absent).
    version: Arc<RwLock<String>>,
}

impl TagDb {
    /// Load any usable on-disk copy; the DB starts unavailable if there is
    /// none or it fails verification.
    pub fn load(dir: PathBuf) -> TagDb {
        let loaded = download::load_from_disk(&dir);
        let version = loaded
            .as_ref()
            .map(|(_, sha1)| sha1.clone())
            .unwrap_or_default();
        TagDb {
            dir,
            db: Arc::new(RwLock::new(loaded.map(|(db, _)| db))),
            version: Arc::new(RwLock::new(version)),
        }
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    pub async fn suggest(&self, query: &str) -> Vec<Suggestion> {
        let guard = self.db.read().await;
        guard
            .as_ref()
            .map(|db| db.suggest(query))
            .unwrap_or_default()
    }

    /// Cloneable inner handle so command handlers can move the DB into
    /// `spawn_blocking`: `suggest` linear-scans ~200k entries and must not
    /// occupy an async runtime worker.
    pub fn inner(&self) -> Arc<RwLock<Option<TagDatabase>>> {
        self.db.clone()
    }

    /// Exact translation for a packed-DB key like `f:big_breasts`. The
    /// database stores the literal string "null" for untranslated entries;
    /// surface those as `None`, like the original's callers do.
    pub async fn translate(&self, key: &str) -> Option<String> {
        let guard = self.db.read().await;
        guard
            .as_ref()
            .and_then(|db| db.translate(key))
            .filter(|t| t != "null")
    }

    /// Batch translation with a single read guard for the whole list — one
    /// lock acquisition for a detail page's 50-150 tags instead of one per
    /// tag. The "null" sentinel is filtered exactly like [`TagDb::translate`].
    pub async fn translate_many(&self, keys: &[String]) -> Vec<Option<String>> {
        let guard = self.db.read().await;
        let Some(db) = guard.as_ref() else {
            return vec![None; keys.len()];
        };
        keys.iter()
            .map(|key| db.translate(key).filter(|t| t != "null"))
            .collect()
    }

    /// `(available, version, entry_count)` for `tag_db_status`.
    pub async fn status(&self) -> (bool, String, usize) {
        let guard = self.db.read().await;
        match guard.as_ref() {
            Some(db) => {
                let version = self.version.read().await.clone();
                (true, version, db.entry_count())
            }
            None => (false, String::new(), 0),
        }
    }

    /// Force or schedule a refresh and swap the result into memory.
    pub async fn update(&self, force: bool) -> Result<(), String> {
        let (db, sha1) = download::refresh(&self.dir, force).await?;
        *self.version.write().await = sha1;
        *self.db.write().await = Some(db);
        Ok(())
    }
}

/// Startup maintenance: refresh only when the copy is missing or older than
/// a week. Never panics and never surfaces errors — the feature is simply
/// unavailable while offline, exactly like the original.
pub async fn maintain(handle: Arc<TagDb>) {
    if !download::needs_recheck(&handle.dir) {
        return;
    }
    if let Err(e) = handle.update(false).await {
        eprintln!("[tags] background refresh failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;

    /// In-memory TagDb over a synthetic packed artifact.
    fn test_db(lines: &[(&str, &str)]) -> TagDb {
        let mut text = String::new();
        for (en, zh) in lines {
            text.push_str(en);
            text.push('\r');
            text.push_str(&BASE64.encode(zh.as_bytes()));
            text.push('\n');
        }
        let mut packed = (text.len() as u32).to_be_bytes().to_vec();
        packed.extend_from_slice(text.as_bytes());
        let db = TagDatabase::from_packed(&packed).expect("synthetic blob parses");
        TagDb {
            dir: PathBuf::new(),
            db: Arc::new(RwLock::new(Some(db))),
            version: Arc::new(RwLock::new(String::new())),
        }
    }

    #[tokio::test]
    async fn translate_returns_none_for_null_sentinel() {
        let db = test_db(&[("f:big_breasts", "巨乳"), ("m:males_only", "null")]);
        assert_eq!(db.translate("f:big_breasts").await.as_deref(), Some("巨乳"));
        assert_eq!(db.translate("m:males_only").await, None);
        assert_eq!(db.translate("f:missing").await, None);
    }

    #[tokio::test]
    async fn translate_many_looks_up_all_keys_and_filters_null() {
        let db = test_db(&[("f:big_breasts", "巨乳"), ("m:males_only", "null")]);
        let out = db
            .translate_many(&[
                "f:big_breasts".to_string(),
                "m:males_only".to_string(),
                "f:missing".to_string(),
            ])
            .await;
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].as_deref(), Some("巨乳"));
        assert_eq!(out[1], None);
        assert_eq!(out[2], None);
    }

    #[tokio::test]
    async fn translate_many_with_unavailable_db_is_all_none() {
        let db = TagDb {
            dir: PathBuf::new(),
            db: Arc::new(RwLock::new(None)),
            version: Arc::new(RwLock::new(String::new())),
        };
        let out = db
            .translate_many(&["f:x".to_string(), "m:y".to_string()])
            .await;
        assert_eq!(out, vec![None, None]);
    }
}
