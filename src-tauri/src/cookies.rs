//! Cookie import / persistence.
//!
//! Port of the sign-in cookie handling of the original app
//! (`client/EhCookieStore.java`, `ui/scene/SignInScene` + settings cookie
//! paste): the user pastes `document.cookie` output (or a full `Cookie:`
//! header, or one `name=value` per line); we extract `ipb_member_id`,
//! `ipb_pass_hash` and the optional `igneous`, and install them on the
//! forums source domain plus both gallery domains — exactly like
//! `EhCookieStore.newCookie` with `forceLongLive` + `forceNotHostOnly` —
//! together with the content-warning cookie (`nw=1`,
//! `EhConfig.KEY_CONTENT_WARNING`/`CONTENT_WARNING_NOT_SHOW`).
//!
//! Persistence mirrors the original SQLite cookie DB loosely: the live
//! in-memory jar is dumped to `{app_data}/cookies.json` on shutdown
//! (`RunEvent::ExitRequested`) and on a 60-second timer. A SQLite cookie
//! store can later implement the same `EhCookieStore` trait without touching
//! this module.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use crate::core::client::url::{DOMAIN_E, DOMAIN_EX};
use crate::core::error::EhError;
use crate::core::parser;
use crate::core::{EhClient, EhCookieStore, InMemoryCookieStore, Site};

/// Cookies the paste must contain for a usable session.
pub const KEY_MEMBER_ID: &str = "ipb_member_id";
pub const KEY_PASS_HASH: &str = "ipb_pass_hash";
pub const KEY_IGNEOUS: &str = "igneous";
/// `EhConfig.KEY_CONTENT_WARNING` / `CONTENT_WARNING_NOT_SHOW`.
pub const KEY_CONTENT_WARNING: &str = "nw";
pub const CONTENT_WARNING_NOT_SHOW: &str = "1";

/// Far-future expiry used for imported session cookies (the original passes
/// `forceLongLive`, i.e. `expiresAt(Long.MAX_VALUE)`).
const LONG_LIVE_EXPIRES: &str = "Tue, 19 Jan 2038 03:14:07 GMT";

/// Parse a pasted cookie string into `name → value`.
///
/// Accepted formats (all seen in the wild and in the original's settings):
/// * `document.cookie` style: `ipb_member_id=1; ipb_pass_hash=abc`
/// * a full request header: `Cookie: ipb_member_id=1; ipb_pass_hash=abc`
///   (case-insensitive header name, optionally on its own first line)
/// * one cookie per line (`name=value` lines, `#` comments ignored)
/// * JSON objects (`{"ipb_member_id":"1", ...}`)
pub fn parse_cookie_paste(input: &str) -> Result<HashMap<String, String>, EhError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(EhError::Cookie("empty cookie string".into()));
    }

    let mut out = HashMap::new();
    let trimmed = input.trim_start_matches('\u{feff}').trim();

    // JSON object form.
    if trimmed.starts_with('{') {
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|e| EhError::Cookie(format!("invalid cookie JSON: {e}")))?;
        let Some(obj) = value.as_object() else {
            return Err(EhError::Cookie("cookie JSON must be an object".into()));
        };
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
        return Ok(out);
    }

    // Split into lines, drop `#` comments, strip a leading "Cookie:" header
    // line, then split each remaining line on ';'.
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = if line.len() >= 7 && line[..7].eq_ignore_ascii_case("cookie:") {
            line[7..].trim()
        } else {
            line
        };
        for part in line.split(';') {
            let part = part.trim();
            let Some((name, value)) = part.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            pairs.push((name.to_string(), value.trim().to_string()));
        }
    }
    for (name, value) in pairs {
        out.entry(name).or_insert(value);
    }
    if out.is_empty() {
        return Err(EhError::Cookie(
            "no name=value cookie pairs found in the pasted string".into(),
        ));
    }
    Ok(out)
}

