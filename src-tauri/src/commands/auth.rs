//! Auth commands: cookie-paste sign-in, webview sign-in, sign-out, probes.

use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use crate::app_state::{AppState, SignInOutcome};
use crate::cookies;
use crate::core::error::EhError;
use crate::core::EhCookieStore;

/// Sign-in window label (a second capability file scopes what it may do).
pub const SIGNIN_WINDOW_LABEL: &str = "signin";

/// JS evaluated in the sign-in webview every poll tick: when the forums
/// cookie jar shows the member id, hand the whole `document.cookie` string to
/// the hidden `__cookie_probe` command.
const COOKIE_PROBE_JS: &str = r#"
try {
    var c = document.cookie;
    if (c && /(^|;\s*)ipb_member_id=/.test(c)) {
        window.__TAURI_INTERNALS__.invoke('__cookie_probe', { cookies: c });
    }
} catch (e) { }
"#;

/// Report whether the app holds a session the site still accepts.
///
/// Cookie presence alone lies once the session expires server-side (rotated
/// `igneous`, password change), so a present session is validated live
/// against `favorites.php` and the result cached for
/// [`SESSION_CHECK_TTL`]. Network errors fall back to presence so a flaky
/// connection never signs the user out of the UI.
#[tauri::command]
pub async fn check_sign_in(state: State<'_, AppState>) -> Result<bool, EhError> {
    if !state.cookie_store.has_signed_in() {
        return Ok(false);
    }
    let now = std::time::Instant::now();
    if let Some((valid, at)) = *state
        .session_check
        .lock()
        .unwrap_or_else(|p| p.into_inner())
    {
        if now.duration_since(at) < crate::app_state::SESSION_CHECK_TTL {
            return Ok(valid);
        }
    }
    let valid = match cookies::verify_sign_in(&state.client).await {
        Ok(()) => true,
        // Rejected by the site: the stored session is dead.
        Err(EhError::NotSignedIn) | Err(EhError::Eh(_)) => false,
        // Network-level failure: keep the presence-based state.
        Err(_) => true,
    };
    *state
        .session_check
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some((valid, now));
    Ok(valid)
}

#[tauri::command]
pub async fn sign_in_with_cookies(
    state: State<'_, AppState>,
    cookies: String,
) -> Result<(), EhError> {
    let out = cookies::sign_in_with_cookies(&state.client, &state.cookie_store, &cookies).await;
    match &out {
        Ok(()) => eprintln!("[yomira] sign_in_with_cookies OK"),
        Err(e) => eprintln!(
            "[yomira] sign_in_with_cookies FAILED: kind={} msg={}",
            e.kind(),
            e
        ),
    }
    out?;
    *state
        .session_check
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some((true, std::time::Instant::now()));
    let _ = cookies::persist(&state.cookie_store, &state.cookies_path);
    Ok(())
}

#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> Result<(), EhError> {
    state.cookie_store.clear()?;
    *state
        .session_check
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = None;
    let _ = cookies::persist(&state.cookie_store, &state.cookies_path);
    Ok(())
}

/// Hidden command invoked from the sign-in webview once it holds member
/// cookies. Applies them exactly like `sign_in_with_cookies`, reports the
/// outcome to the awaiting `sign_in_with_webview`, then closes the window.
/// Returning `Result<(), String>` keeps a failed probe from surfacing as an
/// unhandled rejection inside the eval'd snippet.
#[tauri::command]
pub async fn __cookie_probe(
    app: AppHandle,
    state: State<'_, AppState>,
    cookies: String,
) -> Result<(), String> {
    let outcome: SignInOutcome =
        cookies::sign_in_with_cookies(&state.client, &state.cookie_store, &cookies).await;
    if outcome.is_ok() {
        *state
            .session_check
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some((true, std::time::Instant::now()));
        let _ = cookies::persist(&state.cookie_store, &state.cookies_path);
    }
    if let Some(tx) = state
        .signin_tx
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
    {
        let _ = tx.send(outcome.clone());
    }
    if let Some(window) = app.get_webview_window(SIGNIN_WINDOW_LABEL) {
        let _ = window.close();
    }
    outcome.map_err(|e| e.to_string())
}

/// Open the forums login page in a child webview window and wait until the
/// session cookies appear (polled via `__cookie_probe`), the window is
/// closed, or five minutes pass.
#[tauri::command]
pub async fn sign_in_with_webview(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), EhError> {
    if app.get_webview_window(SIGNIN_WINDOW_LABEL).is_some() {
        return Err(EhError::Eh("sign-in is already in progress".into()));
    }

    let (tx, rx) = tokio::sync::oneshot::channel::<SignInOutcome>();
    *state.signin_tx.lock().unwrap_or_else(|p| p.into_inner()) = Some(tx);

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        SIGNIN_WINDOW_LABEL,
        tauri::WebviewUrl::External(
            tauri::Url::parse("https://forums.e-hentai.org/index.php?act=Login")
                .map_err(|e| EhError::Eh(format!("invalid sign-in URL: {e}")))?,
        ),
    )
    .title("Sign in — Yomira")
    .inner_size(1000.0, 800.0)
    .resizable(true)
    .build()
    .map_err(|e| {
        // Registration failed; clear the pending slot.
        if let Some(tx) = state
            .signin_tx
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            let _ = tx.send(Err(EhError::Cancelled));
        }
        EhError::Eh(format!("could not open sign-in window: {e}"))
    })?;

    let app_clone = app.clone();
    let mut poll = tokio::time::interval(Duration::from_secs(2));
    // Poll loop: only returns when the window disappears (user closed it);
    // otherwise it keeps evaluating the probe snippet until `rx` resolves.
    let poll_loop = async {
        loop {
            poll.tick().await;
            if app_clone.get_webview_window(SIGNIN_WINDOW_LABEL).is_none() {
                return Err(EhError::Cancelled);
            }
            if let Some(window) = app_clone.get_webview_window(SIGNIN_WINDOW_LABEL) {
                let _ = window.eval(COOKIE_PROBE_JS);
            }
        }
    };
    let mut poll_loop = Box::pin(poll_loop);

    let mut rx = rx;
    let result = tokio::time::timeout(Duration::from_secs(300), async {
        tokio::select! {
            res = &mut rx => res.unwrap_or(Err(EhError::Cancelled)),
            poll_res = &mut poll_loop => poll_res,
        }
    })
    .await
    .unwrap_or(Err(EhError::Cancelled));

    // Whatever happened, make sure the window is gone and the slot is free.
    if let Some(window) = app.get_webview_window(SIGNIN_WINDOW_LABEL) {
        let _ = window.destroy();
    }
    if let Some(tx) = state
        .signin_tx
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
    {
        let _ = tx.send(Err(EhError::Cancelled));
    }
    result
}
