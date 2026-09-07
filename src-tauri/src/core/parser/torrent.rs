//! Ported from `client/parser/TorrentParser.java`.
//!
//! The torrent list lives in repeated `<form>` blocks on `gallerytorrents.php`;
//! each block holds the torrent link (`<td colspan="5">...<a href=...>`) and a
//! "Posted:" span. The `?p=` part is stripped from the URL to make the
//! torrent redistributable, exactly like the original.

use std::sync::LazyLock;

use regex::Regex;

use crate::core::models::{TorrentEntry, TorrentInfo};

static PATTERN_TORRENT_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<form\b[^>]*>.*?</form>").unwrap());
static PATTERN_POSTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<span[^>]*>\s*Posted:\s*</span>\s*<span>([^<]+)</span>").unwrap()
});
static PATTERN_TORRENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<td colspan="5">\s*&nbsp;\s*<a href="([^"]+)"[^<]*>([^<]+)</a></td>"#)
        .unwrap()
});

/// Current `gallerytorrents.php` markup: one bordered div per torrent.
static PATTERN_ENTRY_DIV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div style="margin:10px 5px; padding:3px; border:1px solid #C7B5A3">"#)
        .unwrap()
});
static PATTERN_ENTRY_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<td colspan="5">\s*&nbsp;\s*<a href="([^"]+)"[^>]*>([^<]+)</a></td>"#)
        .unwrap()
});
static PATTERN_ENTRY_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)Size:</span>\s*([^<]+?)\s*</td>").unwrap());
static PATTERN_ENTRY_SEEDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)Seeds:</span>\s*([^<]+?)\s*</td>").unwrap());
static PATTERN_ENTRY_PEERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)Peers:</span>\s*([^<]+?)\s*</td>").unwrap());
static PATTERN_ENTRY_DOWNLOADS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)Downloads:</span>\s*([^<]+?)\s*</td>").unwrap());

