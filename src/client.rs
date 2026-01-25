//! Main Refyne client implementation.

use crate::cache::{create_cache_entry, generate_cache_key, hash_string, Cache, MemoryCache};
use crate::error::{Error, Result};
use crate::types::*;
use crate::version::{build_user_agent, check_api_version_compatibility};
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

/// Calculate exponential backoff with jitter.
fn calculate_backoff(attempt: u32) -> Duration {
    // Exponential backoff: 2^(attempt-1) seconds, capped at 30s
    let base_secs = 2u64.pow(attempt - 1).min(30);
    // Add jitter: random value between 0% and 25% of the base
    let jitter_ms = rand::rng().random_range(0..=(base_secs * 250));
    Duration::from_millis(base_secs * 1000 + jitter_ms)
}

const DEFAULT_BASE_URL: &str = "https://api.refyne.uk";
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Builder for constructing a [`Client`].
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
    cache: Option<Arc<dyn Cache>>,
    cache_enabled: bool,
    user_agent_suffix: Option<String>,
}

impl ClientBuilder {
    /// Create a new client builder with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            cache: None,
            cache_enabled: true,
            user_agent_suffix: None,
        }
    }

    /// Set the API base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum retry attempts.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set a custom cache implementation.
    pub fn cache(mut self, cache: Arc<dyn Cache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Enable or disable caching.
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set a custom User-Agent suffix.
    pub fn user_agent_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.user_agent_suffix = Some(suffix.into());
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client> {
        if self.api_key.is_empty() {
            return Err(Error::Config("API key is required".into()));
        }

        // Warn about insecure connections
        if !self.base_url.starts_with("https://") {
            warn!(
                base_url = %self.base_url,
                "API base URL is not using HTTPS. This is insecure."
            );
        }

        let http_client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(Error::Http)?;

        let cache: Arc<dyn Cache> = self
            .cache
            .unwrap_or_else(|| Arc::new(MemoryCache::default()));

        let user_agent = build_user_agent(self.user_agent_suffix.as_deref());
        let auth_hash = hash_string(&self.api_key);

        Ok(Client {
            api_key: self.api_key,
            base_url: self.base_url,
            http_client,
            cache,
            cache_enabled: self.cache_enabled,
            user_agent,
            max_retries: self.max_retries,
            auth_hash,
            api_version_checked: Arc::new(AtomicBool::new(false)),
        })
    }
}

