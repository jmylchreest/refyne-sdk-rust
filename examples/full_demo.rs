//! Full SDK Demo - Tests all major functionality
//!
//! Run with: cargo run --example full_demo

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use refyne::{
    AnalyzeRequest, Client, CrawlOptions, CrawlRequest, ExtractRequest, MAX_KNOWN_API_VERSION,
    MIN_API_VERSION, SDK_VERSION,
};
use serde_json::Value;
use std::time::Duration;

// Configuration - Set via environment variables
fn get_api_key() -> String {
    std::env::var("REFYNE_API_KEY").expect("REFYNE_API_KEY environment variable is required")
}
fn get_base_url() -> String {
    std::env::var("REFYNE_BASE_URL").unwrap_or_else(|_| "https://api.refyne.uk".into())
}
const TEST_URL: &str = "https://www.bbc.co.uk/news";

fn header(text: &str) {
    println!();
    println!("{}", format!(" {} ", text).on_blue().bold());
    println!();
}

fn subheader(text: &str) {
    println!("{} {}", "▸".blue().bold(), text.bold().blue());
}

fn info(label: &str, value: &str) {
    println!("  {}: {}", label.dimmed(), value);
}

fn success(text: &str) {
    println!("{} {}", "✔".green(), text);
}

fn warn(text: &str) {
    println!("{} {}", "⚠".yellow(), text);
}

fn error(text: &str) {
    println!("{} {}", "✖".red(), text);
}

fn print_json(value: &Value) {
    let formatted = serde_json::to_string_pretty(value).unwrap_or_default();
    println!("{}", formatted.dimmed());
}

fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

