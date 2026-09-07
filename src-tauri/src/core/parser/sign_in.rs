//! Ported from `client/parser/SignInParser.java`.
//!
//! Two related helpers:
//! * [`parse_sign_in`] parses the login-result page (username, or the error
//!   message returned by the forums software).
//! * [`detect_signed_in`] answers "does this HTML belong to a logged-in
//!   session" by checking for member-only markers (the original does this
//!   heuristically in the sign-in scenes).

use std::sync::LazyLock;

use regex::Regex;

use crate::core::error::EhError;

static NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<p>You are now logged in as: (.+?)<").unwrap());
static ERROR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:<h4>The error returned was:</h4>\s*<p>(.+?)</p>)|(?:<span class="postcolor">(.+?)</span>)"#,
    )
    .unwrap()
});

/// Parse the sign-in result page (port of `SignInParser.parse`); returns the
/// logged-in display name.
pub fn parse_sign_in(body: &str) -> Result<String, EhError> {
    if let Some(m) = NAME_PATTERN.captures(body) {
        return Ok(m[1].to_string());
    }
    if let Some(m) = ERROR_PATTERN.captures(body) {
        let msg = m
            .get(1)
            .or_else(|| m.get(2))
            .map(|g| g.as_str().to_string())
            .unwrap_or_default();
        return Err(EhError::Eh(msg));
    }
    Err(EhError::Parse("Can't parse sign in".to_string()))
}

/// Heuristic logged-in detector for forum index / EH homepage HTML: look for
/// the member cookie script or account UI markers that only appear for
/// logged-in users.
pub fn detect_signed_in(body: &str) -> bool {
    // The forums inject `ipb.vars['session_id']` / member id for members.
    if body.contains("ipb_member_id") || body.contains("member_id=") {
        return true;
    }
    // EH homepage shows "My Home" / has the logout link for signed-in users.
    if body.contains("You are now logged in as:") {
        return true;
    }
    let upper = body.to_ascii_lowercase();
    upper.contains("act=login&code=03") // "Log out" link target
        || upper.contains("id=\"logout\"")
        || upper.contains(">log out<")
}
