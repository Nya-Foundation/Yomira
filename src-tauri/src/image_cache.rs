//! Bounded on-disk image cache + local HTTP URL formatting.
//!
//! All thumbnails and reader pages go through here: `get_image` fetches the
//! bytes through [`EhClient::get_image_bytes`], stores them under
//! `{app_data}/image_cache/<key>.<ext>` and returns a
//! `http://127.0.0.1:<port>/<file_name>` URL served by
//! [`crate::http_images`] that the frontend can drop straight into
//! `<img src>`. (The Tauri `asset://` scheme would be nicer, but WebKit2GTK
//! refuses to load custom schemes from the dev origin — hence the loopback
//! server.)
//!
//! Keys are FNV-1a 64 (inline, no extra deps — deterministic across runs).
//! Eviction is LRU over an in-memory index (`atime` = a logical clock bumped
//! on every hit); when the total size passes the cap the oldest entries are
//! removed until at least ~10% of the cap in bytes is freed, so the cap is
//! not re-hit immediately. The index is rebuilt by scanning the directory
//! when it is missing (e.g. first run after an upgrade).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::error::EhError;
use crate::core::EhClient;
use crate::http_images::http_url;

/// Default cap, mirroring the original's 512 MiB `cacheThumb` budget.
pub const DEFAULT_MAX_BYTES: u64 = 500 * 1024 * 1024;
/// Fraction of entries dropped each time the cap is exceeded.
const EVICT_FRACTION: usize = 10;

