//! Cookie storage abstraction.
//!
//! The original app persists cookies in SQLite (`client/EhCookieStore.java`).
//! This port keeps the live store in memory (a thin, shadow-indexed wrapper
//! around reqwest's `cookie::Jar`) and persists a JSON dump of every cookie
//! seen to `{app_data}/cookies.json` at shutdown and on a timer (see
//! `crate::cookies`). The shadow index is what makes `dump`/`restore`
//! possible: reqwest's `Jar` is write-only from the outside, so every raw
//! `Set-Cookie` value is recorded together with its domain as it passes
//! through [`InMemoryCookieStore::set_cookies`].

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use reqwest::cookie::CookieStore as ReqwestCookieStore;
use reqwest::header::HeaderValue;
use reqwest::Url;

use crate::core::error::EhError;

/// Pluggable cookie store. A blanket impl makes any `Arc<T>` where
/// `T: EhCookieStore` usable as a reqwest cookie provider, so `EhClient` can
/// hand `Arc<dyn EhCookieStore>` straight to `ClientBuilder::cookie_provider`.
pub trait EhCookieStore: ReqwestCookieStore {
    /// Remove all cookies — port of `EhCookieStore.signOut()`.
    fn clear(&self) -> Result<(), EhError>;

    /// Serialize all cookies so a persistent store can save them.
    /// The returned map is keyed `"{domain}|{name}"`; the value is the raw
    /// `Set-Cookie` string (attributes included) so [`EhCookieStore::restore`]
    /// can replay it losslessly.
    fn dump(&self) -> Result<HashMap<String, String>, EhError> {
        Ok(HashMap::new())
    }

    /// Restore cookies previously produced by `dump`.
    fn restore(&self, _cookies: HashMap<String, String>) -> Result<(), EhError> {
        Ok(())
    }

    /// Whether the session carries the member cookie (`ipb_member_id`).
    /// The original checks this to know if the user is signed in.
    fn has_signed_in(&self) -> bool;

    /// Value of cookie `name` for `url` (parsed out of the `Cookie` header
    /// the jar would send), e.g. the `ipb_member_id`/`ipb_pass_hash` pair
    /// used as `apiuid`/`apikey` by the rating API.
    fn get_cookie_value(&self, url: &Url, name: &str) -> Option<String> {
        let header = <Self as ReqwestCookieStore>::cookies(self, url)?;
        let value = header.to_str().ok()?;
        parse_cookie_pairs(value)
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }
}

/// Newtype so `Arc<dyn EhCookieStore>` can be handed to reqwest's
/// `cookie_provider` (orphan rules forbid `impl ReqwestCookieStore for
/// Arc<T>` directly).
#[derive(Clone)]
pub struct CookieStoreHandle(pub Arc<dyn EhCookieStore>, pub String);

