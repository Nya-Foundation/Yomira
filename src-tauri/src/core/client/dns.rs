//! DNS resolution for the EH client.
//!
//! The original app resolves through `client/EhHosts.java`:
//! user hosts table → built-in fallback IPs → DoH → system DNS.
//! This module mirrors that chain as composable [`DnsResolver`] impls and
//! exposes [`EhResolver`] which adapts any [`DnsResolver`] to reqwest's
//! public `reqwest::dns::Resolve` trait (wired via `ClientBuilder::dns_resolver`).

use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::error::EhError;

/// A pluggable DNS resolver. Implementations must be cheap to call from many
/// tasks (`Send + Sync`, futures boxed).
pub trait DnsResolver: Send + Sync {
    /// Resolve `host` to IP addresses. Empty results mean "not found here" and
    /// should be treated as an error by the caller of the chain.
    fn resolve(
        &self,
        host: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, EhError>> + Send + '_>>;
}

/// Default resolver: plain system lookup via tokio (`InetAddress.getAllByName`
/// equivalent).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemResolver;

impl DnsResolver for SystemResolver {
    fn resolve(
        &self,
        host: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, EhError>> + Send + '_>> {
        let host = host.to_string();
        Box::pin(async move {
            let addrs = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|e| EhError::Eh(format!("dns lookup failed for {host}: {e}")))?;
            Ok(addrs.map(|a| a.ip()).collect())
        })
    }
}

/// The built-in fallback host table, ported from `client/EhHosts.java`
/// (`builtInHosts`). Used as the initial content of the user-editable hosts
/// table; kept here so the settings UI can reset to these values.
pub const BUILT_IN_HOSTS_TEXT: &str = include_str!("../../../resources/hosts.txt");

pub fn built_in_hosts() -> HashMap<String, Vec<IpAddr>> {
    let mut map: HashMap<String, Vec<IpAddr>> = HashMap::new();
    for line in BUILT_IN_HOSTS_TEXT
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let mut parts = line.split_whitespace();
        if let (Some(ip), Some(host)) = (parts.next(), parts.next()) {
            map.entry(host.to_string())
                .or_default()
                .push(ip.parse().expect("valid bundled host IP"));
        }
    }
    map
}

