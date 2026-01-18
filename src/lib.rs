//! Official Rust SDK for the Refyne API.
//!
//! Refyne is an LLM-powered web extraction API that transforms unstructured
//! websites into clean, typed data.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use refyne::{Client, ExtractRequest};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), refyne::Error> {
//!     let client = Client::builder("your-api-key").build()?;
//!
//!     let result = client.extract(ExtractRequest {
//!         url: "https://example.com/product".into(),
//!         schema: json!({
//!             "name": "string",
//!             "price": "number",
//!         }),
//!         ..Default::default()
//!     }).await?;
//!
//!     println!("{:?}", result.data);
//!     Ok(())
//! }
//! ```

mod cache;
mod client;
mod error;
mod types;
mod version;

pub use cache::{Cache, CacheEntry, MemoryCache};
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use types::*;
pub use version::{
    check_api_version_compatibility, compare_versions, parse_version, MAX_KNOWN_API_VERSION,
    MIN_API_VERSION, SDK_VERSION,
};
