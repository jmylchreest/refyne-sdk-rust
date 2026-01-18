//! Error types for the Refyne SDK.

use std::collections::HashMap;
use thiserror::Error;

/// Result type for Refyne operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the Refyne SDK.
#[derive(Error, Debug)]
pub enum Error {
    /// The API returned an error response.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
        /// Additional detail
        detail: Option<String>,
    },

    /// Rate limit exceeded.
    #[error("Rate limited. Retry after {retry_after} seconds")]
    RateLimit {
        /// Seconds to wait before retrying
        retry_after: u64,
        /// Error message
        message: String,
    },

    /// Request validation failed.
    #[error("Validation error: {message}")]
    Validation {
        /// Error message
        message: String,
        /// Field-level errors
        errors: HashMap<String, Vec<String>>,
    },

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Access forbidden.
    #[error("Access forbidden: {0}")]
    Forbidden(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// API version is incompatible with this SDK.
    #[error("Unsupported API version {api_version}. This SDK requires >= {min_version}")]
    UnsupportedApiVersion {
        /// The API version detected
        api_version: String,
        /// Minimum supported version
        min_version: String,
        /// Maximum known version
        max_known_version: String,
    },

    /// Network or HTTP error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Request timeout.
    #[error("Request timed out")]
    Timeout,
}

impl Error {
    /// Create an API error from a response.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();

        // Try to get retry-after header for rate limiting
        let retry_after = response
            .headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        // Try to parse error body
        let body: std::result::Result<ErrorResponse, _> = response.json().await;
        let (message, detail, errors) = match body {
            Ok(err) => (
                err.error.unwrap_or_else(|| "Unknown error".into()),
                err.detail,
                err.errors,
            ),
            Err(_) => ("Unknown error".into(), None, None),
        };

        match status {
            400 => Error::Validation {
                message,
                errors: errors.unwrap_or_default(),
            },
            401 => Error::Authentication(message),
            403 => Error::Forbidden(message),
            404 => Error::NotFound(message),
            429 => Error::RateLimit {
                retry_after,
                message,
            },
            _ => Error::Api {
                status,
                message,
                detail,
            },
        }
    }
}

#[derive(serde::Deserialize)]
struct ErrorResponse {
    error: Option<String>,
    detail: Option<String>,
    errors: Option<HashMap<String, Vec<String>>>,
}
