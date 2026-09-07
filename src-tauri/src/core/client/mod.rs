//! Port of `client/EhClient.java` + `client/EhEngine.java` — the async HTTP
//! layer of the app.
//!
//! Architecture notes:
//! * The `reqwest::Client` lives behind an `Arc<RwLock<_>>` so proxy/site
//!   changes can rebuild it at runtime without dropping the rest of the state.
//! * Cookies live outside the client in an `Arc<dyn EhCookieStore>` so they
//!   survive rebuilds; a SQLite store can replace the in-memory one later.
//! * DNS goes through [`dns::EhResolver`] (hosts table → DoH → system),
//!   matching `client/EhHosts.java`.
//! * Image fetches dedup concurrent identical requests.

mod previews;
mod transfers;
use previews::PreviewCache;

pub mod cookie;
pub mod dns;
pub mod url;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::core::error::EhError;
use crate::core::models::{
    Comment, GalleryDetail, GalleryInfo, GalleryPageData, NewsItem, Quota, TopListDetail,
};
use crate::core::parser::{self, FavoritesResult, GalleryListResult};

pub use cookie::{CookieStoreHandle, EhCookieStore, InMemoryCookieStore};
pub use dns::{
    built_in_hosts, DnsResolver, DohResolver, EhResolver, HostsResolver, SystemResolver,
    DEFAULT_DOH_ENDPOINT,
};
pub use url::Site;

use url as eh_url;

/// User agent mimicking a current desktop Firefox, like the original OkHttp
/// configuration does.
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:131.0) Gecko/20100101 Firefox/131.0";

/// `Content-Disposition` of the sad-panda response (EhEngine.SAD_PANDA_*).
const SAD_PANDA_DISPOSITION: &str = "inline; filename=\"sadpanda.jpg\"";
const SAD_PANDA_TYPE: &str = "image/gif";
const SAD_PANDA_LENGTH: &str = "9615";
const KOKOMADE_URL: &str = "https://exhentai.org/img/kokomade.jpg";

/// Client configuration. `update_config` rebuilds the inner reqwest client
/// when the network-relevant parts change.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub site: Site,
    /// `http://host:port`, `https://...` or `socks5://host:port`.
    pub proxy: Option<String>,
    pub doh_enabled: bool,
    pub connect_timeout: Duration,
    /// 0 = original resolution, 1 = 250px, 2 = 300px
    /// (port of `Settings.getThumbResolution()`).
    pub thumb_resolution: i32,
    pub image_resolution: String,
    /// Port of `Settings.getFixThumbUrl()`.
    pub fix_thumb_url: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            site: Site::E,
            proxy: None,
            doh_enabled: false,
            connect_timeout: Duration::from_secs(15),
            thumb_resolution: 0,
            image_resolution: "a".into(),
            fix_thumb_url: true,
        }
    }
}

/// Shared, cheaply-cloneable handle to the EH HTTP layer.
pub struct EhClient {
    client: Arc<RwLock<reqwest::Client>>,
    cookies: Arc<dyn EhCookieStore>,
    hosts: Arc<RwLock<HashMap<String, Vec<IpAddr>>>>,
    doh: Arc<DohResolver>,
    site: RwLock<Site>,
    proxy: RwLock<Option<String>>,
    thumb_resolution: AtomicI32,
    fix_thumb_url: AtomicBool,
    /// In-flight image dedup: url|referer → shared fetch result.
    in_flight: Arc<InFlightMap>,
    /// Preview sets and harvested ptokens for ptoken resolution, keyed
    /// `(gid, gtoken, ...)`; shared so the reader and the download
    /// engine don't re-hit the site for the same preview page.
    preview_cache: Arc<tokio::sync::Mutex<PreviewCache>>,
    /// Site-wide request throttle: HTML/API calls are spaced at least this
    /// far apart so a browsing session (or the reader's preload burst) can
    /// never trip e-hentai's "excessive request rate" IP ban. Image byte
    /// fetches bypass this (they use `request` directly).
    throttle: tokio::sync::Mutex<Instant>,
}

/// Minimum spacing between site HTML/API requests (e-hentai rate-limits by
/// IP; the original client throttles to roughly this rate too).
const SITE_REQUEST_INTERVAL: Duration = Duration::from_millis(500);

