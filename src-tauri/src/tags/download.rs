//! Download and on-disk maintenance of the packed tag-translation database.
//!
//! This ports `EhTagDatabase.update()` from Ehviewer_CN_SXJ together with the
//! `tag_translation_metadata` string array in its `res/values/arrays.xml`:
//! the artifact is Mapaler's `tag-translations-zh-rCN` (plus its `.sha1`,
//! which is 20 raw digest bytes, not hex text).
//! raw.githubusercontent.com is the canonical source; jsDelivr mirrors are
//! tried as fallbacks because raw is unreachable from several networks.
//! Files live under
//! `{app_data}/tag_db/`; a `updated_at` marker records the last successful
//! check so re-downloads happen at most once a week. Every failure path is
//! non-fatal — the feature simply stays unavailable, like the original.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::db::TagDatabase;
use super::sha1::sha1_hex;

/// Metadata of `tag_translation_metadata` in the original's arrays.xml.
pub const SHA1_NAME: &str = "tag-translations-zh-rCN.sha1";
pub const DATA_NAME: &str = "tag-translations-zh-rCN";

/// Download mirrors tried in order; the first success wins.
pub const SHA1_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/Mapaler/EhViewer/tag-translations/tag-translations/tag-translations-zh-rCN.sha1",
    "https://cdn.jsdelivr.net/gh/Mapaler/EhViewer@tag-translations/tag-translations/tag-translations-zh-rCN.sha1",
    "https://fastly.jsdelivr.net/gh/Mapaler/EhViewer@tag-translations/tag-translations/tag-translations-zh-rCN.sha1",
];
pub const DATA_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/Mapaler/EhViewer/tag-translations/tag-translations/tag-translations-zh-rCN",
    "https://cdn.jsdelivr.net/gh/Mapaler/EhViewer@tag-translations/tag-translations/tag-translations-zh-rCN",
    "https://fastly.jsdelivr.net/gh/Mapaler/EhViewer@tag-translations/tag-translations/tag-translations-zh-rCN",
];

/// Weekly re-check, per spec.
pub const RECHECK_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

const MARKER_NAME: &str = "updated_at";

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("tag db http client: {e}"))
}

/// The published `.sha1` artifact is 20 raw digest bytes (not hex text);
/// a 40-char hex line is accepted too for hand-placed copies.
fn parse_sha1(raw: &[u8]) -> Option<String> {
    if raw.len() == 20 {
        return Some(raw.iter().map(|b| format!("{b:02x}")).collect());
    }
    let text = String::from_utf8_lossy(raw).trim().to_lowercase();
    if text.len() == 40 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(text);
    }
    None
}

/// Load and verify the on-disk copy without touching the network.
/// Returns the parsed database and the sha1 it was verified against.
pub fn load_from_disk(dir: &Path) -> Option<(TagDatabase, String)> {
    let sha1 = parse_sha1(&std::fs::read(dir.join(SHA1_NAME)).ok()?)?;
    let data = std::fs::read(dir.join(DATA_NAME)).ok()?;
    if sha1_hex(&data) != sha1 {
        return None;
    }
    TagDatabase::from_packed(&data).ok().map(|db| (db, sha1))
}

/// True when a re-check is due: no usable copy, or the last successful check
/// is older than [`RECHECK_AGE`]. Falls back to the data file's mtime when
/// the marker is missing.
pub fn needs_recheck(dir: &Path) -> bool {
    if load_from_disk(dir).is_none() {
        return true;
    }
    let checked_at = marker_time(dir).or_else(|| {
        std::fs::metadata(dir.join(DATA_NAME))
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
    });
    match checked_at {
        Some(t) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            now.saturating_sub(t) > RECHECK_AGE.as_secs() as i64
        }
        None => true,
    }
}

fn marker_time(dir: &Path) -> Option<i64> {
    let raw = std::fs::read_to_string(dir.join(MARKER_NAME)).ok()?;
    raw.trim().parse().ok()
}

fn write_marker(dir: &Path) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let _ = std::fs::write(dir.join(MARKER_NAME), now.to_string());
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("tag db fetch {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("tag db fetch {url}: HTTP {}", resp.status()));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("tag db read {url}: {e}"))
}

