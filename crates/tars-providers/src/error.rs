//! Provider error types

use thiserror::Error;

/// Errors returned by provider operations
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Unauthorized (HTTP {status})")]
    Unauthorized { status: u16 },

    #[error("Rate limited")]
    RateLimited,

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    #[error("Feature not supported by this provider")]
    Unsupported,

    #[error("Response parse error: {0}")]
    Parse(String),

    #[error("Request timed out")]
    Timeout,

    #[error("Not yet implemented")]
    NotImplemented,
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            return Self::Timeout;
        }
        if let Some(status) = e.status() {
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                return Self::Unauthorized {
                    status: status.as_u16(),
                };
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Self::RateLimited;
            }
        }
        // Strip the URL before stringifying — reqwest's Display includes the
        // full request URL, which for Gemini carries the API key as a query
        // parameter. Leaking that into a user-visible error string would
        // surface the key in logs and the Tauri IPC error channel.
        Self::Http(e.without_url().to_string())
    }
}