type InFlightCell = Arc<tokio::sync::OnceCell<Arc<Result<(Vec<u8>, String), EhError>>>>;
type InFlightMap = Mutex<HashMap<String, InFlightCell>>;
impl std::fmt::Debug for EhClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EhClient")
            .field("site", &self.site)
            .field("proxy", &self.proxy)
            .field(
                "thumb_resolution",
                &self.thumb_resolution.load(Ordering::Relaxed),
            )
            .field("fix_thumb_url", &self.fix_thumb_url.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl EhClient {
    /// Create a client with the given config, cookie store and hosts table.
    pub fn new(
        config: ClientConfig,
        cookies: Arc<dyn EhCookieStore>,
        hosts: Arc<RwLock<HashMap<String, Vec<IpAddr>>>>,
    ) -> Result<Self, EhError> {
        let doh = Arc::new(DohResolver::new(
            DEFAULT_DOH_ENDPOINT,
            config.doh_enabled,
            Arc::new(SystemResolver),
        ));
        let chain: Arc<dyn DnsResolver> = Arc::new(HostsResolver::new(hosts.clone(), doh.clone()));
        let resolver = Arc::new(EhResolver::new(chain));
        let client = Self::build_client(&config, cookies.clone(), resolver)?;
        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            cookies,
            hosts,
            doh,
            site: RwLock::new(config.site),
            proxy: RwLock::new(config.proxy.clone()),
            thumb_resolution: AtomicI32::new(config.thumb_resolution),
            fix_thumb_url: AtomicBool::new(config.fix_thumb_url),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            preview_cache: Arc::new(tokio::sync::Mutex::new(PreviewCache::default())),
            throttle: tokio::sync::Mutex::new(
                Instant::now()
                    .checked_sub(SITE_REQUEST_INTERVAL)
                    .unwrap_or_else(Instant::now),
            ),
        })
    }

    /// Convenience constructor with an in-memory cookie store and the built-in
    /// hosts table from `EhHosts.java`.
    pub fn with_defaults(config: ClientConfig) -> Result<Self, EhError> {
        Self::new(
            config,
            Arc::new(InMemoryCookieStore::new()),
            Arc::new(RwLock::new(built_in_hosts())),
        )
    }

    fn build_client(
        config: &ClientConfig,
        cookies: Arc<dyn EhCookieStore>,
        resolver: Arc<EhResolver>,
    ) -> Result<reqwest::Client, EhError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
            ),
        );
        headers.insert(
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.5"),
        );

        let mut builder = reqwest::Client::builder()
            // OkHttp defaults to 21 redirects; keep the same limit.
            .redirect(reqwest::redirect::Policy::limited(21))
            .default_headers(headers)
            .user_agent(DEFAULT_USER_AGENT)
            .cookie_provider(Arc::new(CookieStoreHandle(
                cookies,
                config.image_resolution.clone(),
            )))
            .dns_resolver(resolver)
            .connect_timeout(config.connect_timeout)
            .read_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(5);

        if let Some(proxy) = &config.proxy {
            let proxy = reqwest::Proxy::all(proxy)
                .map_err(|e| EhError::Eh(format!("invalid proxy: {e}")))?;
            builder = builder.proxy(proxy);
        }

        builder.build().map_err(EhError::from)
    }

    /// Apply a new configuration; rebuilds the reqwest client when the
    /// network-relevant parts changed. Cookies and in-flight state survive.
    pub fn update_config(&self, config: ClientConfig) -> Result<(), EhError> {
        *self.site.write().map_err(|_| EhError::Cancelled)? = config.site;
        *self.proxy.write().map_err(|_| EhError::Cancelled)? = config.proxy.clone();
        self.doh.set_enabled(config.doh_enabled);
        self.thumb_resolution
            .store(config.thumb_resolution, Ordering::Relaxed);
        self.fix_thumb_url
            .store(config.fix_thumb_url, Ordering::Relaxed);

        {
            // Applying network preferences also refreshes pooled connections and cookies.
            let chain: Arc<dyn DnsResolver> =
                Arc::new(HostsResolver::new(self.hosts.clone(), self.doh.clone()));
            let resolver = Arc::new(EhResolver::new(chain));
            let client = Self::build_client(&config, self.cookies.clone(), resolver)?;
            *self.client.write().map_err(|_| EhError::Cancelled)? = client;
        }
        Ok(())
    }

    /// Cookie store shared with account commands.
    pub fn cookies(&self) -> Arc<dyn EhCookieStore> {
        self.cookies.clone()
    }

    /// Current gallery site.
    pub fn site(&self) -> Site {
        *self.site.read().unwrap_or_else(|p| p.into_inner())
    }

    // ------------------------------------------------------------------
    // Low-level helpers
    // ------------------------------------------------------------------

    fn client(&self) -> reqwest::Client {
        // Clone the Arc so the read guard is never held across awaits.
        self.client
            .read()
            .map(|c| c.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }

    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client().request(method, url)
    }

    /// Wait until the next site request slot is due, spacing HTML/API calls
    /// at least [`SITE_REQUEST_INTERVAL`] apart. Wakers reserve their slot
    /// before sleeping so a burst of waiters drains sequentially without a
    /// thundering herd re-firing together.
    async fn throttle_acquire(&self) {
        let mut last = self.throttle.lock().await;
        let next = (*last + SITE_REQUEST_INTERVAL).max(Instant::now());
        *last = next;
        drop(last);
        let now = Instant::now();
        if next > now {
            tokio::time::sleep(next - now).await;
        }
    }

    /// Execute a request, map transport failures and check the status code,
    /// mirroring `EhEngine.throwException` (sad panda / kokomade checks).
    async fn execute(&self, req: RequestBuilder) -> Result<Response, EhError> {
        let response = req.send().await.map_err(EhError::from)?;
        let status = response.status();
        if status == StatusCode::from_u16(509).unwrap() {
            return Err(EhError::Image509);
        }
        if status >= StatusCode::BAD_REQUEST {
            let body = response.text().await.unwrap_or_default();
            return Err(EhError::HttpStatus(status.as_u16(), snippet(&body)));
        }
        Ok(response)
    }

    /// GET `url` and return the body as String, running the shared
    /// sad-panda/kokomade/509 checks.
    pub async fn get_string(&self, url: &str, referer: Option<&str>) -> Result<String, EhError> {
        self.throttle_acquire().await;
        let mut req = self
            .request(Method::GET, url)
            .timeout(Duration::from_secs(45));
        if let Some(referer) = referer {
            req = req.header(header::REFERER, referer);
        }
        let response = self.execute(req).await?;
        check_sad_panda(&response)?;
        let body = response.text().await.map_err(EhError::from)?;
        check_body_errors(&body)?;
        Ok(body)
    }

    // ------------------------------------------------------------------
    // Gallery list / detail (port of EhEngine.getGalleryList & friends)
    // ------------------------------------------------------------------

    /// Fetch and parse a gallery list page. `url` is typically produced by
    /// [`crate::core::models::ListUrlBuilder`].
    pub async fn get_gallery_list(&self, url: &str) -> Result<GalleryListResult, EhError> {
        let referer = eh_url::referer(self.site());
        let body = self.get_string(url, Some(referer)).await?;
        let mut result = parser::parse_gallery_list(&body)?;
        let fix = self.fix_thumb_url.load(Ordering::Relaxed);
        for gi in &mut result.galleries {
            gi.thumb = self.fix_thumb(&gi.thumb);
            if fix {
                gi.thumb = eh_url::fixed_preview_thumb_url(&gi.thumb);
            }
        }
        Ok(result)
    }

    /// Fetch and parse a gallery detail page.
    pub async fn get_gallery_detail(
        &self,
        gid: u64,
        token: &str,
    ) -> Result<GalleryDetail, EhError> {
        let url = eh_url::gallery_detail_url(self.site(), gid, token, 0, false);
        let referer = eh_url::referer(self.site());
        let body = self.get_string(&url, Some(referer)).await?;
        let mut detail = parser::parse_gallery_detail(&body)?;
        detail.info.thumb = self.fix_thumb(&detail.info.thumb);
        Ok(detail)
    }

    // ------------------------------------------------------------------
    // Gallery page (port of EhEngine.getGalleryPage)
    // ------------------------------------------------------------------

    /// Fetch one gallery page (`s/{ptoken}/{gid}-{page}`) and parse the
    /// image URL / showkey / skip key. The `ptoken` is the page token from a
    /// preview set or from the previous page (see
    /// [`EhClient::get_gallery_token`]).
    pub async fn get_page_data(
        &self,
        gid: u64,
        ptoken: &str,
        page: i32,
    ) -> Result<GalleryPageData, EhError> {
        let url = eh_url::page_url(self.site(), gid, page, ptoken);
        let referer = eh_url::referer(self.site());
        let body = self.get_string(&url, Some(referer)).await?;
        parser::parse_gallery_page(&body, ptoken)
    }

    /// `api.php` `gdata` call used for detail/list enrichment
    /// (port of `EhEngine.fillGalleryListByApi`).
    pub async fn get_gallery_metadata(
        &self,
        gid_tokens: &[(u64, String)],
    ) -> Result<Value, EhError> {
        let gidlist: Vec<Value> = gid_tokens
            .iter()
            .map(|(gid, token)| json!([gid, token]))
            .collect();
        let payload = json!({
            "method": "gdata",
            "gidlist": gidlist,
            "namespace": 1,
        });
        let body = self
            .post_api_json(&payload, Some(eh_url::referer(self.site())))
            .await?;
        serde_json::from_str(&body).map_err(|e| EhError::Parse(e.to_string()))
    }

    /// The `apiuid`/`apikey` pair for api.php calls: the `ipb_member_id` /
    /// `ipb_pass_hash` cookie pair, exactly like the original reads them.
    fn api_credentials(&self) -> Result<(i64, String), EhError> {
        let site = self.site();
        let origin = reqwest::Url::parse(eh_url::origin(site))
            .map_err(|e| EhError::Cookie(format!("invalid origin: {e}")))?;
        let apiuid: i64 = self
            .cookies
            .get_cookie_value(&origin, "ipb_member_id")
            .ok_or(EhError::NotSignedIn)?
            .trim_matches('"')
            .parse()
            .map_err(|_| EhError::Cookie("ipb_member_id is not numeric".into()))?;
        let apikey = self
            .cookies
            .get_cookie_value(&origin, "ipb_pass_hash")
            .ok_or(EhError::NotSignedIn)?;
        Ok((apiuid, apikey))
    }

    /// `api.php` `rategallery` call (port of `EhEngine.rateGallery`); the
    /// `apiuid`/`apikey` pair is the `ipb_member_id`/`ipb_pass_hash` cookie
    /// pair, exactly like the original reads them. Returns the new average
    /// rating (0.0-5.0) from the response.
    pub async fn rate_gallery(&self, gid: u64, token: &str, rating: f64) -> Result<f32, EhError> {
        let (apiuid, apikey) = self.api_credentials()?;
        let payload = json!({
            "method": "rategallery",
            "apiuid": apiuid,
            "apikey": apikey,
            "gid": gid,
            "token": token,
            "rating": (rating * 2.0).ceil() as i32,
        });
        let referer = eh_url::gallery_detail_url(self.site(), gid, token, 0, false);
        let body = self.post_api_json(&payload, Some(&referer)).await?;
        let value: Value =
            serde_json::from_str(&body).map_err(|e| EhError::Parse(e.to_string()))?;
        let average = value["rating_ave"]
            .as_str()
            .and_then(|s| s.parse::<f32>().ok())
            .or_else(|| value["rating_ave"].as_f64().map(|f| f as f32))
            .ok_or_else(|| {
                EhError::Parse(format!("rategallery response has no rating_ave: {body}"))
            })?;
        Ok(average)
    }

    /// POST a URL-encoded form (port of the `EhRequestBuilder` POST flow used
    /// by `EhEngine.addFavorites` et al.): form fields, `Referer` defaulting
    /// to the request URL itself, `Origin` from the current site.
    pub async fn post_form(
        &self,
        url: &str,
        referer: Option<&str>,
        fields: &[(&str, &str)],
    ) -> Result<String, EhError> {
        self.throttle_acquire().await;
        let req = self
            .request(Method::POST, url)
            .header(header::REFERER, referer.unwrap_or(url))
            .header(header::ORIGIN, eh_url::origin(self.site()))
            .form(fields);
        let response = self.execute(req).await?;
        check_sad_panda(&response)?;
        let body = response.text().await.map_err(EhError::from)?;
        check_body_errors(&body)?;
        Ok(body)
    }

    // ------------------------------------------------------------------
    // Favorites / toplist / comments / news
    // ------------------------------------------------------------------

    /// Fetch and parse `favorites.php` (port of `EhEngine.getFavorites`).
    pub async fn get_favorites(&self, url: &str) -> Result<FavoritesResult, EhError> {
        let referer = eh_url::referer(self.site());
        let body = self.get_string(url, Some(referer)).await?;
        parser::parse_favorites(&body)
    }

    /// Fetch and parse `toplist.php` (port of `EhEngine.getTopList`).
    pub async fn get_toplist(&self, url: &str) -> Result<TopListDetail, EhError> {
        let referer = eh_url::referer(self.site());
        let body = self.get_string(url, Some(referer)).await?;
        parser::parse_toplist(&body)
    }

    /// Fetch a detail page and return its comments plus the has-more flag.
    pub async fn get_comments(
        &self,
        gid: u64,
        gtoken: &str,
    ) -> Result<(Vec<Comment>, bool), EhError> {
        let detail = self.get_gallery_detail(gid, gtoken).await?;
        Ok((detail.comments, detail.has_more_comments))
    }

    /// `api.php` `votecomment` call (port of `EhEngine.voteComment`): posts
    /// `comment_vote` (1 or -1) for `comment_id` and returns the updated
    /// `comment_score` from the response.
    pub async fn vote_comment(
        &self,
        gid: u64,
        token: &str,
        comment_id: i64,
        vote: i64,
    ) -> Result<i64, EhError> {
        let (apiuid, apikey) = self.api_credentials()?;
        let payload = json!({
            "method": "votecomment",
            "apiuid": apiuid,
            "apikey": apikey,
            "gid": gid,
            "token": token,
            "comment_id": comment_id,
            "comment_vote": vote,
        });
        let body = self
            .post_api_json(&payload, Some(eh_url::referer(self.site())))
            .await?;
        let value: Value =
            serde_json::from_str(&body).map_err(|e| EhError::Parse(e.to_string()))?;
        value["comment_score"].as_i64().ok_or_else(|| {
            EhError::Parse(format!("votecomment response has no comment_score: {body}"))
        })
    }

    /// Post a new comment (port of `EhEngine.commentGallery` with
    /// `commenttext_new`) as a form POST to the gallery detail URL. The
    /// `#chd + p` notice page (too long, banned text, ...) surfaces its text
    /// as an error, like the original's EhException.
    pub async fn send_comment(&self, gid: u64, token: &str, text: &str) -> Result<(), EhError> {
        let url = eh_url::gallery_detail_url(self.site(), gid, token, 0, false);
        let body = self
            .post_form(&url, Some(&url), &[("commenttext_new", text)])
            .await?;
        if let Some(message) = parser::parse_comment_error(&body) {
            return Err(EhError::Eh(message));
        }
        Ok(())
    }

    /// Fetch `news.php` and return the structured news posts
    /// (port of `EhEngine.getEhNews`; the original wraps the raw page in
    /// `EhNewsDetail.webData`, the desktop port parses it into items).
    /// Pages beyond the first are requested with `?p=N`.
    pub async fn get_news(&self, page: i32) -> Result<Vec<NewsItem>, EhError> {
        let base = eh_url::news_url(self.site());
        let url = if page > 0 {
            format!("{base}?p={page}")
        } else {
            base.to_string()
        };
        let body = self.get_string(&url, None).await?;
        Ok(parser::parse_news(&body))
    }

    /// Fetch `home.php` and parse the quota/rate-limit numbers
    /// (port of `EhEngine.getHomeDetail` + `EhHomeParser.parse`). Never
    /// fails on markup drift: anonymous sessions get the login page and
    /// simply yield the empty/default quota.
    pub async fn get_quota(&self) -> Result<Quota, EhError> {
        let url = eh_url::home_url(self.site());
        let referer = eh_url::referer(self.site());
        let body = self.get_string(url, Some(referer)).await?;
        Ok(parser::parse_quota(&body))
    }

    // ------------------------------------------------------------------
    // Image search (port of EhEngine.imageSearch)
    // ------------------------------------------------------------------

    /// The `image_lookup.php` text parts for the given toggles, in the order
    /// the original `EhEngine.imageSearch` adds them: `fs_similar`,
    /// `fs_covers`, `fs_exp` (each "on" when enabled), then the file-search
    /// submit button `f_sfile=File Search`. Split out of the multipart
    /// builder so the field assembly can be unit-tested without a server.
    fn image_lookup_form_fields(
        similarity_scan: bool,
        only_covers: bool,
        show_expunged: bool,
    ) -> Vec<(&'static str, &'static str)> {
        let mut fields = Vec::with_capacity(4);
        if similarity_scan {
            fields.push(("fs_similar", "on"));
        }
        if only_covers {
            fields.push(("fs_covers", "on"));
        }
        if show_expunged {
            fields.push(("fs_exp", "on"));
        }
        fields.push(("f_sfile", "File Search"));
        fields
    }

    /// Build the multipart body of an image-lookup request
    /// (port of the `MultipartBody.Builder` block in `EhEngine.imageSearch`):
    /// the file under `sfile`, then [`Self::image_lookup_form_fields`].
    fn build_image_lookup_form(
        file_name: &str,
        bytes: Vec<u8>,
        similarity_scan: bool,
        only_covers: bool,
        show_expunged: bool,
    ) -> Result<reqwest::multipart::Form, EhError> {
        let mut form = reqwest::multipart::Form::new().part(
            "sfile",
            reqwest::multipart::Part::bytes(bytes)
                .file_name(file_name.to_string())
                .mime_str("application/octet-stream")
                .map_err(|e| EhError::Parse(e.to_string()))?,
        );
        for (name, value) in
            Self::image_lookup_form_fields(similarity_scan, only_covers, show_expunged)
        {
            form = form.text(name, value);
        }
        Ok(form)
    }

    /// Upload an image to `image_lookup.php` and return the final search URL
    /// (after the 302) together with the parsed result list.
    pub async fn image_lookup(
        &self,
        file_name: &str,
        bytes: Vec<u8>,
        similarity_scan: bool,
        only_covers: bool,
        show_expunged: bool,
    ) -> Result<(String, GalleryListResult), EhError> {
        let site = self.site();
        let url = eh_url::image_search_url(site);
        let referer = format!("{}/", eh_url::referer(site));
        let origin = eh_url::origin(site);

        let form = Self::build_image_lookup_form(
            file_name,
            bytes,
            similarity_scan,
            only_covers,
            show_expunged,
        )?;

        let client = self.client();
        // reqwest follows the 302 automatically; response.url() is the final
        // search URL (the original handles the 302 manually).
        let response = client
            .post(url)
            .header(header::REFERER, &referer)
            .header(header::ORIGIN, origin)
            .multipart(form)
            .send()
            .await
            .map_err(EhError::from)?;

        let response = check_status(response).await?;
        check_sad_panda(&response)?;
        let final_url = response.url().to_string();
        let body = response.text().await.map_err(EhError::from)?;
        check_body_errors(&body)?;

        let mut result = parser::parse_gallery_list(&body)?;
        for gi in &mut result.galleries {
            gi.thumb = self.fix_thumb(&gi.thumb);
        }
        Ok((final_url, result))
    }

    // ------------------------------------------------------------------
    // Images
    // ------------------------------------------------------------------

    /// Fetch an image (thumbnail / page image) with in-flight dedup.
    /// Returns the bytes and the content type. 509 placeholder images map to
    /// [`EhError::Image509`].
    pub async fn get_image_bytes(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<(Vec<u8>, String), EhError> {
        let key = format!("{}|{}", url, referer.unwrap_or_default());
        let cell = {
            let mut map = self.in_flight.lock().await;
            Arc::clone(
                map.entry(key.clone())
                    .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new())),
            )
        };
        let shared = cell
            .get_or_init(|| self.fetch_image(url, referer))
            .await
            .clone();
        // Drop the map entry once the fetch settled; the Arc keeps the result
        // alive for any waiter still holding it.
        {
            let mut map = self.in_flight.lock().await;
            if Arc::strong_count(&cell) <= 2 {
                map.remove(&key);
            }
        }
        Arc::try_unwrap(shared).unwrap_or_else(|arc| (*arc).clone())
    }

    async fn fetch_image(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Arc<Result<(Vec<u8>, String), EhError>> {
        let result = async {
            let mut req = self.request(Method::GET, url);
            if let Some(referer) = referer {
                req = req.header(header::REFERER, referer);
            }
            let response = self.execute(req).await?;
            check_sad_panda(&response)?;
            let content_type = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let bytes = response.bytes().await.map_err(EhError::from)?.to_vec();
            if parser::is_509_image(&content_type, &bytes) {
                return Err(EhError::Image509);
            }
            Ok((bytes, content_type))
        }
        .await;
        Arc::new(result)
    }

    // ------------------------------------------------------------------
    // Misc helpers
    // ------------------------------------------------------------------

    /// Port of `EhUtils.handleThumbUrlResolution`.
    pub fn fix_thumb(&self, url: &str) -> String {
        let resolution = self.thumb_resolution.load(Ordering::Relaxed);
        let target = match resolution {
            1 => "250",
            2 => "300",
            _ => return url.to_string(),
        };
        let index1 = url.rfind('_');
        let index2 = url.rfind('.');
        match (index1, index2) {
            (Some(i1), Some(i2)) if i1 < i2 => {
                format!("{}{}{}", &url[..=i1], target, &url[i2..])
            }
            _ => url.to_string(),
        }
    }

    async fn post_api_json(
        &self,
        payload: &Value,
        referer: Option<&str>,
    ) -> Result<String, EhError> {
        let url = eh_url::api_url(self.site());
        let mut req = self
            .request(Method::POST, url)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .header(header::ORIGIN, eh_url::origin(self.site()))
            .json(payload);
        if let Some(referer) = referer {
            req = req.header(header::REFERER, referer);
        }
        let response = self.execute(req).await?;
        check_sad_panda(&response)?;
        let body = response.text().await.map_err(EhError::from)?;
        check_body_errors(&body)?;
        Ok(body)
    }
}

