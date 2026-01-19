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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let err = Error::Api {
            status: 500,
            message: "Internal server error".into(),
            detail: Some("Something went wrong".into()),
        };
        assert!(err.to_string().contains("500"));
        assert!(err.to_string().contains("Internal server error"));
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = Error::RateLimit {
            retry_after: 30,
            message: "Too many requests".into(),
        };
        assert!(err.to_string().contains("30"));
        assert!(err.to_string().contains("Rate limited"));
    }

    #[test]
    fn test_validation_error_display() {
        let mut errors = HashMap::new();
        errors.insert("url".to_string(), vec!["URL is required".to_string()]);
        let err = Error::Validation {
            message: "Invalid input".into(),
            errors,
        };
        assert!(err.to_string().contains("Validation error"));
    }

    #[test]
    fn test_authentication_error_display() {
        let err = Error::Authentication("Invalid API key".into());
        assert!(err.to_string().contains("Authentication failed"));
        assert!(err.to_string().contains("Invalid API key"));
    }

    #[test]
    fn test_forbidden_error_display() {
        let err = Error::Forbidden("Insufficient permissions".into());
        assert!(err.to_string().contains("Access forbidden"));
    }

    #[test]
    fn test_not_found_error_display() {
        let err = Error::NotFound("Job not found".into());
        assert!(err.to_string().contains("Not found"));
    }

    #[test]
    fn test_unsupported_api_version_error_display() {
        let err = Error::UnsupportedApiVersion {
            api_version: "0.5.0".into(),
            min_version: "1.0.0".into(),
            max_known_version: "1.1.0".into(),
        };
        assert!(err.to_string().contains("0.5.0"));
        assert!(err.to_string().contains("1.0.0"));
    }

    #[test]
    fn test_config_error_display() {
        let err = Error::Config("API key is required".into());
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("API key is required"));
    }

    #[test]
    fn test_timeout_error_display() {
        let err = Error::Timeout;
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_error_is_debug() {
        let err = Error::Api {
            status: 404,
            message: "Not found".into(),
            detail: None,
        };
        // Ensure Debug is implemented
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Api"));
    }
}