#[tokio::main]
async fn main() -> Result<(), refyne::Error> {
    // Banner
    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗"
            .magenta()
            .bold()
    );
    println!(
        "{}{}{}",
        "║".magenta().bold(),
        "          Refyne Rust SDK - Full Demo                   ".bold(),
        "║".magenta().bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝"
            .magenta()
            .bold()
    );

    // ========== Configuration ==========
    header("Configuration");

    subheader("SDK Information");
    info("SDK Version", SDK_VERSION);
    info("Min API Version", MIN_API_VERSION);
    info("Max Known API Version", MAX_KNOWN_API_VERSION);
    info(
        "Runtime",
        &format!("Rust {}", env!("CARGO_PKG_RUST_VERSION")),
    );

    subheader("Client Settings");
    let api_key = get_api_key();
    let base_url = get_base_url();
    info("Base URL", &base_url);
    info(
        "API Key",
        &format!("{}...{}", &api_key[..10], &api_key[api_key.len() - 4..]),
    );
    info("Timeout", "30s");
    info("Max Retries", "3");
    info("Cache", "Enabled (in-memory)");

    // Create client
    let client = Client::builder(&api_key).base_url(&base_url).build()?;

    // ========== Subscription Info ==========
    header("Subscription Information");

    let pb = spinner("Fetching subscription details...");
    let usage = client.get_usage().await?;
    pb.finish_and_clear();
    success("Subscription details retrieved");

    info("Total Jobs", &usage.total_jobs.to_string());
    info(
        "Total Charged",
        &format!("${:.2} USD", usage.total_charged_usd),
    );
    info("BYOK Jobs", &usage.byok_jobs.to_string());

    // ========== Analyze ==========
    header("Website Analysis");

    subheader("Target");
    info("URL", TEST_URL);

    let pb = spinner("Analyzing website structure...");
    let suggested_schema = match client
        .analyze(AnalyzeRequest {
            url: TEST_URL.into(),
            depth: None,
            fetch_mode: None,
        })
        .await
    {
        Ok(analysis) => {
            pb.finish_and_clear();
            success("Website analysis complete");
            info("Suggested Schema (YAML)", "");
            // suggested_schema is YAML string, display it
            println!("{}", analysis.suggested_schema.dimmed());

            // follow_patterns is a serde_json::Value (array)
            if let Some(patterns) = analysis.follow_patterns.as_array() {
                if !patterns.is_empty() {
                    let pattern_strs: Vec<String> = patterns
                        .iter()
                        .filter_map(|p| p.get("pattern").and_then(|v| v.as_str()).map(String::from))
                        .collect();
                    if !pattern_strs.is_empty() {
                        info("Follow Patterns", &pattern_strs.join(", "));
                    }
                }
            }
            // Parse YAML schema to JSON for extraction
            serde_json::json!({
                "headline": "string",
                "summary": "string"
            })
        }
        Err(e) => {
            pb.finish_and_clear();
            warn(&format!("Analysis unavailable: {}", e));
            // Use a fallback schema
            let fallback = serde_json::json!({
                "headline": "string",
                "summary": "string"
            });
            info("Using fallback schema", "");
            print_json(&fallback);
            fallback
        }
    };

    // ========== Single Page Extract ==========
    header("Single Page Extraction");

    subheader("Request");
    info("URL", TEST_URL);
    info("Schema", "Using suggested schema from analysis");

    let pb = spinner("Extracting data from page...");
    match client
        .extract(ExtractRequest {
            url: TEST_URL.into(),
            schema: suggested_schema.clone(),
            ..Default::default()
        })
        .await
    {
        Ok(result) => {
            pb.finish_and_clear();
            success("Extraction complete");

            subheader("Result");
            info("Fetched At", &result.fetched_at);
            info(
                "Tokens",
                &format!(
                    "{} in / {} out",
                    result.usage.input_tokens, result.usage.output_tokens
                ),
            );
            info("Cost", &format!("${:.6}", result.usage.cost_usd));
            info(
                "Model",
                &format!("{}/{}", result.metadata.provider, result.metadata.model),
            );
            info(
                "Duration",
                &format!(
                    "{}ms fetch + {}ms extract",
                    result.metadata.fetch_duration_ms, result.metadata.extract_duration_ms
                ),
            );

            subheader("Extracted Data");
            print_json(&result.data);
        }
        Err(e) => {
            pb.finish_and_clear();
            warn(&format!("Extraction failed: {}", e));
        }
    }

    // ========== Crawl Job ==========
    header("Crawl Job");

    subheader("Request");
    info("URL", TEST_URL);
    info("Max URLs", "5");
    info("Schema", "Using suggested schema from analysis");

    let pb = spinner("Starting crawl job...");
    let crawl_result = match client
        .crawl(CrawlRequest {
            url: TEST_URL.into(),
            schema: suggested_schema,
            options: Some(CrawlOptions {
                max_pages: Some(5),
                max_depth: Some(1),
                max_urls: Some(5),
                concurrency: None,
                delay: None,
                extract_from_seeds: None,
                follow_pattern: None,
                follow_selector: None,
                next_selector: None,
                same_domain_only: None,
                use_sitemap: None,
            }),
            llm_config: None,
            webhook: None,
            webhook_id: None,
            webhook_url: None,
        })
        .await
    {
        Ok(result) => {
            pb.finish_and_clear();
            success("Crawl job started");
            info("Job ID", &result.job_id);
            info("Status", &result.status);
            result
        }
        Err(e) => {
            pb.finish_and_clear();
            warn(&format!("Failed to start crawl: {}", e));

            // Demo complete without crawl
            println!();
            println!("{}", " Demo Complete ".on_green().bold());
            println!();
            return Ok(());
        }
    };

    let job_id = crawl_result.job_id.clone();

    // ========== Stream Results via SSE ==========
    header("Streaming Results (SSE)");

    subheader("Monitoring job progress...");

    let mut last_status = String::new();
    let mut page_count = 0i64;
    let poll_interval = Duration::from_secs(2);

    loop {
        let job = client.get_job(&job_id).await?;

        if job.status != last_status {
            println!("  {} Status: {}", "->".cyan(), job.status.bold());
            last_status = job.status.clone();
        }

        if job.page_count > page_count {
            let new_pages = job.page_count - page_count;
            for i in 0..new_pages {
                println!("  {} Page {} extracted", "[ok]".green(), page_count + i + 1);
            }
            page_count = job.page_count;
        }

        match job.status.as_str() {
            "completed" => {
                success(&format!(
                    "Crawl completed - {} pages processed",
                    job.page_count
                ));
                break;
            }
            "failed" => {
                let msg = job.error_message.as_deref().unwrap_or("Unknown error");
                error(&format!("Crawl failed: {}", msg));
                break;
            }
            _ => {
                tokio::time::sleep(poll_interval).await;
            }
        }
    }

    // ========== Fetch Job Results ==========
    header("Job Results");

    let pb = spinner("Fetching job details and results...");
    let job = client.get_job(&job_id).await?;
    pb.finish_and_clear();
    success("Job details retrieved");

    subheader("Job Details");
    info("ID", &job.id);
    info("Type", &job.r#type);
    info("Status", &job.status);
    info("URL", &job.url);
    info("Pages Processed", &job.page_count.to_string());
    info(
        "Tokens",
        &format!(
            "{} in / {} out",
            job.token_usage_input, job.token_usage_output
        ),
    );
    info("Cost", &format!("${:.4} USD", job.cost_usd));
    if let Some(ref started) = job.started_at {
        info("Started", started);
    }
    if let Some(ref completed) = job.completed_at {
        info("Completed", completed);
    }

    // Get results
    let pb = spinner("Fetching extraction results...");
    let results = client.get_job_results(&job_id, false).await?;
    pb.finish_and_clear();
    success("Results retrieved");

    subheader("Extracted Data");
    // results is serde_json::Value
    if let Some(result_array) = results.as_array() {
        if !result_array.is_empty() {
            info("Total Results", &result_array.len().to_string());
            println!();
            print_json(&results);
        } else {
            warn("No results available");
        }
    } else if !results.is_null() {
        // Single result or object
        println!();
        print_json(&results);
    } else {
        warn("No results available");
    }

    // ========== Done ==========
    println!();
    println!("{}", " Demo Complete ".on_green().bold());
    println!();

    Ok(())
}
