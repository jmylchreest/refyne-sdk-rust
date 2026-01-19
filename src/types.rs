//! API types for the Refyne SDK.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request for data extraction.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRequest {
    /// URL to extract data from.
    pub url: String,
    /// Schema defining the data structure to extract.
    pub schema: Value,
    /// Fetch mode: auto, static, or dynamic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_mode: Option<FetchMode>,
    /// Custom LLM configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_config: Option<LlmConfig>,
}

/// Fetch mode for extraction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FetchMode {
    /// Automatic detection.
    Auto,
    /// Static HTML fetch.
    Static,
    /// Dynamic JavaScript rendering.
    Dynamic,
}

/// Response from data extraction.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResponse {
    /// Extracted data matching the schema.
    pub data: Value,
    /// URL that was extracted.
    pub url: String,
    /// Timestamp when the page was fetched.
    pub fetched_at: String,
    /// Token usage information.
    pub usage: Option<TokenUsage>,
    /// Extraction metadata.
    pub metadata: Option<ExtractionMetadata>,
}

/// Token usage information.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    /// Number of input tokens used.
    pub input_tokens: u64,
    /// Number of output tokens used.
    pub output_tokens: u64,
    /// Total USD cost charged.
    pub cost_usd: f64,
    /// Actual LLM cost from provider.
    pub llm_cost_usd: f64,
    /// True if user's own API key was used.
    pub is_byok: bool,
}

/// Extraction metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionMetadata {
    /// Time to fetch the page in milliseconds.
    pub fetch_duration_ms: u64,
    /// Time to extract data in milliseconds.
    pub extract_duration_ms: u64,
    /// Model used for extraction.
    pub model: String,
    /// LLM provider used.
    pub provider: String,
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    /// LLM provider name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// API key for the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Custom base URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Request for starting a crawl job.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrawlRequest {
    /// Seed URL to start crawling from.
    pub url: String,
    /// Schema defining the data structure to extract.
    pub schema: Value,
    /// Crawl options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CrawlOptions>,
    /// Webhook URL for completion notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    /// Custom LLM configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_config: Option<LlmConfig>,
}

/// Options for crawl jobs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrawlOptions {
    /// CSS selector for links to follow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_selector: Option<String>,
    /// Regex pattern for URLs to follow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_pattern: Option<String>,
    /// Maximum crawl depth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// CSS selector for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_selector: Option<String>,
    /// Maximum pages to crawl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_pages: Option<u32>,
    /// Maximum total URLs to process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_urls: Option<u32>,
    /// Delay between requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
    /// Concurrent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,
    /// Only follow same-domain links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_domain_only: Option<bool>,
    /// Extract data from seed URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_from_seeds: Option<bool>,
}

/// Response when a crawl job is created.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlJobCreated {
    /// Unique job identifier.
    pub job_id: String,
    /// Initial job status.
    pub status: JobStatus,
    /// URL to check job status.
    pub status_url: String,
}

/// Job status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is pending.
    Pending,
    /// Job is running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
}

/// Job details.
#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    /// Job ID.
    pub id: String,
    /// Job type.
    #[serde(rename = "type")]
    pub job_type: String,
    /// Current status.
    pub status: JobStatus,
    /// Seed URL.
    pub url: String,
    /// Number of URLs queued.
    pub urls_queued: u32,
    /// Number of pages processed.
    pub page_count: u32,
    /// Input tokens used.
    pub token_usage_input: u64,
    /// Output tokens used.
    pub token_usage_output: u64,
    /// Cost in USD.
    pub cost_usd: f64,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// When the job started.
    pub started_at: Option<String>,
    /// When the job completed.
    pub completed_at: Option<String>,
    /// When the job was created.
    pub created_at: String,
}

/// List of jobs.
#[derive(Debug, Clone, Deserialize)]
pub struct JobList {
    /// List of jobs.
    pub jobs: Vec<Job>,
}

