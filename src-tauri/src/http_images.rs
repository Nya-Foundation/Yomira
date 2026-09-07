//! Tiny read-only HTTP server that exposes the on-disk image cache to the
//! webview.
//!
//! Why this exists: WebKit2GTK (and Windows WebView2 in some setups) refuses
//! to load Tauri's custom `asset://` scheme from the dev origin
//! (`http://localhost:1420`), so cached images never rendered. Serving the
//! cache over `http://127.0.0.1:<ephemeral>` sidesteps the scheme
//! restrictions entirely: the frontend keeps using whatever URL string the
//! backend returns, verbatim.
//!
//! Deliberately minimal (no new dependencies):
//! * HTTP/1.1, `GET` only, hand-parsed request line + headers.
//! * Keep-alive: the request's `Connection` header is honored (default
//!   keep-alive for HTTP/1.1), so WebView2 reuses connections instead of
//!   paying a TCP handshake per image. `Connection: close` closes after the
//!   response, as does any idle connection after [`IDLE_TIMEOUT`].
//! * Serves ONLY regular files directly inside the cache dir whose names
//!   match the cache key pattern (`[0-9a-f]{16}.{jpg,png,gif,webp}` — the
//!   exact shape `image_cache::cache_key` + extension produce). There is no
//!   path decoding and no directory traversal by construction: anything that
//!   doesn't match the pattern gets a 404 unless it is an explicitly
//!   registered saved-page name. Saved pages use no-store and are streamed
//!   from the download folder, independently of cache eviction.
//! * Bodies are streamed with `tokio::fs` + fixed-size chunks; every
//!   response carries `Content-Length` so the client knows where each
//!   response ends on a reused connection.
//! * Runs on a dedicated std::thread with its own single-threaded tokio
//!   runtime, fully independent of the app's runtime; the resolved port
//!   travels back through an mpsc channel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Saved pages have stable opaque names and are streamed from their original
/// files. The webview cannot supply a filesystem path. Entries are re-created
/// on demand after restart; nothing depends on a previous ephemeral port.
#[derive(Clone, Default)]
pub struct LocalImages(Arc<RwLock<HashMap<String, (PathBuf, PathBuf)>>>);

impl LocalImages {
    pub fn register(&self, gallery_dir: &Path, path: &Path) -> Option<String> {
        let dir = gallery_dir.canonicalize().ok()?;
        let path = path.canonicalize().ok()?;
        if path.parent()? != dir {
            return None;
        }
        let ext = path.extension()?.to_str()?;
        if !matches!(ext, "jpg" | "png" | "gif" | "webp" | "avif") {
            return None;
        }
        let meta = std::fs::symlink_metadata(&path).ok()?;
        if !meta.is_file() || meta.len() == 0 {
            return None;
        }
        let name = format!(
            "local-{}.{}",
            crate::image_cache::cache_key(&path.to_string_lossy()),
            ext
        );
        self.0.write().ok()?.insert(name.clone(), (dir, path));
        Some(name)
    }

    fn resolve(&self, name: &str) -> Option<PathBuf> {
        let (dir, path) = self.0.read().ok()?.get(name)?.clone();
        // Revalidate if a folder/file was moved, removed or replaced by a link.
        let actual = path.canonicalize().ok()?;
        (actual == path && actual.parent()? == dir).then_some(path)
    }
}

/// Maximum request head we'll read (plenty for one request line + headers).
const MAX_HEAD: usize = 16 * 1024;

/// Idle keep-alive timeout: how long a connection with no in-flight request
/// waits for the next one before we close it (bounds leaked tasks).
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Body copy buffer for streamed file responses.
const FILE_CHUNK: usize = 64 * 1024;

/// A parsed request head — only what we act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHead {
    pub method: String,
    /// Raw, undecoded path from the request line (e.g. `/abc.jpg`).
    pub path: String,
    /// Whether the client wants the connection kept open after the response
    /// (HTTP/1.1 default; an explicit `Connection` header overrides).
    pub keep_alive: bool,
}

impl RequestHead {
    /// The cache file name, if the path is exactly `/` + a valid cache file
    /// name (no subdirectories, no `..`, no percent-encoding).
    pub fn file_name(&self) -> Option<&str> {
        let name = self.path.strip_prefix('/')?;
        is_safe_file_name(name).then_some(name)
    }
}

