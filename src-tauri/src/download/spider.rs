//! Per-gallery page pipeline — the download-mode half of `SpiderQueen.java`,
//! restructured from a lock-based state machine into one async function per
//! gallery (`run_gallery`).
//!
//! Per page `n` (0-based): resolve the page token through the `gtoken` API,
//! fetch the `s/{ptoken}/{gid}-{n}` page, parse the image URL / showkey,
//! follow a H@H JS redirect once when no `<img>` is present, then download
//! the image bytes. Pages are processed strictly in order (like
//! SpiderQueen); the *page resolution* of page n+1 runs concurrently with
//! the image download of page n (limited lookahead, like the original's
//! request-ahead).
//!
//! Network access is isolated behind [`PageSource`] so the whole pipeline
//! is unit-testable without a network (the mock lives in `manager::tests`).

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::client::url as eh_url;
use crate::core::error::EhError;
use crate::core::parser;
use crate::core::EhClient;

use super::den::{GalleryDen, SpiderInfo};

/// One resolved page: the image URL plus everything needed to fetch it.
/// Port of the REQUEST stage of SpiderQueen's per-page state machine.
#[derive(Debug, Clone)]
pub struct ResolvedPage {
    pub image_url: String,
    /// Referer for the image request (the page URL itself).
    pub referer: String,
    /// Page token used to resolve this page; recorded into `.ehviewer`.
    pub ptoken: Option<String>,
}

/// One downloaded page image, ready to be written to disk (the DOWNLOADING
/// stage's result).
pub struct FetchedImage {
    pub bytes: Vec<u8>,
    /// File extension without dot, derived from the content type.
    pub extension: String,
}

/// The network seam of the download pipeline. Implemented by
/// [`EhPageSource`] in production and by a mock in tests.
pub trait PageSource: Send + Sync {
    /// Resolve page `index` to its image URL (gtoken API → gallery page →
    /// redirect follow). Runs ahead of the image download (limited
    /// lookahead), exactly like SpiderQueen's REQUEST stage.
    fn resolve_page<'a>(
        &'a self,
        gid: u64,
        gtoken: &'a str,
        index: i32,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedPage, EhError>> + Send + 'a>>;

    /// Download the image bytes for a resolved page.
    fn fetch_image<'a>(
        &'a self,
        image_url: &'a str,
        referer: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<FetchedImage, EhError>> + Send + 'a>>;

    /// Apply network-config changes to an underlying client. No-op for
    /// sources without a client (mocks).
    fn update_config(&self, _config: &crate::core::ClientConfig) -> Result<(), EhError> {
        Ok(())
    }
}

/// Production [`PageSource`] driving the download engine's own [`EhClient`]
/// (port of the SpiderQueen → EhEngine call chain).
///
/// The engine uses a second client instance sharing the app's cookie store
/// and hosts table (the reader's client lives in `AppState` and cannot be
/// aliased); `apply_client_config` keeps proxy/site changes in sync — the
/// `set_network_config` command should call `DownloadManager::apply_client_config`.
pub struct EhPageSource {
    client: EhClient,
}

impl EhPageSource {
    pub fn new(client: EhClient) -> Self {
        Self { client }
    }
}

impl PageSource for EhPageSource {
    fn update_config(&self, config: &crate::core::ClientConfig) -> Result<(), EhError> {
        self.client.update_config(config.clone())
    }

    fn resolve_page<'a>(
        &'a self,
        gid: u64,
        gtoken: &'a str,
        index: i32,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedPage, EhError>> + Send + 'a>> {
        Box::pin(async move {
            // The page endpoint needs the page token (ptoken). The original
            // resolved it through the gtoken API, which now answers "Invalid
            // page." to everything, so ptokens are harvested from the gallery
            // preview pages instead (API kept as fallback inside
            // resolve_ptoken).
            let site = self.client.site();
            let ptoken = self.client.resolve_ptoken(gid, gtoken, index).await?;
            let mut data = self.client.get_page_data(gid, &ptoken, index).await?;

            // H@H redirect pages carry no <img>; follow the JS redirect once
            // and re-parse (the original does this through nl(skipHathKey)).
            if data.image_url.is_none() {
                if let Some(redirect) = data.redirect_url.clone() {
                    let referer = eh_url::gallery_detail_url(site, gid, gtoken, 0, false);
                    let body = self.client.get_string(&redirect, Some(&referer)).await?;
                    data = parser::parse_gallery_page(&body, &ptoken)?;
                }
            }

            let image_url = data
                .image_url
                .ok_or_else(|| EhError::Parse("gallery page has no image URL".into()))?;
            Ok(ResolvedPage {
                image_url,
                referer: eh_url::page_url(site, gid, index, &ptoken),
                ptoken: Some(ptoken),
            })
        })
    }

    fn fetch_image<'a>(
        &'a self,
        image_url: &'a str,
        referer: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<FetchedImage, EhError>> + Send + 'a>> {
        Box::pin(async move {
            let (bytes, content_type) = self
                .client
                .get_image_bytes(image_url, Some(referer))
                .await?;
            Ok(FetchedImage {
                bytes,
                extension: extension_from_content_type(&content_type),
            })
        })
    }
}