impl ReqwestCookieStore for CookieStoreHandle {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        self.0.set_cookies(cookie_headers, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let original = self.0.cookies(url);
        // EH's xr preference affects newly fetched reader/download pages.
        // Preserve authentication and every other uconfig preference.
        if !matches!(url.host_str(), Some("e-hentai.org" | "exhentai.org")) {
            return original;
        }
        let mut pairs = original
            .as_ref()
            .and_then(|h| h.to_str().ok())
            .map(parse_cookie_pairs)
            .unwrap_or_default();
        let mut config = pairs
            .iter()
            .find(|(name, _)| name == "uconfig")
            .map(|(_, value)| {
                value
                    .split('-')
                    .filter(|part| !part.starts_with("xr_") && !part.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        config.push(format!("xr_{}", self.1));
        pairs.retain(|(name, _)| name != "uconfig");
        pairs.push(("uconfig".into(), config.join("-")));
        HeaderValue::from_str(
            &pairs
                .into_iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
        .ok()
    }
}

impl EhCookieStore for CookieStoreHandle {
    fn clear(&self) -> Result<(), EhError> {
        self.0.clear()
    }

    fn dump(&self) -> Result<HashMap<String, String>, EhError> {
        self.0.dump()
    }

    fn restore(&self, cookies: HashMap<String, String>) -> Result<(), EhError> {
        self.0.restore(cookies)
    }

    fn has_signed_in(&self) -> bool {
        self.0.has_signed_in()
    }

    fn get_cookie_value(&self, url: &Url, name: &str) -> Option<String> {
        self.0.get_cookie_value(url, name)
    }
}

/// In-memory cookie store with a JSON-persistable shadow index.
#[derive(Debug, Default)]
pub struct InMemoryCookieStore {
    jar: RwLock<reqwest::cookie::Jar>,
    /// `"{domain}|{name}"` → raw `Set-Cookie` value as last seen.
    shadow: RwLock<HashMap<String, String>>,
}

impl InMemoryCookieStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn jar(&self) -> std::sync::RwLockReadGuard<'_, reqwest::cookie::Jar> {
        // Unwrap-free accessors: poisoning only happens on a panicking writer,
        // in which case recovering the data is the best we can do.
        self.jar.read().unwrap_or_else(|p| p.into_inner())
    }

    /// Add a raw `Set-Cookie` style value for `url` (used to import an
    /// existing session, e.g. from the settings UI).
    pub fn add_cookie_value(&self, value: &str, url: &Url) -> Result<(), EhError> {
        let header = HeaderValue::from_str(value)
            .map_err(|e| EhError::Cookie(format!("invalid cookie value: {e}")))?;
        let mut iter = std::iter::once(&header);
        {
            let jar = self.jar.read().unwrap_or_else(|p| p.into_inner());
            ReqwestCookieStore::set_cookies(&*jar, &mut iter, url);
        }
        self.record_shadow(value, url);
        Ok(())
    }

    /// Record `raw` in the shadow index under the cookie's effective domain.
    fn record_shadow(&self, raw: &str, url: &Url) {
        let parsed = parse_set_cookie(raw);
        let domain = parsed
            .domain
            .unwrap_or_else(|| url.host_str().unwrap_or_default().to_string());
        let name = parsed.name;
        if name.is_empty() {
            return;
        }
        let mut shadow = self.shadow.write().unwrap_or_else(|p| p.into_inner());
        shadow.insert(format!("{domain}|{name}"), raw.to_string());
    }

    /// Import a dump produced by [`EhCookieStore::dump`] (or by
    /// `crate::cookies` loading `cookies.json`).
    pub fn import_dump(&self, cookies: HashMap<String, String>) -> Result<(), EhError> {
        for (key, raw) in cookies {
            // `cookies.json` values may be a bare `name=value` (legacy hand
            // edits); normalise them to a full Set-Cookie string first.
            let raw = if looks_like_set_cookie(&raw) {
                raw
            } else {
                let domain = key.split('|').next().unwrap_or_default();
                format!("{raw}; Domain={domain}; Path=/")
            };
            let url = Url::parse(&format!(
                "https://{}/",
                key.split('|').next().unwrap_or("e-hentai.org")
            ))
            .map_err(|e| EhError::Cookie(format!("invalid cookie key {key:?}: {e}")))?;
            self.add_cookie_value(&raw, &url)?;
        }
        Ok(())
    }
}

impl ReqwestCookieStore for InMemoryCookieStore {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let headers: Vec<HeaderValue> = cookie_headers.cloned().collect();
        {
            let jar = self.jar.read().unwrap_or_else(|p| p.into_inner());
            let mut iter = headers.iter();
            ReqwestCookieStore::set_cookies(&*jar, &mut iter, url);
        }
        for header in &headers {
            if let Ok(raw) = header.to_str() {
                self.record_shadow(raw, url);
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        ReqwestCookieStore::cookies(&*self.jar(), url)
    }
}

impl EhCookieStore for InMemoryCookieStore {
    fn clear(&self) -> Result<(), EhError> {
        *self.jar.write().unwrap_or_else(|p| p.into_inner()) = reqwest::cookie::Jar::default();
        self.shadow
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
        Ok(())
    }

    fn dump(&self) -> Result<HashMap<String, String>, EhError> {
        Ok(self
            .shadow
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone())
    }

    fn restore(&self, cookies: HashMap<String, String>) -> Result<(), EhError> {
        self.import_dump(cookies)
    }

    fn has_signed_in(&self) -> bool {
        for site in [
            crate::core::client::url::REFERER_E,
            crate::core::client::url::REFERER_EX,
            crate::core::client::url::URL_FORUMS.trim_end_matches('/'),
        ] {
            let Ok(url) = Url::parse(site) else {
                continue;
            };
            if self.get_cookie_value(&url, "ipb_member_id").is_some() {
                return true;
            }
        }
        false
    }
}

/// Minimal `Set-Cookie` decomposition: name/value plus the `Domain`
/// attribute when present. Deliberately tolerant — expiry/path are left in
/// the raw string, which is what gets replayed on restore.
struct ParsedSetCookie {
    name: String,
    domain: Option<String>,
}

fn parse_set_cookie(raw: &str) -> ParsedSetCookie {
    let mut name = String::new();
    let mut domain = None;
    for (i, part) in raw.split(';').enumerate() {
        let part = part.trim();
        if i == 0 {
            name = part
                .split('=')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
        } else if let Some(value) = part.split_once('=') {
            if value.0.trim().eq_ignore_ascii_case("domain") {
                domain = Some(value.1.trim().trim_start_matches('.').to_string());
            }
        }
    }
    ParsedSetCookie { name, domain }
}

/// True when the string carries cookie attributes beyond `name=value`.
fn looks_like_set_cookie(raw: &str) -> bool {
    raw.split(';').count() > 1
}

/// Parse a merged `Cookie` header (`a=1; b=2`) into pairs.
pub(crate) fn parse_cookie_pairs(header: &str) -> Vec<(String, String)> {
    header
        .split(';')
        .filter_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            Some((name.to_string(), value.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_roundtrip() {
        let store = InMemoryCookieStore::new();
        let url = Url::parse("https://e-hentai.org/").unwrap();
        let header = HeaderValue::from_static("ipb_member_id=12345; Domain=e-hentai.org; Path=/");
        let mut iter = std::iter::once(&header);
        ReqwestCookieStore::set_cookies(&store, &mut iter, &url);
        assert!(store.has_signed_in());
        let cookies = ReqwestCookieStore::cookies(&store, &url).unwrap();
        assert!(cookies.to_str().unwrap().contains("ipb_member_id=12345"));
    }

    #[test]
    fn signed_out_when_no_member_cookie() {
        let store = InMemoryCookieStore::new();
        assert!(!store.has_signed_in());
    }

    #[test]
    fn dump_restore_roundtrip_keeps_sign_in() {
        let store = InMemoryCookieStore::new();
        let url = Url::parse("https://e-hentai.org/").unwrap();
        store
            .add_cookie_value("ipb_member_id=12345; Domain=e-hentai.org; Path=/", &url)
            .unwrap();
        store
            .add_cookie_value("ipb_pass_hash=deadbeef; Domain=e-hentai.org; Path=/", &url)
            .unwrap();
        let dump = EhCookieStore::dump(&store).unwrap();
        assert!(dump.contains_key("e-hentai.org|ipb_member_id"));

        let fresh = InMemoryCookieStore::new();
        assert!(!fresh.has_signed_in());
        EhCookieStore::restore(&fresh, dump).unwrap();
        assert!(fresh.has_signed_in());
        let forums = Url::parse("https://forums.e-hentai.org/").unwrap();
        assert_eq!(
            fresh.get_cookie_value(&forums, "ipb_member_id").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn clear_drops_everything() {
        let store = InMemoryCookieStore::new();
        let url = Url::parse("https://e-hentai.org/").unwrap();
        store
            .add_cookie_value("ipb_member_id=12345; Domain=e-hentai.org; Path=/", &url)
            .unwrap();
        assert!(store.has_signed_in());
        EhCookieStore::clear(&store).unwrap();
        assert!(!store.has_signed_in());
        assert!(EhCookieStore::dump(&store).unwrap().is_empty());
    }

    #[test]
    fn bare_value_import_gets_domain_from_key() {
        let store = InMemoryCookieStore::new();
        let mut dump = HashMap::new();
        dump.insert(
            "e-hentai.org|ipb_member_id".to_string(),
            "ipb_member_id=42".to_string(),
        );
        EhCookieStore::restore(&store, dump).unwrap();
        let url = Url::parse("https://e-hentai.org/").unwrap();
        assert_eq!(
            store.get_cookie_value(&url, "ipb_member_id").as_deref(),
            Some("42")
        );
    }
}