/// Truncate a body for error messages.
fn snippet(body: &str) -> String {
    const MAX: usize = 200;
    if body.chars().count() <= MAX {
        body.to_string()
    } else {
        body.chars().take(MAX).collect()
    }
}

async fn check_status(response: Response) -> Result<Response, EhError> {
    let status = response.status();
    if status >= StatusCode::BAD_REQUEST {
        let body = response.text().await.unwrap_or_default();
        return Err(EhError::HttpStatus(status.as_u16(), snippet(&body)));
    }
    Ok(response)
}

/// Sad-panda detection, ported from `EhEngine.doThrowException`.
fn check_sad_panda(response: &Response) -> Result<(), EhError> {
    let headers = response.headers();
    let disposition = headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let content_length = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if disposition.contains(SAD_PANDA_DISPOSITION)
        && content_type.starts_with(SAD_PANDA_TYPE)
        && content_length == SAD_PANDA_LENGTH
    {
        return Err(EhError::Eh("Sad Panda".to_string()));
    }
    Ok(())
}

/// Kokomade / 509 / bandwidth / IP-ban checks for HTML bodies.
fn check_body_errors(body: &str) -> Result<(), EhError> {
    if body.contains(KOKOMADE_URL) {
        return Err(EhError::Eh(
            "今回はここまで (image limit reached for this period)".to_string(),
        ));
    }
    if body.contains("You have exceeded your image viewing limits") {
        return Err(EhError::Image509);
    }
    if body.contains("exceeded your bandwidth limits") {
        return Err(EhError::BandwidthExceeded);
    }
    if body.contains("temporarily banned") {
        // e-hentai's rate-limit page names the expiry; surface it so the
        // user sees "wait 3 minutes" instead of a cryptic parse failure.
        let detail = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let msg = detail
            .split("The ban expires in")
            .nth(1)
            .map(|rest| format!("IP temporarily banned — expires in{rest}"));
        return Err(EhError::Eh(msg.unwrap_or_else(|| {
            "IP temporarily banned (excessive request rate)".to_string()
        })));
    }
    Ok(())
}

// Preserve thread-safety guarantees without runtime scaffolding.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EhClient>();
    assert_send_sync::<GalleryInfo>();
    assert_send_sync::<GalleryPageData>();
};

#[cfg(test)]
mod tests;