/// Does `name` have exactly the shape the cache produces?
/// `cache_key()` is 16 lowercase hex chars; `ImageCache::store` appends
/// `.` + one of the extensions `extension_for` can return.
pub fn is_safe_file_name(name: &str) -> bool {
    let Some((key, ext)) = name.rsplit_once('.') else {
        return false;
    };
    let ext_ok = matches!(ext, "jpg" | "png" | "gif" | "webp");
    let key_ok = key.len() == 16
        && key
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    ext_ok && key_ok
}

/// `Content-Type` for a validated extension.
pub fn content_type(file_name: &str) -> &'static str {
    match file_name.rsplit('.').next().unwrap_or_default() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

/// Parse an HTTP request head (request line + headers, up to `\r\n\r\n`).
///
/// Returns `Ok(None)` when more bytes are needed, `Ok(Some(head))` when the
/// head is complete, and `Err(())` for malformed input. Reading stops at the
/// header terminator or at [`MAX_HEAD`].
fn parse_request_head(buffer: &[u8]) -> Result<Option<RequestHead>, ()> {
    let Some(end) = find_subslice(buffer, b"\r\n\r\n") else {
        return if buffer.len() >= MAX_HEAD {
            Err(())
        } else {
            Ok(None)
        };
    };
    let head = &buffer[..end];
    let mut lines = head.split(|&b| b == b'\n');
    let Some(request_line) = lines.next() else {
        return Err(());
    };
    let request_line = trim_cr(request_line);
    let mut parts = request_line.split(|&b| b == b' ');
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    if method.is_empty()
        || path.is_empty()
        || !version.starts_with(b"HTTP/")
        || parts.next().is_some()
    {
        return Err(());
    }
    // Connection preference: HTTP/1.1 keeps alive unless "close" is named,
    // HTTP/1.0 only with an explicit "keep-alive" (what the client sent on
    // the wire; header names and tokens are case-insensitive).
    let mut keep_alive = version != b"HTTP/1.0";
    for line in lines {
        let line = trim_cr(line);
        let Some(colon) = line.iter().position(|&b| b == b':') else {
            continue;
        };
        let (name, value) = (&line[..colon], &line[colon + 1..]);
        if !name.eq_ignore_ascii_case(b"connection") {
            continue;
        }
        let value = String::from_utf8_lossy(value).to_lowercase();
        let tokens: Vec<&str> = value.split(',').map(|t| t.trim()).collect();
        if tokens.contains(&"close") {
            keep_alive = false;
        } else if tokens.contains(&"keep-alive") {
            keep_alive = true;
        }
    }
    let method = std::str::from_utf8(method).map_err(|_| ())?.to_string();
    let path = std::str::from_utf8(path).map_err(|_| ())?.to_string();
    Ok(Some(RequestHead {
        method,
        path,
        keep_alive,
    }))
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// `GET http://127.0.0.1:<port>/<file_name>` — the URL shape the frontend
/// receives for every cached image.
pub fn http_url(port: u16, file_name: &str) -> String {
    format!("http://127.0.0.1:{port}/{file_name}")
}

/// Bind `127.0.0.1:0`, report the port on `port_tx`, then serve until the
/// process exits. Runs on its own single-threaded runtime (see
/// [`spawn_server`]).
pub async fn serve(
    dir: PathBuf,
    local: LocalImages,
    port_tx: mpsc::Sender<u16>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    // The receiver only errors if the app gave up on us; keep serving.
    let _ = port_tx.send(port);
    eprintln!(
        "[image-server] serving {} on http://127.0.0.1:{port}",
        dir.display()
    );

    loop {
        let (stream, _peer) = listener.accept().await?;
        let dir = dir.clone();
        let local = local.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, dir, local).await {
                eprintln!("[image-server] connection error: {e}");
            }
        });
    }
}

/// Spawn the server on a dedicated OS thread with its own tokio runtime and
/// return the resolved port (blocks until the listener is up).
pub fn spawn_server(dir: PathBuf, local: LocalImages) -> std::io::Result<u16> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("image-server runtime");
        if let Err(e) = rt.block_on(serve(dir, local, tx)) {
            eprintln!("[image-server] fatal: {e}");
        }
    });
    rx.recv()
        .map_err(|e| std::io::Error::other(format!("server died: {e}")))
}

