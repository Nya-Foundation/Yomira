//! Port of the news.php parsing (the original wraps the raw page in
//! `EhNewsDetail.webData` and renders it in a WebView; the desktop port
//! exposes structured items instead).
//!
//! news.php renders each post as
//! ```html
//! <div class="newstable">
//!   <div class="newstitle"><a href="...">Title</a></div>
//!   <div class="newsdate">Posted Wednesday, 2nd of November 2022, 10:09 UTC</div>
//!   <div class="newstext">...html...</div>
//! </div>
//! ```
//! inside `<td class="nd">` cells of `#nt`. The parser never fails: markup
//! drift or the anonymous login redirect just yield fewer/no items.

use regex::Regex;
use scraper::{Html, Selector};

use crate::core::models::NewsItem;

use super::selector;

/// Extract a stable id from a post link (`showtopic=<id>`), if present.
fn id_from_link(link: &str) -> Option<i64> {
    static TOPIC: &str = r"showtopic=(\d+)";
    let re = Regex::new(TOPIC).expect("static regex must compile");
    re.captures(link)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Sanitize post HTML into plain text: `<br>` becomes a newline, block
/// boundaries (`</p>`, `<p>`, `<div>`, `<li>`) become (double) newlines,
/// every other tag is stripped and entities unescaped. Runs of blank lines
/// collapse to one.
fn sanitize_html(html: &str) -> String {
    static BR_RUN: &str = r"(?i)(?:<br\s*/?>\s*){2,}";
    static BR: &str = r"(?i)<br\s*/?>";
    static BLOCK_OPEN: &str = r"(?i)<(p|div|h[1-6]|table|tr|ul|ol)\b[^>]*>";
    static BLOCK_CLOSE: &str = r"(?i)</(p|div|h[1-6]|table|tr|ul|ol)>";
    static LI: &str = r"(?i)<li\b[^>]*>";
    static ANY_TAG: &str = r"(?s)<[^>]*>";
    /// Sentinels: paragraph (hard) break and soft line break; both are
    /// private-use/control chars that never occur in page text.
    const HARD_BREAK: char = '\u{e000}';
    const SOFT_BREAK: char = '\u{e001}';

    let re_br_run = Regex::new(BR_RUN).expect("static regex must compile");
    let re_br = Regex::new(BR).expect("static regex must compile");
    let re_open = Regex::new(BLOCK_OPEN).expect("static regex must compile");
    let re_close = Regex::new(BLOCK_CLOSE).expect("static regex must compile");
    let re_li = Regex::new(LI).expect("static regex must compile");
    let re_tag = Regex::new(ANY_TAG).expect("static regex must compile");

    let hard = HARD_BREAK.to_string();
    let soft = SOFT_BREAK.to_string();
    let text = re_br_run.replace_all(html, &hard);
    let text = re_close.replace_all(&text, &hard);
    let text = re_open.replace_all(&text, &hard);
    let text = re_li.replace_all(&text, format!("{soft}• "));
    let text = re_br.replace_all(&text, &soft);
    let text = re_tag.replace_all(&text, "");
    let text = super::unescape_xml(&text);

    // Split into paragraphs at the hard breaks. Inside a paragraph, markup
    // line-wrapping joins with a space while <br> keeps a newline.
    text.split(HARD_BREAK)
        .map(|paragraph| {
            paragraph
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .split(SOFT_BREAK)
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Collapse interior whitespace (the markup wraps lines mid-sentence).
fn normalize_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a news.php page into its posts, newest first as on the page.
pub fn parse_news(body: &str) -> Vec<NewsItem> {
    let document = Html::parse_document(body);
    let items = Selector::parse(".newstable").expect("static selector must compile");
    let title_sel = selector(".newstitle a");
    let date_sel = selector(".newsdate");
    let text_sel = selector(".newstext");

    let mut out = Vec::new();
    for item in document.select(&items) {
        let (title, link) = item
            .select(&title_sel)
            .next()
            .map(|a| {
                (
                    normalize_space(&a.text().collect::<String>()),
                    a.attr("href").unwrap_or_default().to_string(),
                )
            })
            .unwrap_or_default();
        let date = item
            .select(&date_sel)
            .next()
            .map(|d| {
                normalize_space(&d.text().collect::<String>())
                    .trim_start_matches("Posted")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();
        let body_text = item
            .select(&text_sel)
            .next()
            .map(|t| sanitize_html(&t.inner_html()))
            .unwrap_or_default();
        if title.is_empty() && body_text.is_empty() {
            continue;
        }
        out.push(NewsItem {
            id: id_from_link(&link),
            title,
            date,
            body: body_text,
            link,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_converts_breaks_and_strips_tags() {
        let html = "<p>First para</p><p>Second<br>line</p><ul><li>one</li><li>two</li></ul>";
        let text = sanitize_html(html);
        assert_eq!(text, "First para\n\nSecond\nline\n\n• one\n• two");
    }

    #[test]
    fn sanitize_unescapes_entities_and_collapses_blanks() {
        // A run of <br>s is a paragraph break; single <br>s join lines.
        let html = "a &amp; b<br><br><br>c<br>\n\n\n<br>d";
        assert_eq!(sanitize_html(html), "a & b\n\nc\n\nd");
    }

    #[test]
    fn id_from_topic_link() {
        assert_eq!(
            id_from_link("https://forums.e-hentai.org/index.php?showtopic=261743"),
            Some(261743)
        );
        assert_eq!(id_from_link("https://e-hentai.org/news.php"), None);
    }
}
