//! Error type for the EH client core.
//!
//! Every fallible operation in `core` returns `Result<T, EhError>`. The type
//! implements [`serde::Serialize`] manually so Tauri commands can return it
//! directly to the frontend as `{ kind, message }`.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum EhError {
    /// reqwest-level failure (connect, TLS, body read, timeout, ...).
    /// Wrapped in `Arc` so the error stays `Clone` (image fetches dedup
    /// in-flight requests by sharing the result).
    #[error("network error: {0}")]
    Network(Arc<reqwest::Error>),

    /// The server answered with a non-success HTTP status code.
    #[error("http status {0}: {1}")]
    HttpStatus(u16, String),

    /// HTML/JSON parsing failed.
    #[error("parse error: {0}")]
    Parse(String),

    /// The action requires a signed-in session but no valid cookies exist.
    #[error("not signed in")]
    NotSignedIn,

    /// The image limit (509) page was hit.
    #[error("image limit reached (509)")]
    Image509,

    /// Bandwidth limit exceeded page was hit.
    #[error("bandwidth limit exceeded")]
    BandwidthExceeded,

    /// Cookie store operation failed.
    #[error("cookie error: {0}")]
    Cookie(String),

    /// Database operation failed.
    #[error("database error: {0}")]
    Db(String),

    #[error("This page is not available in the download folder.")]
    PageUnavailable,

    /// The operation was cancelled by the caller.
    #[error("cancelled")]
    Cancelled,

    /// Server-side error message or generic EH error (port of EhException).
    #[error("eh error: {0}")]
    Eh(String),
}

impl EhError {
    /// Stable, camelCase discriminator used as the `kind` field on the wire.
    pub fn kind(&self) -> &'static str {
        match self {
            EhError::Network(_) => "network",
            EhError::HttpStatus(..) => "httpStatus",
            EhError::Parse(_) => "parse",
            EhError::NotSignedIn => "notSignedIn",
            EhError::Image509 => "image509",
            EhError::BandwidthExceeded => "bandwidthExceeded",
            EhError::Cookie(_) => "cookie",
            EhError::Db(_) => "db",
            EhError::PageUnavailable => "pageUnavailable",
            EhError::Cancelled => "cancelled",
            EhError::Eh(_) => "eh",
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl Serialize for EhError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("EhError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.message())?;
        state.end()
    }
}

impl From<reqwest::Error> for EhError {
    fn from(e: reqwest::Error) -> Self {
        EhError::Network(Arc::new(e))
    }
}

impl From<std::io::Error> for EhError {
    fn from(e: std::io::Error) -> Self {
        EhError::Eh(format!("io error: {e}"))
    }
}

impl From<tokio::task::JoinError> for EhError {
    fn from(e: tokio::task::JoinError) -> Self {
        EhError::Db(format!("background task failed: {e}"))
    }
}
