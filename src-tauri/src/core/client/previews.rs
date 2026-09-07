//! Preview page caching, token harvesting and page-token resolution.
use super::{url as eh_url, EhClient};
use crate::core::error::EhError;
use crate::core::models::PreviewSet;
use crate::core::parser;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Preview-fetch dedup: `(gid, gtoken, page)` → shared in-flight result.
type PreviewFetchCell = Arc<tokio::sync::OnceCell<Arc<Result<PreviewSet, EhError>>>>;

/// Preview state kept for ptoken resolution, keyed by gallery.
#[derive(Default)]
pub(super) struct PreviewCache {
    /// Fetched preview sets, keyed `(gid, gtoken, page)`.
    sets: HashMap<(u64, String, i32), Arc<PreviewSet>>,
    /// Every ptoken harvested from the cached sets, per gallery: one fetched
    /// preview page arms ptokens for ALL its pages, so resolving neighbours
    /// of an already-seen page costs zero network.
    ptokens: HashMap<(u64, String), BTreeMap<i32, String>>,
    /// Preview pages currently being fetched; concurrent resolutions join
    /// the same fetch instead of hitting the site twice.
    in_flight: HashMap<(u64, String, i32), PreviewFetchCell>,
}

/// Insert all non-empty ptokens of a preview set into the harvested map.
fn harvest_ptokens(cache: &mut PreviewCache, gid: u64, gtoken: &str, set: &PreviewSet) {
    for row in &set.rows {
        if !row.token.is_empty() {
            cache
                .ptokens
                .entry((gid, gtoken.to_string()))
                .or_default()
                .insert(row.position, row.token.clone());
        }
    }
}

/// Look up a harvested ptoken without touching the network.
fn cached_ptoken(cache: &PreviewCache, gid: u64, gtoken: &str, page: i32) -> Option<String> {
    cache
        .ptokens
        .get(&(gid, gtoken.to_string()))
        .and_then(|m| m.get(&page))
        .cloned()
}

impl EhClient {
    /// Fetch one preview page of a gallery detail and parse its preview set
    /// (port of `EhEngine.getPreviewSet`).
    pub async fn get_preview_set(
        &self,
        gid: u64,
        token: &str,
        page: i32,
    ) -> Result<PreviewSet, EhError> {
        let url = eh_url::gallery_detail_url(self.site(), gid, token, page, false);
        let referer = eh_url::referer(self.site());
        let body = self.get_string(&url, Some(referer)).await?;
        Ok(parser::parse_preview_set(&body, page))
    }