/// Try each mirror in turn; the first success wins. When everything fails
/// the LAST error is returned (most representative of the network state).
async fn download_first(client: &reqwest::Client, urls: &[&str]) -> Result<Vec<u8>, String> {
    let mut last_err = String::from("tag db: no mirrors configured");
    for url in urls {
        match download(client, url).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Port of the original `update()`: fetch the published sha1, compare with
/// the current data, download the data only when it changed, verify, swap in
/// atomically and re-parse. With `force` the staleness check is skipped but
/// the "already up to date" short-circuit still applies.
///
/// On success the parsed database is returned together with its directory
/// (caller swaps it into shared state); errors are returned as strings and
/// leave any existing usable copy untouched.
pub async fn refresh(dir: &PathBuf, force: bool) -> Result<(TagDatabase, String), String> {
    if !force && !needs_recheck(dir) {
        return load_from_disk(dir).ok_or_else(|| "tag db: fresh copy failed to load".to_string());
    }

    let client = http_client()?;
    let sha1 = parse_sha1(&download_first(&client, SHA1_URLS).await?)
        .ok_or_else(|| "tag db: invalid sha1 file".to_string())?;

    // Up to date already? Just bump the marker.
    if let Some((db, current)) = load_from_disk(dir) {
        if current == sha1 {
            write_marker(dir);
            return Ok((db, sha1));
        }
    }

    let data = download_first(&client, DATA_URLS).await?;
    if sha1_hex(&data) != sha1 {
        return Err("tag db: downloaded data failed sha1 check".to_string());
    }
    let db = TagDatabase::from_packed(&data)?;

    std::fs::create_dir_all(dir).map_err(|e| format!("tag db dir: {e}"))?;
    let tmp_data = dir.join(format!("{DATA_NAME}.tmp"));
    let tmp_sha1 = dir.join(format!("{SHA1_NAME}.tmp"));
    std::fs::write(&tmp_data, &data).map_err(|e| format!("tag db write: {e}"))?;
    std::fs::write(&tmp_sha1, format!("{sha1}\n")).map_err(|e| format!("tag db write: {e}"))?;
    std::fs::rename(&tmp_data, dir.join(DATA_NAME)).map_err(|e| format!("tag db swap: {e}"))?;
    std::fs::rename(&tmp_sha1, dir.join(SHA1_NAME)).map_err(|e| format!("tag db swap: {e}"))?;
    write_marker(dir);

    Ok((db, sha1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "exviewer-tags-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pack(lines: &[(&str, &str)]) -> Vec<u8> {
        use base64::Engine as _;
        let mut text = String::new();
        for (en, zh) in lines {
            text.push_str(&format!(
                "{en}\r{}\n",
                base64::engine::general_purpose::STANDARD.encode(zh.as_bytes())
            ));
        }
        let mut out = (text.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(text.as_bytes());
        out
    }

    #[test]
    fn parse_sha1_accepts_raw_bytes_and_hex_text() {
        // The published artifact is 20 raw digest bytes.
        let raw = [
            0x82, 0xee, 0xf5, 0x70, 0xde, 0xca, 0x04, 0xaa, 0x63, 0xf4, 0x05, 0xae, 0x8b, 0x82,
            0xf3, 0xd3, 0xb8, 0x13, 0xfd, 0x64,
        ];
        assert_eq!(
            parse_sha1(&raw).unwrap(),
            "82eef570deca04aa63f405ae8b82f3d3b813fd64"
        );
        // A 40-char hex line (hand-placed copy) works too, whitespace trimmed.
        assert_eq!(
            parse_sha1(b"82EEF570DECA04AA63F405AE8B82F3D3B813FD64\n").unwrap(),
            "82eef570deca04aa63f405ae8b82f3d3b813fd64"
        );
        assert!(parse_sha1(b"not-a-sha").is_none());
        assert!(parse_sha1(&[0u8; 19]).is_none());
    }

    #[test]
    fn load_from_disk_verifies_sha1() {
        let dir = tempdir();
        let data = pack(&[("f:yuri", "百合")]);
        let sha1 = sha1_hex(&data);
        std::fs::write(dir.join(DATA_NAME), &data).unwrap();
        // Missing sha1 file → unusable.
        assert!(load_from_disk(&dir).is_none());
        std::fs::write(dir.join(SHA1_NAME), format!("{sha1}\n")).unwrap();
        let (db, version) = load_from_disk(&dir).unwrap();
        assert_eq!(version, sha1);
        // Raw 20-byte digest (the published artifact format) also verifies.
        let raw: Vec<u8> = (0..20)
            .map(|i| u8::from_str_radix(&sha1[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        std::fs::write(dir.join(SHA1_NAME), &raw).unwrap();
        assert_eq!(load_from_disk(&dir).unwrap().1, sha1);
        // Trailing newline in the packed text adds one empty entry, as in
        // the original parser.
        assert_eq!(db.entry_count(), 2);
        // Corrupt data → sha1 mismatch → unusable.
        std::fs::write(dir.join(DATA_NAME), b"junk").unwrap();
        assert!(load_from_disk(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn needs_recheck_is_true_without_usable_copy() {
        let dir = tempdir();
        assert!(needs_recheck(&dir));
        let data = pack(&[("f:yuri", "百合")]);
        std::fs::write(dir.join(DATA_NAME), &data).unwrap();
        std::fs::write(dir.join(SHA1_NAME), sha1_hex(&data)).unwrap();
        // Fresh copy without marker → falls back to mtime → not stale.
        assert!(!needs_recheck(&dir));
        // Ancient marker → stale.
        let ancient = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - RECHECK_AGE.as_secs() as i64
            - 60;
        std::fs::write(dir.join(MARKER_NAME), ancient.to_string()).unwrap();
        assert!(needs_recheck(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }
}