/// The main Refyne SDK client.
///
/// # Example
///
/// ```rust,no_run
/// use refyne::{Client, ExtractRequest};
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() -> Result<(), refyne::Error> {
///     let client = Client::builder("your-api-key").build()?;
///
///     let result = client.extract(ExtractRequest {
///         url: "https://example.com".into(),
///         schema: json!({"title": "string"}),
///         ..Default::default()
///     }).await?;
///
///     println!("{:?}", result.data);
///     Ok(())
/// }
/// ```
pub struct Client {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
    cache: Arc<dyn Cache>,
    cache_enabled: bool,
    user_agent: String,
    max_retries: u32,
    auth_hash: String,
    api_version_checked: Arc<AtomicBool>,
}

impl Client {
    /// Create a new client builder.
    pub fn builder(api_key: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Access job-related operations.
    pub fn jobs(&self) -> JobsClient<'_> {
        JobsClient { client: self }
    }

    /// Access schema-related operations.
    pub fn schemas(&self) -> SchemasClient<'_> {
        SchemasClient { client: self }
    }

    /// Access site-related operations.
    pub fn sites(&self) -> SitesClient<'_> {
        SitesClient { client: self }
    }

    /// Access API key operations.
    pub fn keys(&self) -> KeysClient<'_> {
        KeysClient { client: self }
    }

    /// Access LLM configuration operations.
    pub fn llm(&self) -> LlmClient<'_> {
        LlmClient { client: self }
    }

    /// Extract structured data from a single web page.
    pub async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse> {
        self.post("/api/v1/extract", &request).await
    }

    /// Start an asynchronous crawl job.
    pub async fn crawl(&self, request: CrawlRequest) -> Result<CrawlJobCreated> {
        self.post("/api/v1/crawl", &request).await
    }

    /// Analyze a website to detect structure and suggest schemas.
    pub async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse> {
        self.post("/api/v1/analyze", &request).await
    }

    /// Get usage statistics for the current billing period.
    pub async fn get_usage(&self) -> Result<GetUsageOutputBody> {
        self.get("/api/v1/usage").await
    }

    // === Jobs ===

    /// List all jobs.
    pub async fn list_jobs(&self, limit: Option<u32>, offset: Option<u32>) -> Result<JobList> {
        let mut path = "/api/v1/jobs".to_string();
        let mut params = vec![];
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        self.get(&path).await
    }

    /// Get a job by ID.
    pub async fn get_job(&self, id: &str) -> Result<Job> {
        self.get_skip_cache(&format!("/api/v1/jobs/{}", id)).await
    }

    /// Get job results.
    pub async fn get_job_results(&self, id: &str, merge: bool) -> Result<JobResults> {
        let path = if merge {
            format!("/api/v1/jobs/{}/results?merge=true", id)
        } else {
            format!("/api/v1/jobs/{}/results", id)
        };
        self.get_skip_cache(&path).await
    }

    // === Schemas ===

    /// List all schemas.
    pub async fn list_schemas(&self) -> Result<SchemaList> {
        self.get("/api/v1/schemas").await
    }

    /// Get a schema by ID.
    pub async fn get_schema(&self, id: &str) -> Result<Schema> {
        self.get(&format!("/api/v1/schemas/{}", id)).await
    }

    /// Create a new schema.
    pub async fn create_schema(&self, request: CreateSchemaRequest) -> Result<Schema> {
        self.post("/api/v1/schemas", &request).await
    }

    /// Update a schema.
    pub async fn update_schema(&self, id: &str, request: CreateSchemaRequest) -> Result<Schema> {
        self.put(&format!("/api/v1/schemas/{}", id), &request).await
    }

    /// Delete a schema.
    pub async fn delete_schema(&self, id: &str) -> Result<()> {
        self.delete(&format!("/api/v1/schemas/{}", id)).await
    }

    // === Sites ===

    /// List all sites.
    pub async fn list_sites(&self) -> Result<SiteList> {
        self.get("/api/v1/sites").await
    }

    /// Get a site by ID.
    pub async fn get_site(&self, id: &str) -> Result<Site> {
        self.get(&format!("/api/v1/sites/{}", id)).await
    }

    /// Create a new site.
    pub async fn create_site(&self, request: CreateSiteRequest) -> Result<Site> {
        self.post("/api/v1/sites", &request).await
    }

    /// Update a site.
    pub async fn update_site(&self, id: &str, request: CreateSiteRequest) -> Result<Site> {
        self.put(&format!("/api/v1/sites/{}", id), &request).await
    }

    /// Delete a site.
    pub async fn delete_site(&self, id: &str) -> Result<()> {
        self.delete(&format!("/api/v1/sites/{}", id)).await
    }

    // === Keys ===

    /// List all API keys.
    pub async fn list_keys(&self) -> Result<ApiKeyList> {
        self.get("/api/v1/keys").await
    }

    /// Create a new API key.
    pub async fn create_key(&self, name: &str) -> Result<ApiKeyCreated> {
        self.post("/api/v1/keys", &serde_json::json!({"name": name}))
            .await
    }

    /// Revoke an API key.
    pub async fn revoke_key(&self, id: &str) -> Result<()> {
        self.delete(&format!("/api/v1/keys/{}", id)).await
    }

    // === LLM ===

    /// List available LLM providers.
    pub async fn list_providers(&self) -> Result<ProvidersResponse> {
        self.get("/api/v1/llm/providers").await
    }

    /// List configured LLM keys.
    pub async fn list_llm_keys(&self) -> Result<LlmKeyList> {
        self.get("/api/v1/llm/keys").await
    }

    /// Add or update an LLM key.
    pub async fn upsert_llm_key(&self, request: UpsertLlmKeyRequest) -> Result<LlmKey> {
        self.put("/api/v1/llm/keys", &request).await
    }

    /// Delete an LLM key.
    pub async fn delete_llm_key(&self, id: &str) -> Result<()> {
        self.delete(&format!("/api/v1/llm/keys/{}", id)).await
    }

    /// Get the LLM fallback chain.
    pub async fn get_llm_chain(&self) -> Result<LlmChain> {
        self.get("/api/v1/llm/chain").await
    }

    /// Set the LLM fallback chain.
    pub async fn set_llm_chain(&self, chain: Vec<LlmChainEntry>) -> Result<()> {
        self.put("/api/v1/llm/chain", &serde_json::json!({"chain": chain}))
            .await
    }

    /// List available models for a provider.
    pub async fn list_models(&self, provider: &str) -> Result<ModelList> {
        self.get(&format!("/api/v1/llm/models/{}", provider)).await
    }

    // === Internal methods ===

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request("GET", path, None::<&()>, false).await
    }

    async fn get_skip_cache<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request("GET", path, None::<&()>, true).await
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request("POST", path, Some(body), false).await
    }

    async fn put<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request("PUT", path, Some(body), false).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .execute_with_retry("DELETE", &url, None::<&()>, 1)
            .await?;

        if !response.status().is_success() {
            return Err(Error::from_response(response).await);
        }

        Ok(())
    }

    async fn request<T, B>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
        skip_cache: bool,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, path);
        let cache_key = generate_cache_key(method, &url, Some(&self.auth_hash));

        // Check cache for GET requests
        if method == "GET" && self.cache_enabled && !skip_cache {
            if let Some(entry) = self.cache.get(&cache_key) {
                return serde_json::from_value(entry.value).map_err(Error::Json);
            }
        }

        let response = self.execute_with_retry(method, &url, body, 1).await?;

        // Check API version on first request
        if !self.api_version_checked.swap(true, Ordering::SeqCst) {
            if let Some(api_version) = response.headers().get("X-API-Version") {
                if let Ok(v) = api_version.to_str() {
                    check_api_version_compatibility(v)?;
                }
            } else {
                warn!("API did not return X-API-Version header");
            }
        }

        if !response.status().is_success() {
            return Err(Error::from_response(response).await);
        }

        // Get cache control header before consuming response
        let cache_control = response
            .headers()
            .get("Cache-Control")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Parse response as Value first for caching, then deserialize
        let value: serde_json::Value = response.json().await.map_err(Error::Http)?;

        // Cache GET responses
        if method == "GET" && self.cache_enabled {
            if let Some(entry) = create_cache_entry(value.clone(), cache_control.as_deref()) {
                self.cache.set(&cache_key, entry);
            }
        }

        serde_json::from_value(value).map_err(Error::Json)
    }

    async fn execute_with_retry<B: serde::Serialize>(
        &self,
        method: &str,
        url: &str,
        body: Option<&B>,
        attempt: u32,
    ) -> Result<reqwest::Response> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent).unwrap());

        let mut req = self.http_client.request(method.parse().unwrap(), url);
        req = req.headers(headers);

        if let Some(b) = body {
            req = req.json(b);
        }

        let response = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() {
                    return Err(Error::Timeout);
                }
                // Retry on network errors
                if attempt <= self.max_retries {
                    let backoff = calculate_backoff(attempt);
                    warn!(
                        error = %e,
                        attempt = attempt,
                        max_retries = self.max_retries,
                        "Network error. Retrying in {:?}",
                        backoff
                    );
                    sleep(backoff).await;
                    return Box::pin(self.execute_with_retry(method, url, body, attempt + 1)).await;
                }
                return Err(Error::Http(e));
            }
        };

        let status = response.status();

        // Handle rate limiting
        if status.as_u16() == 429 && attempt <= self.max_retries {
            let retry_after: u64 = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            warn!(
                retry_after = retry_after,
                attempt = attempt,
                max_retries = self.max_retries,
                "Rate limited. Retrying"
            );
            sleep(Duration::from_secs(retry_after)).await;
            return Box::pin(self.execute_with_retry(method, url, body, attempt + 1)).await;
        }

        // Handle server errors
        if status.is_server_error() && attempt <= self.max_retries {
            let backoff = calculate_backoff(attempt);
            warn!(
                status = %status,
                attempt = attempt,
                max_retries = self.max_retries,
                "Server error. Retrying in {:?}",
                backoff
            );
            sleep(backoff).await;
            return Box::pin(self.execute_with_retry(method, url, body, attempt + 1)).await;
        }

        Ok(response)
    }
}

