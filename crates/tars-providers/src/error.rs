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
            Self::Timeout
        } else if let Some(status) = e.status() {
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                Self::Unauthorized {
                    status: status.as_u16(),
                }
            } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Self::RateLimited
            } else {
                Self::Http(e.to_string())
            }
        } else {
            Self::Http(e.to_string())
        }
    }
}
