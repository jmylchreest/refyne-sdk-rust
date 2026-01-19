//! Full SDK Demo - Tests all major functionality
//!
//! Run with: cargo run --example full_demo

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use refyne::{
    AnalyzeRequest, Client, CrawlOptions, CrawlRequest, ExtractRequest, JobStatus,
    MAX_KNOWN_API_VERSION, MIN_API_VERSION, SDK_VERSION,
};
use serde_json::Value;
use std::time::Duration;

const API_KEY: &str = "YOUR_API_KEY";
const BASE_URL: &str = "http://localhost:8080";
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
    info("Base URL", BASE_URL);
    info(
        "API Key",
        &format!("{}...{}", &API_KEY[..10], &API_KEY[API_KEY.len() - 4..]),
    );
    info("Timeout", "30s");
    info("Max Retries", "3");
    info("Cache", "Enabled (in-memory)");

    // Create client
    let client = Client::builder(API_KEY).base_url(BASE_URL).build()?;

    // ========== Subscription Info ==========
    header("Subscription Information");

    let pb = spinner("Fetching subscription details...");
    let usage = client.get_usage().await?;
    pb.finish_and_clear();
    success("Subscription details retrieved");

    info("Total Jobs", &usage.total_jobs.to_string());
    info("Total Charged", &format!("${:.2} USD", usage.total_charged_usd));
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
        })
        .await
    {
        Ok(analysis) => {
            pb.finish_and_clear();
            success("Website analysis complete");
            info("Suggested Schema", "");
            print_json(&analysis.suggested_schema);

            if !analysis.follow_patterns.is_empty() {
                info("Follow Patterns", &analysis.follow_patterns.join(", "));
            }
            analysis.suggested_schema
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
            if let Some(ref usage) = result.usage {
                info(
                    "Tokens",
                    &format!("{} in / {} out", usage.input_tokens, usage.output_tokens),
                );
                info("Cost", &format!("${:.6}", usage.cost_usd));
            }
            if let Some(ref metadata) = result.metadata {
                info("Model", &format!("{}/{}", metadata.provider, metadata.model));
                info(
                    "Duration",
                    &format!(
                        "{}ms fetch + {}ms extract",
                        metadata.fetch_duration_ms, metadata.extract_duration_ms
                    ),
                );
            }

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
                max_urls: Some(5),
                max_depth: Some(1),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await
    {
        Ok(result) => {
            pb.finish_and_clear();
            success("Crawl job started");
            info("Job ID", &result.job_id);
            info("Status", &format!("{:?}", result.status));
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
    let mut page_count = 0u32;
    let poll_interval = Duration::from_secs(2);

    loop {
        let job = client.get_job(&job_id).await?;

        let status = format!("{:?}", job.status);
        if status != last_status {
            println!(
                "  {} Status: {}",
                "→".cyan(),
                status.bold()
            );
            last_status = status;
        }

        if job.page_count > page_count {
            let new_pages = job.page_count - page_count;
            for i in 0..new_pages {
                println!(
                    "  {} Page {} extracted",
                    "✔".green(),
                    page_count + i + 1
                );
            }
            page_count = job.page_count;
        }

        match job.status {
            JobStatus::Completed => {
                success(&format!("Crawl completed - {} pages processed", job.page_count));
                break;
            }
            JobStatus::Failed => {
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
    info("Type", &job.job_type);
    info("Status", &format!("{:?}", job.status));
    info("URL", &job.url);
    info("Pages Processed", &job.page_count.to_string());
    info(
        "Tokens",
        &format!("{} in / {} out", job.token_usage_input, job.token_usage_output),
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
    if let Some(ref result_data) = results.results {
        if !result_data.is_empty() {
            info("Total Results", &result_data.len().to_string());
            println!();
            print_json(&serde_json::to_value(result_data).unwrap());
        } else {
            warn("No results available");
        }
    } else {
        warn("No results available");
    }

    // ========== Done ==========
    println!();
    println!("{}", " Demo Complete ".on_green().bold());
    println!();

    Ok(())
}
