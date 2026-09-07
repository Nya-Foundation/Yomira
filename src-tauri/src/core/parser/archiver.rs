//! Port of `ArchiveParser.java` — the `archiver.php` page.
//!
//! The page carries one `<form action="archiver.php?...&or=..." method="post">`
//! per resolution ("org"/"res"), each with a hidden `dltype` input and a
//! `dlcheck` submit button, plus the "Download Cost" / "Estimated Size"
//! snippets around it. Posting the form returns a page with
//! `document.location = "..."`; following it yields the
//! "Click Here To Start Downloading" link (see
//! [`parse_archiver_download_link`]).

use std::sync::LazyLock;

use regex::Regex;

/// One archiver form (original or resample resolution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiverForm {
    /// Form `action`, entity-unescaped — this is the POST target and already
    /// carries the hidden `or` token.
    pub action: String,
    /// Hidden `dltype` value: "org" or "res".
    pub dltype: String,
    /// `dlcheck` submit value, posted back verbatim.
    pub dlcheck: String,
    /// "Download Cost" text, e.g. "Free!" or "1 GP + 1 Credit".
    pub cost: String,
    /// "Estimated Size" text, e.g. "18.46 MiB".
    pub size: String,
}

/// Everything the archiver dialog needs (port of `ArchiverData`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiverInfo {
    pub original: Option<ArchiverForm>,
    pub resample: Option<ArchiverForm>,
    /// Funds line when present (e-hentai shows GP/credit balance text).
    pub funds: Option<String>,
}

impl ArchiverInfo {
    /// The form matching a resolution ("res"/"org"), like the original
    /// dialog's two download buttons.
    pub fn form_for(&self, resolution: &str) -> Option<&ArchiverForm> {
        match resolution {
            "org" => self.original.as_ref().or(self.resample.as_ref()),
            "res" => self.resample.as_ref().or(self.original.as_ref()),
            _ => None,
        }
    }
}

/// `dltype` forms only — the `hathdl_form` (H@H downloader) is skipped.
static PATTERN_FORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<form\b[^>]*action="([^"]+)"[^>]*>(.*?)</form>"#).unwrap());
static PATTERN_DLTYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name="dltype"[^>]*value="([^"]+)""#).unwrap());
static PATTERN_DLCHECK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name="dlcheck"[^>]*value="([^"]+)""#).unwrap());
static PATTERN_COST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)Download\s*Cost:\s*(?:&nbsp;)?\s*<strong>([^<]+)</strong>").unwrap()
});
static PATTERN_ESTIMATED_SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)Estimated\s*Size:\s*(?:&nbsp;)?\s*<strong>([^<]+)</strong>").unwrap()
});
static PATTERN_FUNDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)>([^<>]*(?:GP|Credits)[^<>]*(?:GP|Credits)[^<>]*)<").unwrap()
});
static PATTERN_DOWNLOAD_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]+)">Click Here To Start Downloading"#).unwrap());

/// Window around each form the cost/size snippets are searched in (they sit
/// in sibling nodes, not inside the form itself).
const FORM_CONTEXT: usize = 400;

/// Parse the `archiver.php` GET page (port of `ArchiveParser.parseArchiver`;
/// the original walks fragile jsoup child indices, this matches the labeled
/// snippets directly).
pub fn parse_archiver(body: &str) -> ArchiverInfo {
    let mut info = ArchiverInfo::default();
    for m in PATTERN_FORM.captures_iter(body) {
        let inner = &m[2];
        let Some(dltype) = PATTERN_DLTYPE.captures(inner) else {
            continue; // not an org/res form (e.g. the H@H downloader form)
        };
        // Cost precedes the form, estimated size follows it; both live in
        // the same column div, so a small window around the form suffices.
        let start = m.get(0).map(|g| g.start()).unwrap_or(0);
        let end = m.get(0).map(|g| g.end()).unwrap_or(0);
        let before = &body[start.saturating_sub(FORM_CONTEXT)..start];
        let after = &body[end..(end + FORM_CONTEXT).min(body.len())];
        let form = ArchiverForm {
            action: super::unescape_xml(m[1].trim()),
            dltype: dltype[1].trim().to_string(),
            dlcheck: PATTERN_DLCHECK
                .captures(inner)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default(),
            cost: PATTERN_COST
                .captures(before)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default(),
            size: PATTERN_ESTIMATED_SIZE
                .captures(after)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default(),
        };
        match form.dltype.as_str() {
            "org" => info.original = Some(form),
            "res" => info.resample = Some(form),
            _ => {}
        }
    }
    info.funds = PATTERN_FUNDS
        .captures(body)
        .map(|c| c[1].trim().to_string());
    info
}