/// FNV-1a 64-bit hash, hex-encoded — stable cache keys without extra crates.
pub fn cache_key(url: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for &b in url.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// Turn a cache file path into the loopback URL the webview loads.
fn serve_url(port: u16, path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    http_url(port, &name)
}

/// File extension for a cached image: prefer the original URL's suffix,
/// fall back to the response content type, finally `.jpg`.
pub fn extension_for(url: &str, content_type: &str) -> &'static str {
    let from_url = url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match from_url.as_str() {
        "jpg" | "jpeg" => return "jpg",
        "png" => return "png",
        "gif" => return "gif",
        "webp" => return "webp",
        _ => {}
    }
    if content_type.contains("png") {
        "png"
    } else if content_type.contains("gif") {
        "gif"
    } else if content_type.contains("webp") {
        "webp"
    } else {
        "jpg"
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    /// File name inside the cache dir (`<key>.<ext>`).
    file_name: String,
    size: u64,
    /// Logical access clock; higher = more recently used.
    atime: u64,
}

#[derive(Debug, Default)]
struct CacheInner {
    entries: HashMap<String, CacheEntry>,
    total: u64,
    clock: u64,
}

/// On-disk LRU cache rooted at a fixed directory.
pub struct ImageCache {
    dir: PathBuf,
    max_bytes: std::sync::atomic::AtomicU64,
    inner: Mutex<CacheInner>,
}

impl ImageCache {
    /// Open the cache rooted at `dir`, rebuilding the index from disk.
    pub fn open(dir: PathBuf, max_bytes: u64) -> Result<Self, EhError> {
        std::fs::create_dir_all(&dir)?;
        let mut inner = CacheInner::default();
        for item in std::fs::read_dir(&dir)? {
            let item = item?;
            if !item.file_type()?.is_file() {
                continue;
            }
            let name = item.file_name().to_string_lossy().into_owned();
            let Some(key) = name.split('.').next().filter(|k| k.len() == 16) else {
                continue;
            };
            let size = item.metadata()?.len();
            inner.clock += 1;
            inner.entries.insert(
                key.to_string(),
                CacheEntry {
                    file_name: name,
                    size,
                    atime: inner.clock,
                },
            );
            inner.total += size;
        }
        let cache = Self {
            dir,
            max_bytes: std::sync::atomic::AtomicU64::new(max_bytes),
            inner: Mutex::new(inner),
        };
        cache.evict_if_needed()?;
        Ok(cache)
    }

    /// Cache root (served by the loopback image server).
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_max_bytes(&self, max_bytes: u64) -> Result<(), EhError> {
        self.max_bytes
            .store(max_bytes, std::sync::atomic::Ordering::Relaxed);
        self.evict_if_needed()
    }

    /// Return the cached file for `url` if present (bumping its LRU position),
    /// or `None` on a miss.
    pub fn get(&self, url: &str) -> Result<Option<PathBuf>, EhError> {
        let key = cache_key(url);
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| EhError::Db("cache lock poisoned".into()))?;
        let Some(file_name) = inner.entries.get(&key).map(|e| e.file_name.clone()) else {
            return Ok(None);
        };
        let path = self.dir.join(&file_name);
        if !path.is_file() {
            if let Some(entry) = inner.entries.remove(&key) {
                inner.total = inner.total.saturating_sub(entry.size);
            }
            return Ok(None);
        }
        inner.clock += 1;
        let clock = inner.clock;
        if let Some(entry) = inner.entries.get_mut(&key) {
            entry.atime = clock;
        }
        Ok(Some(path))
    }

    /// Store `bytes` for `url` and return the file path (replacing any
    /// previous entry). The write runs on the async fs so multi-MB reader
    /// pages don't block a runtime worker; eviction runs after the write.
    pub async fn store(
        &self,
        url: &str,
        bytes: &[u8],
        extension: &str,
    ) -> Result<PathBuf, EhError> {
        let key = cache_key(url);
        let file_name = format!("{key}.{extension}");
        let path = self.dir.join(&file_name);
        let tmp = self.dir.join(format!("{key}.tmp"));
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(&tmp, &path).await?;

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| EhError::Db("cache lock poisoned".into()))?;
        if let Some(old) = inner.entries.remove(&key) {
            inner.total = inner.total.saturating_sub(old.size);
        }
        inner.clock += 1;
        let clock = inner.clock;
        inner.entries.insert(
            key,
            CacheEntry {
                file_name,
                size: bytes.len() as u64,
                atime: clock,
            },
        );
        inner.total += bytes.len() as u64;
        drop(inner);
        self.evict_if_needed()?;
        Ok(path)
    }

    /// Drop the oldest entries until the total is back under the cap. When
    /// eviction triggers, at least 1/`EVICT_FRACTION` of the cap in *bytes*
    /// goes, so the cap is not re-hit immediately on the next stores; the
    /// single pass always continues until the total is under the cap.
    fn evict_if_needed(&self) -> Result<(), EhError> {
        let max = self.max_bytes();
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| EhError::Db("cache lock poisoned".into()))?;
        if inner.total <= max {
            return Ok(());
        }
        let target_free = (max / EVICT_FRACTION as u64).max(1);
        let mut victims: Vec<(String, u64)> = inner
            .entries
            .iter()
            .map(|(k, e)| (k.clone(), e.atime))
            .collect();
        victims.sort_by_key(|(_, atime)| *atime);
        let mut freed = 0u64;
        for (key, _) in victims {
            if freed >= target_free && inner.total - freed <= max {
                break;
            }
            if let Some(entry) = inner.entries.remove(&key) {
                freed += entry.size;
                inner.total = inner.total.saturating_sub(entry.size);
                let _ = std::fs::remove_file(self.dir.join(&entry.file_name));
            }
        }
        Ok(())
    }
}

