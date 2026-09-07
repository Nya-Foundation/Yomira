//! Port of the `home.php` limits parsing (`EhHomeParser.java` +
//! `HomeParser`): the "Image Limits" homebox paragraph and the
//! Current/Peak/Regen limits table.
//!
//! Unlike the original (which throws ParseException on markup surprises),
//! this port is deliberately defensive — the homepage layout drifts and
//! anonymous sessions get the login redirect — so anything missing just
//! stays at the [`Quota`] defaults.

use regex::Regex;
use scraper::Html;
use std::sync::LazyLock;

use crate::core::models::Quota;

/// `<p>You are currently at <strong>1,234</strong> towards your account
/// limit of <strong>5,000</strong>.</p>` — the current home.php wording
/// (`PATTERN_IMAGE_LIMIT_NEW` in EhHomeParser.java). Whitespace-tolerant:
/// the real markup wraps lines inside the paragraph.
static IMAGE_LIMIT_NEW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<p>\s*You are currently at\s*<strong>\s*(.+?)\s*</strong>\s*towards your account limit of\s*<strong>\s*(.+?)\s*</strong>\s*\.</p>"#,
    )
    .expect("static regex must compile")
});

/// Older wording: "towards a limit of" (`PATTERN_IMAGE_LIMIT`).
static IMAGE_LIMIT_OLD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<p>\s*You are currently at\s*<strong>\s*(\d+)\s*</strong>\s*towards a limit of\s*<strong>\s*(\d+)\s*</strong>\s*\.</p>"#)
        .expect("static regex must compile")
});

fn parse_number(s: &str) -> Option<i64> {
    // "No restrictions" (donor accounts) and stray words have no digits.
    let digits: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.replace(',', "").parse().ok()
}

/// Parse a reset duration from free text: "2 hours 30 minutes", "45 minutes",
/// "1 day", "90 seconds" or a "H:MM[:SS]" clock. Returns seconds.
fn parse_reset_seconds(text: &str) -> Option<i64> {
    static DURATION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(\d+)\s*(days?|hours?|minutes?|mins?|seconds?|secs?)\b")
            .expect("static regex must compile")
    });
    static CLOCK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(\d{1,3}):(\d{2})(?::(\d{2}))?\b").expect("static regex must compile")
    });

    let mut total: i64 = 0;
    let mut matched = false;
    for caps in DURATION.captures_iter(text) {
        let value: i64 = caps[1].parse().ok()?;
        let unit = caps[2].to_ascii_lowercase();
        let factor = if unit.starts_with("day") {
            86_400
        } else if unit.starts_with("hour") {
            3_600
        } else if unit.starts_with("min") {
            60
        } else {
            1
        };
        total += value * factor;
        matched = true;
    }
    if matched {
        return Some(total);
    }
    CLOCK.captures(text).and_then(|c| {
        let h: i64 = c[1].parse().ok()?;
        let m: i64 = c[2].parse().ok()?;
        let s: i64 = c.get(3).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
        Some(h * 3_600 + m * 60 + s)
    })
}

/// Look for a "<keyword> … reset[s] in/after <duration>" phrase and parse
/// its duration into seconds. The keyword is anchored at the start of the
/// phrase so the image-reset text can't satisfy the search reset and vice
/// versa.
fn reset_seconds_for(text: &str, keyword: &str) -> Option<i64> {
    static PHRASE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)\b(image|search)[a-z]*\b[^.<]{0,40}?\breset\b[^.<]{0,10}?\b(?:in|after)\s+([^.<]+)")
            .expect("static regex must compile")
    });
    for caps in PHRASE.captures_iter(text) {
        if caps[1].eq_ignore_ascii_case(keyword) {
            if let Some(seconds) = parse_reset_seconds(caps[2].trim()) {
                return Some(seconds);
            }
        }
    }
    None
}

/// One row of the Current/Peak/Regen limits table.
#[derive(Debug, Default, PartialEq, Eq)]
struct LimitRow {
    current: Option<i64>,
    peak: Option<i64>,
}

/// Read a limits-table row keyed by its first cell ("Image Limits" /
/// "Searches"). Returns `None` when the table or row is absent.
fn parse_limit_table(body: &str, row_key: &str) -> Option<LimitRow> {
    let document = Html::parse_document(body);
    let mut table = None;
    for t in document.select(&super::selector("table")) {
        let header = t.text().collect::<String>();
        if header.contains("Current") && (header.contains("Peak") || header.contains("Regen")) {
            table = Some(t);
            break;
        }
    }
    let table = table?;
    for tr in table.select(&super::selector("tr")) {
        let tds: Vec<String> = tr
            .select(&super::selector("td"))
            .map(|td| td.text().collect::<String>().trim().to_string())
            .collect();
        if tds.len() >= 3 && tds[0].eq_ignore_ascii_case(row_key) {
            return Some(LimitRow {
                current: parse_number(&tds[1]),
                peak: parse_number(&tds[2]),
            });
        }
    }
    None
}