// =============================================================================
// Sub-clients for organized API access
// =============================================================================

/// Sub-client for job-related operations.
pub struct JobsClient<'a> {
    client: &'a Client,
}

impl<'a> JobsClient<'a> {
    /// List all jobs.
    pub async fn list(&self, limit: Option<u32>, offset: Option<u32>) -> Result<JobList> {
        self.client.list_jobs(limit, offset).await
    }

    /// Get a job by ID.
    pub async fn get(&self, id: &str) -> Result<Job> {
        self.client.get_job(id).await
    }

    /// Get job results.
    pub async fn get_results(&self, id: &str, merge: bool) -> Result<JobResults> {
        self.client.get_job_results(id, merge).await
    }
}

/// Sub-client for schema operations.
pub struct SchemasClient<'a> {
    client: &'a Client,
}

impl<'a> SchemasClient<'a> {
    /// List all schemas.
    pub async fn list(&self) -> Result<SchemaList> {
        self.client.list_schemas().await
    }

    /// Get a schema by ID.
    pub async fn get(&self, id: &str) -> Result<Schema> {
        self.client.get_schema(id).await
    }

    /// Create a new schema.
    pub async fn create(&self, request: CreateSchemaRequest) -> Result<Schema> {
        self.client.create_schema(request).await
    }

    /// Update a schema.
    pub async fn update(&self, id: &str, request: CreateSchemaRequest) -> Result<Schema> {
        self.client.update_schema(id, request).await
    }

    /// Delete a schema.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client.delete_schema(id).await
    }
}

