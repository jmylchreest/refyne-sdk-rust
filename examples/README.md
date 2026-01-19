# Refyne Rust SDK Examples

This directory contains example code demonstrating how to use the Refyne Rust SDK.

## Prerequisites

- Rust 1.70+
- A valid Refyne API key

## Environment Setup

Set the required environment variables:

```bash
export REFYNE_API_KEY="your_api_key_here"
export REFYNE_BASE_URL="https://api.refyne.uk"  # Optional, defaults to production
```

## Examples

### Full Demo (`full_demo.rs`)

A comprehensive demo that tests all major SDK functionality:
- Usage/subscription information retrieval
- Website analysis (structure detection)
- Single page extraction
- Crawl job creation and monitoring
- Job result retrieval

**Run with:**

```bash
cargo run --example full_demo
```

### Minimal Example (`minimal.rs`)

A simple example showing basic extraction:

```bash
cargo run --example minimal
```

## Notes

- The demo uses `colored` and `indicatif` crates for terminal formatting
- All API calls are async using `tokio` runtime
- Error handling uses the `refyne::Error` type
- The client uses a builder pattern for configuration