/// xorshift seeded from the clock — the original shuffles the list with a
/// time-seeded Random to spread load across fallback IPs.
fn shuffle<T>(v: &mut [T]) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut state = nanos | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in (1..v.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// Hosts-table resolver (user hosts + seeded built-ins), with a fallback
/// resolver when the host is not in the table.
pub struct HostsResolver {
    table: Arc<RwLock<HashMap<String, Vec<IpAddr>>>>,
    fallback: Arc<dyn DnsResolver>,
}

impl HostsResolver {
    pub fn new(
        table: Arc<RwLock<HashMap<String, Vec<IpAddr>>>>,
        fallback: Arc<dyn DnsResolver>,
    ) -> Self {
        Self { table, fallback }
    }

    pub fn table(&self) -> Arc<RwLock<HashMap<String, Vec<IpAddr>>>> {
        self.table.clone()
    }
}

impl DnsResolver for HostsResolver {
    fn resolve(
        &self,
        host: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, EhError>> + Send + '_>> {
        let key = host.to_ascii_lowercase();
        let found = self
            .table
            .read()
            .ok()
            .and_then(|t| t.get(&key).cloned())
            .filter(|v| !v.is_empty());
        match found {
            Some(mut ips) => {
                shuffle(&mut ips);
                Box::pin(async move { Ok(ips) })
            }
            None => self.fallback.resolve(host),
        }
    }
}

/// Default DoH endpoint (Cloudflare DNS JSON API). The original uses
/// `https://77.88.8.1/dns-query`; an IP-literal URL breaks rustls SNI, so the
/// desktop port uses the canonical hostname (system DNS is used for this
/// bootstrap client, so there is no recursion).
pub const DEFAULT_DOH_ENDPOINT: &str = "https://cloudflare-dns.com/dns-query";

/// DNS-over-HTTPS resolver (JSON API, port of `Settings.getDoH()` behavior).
/// Falls back to the inner resolver when DoH is disabled or fails.
pub struct DohResolver {
    endpoint: String,
    enabled: AtomicBool,
    inner: Arc<dyn DnsResolver>,
    client: reqwest::Client,
}

impl std::fmt::Debug for DohResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DohResolver")
            .field("endpoint", &self.endpoint)
            .field("enabled", &self.enabled.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl DohResolver {
    pub fn new(endpoint: impl Into<String>, enabled: bool, inner: Arc<dyn DnsResolver>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::core::client::DEFAULT_USER_AGENT)
            .build()
            .expect("bootstrap DoH client must build");
        Self {
            endpoint: endpoint.into(),
            enabled: AtomicBool::new(enabled),
            inner,
            client,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    async fn query_type(&self, host: &str, record_type: &str) -> Vec<IpAddr> {
        let url = format!("{}?name={}&type={}", self.endpoint, host, record_type);
        let Ok(resp) = self
            .client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/dns-json")
            .send()
            .await
        else {
            return Vec::new();
        };
        let Ok(value) = resp.json::<serde_json::Value>().await else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(answers) = value.get("Answer").and_then(|a| a.as_array()) {
            for ans in answers {
                if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                    if let Ok(ip) = data.trim_end_matches('.').parse::<IpAddr>() {
                        out.push(ip);
                    }
                }
            }
        }
        out
    }
}

impl DnsResolver for DohResolver {
    fn resolve(
        &self,
        host: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, EhError>> + Send + '_>> {
        if !self.is_enabled() {
            return self.inner.resolve(host);
        }
        let host_owned = host.to_string();
        let this = self;
        Box::pin(async move {
            let mut ips = this.query_type(&host_owned, "A").await;
            ips.extend(this.query_type(&host_owned, "AAAA").await);
            if ips.is_empty() {
                return this.inner.resolve(&host_owned).await;
            }
            shuffle(&mut ips);
            Ok(ips)
        })
    }
}

/// Adapter from [`DnsResolver`] to reqwest's public `dns::Resolve` trait.
/// Resolution results carry port 0; reqwest/hyper substitutes the connection
/// port for the URL scheme, so this is safe.
pub struct EhResolver {
    inner: Arc<dyn DnsResolver>,
}

impl EhResolver {
    pub fn new(inner: Arc<dyn DnsResolver>) -> Self {
        Self { inner }
    }
}

impl reqwest::dns::Resolve for EhResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let inner = self.inner.clone();
        let host = name.as_str().to_string();
        Box::pin(async move {
            let ips = inner.resolve(&host).await.map_err(
                |e| -> Box<dyn std::error::Error + Send + Sync> {
                    std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()).into()
                },
            )?;
            let addrs: Vec<SocketAddr> = ips.into_iter().map(|ip| SocketAddr::new(ip, 0)).collect();
            if addrs.is_empty() {
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, host).into());
            }
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_table_matches_original_hosts() {
        let table = built_in_hosts();
        assert!(table["e-hentai.org"].contains(&"104.20.18.168".parse().unwrap()));
        assert!(table["ehgt.org"].contains(&"109.236.85.28".parse().unwrap()));
        assert!(table["exhentai.org"].contains(&"178.175.128.254".parse().unwrap()));
        assert_eq!(table["forums.e-hentai.org"].len(), 2);
    }

    #[test]
    fn hosts_resolver_short_circuits_and_falls_back() {
        let table = Arc::new(RwLock::new(HashMap::new()));
        let fallback = Arc::new(SystemResolver);
        let resolver = HostsResolver::new(table.clone(), fallback);
        // localhost is not in the table → falls back to system DNS, which
        // always resolves it.
        let ips = futures_block_on(resolver.resolve("localhost")).unwrap();
        assert!(!ips.is_empty());

        // Insert a table entry → returned instead of system DNS.
        table
            .write()
            .unwrap()
            .insert("localhost".to_string(), vec!["10.1.2.3".parse().unwrap()]);
        let ips = futures_block_on(resolver.resolve("localhost")).unwrap();
        assert_eq!(ips, vec!["10.1.2.3".parse::<IpAddr>().unwrap()]);
    }

    fn futures_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }
}