async fn handle_conn(
    mut stream: TcpStream,
    dir: PathBuf,
    local: LocalImages,
) -> std::io::Result<()> {
    let mut buffer: Vec<u8> = Vec::with_capacity(1024);
    loop {
        // Read one request head. `buffer` may already hold bytes of the
        // next (pipelined) request from the previous iteration.
        let (head, head_len) = loop {
            match parse_request_head(&buffer) {
                Ok(Some(head)) => {
                    let head_len = find_subslice(&buffer, b"\r\n\r\n").unwrap_or(0) + 4;
                    break (head, head_len);
                }
                Ok(None) => {}
                // Malformed head: respond and drop the connection.
                Err(()) => {
                    let body: &[u8] = b"bad request";
                    let mut response =
                        response_head(400, "text/plain", body.len() as u64, false).into_bytes();
                    response.extend_from_slice(body);
                    return finish(&mut stream, &response).await;
                }
            }
            let mut chunk = [0u8; 1024];
            let read = match tokio::time::timeout(IDLE_TIMEOUT, stream.read(&mut chunk)).await {
                Ok(read) => read?,
                Err(_) => return Ok(()), // idle keep-alive timeout
            };
            if read == 0 {
                return Ok(()); // client went away mid-head
            }
            buffer.extend_from_slice(&chunk[..read]);
        };
        buffer.drain(..head_len);

        if head.method != "GET" {
            respond(&mut stream, 400, "text/plain", b"GET only", head.keep_alive).await?;
            if !head.keep_alive {
                return Ok(());
            }
            continue;
        }
        let name = head.path.strip_prefix('/').unwrap_or_default();
        let saved_path = local.resolve(name);
        let Some(path) = saved_path
            .clone()
            .or_else(|| head.file_name().map(|name| dir.join(name)))
        else {
            respond(
                &mut stream,
                404,
                "text/plain",
                b"not found",
                head.keep_alive,
            )
            .await?;
            if !head.keep_alive {
                return Ok(());
            }
            continue;
        };
        // Defense in depth: refuse anything that isn't a regular file
        // directly inside the cache dir (symlinks to elsewhere, etc.).
        let meta = match tokio::fs::symlink_metadata(&path).await {
            Ok(meta) if meta.is_file() => meta,
            _ => {
                respond(
                    &mut stream,
                    404,
                    "text/plain",
                    b"not found",
                    head.keep_alive,
                )
                .await?;
                if !head.keep_alive {
                    return Ok(());
                }
                continue;
            }
        };
        respond_file(
            &mut stream,
            content_type(name),
            &path,
            meta.len(),
            head.keep_alive,
            saved_path.is_some(),
        )
        .await?;
        if !head.keep_alive {
            return Ok(());
        }
    }
}

/// One buffered response for small fixed bodies (errors).
async fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    keep_alive: bool,
) -> std::io::Result<()> {
    let mut response = response_head(status, content_type, body.len() as u64, keep_alive)
        .replace("public, max-age=31536000, immutable", "no-store")
        .into_bytes();
    response.extend_from_slice(body);
    stream.write_all(&response).await?;
    stream.flush().await?;
    if !keep_alive {
        stream.shutdown().await?;
    }
    Ok(())
}

/// Stream a file body in [`FILE_CHUNK`] chunks after a `Content-Length` head,
/// so no huge allocation sits on the single-thread runtime.
async fn respond_file(
    stream: &mut TcpStream,
    content_type: &str,
    path: &Path,
    len: u64,
    keep_alive: bool,
    local: bool,
) -> std::io::Result<()> {
    let head = response_head(200, content_type, len, keep_alive);
    let head = if local {
        head.replace("public, max-age=31536000, immutable", "no-store")
    } else {
        head
    };
    stream.write_all(head.as_bytes()).await?;
    let mut file = tokio::fs::File::open(path).await?.take(len);
    let mut chunk = vec![0u8; FILE_CHUNK];
    loop {
        let read = file.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        stream.write_all(&chunk[..read]).await?;
    }
    stream.flush().await
}

fn response_head(status: u16, content_type: &str, content_length: u64, keep_alive: bool) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {content_length}\r\n\
         Cache-Control: public, max-age=31536000, immutable\r\n\
         Connection: {}\r\n\
         \r\n",
        if keep_alive { "keep-alive" } else { "close" }
    )
}

/// Write the buffered response, then signal EOF when closing.
async fn finish(stream: &mut TcpStream, response: &[u8]) -> std::io::Result<()> {
    stream.write_all(response).await?;
    stream.flush().await?;
    stream.shutdown().await
}

#[cfg(test)]
#[path = "http_images_tests.rs"]
mod tests;