/// Job results.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResults {
    /// Job ID.
    pub job_id: String,
    /// Job status.
    pub status: JobStatus,
    /// Number of pages processed.
    pub page_count: u32,
    /// Array of extraction results.
    pub results: Option<Vec<Value>>,
    /// Merged results object.
    pub merged: Option<Value>,
}

/// Request for website analysis.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnalyzeRequest {
    /// URL to analyze.
    pub url: String,
    /// Analysis depth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// Response from website analysis.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    /// URL that was analyzed.
    pub url: String,
    /// Suggested schema.
    pub suggested_schema: Value,
    /// Suggested URL patterns.
    pub follow_patterns: Vec<String>,
}

/// Schema definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    /// Schema ID.
    pub id: String,
    /// Schema name.
    pub name: String,
    /// Schema description.
    pub description: Option<String>,
    /// Schema definition in YAML.
    pub schema_yaml: String,
    /// Category.
    pub category: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// List of schemas.
#[derive(Debug, Clone, Deserialize)]
pub struct SchemaList {
    /// List of schemas.
    pub schemas: Vec<Schema>,
}

/// Request to create a schema.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSchemaRequest {
    /// Schema name.
    pub name: String,
    /// Schema definition in YAML.
    pub schema_yaml: String,
    /// Schema description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Saved site configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    /// Site ID.
    pub id: String,
    /// Site name.
    pub name: String,
    /// Site URL.
    pub url: String,
    /// Associated schema ID.
    pub schema_id: Option<String>,
    /// Default crawl options.
    pub crawl_options: Option<CrawlOptions>,
    /// Creation timestamp.
    pub created_at: String,
}

/// List of sites.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteList {
    /// List of sites.
    pub sites: Vec<Site>,
}

/// Request to create a site.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSiteRequest {
    /// Site name.
    pub name: String,
    /// Site URL.
    pub url: String,
    /// Associated schema ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    /// Default crawl options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crawl_options: Option<CrawlOptions>,
}

/// API key (without the secret).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    /// Key ID.
    pub id: String,
    /// Key name.
    pub name: String,
    /// Key prefix.
    pub prefix: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Last used timestamp.
    pub last_used_at: Option<String>,
}

/// List of API keys.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyList {
    /// List of keys.
    pub keys: Vec<ApiKey>,
}

/// Newly created API key.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyCreated {
    /// Key ID.
    pub id: String,
    /// Key name.
    pub name: String,
    /// Full API key (only shown once).
    pub key: String,
}

/// Usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageResponse {
    /// Total number of jobs.
    pub total_jobs: u64,
    /// Total USD charged for usage.
    pub total_charged_usd: f64,
    /// Jobs using user's own API keys (not charged).
    pub byok_jobs: u64,
}

/// LLM provider key.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmKey {
    /// Key ID.
    pub id: String,
    /// Provider name.
    pub provider: String,
    /// Default model.
    pub default_model: String,
    /// Custom base URL.
    pub base_url: Option<String>,
    /// Whether enabled.
    pub is_enabled: bool,
    /// Creation timestamp.
    pub created_at: String,
}

/// List of LLM keys.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmKeyList {
    /// List of keys.
    pub keys: Vec<LlmKey>,
}

/// Request to upsert an LLM key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertLlmKeyRequest {
    /// Provider name.
    pub provider: String,
    /// API key.
    pub api_key: String,
    /// Default model.
    pub default_model: String,
    /// Custom base URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Whether enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
}

/// Entry in the LLM fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmChainEntry {
    /// Entry ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Position in chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// Provider name.
    pub provider: String,
    /// Model name.
    pub model: String,
    /// Whether enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
}

/// LLM fallback chain.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmChain {
    /// Chain entries.
    pub chain: Vec<LlmChainEntry>,
}

/// Available model.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    /// Model ID.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// List of models.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelList {
    /// List of models.
    pub models: Vec<Model>,
}

/// Available providers.
#[derive(Debug, Clone, Deserialize)]
pub struct ProvidersResponse {
    /// List of providers.
    pub providers: Vec<String>,
}