/// The temporary archive link on the page behind `document.location`
/// (port of `ArchiveParser.parseArchiverDownloadUrl`).
pub fn parse_archiver_download_link(body: &str) -> Option<String> {
    PATTERN_DOWNLOAD_LINK
        .captures(body)
        .map(|m| super::unescape_xml(m[1].trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of the original repo's `ArchiveDownloadDialog.html`
    /// fixture (exhentai archiver.php page).
    const ARCHIVER_PAGE: &str = r##"
<div id="db">
    <h1 style="font-size:10pt; font-weight:bold">COSPLAYTALE NAKO MASHMAID</h1>
    <div style="position:relative; width:370px; margin:4px auto 4px; padding-top:3px">
        <div style="width:180px; float:left">
            <div style="text-align:center; margin-top:4px">Download Cost: &nbsp;
                <strong>Free!</strong></div>
            <form action="https://exhentai.org/archiver.php?gid=2881867&amp;token=b5727f74ec&amp;or=475687-4d0aae5d32a448278ccdf0e8ac7bd32578e76479"
                  method="post">
                <input type="hidden" name="dltype" value="org"/>
                <div style="margin:3px auto"><input type="submit" name="dlcheck"
                                                    value="Download Original Archive"
                                                    style="width:180px"/></div>
            </form>
            <p>Estimated Size: &nbsp; <strong>18.46 MiB</strong></p>
        </div>
        <div style="width:180px; float:right">
            <div style="text-align:center; margin-top:4px">Download Cost: &nbsp;
                <strong>Free!</strong></div>
            <form action="https://exhentai.org/archiver.php?gid=2881867&amp;token=b5727f74ec&amp;or=475687-4d0aae5d32a448278ccdf0e8ac7bd32578e76479"
                  method="post">
                <input type="hidden" name="dltype" value="res"/>
                <div style="margin:3px auto"><input type="submit" name="dlcheck"
                                                    value="Download Resample Archive"
                                                    style="width:180px"/></div>
            </form>
            <p>Estimated Size: &nbsp; <strong>2.17 MiB</strong></p>
        </div>
        <div style="clear:both"></div>
    </div>
    <div style="position:relative; margin:4px auto 4px; padding-top:3px">
        <p style="font-weight:bold">H@H Downloader</p>
        <form id="hathdl_form"
              action="https://exhentai.org/archiver.php?gid=2881867&amp;token=b5727f74ec&amp;or=475687-4d0aae5d32a448278ccdf0e8ac7bd32578e76479"
              method="post"><input type="hidden" id="hathdl_xres" name="hathdl_xres" value=""/>
        </form>
    </div>
</div>
"##;

    const DOWNLOAD_PAGE: &str = r#"
<html><body>
<a href="/archive/2881867/4d0aae5d32a448278ccdf0e8ac7bd32578e76479/475687">Click Here To Start Downloading</a>
</body></html>
"#;

    #[test]
    fn parses_archiver_forms() {
        let info = parse_archiver(ARCHIVER_PAGE);
        // Resolution lookup with fallback, like the dialog's two buttons.
        assert_eq!(info.form_for("org").unwrap().dltype, "org");
        assert_eq!(info.form_for("res").unwrap().dltype, "res");
        assert!(info.form_for("bogus").is_none());

        let original = info.original.as_ref().unwrap();
        assert_eq!(
            original.action,
            "https://exhentai.org/archiver.php?gid=2881867&token=b5727f74ec&or=475687-4d0aae5d32a448278ccdf0e8ac7bd32578e76479"
        );
        assert_eq!(original.dltype, "org");
        assert_eq!(original.dlcheck, "Download Original Archive");
        assert_eq!(original.cost, "Free!");
        assert_eq!(original.size, "18.46 MiB");

        let resample = info.resample.as_ref().unwrap();
        assert_eq!(resample.dltype, "res");
        assert_eq!(resample.dlcheck, "Download Resample Archive");
        assert_eq!(resample.size, "2.17 MiB");
        // Same action for both forms.
        assert_eq!(resample.action, original.action);
    }

    #[test]
    fn skips_hathdl_form_and_missing_page() {
        // The H@H form has no dltype and must not be picked up.
        let body = r#"<form id="hathdl_form" action="archiver.php?or=x" method="post">
            <input type="hidden" name="hathdl_xres" value=""/></form>"#;
        let info = parse_archiver(body);
        assert!(info.original.is_none());
        assert!(info.resample.is_none());

        let empty = parse_archiver("<html><body>Nothing here</body></html>");
        assert!(empty.original.is_none());
        assert!(empty.funds.is_none());
    }

    #[test]
    fn parses_gp_cost_and_funds() {
        let body = r#"<div>Current Funds: <strong>You have 12 GP and 340 Credits.</strong></div>
            <div>Download Cost: &nbsp;<strong>1 GP + 1 Credit</strong></div>
            <form action="archiver.php?gid=1&token=t&or=k" method="post">
              <input type="hidden" name="dltype" value="org"/>
              <input type="submit" name="dlcheck" value="Download Original Archive"/>
            </form>"#;
        let info = parse_archiver(body);
        assert_eq!(
            info.funds.as_deref(),
            Some("You have 12 GP and 340 Credits.")
        );
        assert_eq!(info.original.unwrap().cost, "1 GP + 1 Credit");
    }

    #[test]
    fn parses_download_link() {
        assert_eq!(
            parse_archiver_download_link(DOWNLOAD_PAGE),
            Some("/archive/2881867/4d0aae5d32a448278ccdf0e8ac7bd32578e76479/475687".to_string())
        );
        assert_eq!(parse_archiver_download_link("<html>no link</html>"), None);
    }
}