/// Content type → file extension without dot (jpeg → jpg, like the original
/// `MimeTypeMap` lookups; unknown types default to jpg).
pub fn extension_from_content_type(content_type: &str) -> String {
    let mime = content_type.split(';').next().unwrap_or_default().trim();
    match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        _ => "jpg", // image/jpeg and everything else
    }
    .to_string()
}

/// Download pages `start..pages` of gallery `gid` into `den`.
///
/// * Pages are written strictly in order; page n+1's image download starts
///   only after page n was fully written (SpiderQueen's in-order guarantee).
/// * The *resolution* of page n+1 (token + gallery page fetch) overlaps with
///   the image download of page n — the "limited lookahead", mirroring
///   SpiderQueen running its REQUEST stage ahead of DOWNLOADING.
/// * Progress (`on_progress`) fires with the number of finished pages after
///   each successful page; the `.ehviewer` file is rewritten each time so an
///   interrupted gallery resumes from durable state.
/// * `cancel` is checked between pages; when set, the run aborts with
///   [`EhError::Cancelled`] (the manager maps this to `Paused`).
///
/// The argument list is the per-gallery SpiderQueen input set; grouping it
/// into a struct would only shuffle the same state around.
#[allow(clippy::too_many_arguments)]
pub async fn run_gallery(
    source: Arc<dyn PageSource>,
    den: &GalleryDen,
    gid: u64,
    gtoken: &str,
    pages: i32,
    start: i32,
    spider_info: &mut SpiderInfo,
    cancel: Arc<AtomicBool>,
    on_progress: &(dyn Fn(i32) + Send + Sync),
) -> Result<(), EhError> {
    let resolve = |index: i32| {
        let source = source.clone();
        let gtoken = gtoken.to_string();
        tokio::spawn(async move { source.resolve_page(gid, &gtoken, index).await })
    };

    let saved = den.available_pages(pages);
    let missing: Vec<i32> = (start..pages)
        .filter(|index| saved.binary_search(index).is_err())
        .collect();
    let Some(&first) = missing.first() else {
        return Ok(());
    };
    if cancel.load(Ordering::Relaxed) {
        return Err(EhError::Cancelled);
    }
    // Resolve only absent pages, preserving the one-page lookahead.
    let mut resolved = source.resolve_page(gid, gtoken, first).await?;

    for (offset, &index) in missing.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(EhError::Cancelled);
        }
        let lookahead = missing.get(offset + 1).map(|&next| resolve(next));

        let image = source
            .fetch_image(&resolved.image_url, &resolved.referer)
            .await?;
        den.write_page(index, &image.extension, &image.bytes)?;
        if let Some(ptoken) = resolved.ptoken {
            spider_info.ptoken_map.insert(index, ptoken);
        }
        spider_info.start_page = missing.get(offset + 1).copied().unwrap_or(pages);
        spider_info.pages = pages;
        // Best effort, like the original: pages already on disk are the real
        // resume source; a metadata write failure must not fail the gallery.
        let _ = spider_info.write_to_dir(den.path());
        on_progress((saved.len() + offset + 1) as i32);

        resolved = match lookahead {
            Some(handle) => handle
                .await
                .map_err(|e| EhError::Db(format!("page resolution task failed: {e}")))??,
            None => return Ok(()),
        };
    }
    Ok(())
}

#[cfg(test)]
#[path = "spider_tests.rs"]
mod tests;