/// Parse the torrent list page (port of `TorrentParser.parse`).
pub fn parse_torrent_page(body: &str) -> Vec<TorrentInfo> {
    let mut list = Vec::new();
    for block in PATTERN_TORRENT_BLOCK.find_iter(body).map(|m| m.as_str()) {
        let Some(torrent) = PATTERN_TORRENT.captures(block) else {
            continue;
        };
        // Remove ?p= to make torrent redistributable
        let mut url = torrent
            .get(1)
            .map(|m| m.as_str().trim())
            .unwrap_or_default()
            .to_string();
        if let Some(index) = url.find("?p=") {
            url.truncate(index);
        }
        let name = torrent
            .get(2)
            .map(|m| m.as_str().trim())
            .unwrap_or_default()
            .to_string();
        let posted = PATTERN_POSTED
            .captures(block)
            .and_then(|m| m.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        list.push(TorrentInfo { url, name, posted });
    }
    list
}

/// Parse the torrent list of `gallerytorrents.php` in its current markup —
/// one `<div style="margin:10px 5px; ...">` block per torrent carrying the
/// Posted/Size/Seeds/Peers/Downloads cells and the `.torrent` link.
pub fn parse_torrent_entries(body: &str) -> Vec<TorrentEntry> {
    let mut list = Vec::new();
    // Split at each torrent div; the first chunk (page chrome, header) is
    // skipped because it contains no link cell.
    for chunk in PATTERN_ENTRY_DIV.split(body).skip(1) {
        let Some(link) = PATTERN_ENTRY_LINK.captures(chunk) else {
            continue;
        };
        let cell = |re: &Regex| {
            re.captures(chunk)
                .map(|m| m[1].trim().to_string())
                .unwrap_or_default()
        };
        let count = |re: &Regex| {
            re.captures(chunk)
                .map(|m| super::parse_int(&m[1], 0))
                .unwrap_or(0)
        };
        list.push(TorrentEntry {
            url: link[1].trim().to_string(),
            name: link[2].trim().to_string(),
            size: cell(&PATTERN_ENTRY_SIZE),
            seeds: count(&PATTERN_ENTRY_SEEDS),
            peers: count(&PATTERN_ENTRY_PEERS),
            downloads: count(&PATTERN_ENTRY_DOWNLOADS),
        });
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of a live `gallerytorrents.php` block (e-hentai, 2026).
    const LIVE_PAGE: &str = r##"
<div id="torrentinfo">
<div style="height:450px; overflow:auto; margin:auto">
<h1 style="font-size:10pt; font-weight:bold; margin:3px; text-align:center">[kumahachi] Short Collection</h1>
<form method="post" action="https://e-hentai.org/gallerytorrents.php?gid=4168962&amp;t=ce0163b3b1">
<div style="margin:10px 5px; padding:3px; border:1px solid #C7B5A3">
<input type="hidden" name="gtid" value="2209249" />
<table style="width:99%">
<tr>
 <td style="width:180px"><span style="font-weight:bold">Posted:</span> <span>2026-09-04 13:59</span></td>
 <td style="width:150px"><span style="font-weight:bold">Size:</span> 2.20 MiB</td>
 <td></td>
 <td style="width:80px"><span style="font-weight:bold">Seeds:</span> 1</td>
 <td style="width:80px"><span style="font-weight:bold">Peers:</span> 7</td>
 <td style="width:100px; text-align:center"><span style="font-weight:bold">Downloads:</span> 2</td>
</tr>
<tr>
 <td colspan="5"><span style="font-weight:bold">Uploader:</span> Konazumi</td>
 <td rowspan="2" style="width:100px; text-align:center"><input type="submit" name="torrent_info" value="Information" style="width:80px" /></td>
</tr>
<tr>
 <td colspan="5"> &nbsp; <a href="https://ehtracker.org/get/4165398/d8354b9e0e562bdca5fd3d2ec8e77348c01fc1fd.torrent" onclick="document.location='https://ehtracker.org/get/4165398/d8354b9e0e562bdca5fd3d2ec8e77348c01fc1fd.torrent'; return false">[kumahachi] Short Collection.zip</a></td>
</tr>
</table>
</div>
<div style="margin:10px 5px; padding:3px; border:1px solid #C7B5A3">
<table style="width:99%">
<tr>
 <td><span style="font-weight:bold">Posted:</span> <span>2026-09-01 08:00</span></td>
 <td><span style="font-weight:bold">Size:</span> 1.05 GiB</td>
 <td></td>
 <td><span style="font-weight:bold">Seeds:</span> 42</td>
 <td><span style="font-weight:bold">Peers:</span> 0</td>
 <td><span style="font-weight:bold">Downloads:</span> 1,337</td>
</tr>
<tr>
 <td colspan="5"> &nbsp; <a href="https://ehtracker.org/get/4165399/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.torrent">[kumahachi] Short Collection [2].zip</a></td>
</tr>
</table>
</div>
</form>
</div>
</div>
"##;

    #[test]
    fn parses_live_torrent_entries() {
        let entries = parse_torrent_entries(LIVE_PAGE);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].url,
            "https://ehtracker.org/get/4165398/d8354b9e0e562bdca5fd3d2ec8e77348c01fc1fd.torrent"
        );
        assert_eq!(entries[0].name, "[kumahachi] Short Collection.zip");
        assert_eq!(entries[0].size, "2.20 MiB");
        assert_eq!(entries[0].seeds, 1);
        assert_eq!(entries[0].peers, 7);
        assert_eq!(entries[0].downloads, 2);
        // Thousands separator, zero peers.
        assert_eq!(entries[1].size, "1.05 GiB");
        assert_eq!(entries[1].seeds, 42);
        assert_eq!(entries[1].peers, 0);
        assert_eq!(entries[1].downloads, 1337);
    }

    #[test]
    fn no_torrents_page_yields_empty_list() {
        let page = r#"<p style="font-style:italic; text-align:center; padding:10px">There are no torrents for this gallery.</p>"#;
        assert!(parse_torrent_entries(page).is_empty());
    }
}