/// Sub-client for site operations.
pub struct SitesClient<'a> {
    client: &'a Client,
}

impl<'a> SitesClient<'a> {
    /// List all saved sites.
    pub async fn list(&self) -> Result<SiteList> {
        self.client.list_sites().await
    }

    /// Get a site by ID.
    pub async fn get(&self, id: &str) -> Result<Site> {
        self.client.get_site(id).await
    }

    /// Create a new site.
    pub async fn create(&self, request: CreateSiteRequest) -> Result<Site> {
        self.client.create_site(request).await
    }

    /// Update a site.
    pub async fn update(&self, id: &str, request: CreateSiteRequest) -> Result<Site> {
        self.client.update_site(id, request).await
    }

    /// Delete a site.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client.delete_site(id).await
    }
}

/// Sub-client for API key operations.
pub struct KeysClient<'a> {
    client: &'a Client,
}

impl<'a> KeysClient<'a> {
    /// List all API keys.
    pub async fn list(&self) -> Result<ApiKeyList> {
        self.client.list_keys().await
    }

    /// Create a new API key.
    pub async fn create(&self, name: &str) -> Result<ApiKeyCreated> {
        self.client.create_key(name).await
    }

    /// Revoke an API key.
    pub async fn revoke(&self, id: &str) -> Result<()> {
        self.client.revoke_key(id).await
    }
}

/// Sub-client for LLM configuration.
pub struct LlmClient<'a> {
    client: &'a Client,
}

impl<'a> LlmClient<'a> {
    /// List available LLM providers.
    pub async fn list_providers(&self) -> Result<ProvidersResponse> {
        self.client.list_providers().await
    }

    /// List available models for a provider.
    pub async fn list_models(&self, provider: &str) -> Result<ModelList> {
        self.client.list_models(provider).await
    }

    /// List configured LLM keys.
    pub async fn list_keys(&self) -> Result<LlmKeyList> {
        self.client.list_llm_keys().await
    }

    /// Add or update an LLM key.
    pub async fn upsert_key(&self, request: UpsertLlmKeyRequest) -> Result<LlmKey> {
        self.client.upsert_llm_key(request).await
    }

    /// Delete an LLM key.
    pub async fn delete_key(&self, id: &str) -> Result<()> {
        self.client.delete_llm_key(id).await
    }

    /// Get the LLM fallback chain.
    pub async fn get_chain(&self) -> Result<LlmChain> {
        self.client.get_llm_chain().await
    }

    /// Set the LLM fallback chain.
    pub async fn set_chain(&self, chain: Vec<LlmChainEntry>) -> Result<()> {
        self.client.set_llm_chain(chain).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder_requires_api_key() {
        let result = ClientBuilder::new("").build();
        assert!(result.is_err());
        if let Err(Error::Config(msg)) = result {
            assert!(msg.contains("API key is required"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_client_builder_default_values() {
        let builder = ClientBuilder::new("test-key");
        assert_eq!(builder.base_url, DEFAULT_BASE_URL);
        assert_eq!(builder.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert_eq!(builder.max_retries, DEFAULT_MAX_RETRIES);
        assert!(builder.cache_enabled);
    }

    #[test]
    fn test_client_builder_custom_base_url() {
        let result = ClientBuilder::new("test-key")
            .base_url("https://custom.api.com/")
            .build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_client_builder_strips_trailing_slash() {
        let result = ClientBuilder::new("test-key")
            .base_url("https://api.example.com/")
            .build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert!(!client.base_url.ends_with('/'));
    }

    #[test]
    fn test_client_builder_custom_timeout() {
        let builder = ClientBuilder::new("test-key").timeout(Duration::from_secs(60));
        assert_eq!(builder.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_client_builder_custom_max_retries() {
        let builder = ClientBuilder::new("test-key").max_retries(5);
        assert_eq!(builder.max_retries, 5);
    }

    #[test]
    fn test_client_builder_cache_disabled() {
        let result = ClientBuilder::new("test-key").cache_enabled(false).build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert!(!client.cache_enabled);
    }

    #[test]
    fn test_client_builder_custom_user_agent_suffix() {
        let result = ClientBuilder::new("test-key")
            .user_agent_suffix("MyApp/1.0")
            .build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert!(client.user_agent.contains("MyApp/1.0"));
    }

    #[test]
    fn test_client_builder_static_method() {
        let result = Client::builder("test-key").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_has_sub_clients() {
        let client = Client::builder("test-key").build().unwrap();

        // Just verify sub-clients can be created
        let _ = client.jobs();
        let _ = client.schemas();
        let _ = client.sites();
        let _ = client.keys();
        let _ = client.llm();
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.refyne.uk");
        assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
        assert_eq!(DEFAULT_MAX_RETRIES, 3);
    }
}
