//! Application state shared by every Tauri command.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::oneshot;

use crate::core::error::EhError;
use crate::core::{built_in_hosts, ClientConfig, EhClient, InMemoryCookieStore, Site};
use crate::db::Db;
use crate::download::DownloadManager;
use crate::image_cache::ImageCache;
use crate::tags::TagDb;

/// Result of a webview sign-in attempt, delivered to the awaiting
/// `sign_in_with_webview` command.
pub type SignInOutcome = Result<(), EhError>;

/// Everything the commands need, built once in `setup()`.
pub struct AppState {
    pub client: EhClient,
    /// Concrete cookie store behind the client's `Arc<dyn EhCookieStore>`;
    /// kept separately so commands can dump/persist/restore it.
    pub cookie_store: Arc<InMemoryCookieStore>,
    /// Hosts override table shared with the client's DNS resolver.
    pub hosts: Arc<RwLock<HashMap<String, Vec<IpAddr>>>>,
    pub db: Arc<Db>,
    pub image_cache: ImageCache,
    /// Port of the loopback image server ([`crate::http_images`]) serving
    /// `image_cache.dir()`; embedded into every image URL we hand out.
    pub image_server_port: u16,
    pub local_images: crate::http_images::LocalImages,
    /// `{app_data}/cookies.json`.
    pub cookies_path: PathBuf,
    /// Tag-translation database handle (`{app_data}/tag_db/` on disk).
    pub tag_db: Arc<TagDb>,
    /// Downloads engine: queue, worker pool and per-gallery storage
    /// (initialized with a startup resume scan, workers spawned in `setup`).
    pub downloads: DownloadManager,
    /// One-shot channel slot for the in-flight webview sign-in, if any.
    pub signin_tx: Mutex<Option<oneshot::Sender<SignInOutcome>>>,
    /// Last live session validation `(valid, when)`; `check_sign_in` reuses it
    /// for [`SESSION_CHECK_TTL`] instead of hitting `favorites.php` on every
    /// UI poll. Invalidated by sign-in/sign-out.
    pub session_check: Mutex<Option<(bool, std::time::Instant)>>,
}

/// How long a live session validation stays authoritative.
pub const SESSION_CHECK_TTL: std::time::Duration = std::time::Duration::from_secs(600);

/// Network-facing settings, persisted in `settings_kv` and applied to the
/// live client via `update_config`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    /// "e" | "ex".
    pub site: String,
    /// "none" | "http" | "socks5".
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub doh_enabled: bool,
    /// Full hosts table (host → IPs) replacing the built-in one.
    pub hosts: HashMap<String, Vec<String>>,
    pub image_cache_size_mb: u64,
    /// 0 = original, 1 = 250px, 2 = 300px.
    pub thumb_resolution: i32,
    pub image_resolution: String,
    pub fix_thumb_url: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            site: "e".to_string(),
            proxy_type: "none".to_string(),
            proxy_host: String::new(),
            proxy_port: 0,
            doh_enabled: false,
            hosts: built_in_hosts()
                .into_iter()
                .map(|(k, v)| (k, v.iter().map(|ip| ip.to_string()).collect()))
                .collect(),
            image_cache_size_mb: crate::image_cache::DEFAULT_MAX_BYTES / 1024 / 1024,
            thumb_resolution: 0,
            image_resolution: "a".into(),
            fix_thumb_url: true,
        }
    }
}

impl NetworkConfig {
    /// Load from `settings_kv`, falling back to defaults per key.
    pub fn load(db: &Db) -> Result<Self, EhError> {
        let mut cfg = Self::default();
        if let Some(site) = db.get_setting("site")? {
            cfg.site = site;
        }
        if let Some(proxy_type) = db.get_setting("proxy_type")? {
            cfg.proxy_type = proxy_type;
        }
        if let Some(host) = db.get_setting("proxy_host")? {
            cfg.proxy_host = host;
        }
        if let Some(port) = db.get_setting("proxy_port")? {
            cfg.proxy_port = port.parse().unwrap_or(0);
        }
        if let Some(doh) = db.get_setting("doh_enabled")? {
            cfg.doh_enabled = doh == "1";
        }
        if let Some(hosts) = db.get_setting("hosts")? {
            if let Ok(map) = serde_json::from_str(&hosts) {
                cfg.hosts = map;
            }
        }
        if let Some(mb) = db.get_setting("image_cache_size_mb")? {
            cfg.image_cache_size_mb = mb.parse().unwrap_or(cfg.image_cache_size_mb);
        }
        if let Some(res) = db.get_setting("thumb_resolution")? {
            cfg.thumb_resolution = res.parse().unwrap_or(0);
        }
        if let Some(res) = db.get_setting("image_resolution")? {
            cfg.image_resolution = res;
        }
        if let Some(fix) = db.get_setting("fix_thumb_url")? {
            cfg.fix_thumb_url = fix == "1";
        }
        Ok(cfg)
    }