/// Extract the session keys (`ipb_member_id`, `ipb_pass_hash`, optional
/// `igneous`) from a parsed cookie map.
pub fn session_keys(cookies: &HashMap<String, String>) -> Result<SessionKeys, EhError> {
    let member_id = cookies
        .get(KEY_MEMBER_ID)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| EhError::Cookie(format!("missing required cookie {KEY_MEMBER_ID}")))?;
    let pass_hash = cookies
        .get(KEY_PASS_HASH)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| EhError::Cookie(format!("missing required cookie {KEY_PASS_HASH}")))?;
    Ok(SessionKeys {
        member_id: member_id.clone(),
        pass_hash: pass_hash.clone(),
        igneous: cookies.get(KEY_IGNEOUS).filter(|v| !v.is_empty()).cloned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKeys {
    pub member_id: String,
    pub pass_hash: String,
    pub igneous: Option<String>,
}

/// Build the long-lived `Set-Cookie` value used to install `name=value` on
/// `domain` (`EhCookieStore.newCookie(..., forceLongLive, forceNotHostOnly)`).
fn long_live_cookie(name: &str, value: &str, domain: &str) -> String {
    format!("{name}={value}; Domain={domain}; Path=/; Expires={LONG_LIVE_EXPIRES}")
}

/// Install a parsed session on the store: member cookies land on the forums
/// source domain and on both gallery domains (the original copies every
/// cookie across `DOMAIN_E`/`DOMAIN_EX`), plus the `igneous` extension cookie
/// on exhentai only, plus the content-warning cookie on e-hentai.
pub fn apply_session(store: &InMemoryCookieStore, keys: &SessionKeys) -> Result<(), EhError> {
    for domain in [DOMAIN_E, DOMAIN_EX] {
        for (name, value) in [
            (KEY_MEMBER_ID, keys.member_id.as_str()),
            (KEY_PASS_HASH, keys.pass_hash.as_str()),
        ] {
            let raw = long_live_cookie(name, value, domain);
            let url = reqwest::Url::parse(&format!("https://{domain}/"))
                .map_err(|e| EhError::Cookie(e.to_string()))?;
            store.add_cookie_value(&raw, &url)?;
        }
        if domain == DOMAIN_EX {
            if let Some(igneous) = &keys.igneous {
                let raw = long_live_cookie(KEY_IGNEOUS, igneous, DOMAIN_EX);
                let url = reqwest::Url::parse("https://exhentai.org/")
                    .map_err(|e| EhError::Cookie(e.to_string()))?;
                store.add_cookie_value(&raw, &url)?;
            }
        }
    }
    // Forums source domain (host-only, like a browser would set it).
    let forums_url = reqwest::Url::parse("https://forums.e-hentai.org/")
        .map_err(|e| EhError::Cookie(e.to_string()))?;
    store.add_cookie_value(
        &format!(
            "{KEY_MEMBER_ID}={}; Path=/; Expires={LONG_LIVE_EXPIRES}",
            keys.member_id
        ),
        &forums_url,
    )?;
    store.add_cookie_value(
        &format!(
            "{KEY_PASS_HASH}={}; Path=/; Expires={LONG_LIVE_EXPIRES}",
            keys.pass_hash
        ),
        &forums_url,
    )?;

    // Content warning accepted (`sTipsCookie`, domain e-hentai.org).
    let e_url =
        reqwest::Url::parse("https://e-hentai.org/").map_err(|e| EhError::Cookie(e.to_string()))?;
    store.add_cookie_value(
        &long_live_cookie(KEY_CONTENT_WARNING, CONTENT_WARNING_NOT_SHOW, DOMAIN_E),
        &e_url,
    )?;
    Ok(())
}

/// Full pasted-string sign-in: parse, install, then verify against the live
/// site like the original sign-in scenes do (fetch the site homepage and run
/// the `SignInParser` heuristics).
pub async fn sign_in_with_cookies(
    client: &EhClient,
    store: &InMemoryCookieStore,
    cookies: &str,
) -> Result<(), EhError> {
    let parsed = parse_cookie_paste(cookies)?;
    let keys = session_keys(&parsed)?;
    apply_session(store, &keys)?;
    verify_sign_in(client).await
}

/// Confirm the session against the live site. The homepage HTML does NOT
/// reflect login state (verified 2026: anonymous and member homepages are
/// markup-identical), so instead fetch `favorites.php`, which bounces
/// anonymous sessions to the forums login page while members get the page
/// with the `favcat` selector (even with zero favorites).
pub async fn verify_sign_in(client: &EhClient) -> Result<(), EhError> {
    let url = match client.site() {
        Site::E => "https://e-hentai.org/favorites.php",
        Site::Ex => "https://exhentai.org/favorites.php",
    };
    let body = client.get_string(url, None).await?;
    let lower = body.to_ascii_lowercase();
    if lower.contains("favcat") || parser::detect_signed_in(&body) {
        return Ok(());
    }
    // Surface the forums error text when present (port of the
    // SignInParser error path); otherwise a plain NotSignedIn.
    match parser::parse_sign_in(&body) {
        Ok(_) => Ok(()),
        Err(EhError::Eh(msg)) => Err(EhError::Eh(format!("sign-in verification failed: {msg}"))),
        Err(EhError::Parse(_)) => Err(EhError::NotSignedIn),
        Err(e) => Err(e),
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Load a previously saved cookie dump from `path`, if any.
pub fn load_persisted(path: &Path) -> HashMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Dump the store and persist it as JSON at `path` (fire-and-forget safe:
/// errors are logged by the caller, never fatal).
pub fn persist(store: &Arc<InMemoryCookieStore>, path: &Path) -> Result<(), EhError> {
    let dump = store.dump()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(EhError::from)?;
    }
    let bytes = serde_json::to_vec_pretty(&dump).map_err(|e| EhError::Cookie(e.to_string()))?;
    // Write via a temp file + rename so a crash mid-write cannot corrupt the
    // only copy of the session.
    let tmp: PathBuf = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(EhError::from)?;
    std::fs::rename(&tmp, path).map_err(EhError::from)?;
    Ok(())
}

/// Periodic persistence loop (every 60 s). Spawn it with
/// `tauri::async_runtime::spawn` — the Tauri setup hook runs outside Tokio's
/// reactor, so `tokio::spawn` cannot be used from there.
pub async fn persist_loop(store: Arc<InMemoryCookieStore>, path: PathBuf) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
    tick.tick().await; // first tick fires immediately; skip it
    loop {
        tick.tick().await;
        if let Err(e) = persist(&store, &path) {
            eprintln!("[cookies] persistence failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_document_cookie_format() {
        let map = parse_cookie_paste(
            "ipb_member_id=12345; ipb_pass_hash=deadbeef; sk=whatever; igneous=aaa111",
        )
        .unwrap();
        assert_eq!(map[KEY_MEMBER_ID], "12345");
        assert_eq!(map[KEY_PASS_HASH], "deadbeef");
        assert_eq!(map[KEY_IGNEOUS], "aaa111");
        assert_eq!(map["sk"], "whatever");
    }

    #[test]
    fn parses_full_cookie_header() {
        let map =
            parse_cookie_paste("Cookie: ipb_member_id=12345; ipb_pass_hash=deadbeef\r\n").unwrap();
        assert_eq!(map[KEY_MEMBER_ID], "12345");
        let map = parse_cookie_paste("cookie: ipb_member_id=9; ipb_pass_hash=x\n").unwrap();
        assert_eq!(map[KEY_MEMBER_ID], "9");
    }

    #[test]
    fn parses_one_per_line_with_comments() {
        let map =
            parse_cookie_paste("# pasted from browser\nipb_member_id=777\nipb_pass_hash=zzz\n")
                .unwrap();
        assert_eq!(map[KEY_MEMBER_ID], "777");
        assert_eq!(map[KEY_PASS_HASH], "zzz");
    }

    #[test]
    fn parses_json_object() {
        let map = parse_cookie_paste(r#"{"ipb_member_id":"42","ipb_pass_hash":"h","igneous":"i"}"#)
            .unwrap();
        assert_eq!(map[KEY_MEMBER_ID], "42");
        assert_eq!(map[KEY_IGNEOUS], "i");
    }

    #[test]
    fn rejects_garbage_and_missing_keys() {
        assert!(parse_cookie_paste("").is_err());
        assert!(parse_cookie_paste("hello world").is_err());
        let no_member = parse_cookie_paste("ipb_pass_hash=abc").unwrap();
        assert!(session_keys(&no_member).is_err());
    }

    #[test]
    fn apply_session_installs_on_all_domains() {
        use reqwest::cookie::CookieStore as _;
        let store = InMemoryCookieStore::new();
        let keys = SessionKeys {
            member_id: "123".into(),
            pass_hash: "hash".into(),
            igneous: Some("ig".into()),
        };
        apply_session(&store, &keys).unwrap();
        assert!(EhCookieStore::has_signed_in(&store));

        for domain in ["https://e-hentai.org/", "https://exhentai.org/"] {
            let url = reqwest::Url::parse(domain).unwrap();
            let header = store.cookies(&url).unwrap();
            let text = header.to_str().unwrap();
            assert!(text.contains("ipb_member_id=123"), "{domain}: {text}");
            assert!(text.contains("ipb_pass_hash=hash"), "{domain}: {text}");
            // The content-warning cookie (nw=1) is e-hentai only, like
            // EhCookieStore.sTipsCookie.
            if domain == "https://e-hentai.org/" {
                assert!(text.contains("nw=1"), "{domain}: {text}");
            } else {
                assert!(!text.contains("nw=1"), "{domain}: {text}");
            }
        }
        let ex = reqwest::Url::parse("https://exhentai.org/").unwrap();
        assert_eq!(
            EhCookieStore::get_cookie_value(&store, &ex, "igneous").as_deref(),
            Some("ig")
        );
        let forums = reqwest::Url::parse("https://forums.e-hentai.org/").unwrap();
        assert_eq!(
            EhCookieStore::get_cookie_value(&store, &forums, "ipb_member_id").as_deref(),
            Some("123")
        );
    }

    #[test]
    fn persistence_roundtrip_via_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "exviewer-cookies-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cookies.json");

        let store = InMemoryCookieStore::new();
        let keys = SessionKeys {
            member_id: "7".into(),
            pass_hash: "p".into(),
            igneous: None,
        };
        apply_session(&store, &keys).unwrap();
        persist(&Arc::new(store), &path).unwrap();

        let dump = load_persisted(&path);
        assert!(dump.contains_key("e-hentai.org|ipb_member_id"));

        let restored = InMemoryCookieStore::new();
        EhCookieStore::restore(&restored, dump).unwrap();
        assert!(EhCookieStore::has_signed_in(&restored));

        std::fs::remove_dir_all(&dir).ok();
    }
}
