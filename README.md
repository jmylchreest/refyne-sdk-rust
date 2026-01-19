# Refyne Rust SDK

Official Rust SDK for the [Refyne API](https://refyne.uk/docs) - LLM-powered web extraction.

**API Endpoint**: `https://api.refyne.uk` | **Documentation**: [refyne.uk/docs](https://refyne.uk/docs)

[![Crates.io](https://img.shields.io/crates/v/refyne.svg)](https://crates.io/crates/refyne)
[![Documentation](https://docs.rs/refyne/badge.svg)](https://docs.rs/refyne)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
refyne = "0.0"
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
serde_json = "1.0"
```

## Quick Start

```rust
use refyne::{Client, ExtractRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), refyne::Error> {
    let client = Client::builder("your-api-key").build()?;

    let result = client.extract(ExtractRequest {
        url: "https://example.com/product".into(),
        schema: json!({
            "name": "string",
            "price": "number",
        }),
        ..Default::default()
    }).await?;

    println!("{:?}", result.data);
    Ok(())
}
```

## Features

- **Builder Pattern**: Fluent configuration for client setup
- **Cache-Control Aware**: Automatic response caching based on server headers
- **Retry Logic**: Exponential backoff with rate limit handling
- **API Version Checking**: Warns when SDK may be outdated
- **Custom HTTP Client**: Inject your own reqwest client
- **Custom Caching**: Implement the `Cache` trait for custom storage

## Configuration

```rust
use refyne::Client;
use std::time::Duration;

let client = Client::builder("your-api-key")
    .base_url("https://custom.api.example.com")  // Custom API endpoint
    .timeout(Duration::from_secs(60))            // Request timeout
    .max_retries(5)                              // Retry attempts
    .cache_enabled(false)                        // Disable caching
    .user_agent_suffix("MyApp/1.0")              // Custom User-Agent suffix
    .build()?;
```

## API Methods

### Extract Data

```rust
use refyne::{Client, ExtractRequest, FetchMode};

let result = client.extract(ExtractRequest {
    url: "https://example.com".into(),
    schema: json!({"title": "string"}),
    fetch_mode: Some(FetchMode::Dynamic),  // JavaScript rendering
    ..Default::default()
}).await?;
```

### Start a Crawl Job

```rust
use refyne::{Client, CrawlRequest, CrawlOptions};

let job = client.crawl(CrawlRequest {
    url: "https://example.com".into(),
    schema: json!({"title": "string"}),
    options: Some(CrawlOptions {
        max_pages: Some(10),
        max_depth: Some(2),
        ..Default::default()
    }),
    ..Default::default()
}).await?;

println!("Job started: {}", job.job_id);
```

### Monitor Job Status

```rust
let job = client.get_job(&job_id).await?;
println!("Status: {:?}", job.status);

// Get results when complete
let results = client.get_job_results(&job_id, false).await?;
```

### Manage Schemas

```rust
// List all schemas
let schemas = client.list_schemas().await?;

// Create a schema
let schema = client.create_schema(CreateSchemaRequest {
    name: "Product".into(),
    schema_yaml: "name: string\nprice: number".into(),
    description: Some("Product extraction schema".into()),
    category: None,
}).await?;
```

### LLM Configuration (BYOK)

```rust
// List available providers
let providers = client.list_providers().await?;

// Add your own API key
client.upsert_llm_key(UpsertLlmKeyRequest {
    provider: "openai".into(),
    api_key: "sk-...".into(),
    default_model: "gpt-4o".into(),
    base_url: None,
    is_enabled: Some(true),
}).await?;

// Get the fallback chain
let chain = client.get_llm_chain().await?;
```

## Error Handling

```rust
use refyne::Error;

match client.extract(request).await {
    Ok(result) => println!("Success: {:?}", result.data),
    Err(Error::RateLimit { retry_after, .. }) => {
        println!("Rate limited, retry after {} seconds", retry_after);
    }
    Err(Error::Validation { message, errors }) => {
        println!("Validation failed: {}", message);
        for (field, errs) in errors {
            println!("  {}: {:?}", field, errs);
        }
    }
    Err(Error::Authentication(msg)) => {
        println!("Auth failed: {}", msg);
    }
    Err(e) => println!("Error: {}", e),
}
```

## Custom Cache Implementation

```rust
use refyne::{Cache, CacheEntry, Client};
use std::sync::Arc;

struct RedisCache {
    // Your Redis client
}

impl Cache for RedisCache {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        // Fetch from Redis
        None
    }

    fn set(&self, key: &str, entry: CacheEntry) {
        // Store in Redis
    }

    fn delete(&self, key: &str) {
        // Remove from Redis
    }
}

let client = Client::builder("api-key")
    .cache(Arc::new(RedisCache { /* ... */ }))
    .build()?;
```

## Documentation

- [API Documentation](https://docs.refyne.uk)
- [Rust SDK Reference](https://docs.rs/refyne)

## Testing with Demo Site

A demo site is available at [demo.refyne.uk](https://demo.refyne.uk) for testing SDK functionality. The site contains realistic data across multiple content types:

| Endpoint | Content Type | Example Use Case |
|----------|--------------|------------------|
| `https://demo.refyne.uk/products` | Product catalog | Extract prices, descriptions, ratings |
| `https://demo.refyne.uk/jobs` | Job listings | Extract salaries, requirements, companies |
| `https://demo.refyne.uk/blog` | Blog posts | Extract articles, authors, tags |
| `https://demo.refyne.uk/news` | News articles | Extract headlines, sources, timestamps |

Example:

```rust
let result = client.extract(ExtractRequest {
    url: "https://demo.refyne.uk/products/1".into(),
    schema: json!({
        "name": "string",
        "price": "number",
        "description": "string",
        "brand": "string",
        "rating": "number",
    }),
    ..Default::default()
}).await?;
```

## License

MIT License - see [LICENSE](LICENSE) for details.
