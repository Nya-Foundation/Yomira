//! Torrent and archive downloads, using the client's shared transport.
use super::{check_sad_panda, url as eh_url, EhClient};
use crate::core::error::EhError;
use crate::core::models::TorrentEntry;
use crate::core::parser;
use regex::Regex;
use reqwest::{header, Method};
use std::sync::LazyLock;

impl EhClient {
    // ------------------------------------------------------------------
    // Torrents + archiver (port of EhEngine.getTorrentList / getArchiver /
    // downloadArchiver)
    // ------------------------------------------------------------------

    /// GET `gallerytorrents.php?gid&t` and parse the torrent list
    /// (port of `EhEngine.getTorrentList`).
    pub async fn get_torrent_list(
        &self,
        gid: u64,
        token: &str,
    ) -> Result<Vec<TorrentEntry>, EhError> {
        let site = self.site();
        let url = format!(
            "{}gallerytorrents.php?gid={}&t={}",
            eh_url::host(site),
            gid,
            token
        );
        let referer = eh_url::gallery_detail_url(site, gid, token, 0, false);
        let body = self.get_string(&url, Some(&referer)).await?;
        Ok(parser::parse_torrent_entries(&body))
    }

    /// GET the `archiver.php` page for a gallery (port of
    /// `EhEngine.getArchiver`), with the gallery page as referer.
    pub async fn get_archiver_page(&self, gid: u64, token: &str) -> Result<String, EhError> {
        let site = self.site();
        let url = eh_url::download_archive_url(site, gid, token, "");
        let referer = eh_url::gallery_detail_url(site, gid, token, 0, false);
        self.get_string(&url, Some(&referer)).await
    }

    /// Resolve the temporary archive download URL for one resolution
    /// ("res"/"org") — the full `EhEngine.downloadArchiver` chain: parse the
    /// archiver page, POST the matching form (`dltype` + `dlcheck`), follow
    /// the `document.location` redirect, then read the
    /// "Click Here To Start Downloading" link and rebuild the absolute URL
    /// from the final response's host (the original forces https).
    pub async fn resolve_archive_download_url(
        &self,
        gid: u64,
        token: &str,
        resolution: &str,
    ) -> Result<String, EhError> {
        let page = self.get_archiver_page(gid, token).await?;
        let info = parser::parse_archiver(&page);
        let form = info
            .form_for(resolution)
            .ok_or_else(|| EhError::Eh(format!("archiver page has no {resolution:?} form")))?;
        let referer = eh_url::gallery_detail_url(self.site(), gid, token, 0, false);
        let body = self
            .post_form(
                &form.action,
                Some(&referer),
                &[
                    ("dltype", form.dltype.as_str()),
                    ("dlcheck", form.dlcheck.as_str()),
                ],
            )
            .await?;

        // POST response says where to continue (relative URLs resolve
        // against the form action, like a browser would).
        static LOCATION_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"document\.location\s*=\s*"([^"]+)""#).unwrap());
        let location = LOCATION_PATTERN
            .captures(&body)
            .map(|m| m[1].trim().to_string())
            .ok_or_else(|| EhError::Parse("archiver response has no document.location".into()))?;
        let continue_url = reqwest::Url::parse(&location)
            .or_else(|_| reqwest::Url::parse(&form.action).and_then(|b| b.join(&location)))
            .map_err(|e| EhError::Parse(format!("bad archiver redirect: {e}")))?;

        // `EhRequestBuilder(continueUrl, origin, null)` — GET with the site
        // origin as referer. The final response host carries the download.
        let req = self
            .request(Method::GET, continue_url.as_str())
            .header(header::REFERER, eh_url::origin(self.site()));
        let response = self.execute(req).await?;
        check_sad_panda(&response)?;
        let final_url = response.url().clone();
        let body = response.text().await.map_err(EhError::from)?;
        let path = parser::parse_archiver_download_link(&body)
            .ok_or_else(|| EhError::Parse("archiver download page has no download link".into()))?;
        if let Ok(absolute) = reqwest::Url::parse(&path) {
            return Ok(absolute.to_string());
        }
        let host = final_url
            .host_str()
            .ok_or_else(|| EhError::Parse("archiver redirect has no host".into()))?;
        Ok(format!("https://{host}{path}"))
    }

    /// Stream a GET response body into `dest` (`.torrent` files, archiver
    /// zips); returns the number of bytes written. The request carries the
    /// session cookies like every other client call.
    pub async fn download_file(
        &self,
        url: &str,
        referer: Option<&str>,
        dest: &std::path::Path,
    ) -> Result<u64, EhError> {
        let mut req = self.request(Method::GET, url);
        if let Some(referer) = referer {
            req = req.header(header::REFERER, referer);
        }
        let mut response = self.execute(req).await?;
        check_sad_panda(&response)?;
        let mut file = tokio::fs::File::create(dest)
            .await
            .map_err(|e| EhError::Eh(format!("create {}: {e}", dest.display())))?;
        let mut written: u64 = 0;
        while let Some(chunk) = response.chunk().await.map_err(EhError::from)? {
            use tokio::io::AsyncWriteExt as _;
            file.write_all(&chunk)
                .await
                .map_err(|e| EhError::Eh(format!("write {}: {e}", dest.display())))?;
            written += chunk.len() as u64;
        }
        Ok(written)
    }
}