    /// Persist every field to `settings_kv`.
    pub fn save(&self, db: &Db) -> Result<(), EhError> {
        db.set_setting("site", &self.site)?;
        db.set_setting("proxy_type", &self.proxy_type)?;
        db.set_setting("proxy_host", &self.proxy_host)?;
        db.set_setting("proxy_port", &self.proxy_port.to_string())?;
        db.set_setting("doh_enabled", if self.doh_enabled { "1" } else { "0" })?;
        db.set_setting(
            "hosts",
            &serde_json::to_string(&self.hosts).unwrap_or_default(),
        )?;
        db.set_setting("image_cache_size_mb", &self.image_cache_size_mb.to_string())?;
        db.set_setting("image_resolution", &self.image_resolution)?;
        db.set_setting("thumb_resolution", &self.thumb_resolution.to_string())?;
        db.set_setting("fix_thumb_url", if self.fix_thumb_url { "1" } else { "0" })?;
        Ok(())
    }

    /// Build the core client config this network config describes.
    pub fn to_client_config(&self) -> Result<ClientConfig, EhError> {
        let site = match self.site.as_str() {
            "ex" => Site::Ex,
            _ => Site::E,
        };
        let proxy = match self.proxy_type.as_str() {
            "http" if !self.proxy_host.is_empty() => {
                Some(format!("http://{}:{}", self.proxy_host, self.proxy_port))
            }
            "socks5" if !self.proxy_host.is_empty() => {
                Some(format!("socks5://{}:{}", self.proxy_host, self.proxy_port))
            }
            _ => None,
        };
        Ok(ClientConfig {
            site,
            proxy,
            doh_enabled: self.doh_enabled,
            thumb_resolution: self.thumb_resolution,
            image_resolution: self.image_resolution.clone(),
            fix_thumb_url: self.fix_thumb_url,
            ..Default::default()
        })
    }

    /// Parse the hosts table into the resolver's `IpAddr` map (invalid IPs
    /// silently dropped, matching `built_in_hosts`).
    pub fn parsed_hosts(&self) -> HashMap<String, Vec<IpAddr>> {
        self.hosts
            .iter()
            .map(|(host, ips)| {
                (
                    host.clone(),
                    ips.iter().filter_map(|ip| ip.parse().ok()).collect(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_config_roundtrips_through_db() {
        let db = Db::open_in_memory().unwrap();
        let mut cfg = NetworkConfig {
            site: "ex".into(),
            proxy_type: "socks5".into(),
            proxy_host: "127.0.0.1".into(),
            proxy_port: 7890,
            doh_enabled: true,
            image_cache_size_mb: 42,
            ..Default::default()
        };
        cfg.hosts
            .insert("example.org".into(), vec!["1.2.3.4".into()]);
        cfg.save(&db).unwrap();

        let loaded = NetworkConfig::load(&db).unwrap();
        assert_eq!(loaded.site, "ex");
        assert_eq!(loaded.proxy_type, "socks5");
        assert!(loaded.doh_enabled);
        assert_eq!(loaded.image_cache_size_mb, 42);
        assert_eq!(loaded.hosts["example.org"], vec!["1.2.3.4".to_string()]);

        let client_config = loaded.to_client_config().unwrap();
        assert_eq!(
            client_config.proxy.as_deref(),
            Some("socks5://127.0.0.1:7890")
        );
        assert!(client_config.doh_enabled);
        assert_eq!(
            loaded.parsed_hosts()["example.org"],
            vec!["1.2.3.4".parse::<IpAddr>().unwrap()]
        );
    }

    #[test]
    fn defaults_have_no_proxy_and_built_in_hosts() {
        let cfg = NetworkConfig::default();
        let client_config = cfg.to_client_config().unwrap();
        assert!(client_config.proxy.is_none());
        assert!(!cfg.parsed_hosts().is_empty());
    }
}