    /// `api.php` `gtoken` call (port of `EhEngine.getGalleryToken`).
    pub async fn get_gallery_token(
        &self,
        gid: u64,
        gtoken: &str,
        page: i32,
    ) -> Result<String, EhError> {
        let payload = json!({
            "method": "gtoken",
            "pagelist": [[gid, gtoken, page + 1]],
        });
        let body = self
            .post_api_json(&payload, Some(eh_url::referer(self.site())))
            .await?;
        let value: Value =
            serde_json::from_str(&body).map_err(|e| EhError::Parse(e.to_string()))?;
        let token = value["tokenlist"][0]["token"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                let error = value["tokenlist"][0]["error"]
                    .as_str()
                    .unwrap_or("no token");
                EhError::Eh(error.to_string())
            })?;
        Ok(token)
    }

    /// Fetch a preview set page with per-(gid, token, page) caching and
    /// cross-task dedup: concurrent resolutions of pages on the same preview
    /// page join the single in-flight fetch. Every fetched set has ALL its
    /// ptokens harvested into [`PreviewCache::ptokens`] in one pass.
    async fn get_preview_set_cached(
        &self,
        gid: u64,
        gtoken: &str,
        preview_page: i32,
    ) -> Result<Arc<PreviewSet>, EhError> {
        let key = (gid, gtoken.to_string(), preview_page);
        let cell = {
            let mut cache = self.preview_cache.lock().await;
            if let Some(set) = cache.sets.get(&key) {
                return Ok(set.clone());
            }
            Arc::clone(
                cache
                    .in_flight
                    .entry(key.clone())
                    .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new())),
            )
        };
        let shared = cell
            .get_or_init(|| self.fetch_preview_set_shared(gid, gtoken, preview_page))
            .await
            .clone();
        // Drop the in-flight entry once the fetch settled; the Arc keeps the
        // result alive for any waiter still holding it.
        {
            let mut cache = self.preview_cache.lock().await;
            if Arc::strong_count(&cell) <= 2 {
                cache.in_flight.remove(&key);
            }
        }
        Arc::try_unwrap(shared)
            .unwrap_or_else(|arc| (*arc).clone())
            .map(Arc::new)
    }

    /// Fetch one preview page and, on success, cache the set + harvest its
    /// ptokens. Runs at most once per preview page even with waiters.
    async fn fetch_preview_set_shared(
        &self,
        gid: u64,
        gtoken: &str,
        preview_page: i32,
    ) -> Arc<Result<PreviewSet, EhError>> {
        let result = self.get_preview_set(gid, gtoken, preview_page).await;
        if let Ok(set) = &result {
            let mut cache = self.preview_cache.lock().await;
            if cache.sets.len() >= 256 {
                cache.sets.clear();
                cache.ptokens.clear();
            }
            let set = Arc::new(set.clone());
            cache
                .sets
                .insert((gid, gtoken.to_string(), preview_page), set.clone());
            harvest_ptokens(&mut cache, gid, gtoken, &set);
        }
        Arc::new(result)
    }

    /// Resolve the page token (`ptoken`) for `page` of a gallery.
    ///
    /// The original app used the `gtoken` API for this, but that endpoint
    /// currently answers every request with "Invalid page." (verified live
    /// 2026-09), so the ptokens are harvested from the gallery preview pages
    /// instead, whose `.../s/{ptoken}/{gid}-{page}` links carry one per page.
    /// The API stays as a fallback for layouts where preview parsing yields
    /// nothing.
    ///
    /// Hot paths first, zero network: the harvested `page → ptoken` map, then
    /// any preview sets already cached for this gallery (harvested into the
    /// map as a side effect). Only preview pages not cached yet are fetched,
    /// and concurrent resolutions join the same fetch, so opening a run of
    /// pages in the continuous reader pays one preview fetch per 20 pages
    /// rather than one per page.
    pub async fn resolve_ptoken(
        &self,
        gid: u64,
        gtoken: &str,
        page: i32,
    ) -> Result<String, EhError> {
        const MAX_PREVIEW_PAGES: i32 = 64;

        let covered = {
            let mut cache = self.preview_cache.lock().await;
            if let Some(token) = cached_ptoken(&cache, gid, gtoken, page) {
                return Ok(token);
            }
            // Harvest every cached set for this gallery before deciding
            // anything; the ptokens map is the persistent store, the sets
            // only feed it.
            let mut sets: Vec<Arc<PreviewSet>> = cache
                .sets
                .iter()
                .filter(|(k, _)| k.0 == gid && k.1 == gtoken)
                .map(|(_, s)| s.clone())
                .collect();
            sets.sort_by_key(|s| s.pages);
            for set in &sets {
                harvest_ptokens(&mut cache, gid, gtoken, set);
            }
            if let Some(token) = cached_ptoken(&cache, gid, gtoken, page) {
                return Ok(token);
            }
            // A cached set already covers `page` but carries no ptoken for
            // it: scanning further is pointless, go straight to the API.
            sets.iter()
                .any(|s| s.rows.iter().map(|r| r.position).max().unwrap_or(0) >= page)
        };
        if covered {
            return self.get_gallery_token(gid, gtoken, page).await;
        }

        for preview_page in 0..MAX_PREVIEW_PAGES {
            // Cached pages had their ptokens harvested by whoever fetched
            // them (the zero-network pass above proves `page` is not among
            // them), so only the next missing preview page is fetched.
            let known = {
                let cache = self.preview_cache.lock().await;
                cache
                    .sets
                    .contains_key(&(gid, gtoken.to_string(), preview_page))
            };
            if known {
                continue;
            }
            let set = match self.get_preview_set_cached(gid, gtoken, preview_page).await {
                Ok(set) => set,
                // A failing first page is a real problem (gallery gone,
                // network); a failing later page just means we've run past
                // the last preview page.
                Err(e) if preview_page == 0 => return Err(e),
                Err(_) => break,
            };
            if set.rows.is_empty() {
                break;
            }
            if let Some(row) = set.rows.iter().find(|r| r.position == page) {
                if !row.token.is_empty() {
                    return Ok(row.token.clone());
                }
            }
            let max_position = set.rows.iter().map(|r| r.position).max().unwrap_or(0);
            if max_position >= page {
                // The set covers `page` but carries no ptoken for it: give
                // the API fallback a shot rather than scanning further.
                break;
            }
        }
        self.get_gallery_token(gid, gtoken, page).await
    }
}

#[cfg(test)]
#[path = "previews_tests.rs"]
mod tests;
