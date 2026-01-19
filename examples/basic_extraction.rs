//! Basic extraction example.
//!
//! This example shows how to extract structured data from a web page.
//!
//! Run with: `REFYNE_API_KEY=your-key cargo run --example basic_extraction`

use refyne::{Client, ExtractRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), refyne::Error> {
    // Create a client with your API key
    let api_key = std::env::var("REFYNE_API_KEY").expect("REFYNE_API_KEY must be set");
    let client = Client::builder(api_key).build()?;

    // Define the schema for the data you want to extract
    let schema = json!({
        "title": "string",
        "description": "string",
        "price": {
            "amount": "number",
            "currency": "string"
        }
    });

    // Extract data from a URL
    let result = client
        .extract(ExtractRequest {
            url: "https://example.com/product".into(),
            schema,
            ..Default::default()
        })
        .await?;

    println!("Extracted data: {:#?}", result.data);

    // Usage information is always available
    println!(
        "Tokens used: {} input, {} output",
        result.usage.input_tokens, result.usage.output_tokens
    );
    println!("Cost: ${:.6}", result.usage.cost_usd);

    Ok(())
}
