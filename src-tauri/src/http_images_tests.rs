use super::*;

#[test]
fn local_registry_rejects_unregistered_paths_and_files_outside_gallery() {
    let dir = std::env::temp_dir().join(format!(
        "exviewer-registry-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let gallery = dir.join("gallery");
    std::fs::create_dir_all(&gallery).unwrap();
    let page = gallery.join("0001.png");
    let outside = dir.join("secret.png");
    std::fs::write(&page, b"saved image").unwrap();
    std::fs::write(&outside, b"private").unwrap();
    let local = LocalImages::default();
    assert!(local.register(&gallery, &outside).is_none());
    assert!(local.resolve("../secret.png").is_none());
    assert!(local.resolve("local-0123456789abcdef.png").is_none());
    let name = local.register(&gallery, &page).unwrap();
    assert_eq!(local.resolve(&name), Some(page.canonicalize().unwrap()));
    #[cfg(unix)]
    {
        std::fs::remove_file(&page).unwrap();
        std::os::unix::fs::symlink(&outside, &page).unwrap();
        assert!(local.resolve(&name).is_none());
        assert!(local.register(&gallery, &page).is_none());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn safe_names_match_cache_key_shape() {
    assert!(is_safe_file_name("0123456789abcdef.jpg"));
    assert!(is_safe_file_name("ffffffffffffffff.webp"));
    assert!(is_safe_file_name("a1b2c3d4e5f60718.png"));
    assert!(is_safe_file_name("a1b2c3d4e5f60718.gif"));

    // Traversal / malformed attempts.
    assert!(!is_safe_file_name("../x.jpg"));
    assert!(!is_safe_file_name("..%2f..%2fetc.jpg"));
    assert!(!is_safe_file_name("%2e%2e%2fsecret.jpg"));
    assert!(!is_safe_file_name("sub/0123456789abcdef.jpg"));
    assert!(!is_safe_file_name("0123456789abcdef"));
    assert!(!is_safe_file_name("0123456789abcdef.jpg/extra"));
    assert!(!is_safe_file_name("0123456789abcde.jpg")); // 15 hex chars
    assert!(!is_safe_file_name("0123456789abcdefg.jpg")); // 17 chars
    assert!(!is_safe_file_name("0123456789ABCDEF.jpg")); // uppercase
    assert!(!is_safe_file_name("0123456789abcdef.sh"));
    assert!(!is_safe_file_name("0123456789abcdefjpeg"));
    assert!(!is_safe_file_name(""));
    assert!(!is_safe_file_name(".jpg"));
}

#[test]
fn parses_valid_get_request() {
    let raw = b"GET /0123456789abcdef.jpg HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    let head = parse_request_head(raw).unwrap().unwrap();
    assert_eq!(head.method, "GET");
    assert_eq!(head.path, "/0123456789abcdef.jpg");
    assert_eq!(head.file_name(), Some("0123456789abcdef.jpg"));
}

#[test]
fn connection_header_gates_keep_alive() {
    // HTTP/1.1 defaults to keep-alive.
    let head = parse_request_head(b"GET /x HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap()
        .unwrap();
    assert!(head.keep_alive);
    // Explicit close wins.
    let head = parse_request_head(b"GET /x HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .unwrap()
        .unwrap();
    assert!(!head.keep_alive);
    // Case-insensitive name and token list.
    let head =
        parse_request_head(b"GET /x HTTP/1.1\r\nhost: x\r\nCoNNeCTioN: Keep-Alive, foo\r\n\r\n")
            .unwrap()
            .unwrap();
    assert!(head.keep_alive);
    let head = parse_request_head(b"GET /x HTTP/1.1\r\nConnection: foo, CLOSE\r\n\r\n")
        .unwrap()
        .unwrap();
    assert!(!head.keep_alive);
    // HTTP/1.0 defaults to close unless asked to keep alive.
    let head = parse_request_head(b"GET /x HTTP/1.0\r\n\r\n")
        .unwrap()
        .unwrap();
    assert!(!head.keep_alive);
    let head = parse_request_head(b"GET /x HTTP/1.0\r\nConnection: keep-alive\r\n\r\n")
        .unwrap()
        .unwrap();
    assert!(head.keep_alive);
}

#[test]
fn incomplete_head_waits_for_more() {
    assert_eq!(parse_request_head(b"GET /x").unwrap(), None);
    // Over-long head without terminator is an error.
    assert!(parse_request_head(&[b'a'; MAX_HEAD]).is_err());
}

#[test]
fn rejects_malformed_request_lines() {
    assert!(parse_request_head(b"GET\r\n\r\n").is_err());
    assert!(parse_request_head(b"GET /x EXTRA HTTP/1.1\r\n\r\n").is_err());
    assert!(parse_request_head(b"GET /x FTP/1.1\r\n\r\n").is_err());
    assert!(parse_request_head(b"\xff\xff /x HTTP/1.1\r\n\r\n").is_err());
}

#[test]
fn traversal_paths_yield_no_file_name() {
    for path in [
        "/../x.jpg",
        "/%2e%2e/%2e%2e/x.jpg",
        "/..%2f..%2fetc",
        "/etc/passwd",
        "//0123456789abcdef.jpg",
    ] {
        let raw = format!("GET {path} HTTP/1.1\r\n\r\n");
        let head = parse_request_head(raw.as_bytes()).unwrap().unwrap();
        assert_eq!(head.file_name(), None, "{path}");
    }
}

#[test]
fn content_types_by_extension() {
    assert_eq!(content_type("0123456789abcdef.jpg"), "image/jpeg");
    assert_eq!(content_type("0123456789abcdef.png"), "image/png");
    assert_eq!(content_type("0123456789abcdef.gif"), "image/gif");
    assert_eq!(content_type("0123456789abcdef.webp"), "image/webp");
}

#[test]
fn url_formatting() {
    assert_eq!(
        http_url(4321, "0123456789abcdef.jpg"),
        "http://127.0.0.1:4321/0123456789abcdef.jpg"
    );
}

/// Read exactly one HTTP/1.1 response (head + `Content-Length` body) from
/// a keep-alive connection, returning the head and body separately.
fn read_response(stream: &mut std::net::TcpStream) -> (String, Vec<u8>) {
    use std::io::Read;
    let mut raw: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    // Head terminator ends with the previous two bytes being \r\n.
    while !(raw.len() >= 4 && &raw[raw.len() - 4..] == b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        raw.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&raw).into_owned();
    let len: usize = head
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")?
                .trim()
                .parse()
                .ok()
        })
        .unwrap_or(0);
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).unwrap();
    (head, body)
}

/// End-to-end: real server, real socket. 200 for a cached-shaped file,
/// 404 for traversal / missing, 400 for a wrong method — over both
/// close and keep-alive connections.
#[test]
fn server_serves_cache_dir_only() {
    let dir = std::env::temp_dir().join(format!(
        "exviewer-http-test-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0123456789abcdef.png"), b"PNGDATA").unwrap();
    std::fs::write(dir.join("secret.txt"), b"TOPSECRET").unwrap();

    let port = spawn_server(dir.clone(), LocalImages::default()).unwrap();

    // One request per connection, Connection: close → the server closes
    // after responding (read_to_end terminates).
    let get_close = |path: &str| {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        use std::io::{Read, Write};
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes(),
            )
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        String::from_utf8_lossy(&response).into_owned()
    };

    let ok = get_close("/0123456789abcdef.png");
    assert!(ok.starts_with("HTTP/1.1 200 OK"), "{ok}");
    assert!(ok.contains("Content-Type: image/png"), "{ok}");
    assert!(ok.contains("Connection: close"), "{ok}");
    assert!(
        ok.contains("Cache-Control: public, max-age=31536000, immutable"),
        "{ok}"
    );
    assert!(ok.ends_with("PNGDATA"), "{ok}");

    assert!(get_close("/0123456789abcdef.jpg").starts_with("HTTP/1.1 404"));
    assert!(get_close("/../secret.txt").starts_with("HTTP/1.1 404"));
    assert!(get_close("/secret.txt").starts_with("HTTP/1.1 404")); // valid file, wrong shape
    assert!(get_close("/%2e%2e/secret.txt").starts_with("HTTP/1.1 404"));

    // Wrong method → 400 even for a real file.
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    use std::io::Write;
    stream
        .write_all(b"POST /0123456789abcdef.png HTTP/1.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let (head, _) = read_response(&mut stream);
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");

    std::fs::remove_dir_all(&dir).ok();
}

/// Keep-alive: several sequential requests on ONE connection, each
/// answered with Content-Length + Connection: keep-alive; pipelined
/// requests in a single write are answered in order too.
#[test]
fn server_keeps_connections_alive() {
    let dir = std::env::temp_dir().join(format!(
        "exviewer-http-ka-test-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0123456789abcdef.png"), b"PNGDATA").unwrap();
    std::fs::write(dir.join("ffffffffffffffff.jpg"), b"JPEGDATA").unwrap();

    let port = spawn_server(dir.clone(), LocalImages::default()).unwrap();
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    use std::io::Write;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();

    // Two pipelined requests in one write.
    stream
        .write_all(
            b"GET /0123456789abcdef.png HTTP/1.1\r\nHost: x\r\n\r\n\
              GET /ffffffffffffffff.jpg HTTP/1.1\r\nHost: x\r\n\r\n",
        )
        .unwrap();
    let (head1, body1) = read_response(&mut stream);
    assert!(head1.starts_with("HTTP/1.1 200 OK"), "{head1}");
    assert!(head1.contains("Connection: keep-alive"), "{head1}");
    assert_eq!(head1.matches("Content-Length: 7").count(), 1, "{head1}");
    assert_eq!(body1, b"PNGDATA");
    let (head2, body2) = read_response(&mut stream);
    assert!(head2.starts_with("HTTP/1.1 200 OK"), "{head2}");
    assert!(head2.contains("Connection: keep-alive"), "{head2}");
    assert_eq!(body2, b"JPEGDATA");

    // An error must honor keep-alive too, so the next image can reuse the socket.
    stream
        .write_all(b"GET /missing.png HTTP/1.1\r\n\r\n")
        .unwrap();
    let (missing_head, _) = read_response(&mut stream);
    assert!(missing_head.starts_with("HTTP/1.1 404"));
    assert!(missing_head.contains("Connection: keep-alive"));

    // A third request on the same (now warm) connection, then a close.
    stream
        .write_all(b"GET /0123456789abcdef.png HTTP/1.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let (head3, body3) = read_response(&mut stream);
    assert!(head3.contains("Connection: close"), "{head3}");
    assert_eq!(body3, b"PNGDATA");
    // Server shut the connection down after the close response.
    use std::io::Read;
    let mut eof = [0u8; 1];
    assert_eq!(stream.read(&mut eof).unwrap(), 0);

    std::fs::remove_dir_all(&dir).ok();
}