/// Parse `home.php` into a [`Quota`]; never fails.
pub fn parse_quota(body: &str) -> Quota {
    let mut quota = Quota::default();

    // First homebox: the image-limits paragraph (donors get
    // "No restrictions" with a single <strong> instead).
    let document = Html::parse_document(body);
    if let Some(box_html) = document
        .select(&super::selector(".homebox"))
        .next()
        .map(|el| el.inner_html())
    {
        if let Some(caps) = IMAGE_LIMIT_NEW
            .captures(&box_html)
            .or_else(|| IMAGE_LIMIT_OLD.captures(&box_html))
        {
            if caps[1].contains("No restrictions") || caps[2].contains("No restrictions") {
                // Donor account: no image quota.
                quota.images_used = 0;
                quota.images_total = 0;
            } else {
                quota.images_used = parse_number(&caps[1]).unwrap_or(-1);
                quota.images_total = parse_number(&caps[2]).unwrap_or(-1);
            }
        } else if box_html.contains("No restrictions") {
            quota.images_used = 0;
            quota.images_total = 0;
        }
        let box_text = document
            .select(&super::selector(".homebox"))
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();
        if let Some(seconds) = reset_seconds_for(&box_text, "image") {
            quota.image_reset_seconds = Some(seconds);
        }
    }

    // Current/Peak/Regen limits table (absent for anonymous visitors).
    if let Some(images) = parse_limit_table(body, "Image Limits") {
        if quota.images_used == -1 {
            quota.images_used = images.current.unwrap_or(-1);
        }
        if quota.images_total == -1 {
            quota.images_total = images.peak.unwrap_or(-1);
        }
    }
    if let Some(searches) = parse_limit_table(body, "Searches") {
        quota.searches_used = searches.current;
        quota.searches_total = searches.peak;
        let document = Html::parse_document(body);
        let text = document
            .select(&super::selector(".homebox"))
            .map(|el| el.text().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        quota.search_reset_seconds = reset_seconds_for(&text, "search");
    }

    quota
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_home_quota() {
        let body = r#"
        <div class="homebox">
          <p>You are currently at <strong>1,234</strong> towards your account limit of <strong>5,000</strong>.</p>
          <p>You can reset your image quota by spending <strong>30,000</strong> GP.</p>
          <p>Your image limits reset in 2 hours 30 minutes.</p>
          <table>
            <tr><th></th><th>Current</th><th>Peak</th><th>Regen</th></tr>
            <tr><td>Image Limits</td><td>1,234</td><td>2,000</td><td>0/s</td></tr>
            <tr><td>Searches</td><td>3</td><td>9</td><td>0.1/s</td></tr>
          </table>
          <p>Searches reset in 45 minutes.</p>
        </div>"#;
        let quota = parse_quota(body);
        assert_eq!(quota.images_used, 1234);
        assert_eq!(quota.images_total, 5000);
        assert_eq!(quota.image_reset_seconds, Some(9000));
        assert_eq!(quota.searches_used, Some(3));
        assert_eq!(quota.searches_total, Some(9));
        assert_eq!(quota.search_reset_seconds, Some(2700));
    }

    #[test]
    fn parses_legacy_wording() {
        let body = "<div class=\"homebox\"><p>You are currently at <strong>400</strong> towards a limit of <strong>5000</strong>.</p><p>Reset Cost: <strong>25000</strong> GP</p></div>";
        let quota = parse_quota(body);
        assert_eq!(quota.images_used, 400);
        assert_eq!(quota.images_total, 5000);
        assert!(quota.searches_used.is_none());
    }

    #[test]
    fn donor_no_restrictions() {
        let body = "<div class=\"homebox\"><p>You are currently at <strong>No restrictions</strong> towards your account limit of <strong>No restrictions</strong>.</p></div>";
        let quota = parse_quota(body);
        assert_eq!(quota.images_used, 0);
        assert_eq!(quota.images_total, 0);
    }

    #[test]
    fn anonymous_page_yields_empty_quota() {
        let quota = parse_quota("<html><body>Please log on to continue.</body></html>");
        assert_eq!(quota, Quota::default());
    }

    #[test]
    fn reset_duration_formats() {
        assert_eq!(
            parse_reset_seconds("resets in 2 hours 30 minutes"),
            Some(9000)
        );
        assert_eq!(parse_reset_seconds("resets in 45 minutes"), Some(2700));
        assert_eq!(parse_reset_seconds("after 1 day"), Some(86400));
        assert_eq!(parse_reset_seconds("at 14:30"), Some(52200));
        assert_eq!(parse_reset_seconds("nothing here"), None);
    }
}