/// Fetch `url` through the client (with hotlink referer rules) and return a
/// ready-to-display loopback URL, using the disk cache when possible.
pub async fn get_image(
    client: &EhClient,
    cache: &ImageCache,
    port: u16,
    url: &str,
) -> Result<String, EhError> {
    if let Some(path) = cache.get(url)? {
        return Ok(serve_url(port, &path));
    }
    // Thumbs from ehgt need no referer; everything else (reader pages on the
    // exhentai image hosts) requires one or the server answers 403.
    let referer = if url.contains("ehgt.org") {
        None
    } else {
        Some(crate::core::client::url::referer(client.site()))
    };
    let (bytes, content_type) = client.get_image_bytes(url, referer).await?;
    let path = cache
        .store(url, &bytes, extension_for(url, &content_type))
        .await?;
    Ok(serve_url(port, &path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "exviewer-cache-test-{tag}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cache_key_is_stable_and_distinct() {
        assert_eq!(
            cache_key("https://ehgt.org/a.jpg"),
            cache_key("https://ehgt.org/a.jpg")
        );
        assert_ne!(
            cache_key("https://ehgt.org/a.jpg"),
            cache_key("https://ehgt.org/b.jpg")
        );
        assert_eq!(cache_key("x").len(), 16);
        assert!(cache_key("x").chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn serve_url_uses_loopback_http() {
        let dir = temp_dir("url");
        let cache = ImageCache::open(dir.clone(), DEFAULT_MAX_BYTES).unwrap();
        let path = cache
            .store("https://x/a.jpg", &[0u8; 4], "jpg")
            .await
            .unwrap();
        let url = serve_url(4321, &path);
        assert!(url.starts_with("http://127.0.0.1:4321/"), "{url}");
        assert!(url.ends_with(".jpg"), "{url}");
        let name = &url[url.rfind('/').unwrap() + 1..];
        assert!(
            name[..name.rfind('.').unwrap()]
                .chars()
                .all(|c| c.is_ascii_hexdigit()),
            "{url}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extension_prefers_url_then_content_type() {
        assert_eq!(extension_for("https://x/a.JPG?y=1", ""), "jpg");
        assert_eq!(extension_for("https://x/a.webp", ""), "webp");
        assert_eq!(extension_for("https://x/a", "image/png"), "png");
        assert_eq!(
            extension_for("https://x/a.bin", "application/octet-stream"),
            "jpg"
        );
    }

    #[tokio::test]
    async fn store_get_roundtrip_and_lru_eviction() {
        let dir = temp_dir("lru");
        // 100-byte cap: three 40-byte files must trigger eviction.
        let cache = ImageCache::open(dir.clone(), 100).unwrap();

        let path = cache
            .store("https://x/a.jpg", &[0u8; 40], "jpg")
            .await
            .unwrap();
        assert!(path.is_file());
        cache
            .store("https://x/b.jpg", &[0u8; 40], "jpg")
            .await
            .unwrap();
        cache
            .store("https://x/c.jpg", &[0u8; 40], "jpg")
            .await
            .unwrap();

        // Oldest (a) was evicted once the cap was exceeded; b and c remain.
        assert!(cache.get("https://x/a.jpg").unwrap().is_none());
        assert!(cache.get("https://x/b.jpg").unwrap().is_some());
        assert!(cache.get("https://x/c.jpg").unwrap().is_some());

        // b and c were both touched above, so b is the coldest of the two;
        // storing d must evict b and keep the recently used c.
        cache
            .store("https://x/d.jpg", &[0u8; 40], "jpg")
            .await
            .unwrap();
        assert!(cache.get("https://x/b.jpg").unwrap().is_none());
        assert!(cache.get("https://x/c.jpg").unwrap().is_some());
        assert!(cache.get("https://x/d.jpg").unwrap().is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn eviction_gets_under_cap_in_one_pass() {
        let dir = temp_dir("evict-pass");
        // 100-byte cap, 8-byte entries: the old count-based eviction (10% of
        // ~15 entries = 2) could not free enough bytes and re-triggered on
        // every subsequent store. The byte-targeted pass must leave the
        // total under the cap after a single store.
        let cache = ImageCache::open(dir.clone(), 100).unwrap();
        for i in 0..15 {
            cache
                .store(&format!("https://x/{i}.jpg"), &[0u8; 8], "jpg")
                .await
                .unwrap();
        }
        let total = cache.inner.lock().unwrap().total;
        assert!(total <= 100, "total {total} still over cap after eviction");
        // LRU order kept: the most recent entries survive on disk.
        assert!(cache.get("https://x/14.jpg").unwrap().is_some());
        assert!(cache.get("https://x/0.jpg").unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn index_survives_reopen() {
        let dir = temp_dir("reopen");
        let cache = ImageCache::open(dir.clone(), DEFAULT_MAX_BYTES).unwrap();
        cache
            .store("https://x/z.png", &[1u8; 10], "png")
            .await
            .unwrap();
        drop(cache);

        let cache = ImageCache::open(dir.clone(), DEFAULT_MAX_BYTES).unwrap();
        let path = cache
            .get("https://x/z.png")
            .unwrap()
            .expect("entry rebuilt from disk");
        assert!(path.is_file());
        assert_eq!(path.extension().unwrap(), "png");

        std::fs::remove_dir_all(&dir).ok();
    }
}
