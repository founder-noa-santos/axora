//! Provider runtime transports for live and synthetic execution.

use crate::provider::{
    AnthropicProvider, CacheMetrics, ModelRequest, ModelResponse, OpenAiProvider, ProviderClient,
    ProviderKind,
};
use axora_cache::PrefixCache;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tonic::async_trait;

/// Provider runtime transport errors.
#[derive(Debug, Error)]
pub enum ProviderTransportError {
    /// Request building failed.
    #[error("provider request build failed: {0}")]
    Build(String),

    /// HTTP request failed.
    #[error("provider request failed: {0}")]
    Http(String),

    /// Response parsing failed.
    #[error("provider response parse failed: {0}")]
    Parse(String),

    /// Live execution requires credentials.
    #[error("missing credentials for {0:?}")]
    MissingCredentials(ProviderKind),

    /// Synthetic fallback was attempted without explicit opt-in.
    #[error("synthetic fallback is disabled for {0:?}; inject SyntheticTransport explicitly or set AXORA_ALLOW_SYNTHETIC_PROVIDER_FALLBACK=1 for dev mode")]
    SyntheticFallbackDisabled(ProviderKind),
}

/// Runtime configuration for live provider transports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRuntimeConfig {
    /// Anthropic base URL.
    pub anthropic_base_url: String,
    /// OpenAI base URL.
    pub openai_base_url: String,
    /// Anthropic API key.
    #[serde(skip_serializing, skip_deserializing)]
    pub anthropic_api_key: Option<SecretString>,
    /// OpenAI API key.
    #[serde(skip_serializing, skip_deserializing)]
    pub openai_api_key: Option<SecretString>,
    /// Timeout for provider requests.
    pub timeout: Duration,
    /// Retry budget for transient failures.
    pub max_retries: u32,
}

impl Default for ProviderRuntimeConfig {
    fn default() -> Self {
        Self {
            anthropic_base_url: std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            openai_base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com".to_string()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .map(SecretString::new),
            openai_api_key: std::env::var("OPENAI_API_KEY")
                .ok()
                .map(SecretString::new),
            timeout: Duration::from_secs(30),
            max_retries: 2,
        }
    }
}

impl ProviderRuntimeConfig {
    /// Returns true when credentials are available for the provider.
    pub fn has_credentials(&self, provider: ProviderKind) -> bool {
        match provider {
            ProviderKind::Anthropic => self.anthropic_api_key.is_some(),
            ProviderKind::OpenAi => self.openai_api_key.is_some(),
        }
    }
}

/// Telemetry captured during transport execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderExecutionTelemetry {
    /// Compression ratio between serialized TOON and original dynamic JSON estimate.
    pub compression_ratio: f32,
    /// Number of fallbacks to synthetic or plain-text modes.
    pub fallback_count: u32,
    /// Whether the provider accepted the request.
    pub provider_accepted: bool,
    /// Cache metrics from request preparation.
    pub cache_metrics: CacheMetrics,
}

/// Execution transport abstraction used by the coordinator runtime.
#[async_trait]
pub trait ProviderTransport: Send + Sync {
    /// Execute the request and return the parsed provider response.
    async fn execute(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ProviderTransportError>;

    /// Returns the current operating mode.
    fn mode(&self) -> &'static str;
}

/// Synthetic transport for tests and explicit development fallback.
pub struct SyntheticTransport {
    workspace_root: PathBuf,
    anthropic_provider: Mutex<AnthropicProvider>,
    openai_provider: Mutex<OpenAiProvider>,
}

impl SyntheticTransport {
    /// Create a new synthetic transport rooted at the workspace.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            anthropic_provider: Mutex::new(AnthropicProvider::new(PrefixCache::new(128))),
            openai_provider: Mutex::new(OpenAiProvider::new(PrefixCache::new(128))),
        }
    }
}

#[async_trait]
impl ProviderTransport for SyntheticTransport {
    async fn execute(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ProviderTransportError> {
        match request.provider {
            ProviderKind::Anthropic => {
                let mut provider = self.anthropic_provider.lock().await;
                let prepared = provider
                    .prepare_request(request)
                    .map_err(|err| ProviderTransportError::Build(err.to_string()))?;
                let response = synthetic_provider_response(
                    request,
                    &prepared.cache_metrics,
                    &self.workspace_root,
                );
                provider
                    .parse_response(&response)
                    .map_err(|err| ProviderTransportError::Parse(err.to_string()))
            }
            ProviderKind::OpenAi => {
                let mut provider = self.openai_provider.lock().await;
                let prepared = provider
                    .prepare_request(request)
                    .map_err(|err| ProviderTransportError::Build(err.to_string()))?;
                let response = synthetic_provider_response(
                    request,
                    &prepared.cache_metrics,
                    &self.workspace_root,
                );
                provider
                    .parse_response(&response)
                    .map_err(|err| ProviderTransportError::Parse(err.to_string()))
            }
        }
    }

    fn mode(&self) -> &'static str {
        "synthetic"
    }
}

/// Live HTTP transport for cloud-hosted providers.
pub struct LiveHttpTransport {
    config: ProviderRuntimeConfig,
    client: reqwest::Client,
    anthropic_provider: Mutex<AnthropicProvider>,
    openai_provider: Mutex<OpenAiProvider>,
}

impl LiveHttpTransport {
    /// Create a new live transport.
    pub fn new(config: ProviderRuntimeConfig) -> std::result::Result<Self, ProviderTransportError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|err| ProviderTransportError::Http(err.to_string()))?;
        Ok(Self {
            config,
            client,
            anthropic_provider: Mutex::new(AnthropicProvider::new(PrefixCache::new(128))),
            openai_provider: Mutex::new(OpenAiProvider::new(PrefixCache::new(128))),
        })
    }
}

#[async_trait]
impl ProviderTransport for LiveHttpTransport {
    async fn execute(
        &self,
        request: &ModelRequest,
    ) -> std::result::Result<ModelResponse, ProviderTransportError> {
        match request.provider {
            ProviderKind::Anthropic => {
                let Some(api_key) = self.config.anthropic_api_key.as_ref() else {
                    return Err(ProviderTransportError::MissingCredentials(ProviderKind::Anthropic));
                };
                let mut provider = self.anthropic_provider.lock().await;
                let prepared = provider
                    .prepare_request(request)
                    .map_err(|err| ProviderTransportError::Build(err.to_string()))?;
                let url = format!("{}/v1/messages", self.config.anthropic_base_url);
                let mut headers = HeaderMap::new();
                headers.insert("x-api-key", header_value(api_key.expose_secret())?);
                headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let response = send_with_retries(
                    &self.client,
                    &url,
                    headers,
                    &prepared.body,
                    self.config.max_retries,
                )
                .await?;
                provider
                    .parse_response(&response)
                    .map_err(|err| ProviderTransportError::Parse(err.to_string()))
            }
            ProviderKind::OpenAi => {
                let Some(api_key) = self.config.openai_api_key.as_ref() else {
                    return Err(ProviderTransportError::MissingCredentials(ProviderKind::OpenAi));
                };
                let mut provider = self.openai_provider.lock().await;
                let prepared = provider
                    .prepare_request(request)
                    .map_err(|err| ProviderTransportError::Build(err.to_string()))?;
                let url = format!("{}/v1/responses", self.config.openai_base_url);
                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    header_value(&format!("Bearer {}", api_key.expose_secret()))?,
                );
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let response = send_with_retries(
                    &self.client,
                    &url,
                    headers,
                    &prepared.body,
                    self.config.max_retries,
                )
                .await?;
                provider
                    .parse_response(&response)
                    .map_err(|err| ProviderTransportError::Parse(err.to_string()))
            }
        }
    }

    fn mode(&self) -> &'static str {
        "live"
    }
}

/// Build the default transport stack for the provider kind.
pub fn default_transport(
    provider: ProviderKind,
    workspace_root: impl Into<PathBuf>,
) -> std::result::Result<Box<dyn ProviderTransport>, ProviderTransportError> {
    let config = ProviderRuntimeConfig::default();
    if config.has_credentials(provider) {
        Ok(Box::new(LiveHttpTransport::new(config)?))
    } else if synthetic_fallback_enabled() {
        Ok(Box::new(SyntheticTransport::new(workspace_root)))
    } else {
        Err(ProviderTransportError::SyntheticFallbackDisabled(provider))
    }
}

fn synthetic_fallback_enabled() -> bool {
    matches!(
        std::env::var("AXORA_ALLOW_SYNTHETIC_PROVIDER_FALLBACK").as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn header_value(value: &str) -> std::result::Result<HeaderValue, ProviderTransportError> {
    HeaderValue::from_str(value).map_err(|err| ProviderTransportError::Http(err.to_string()))
}

async fn send_with_retries(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
    body: &Value,
    max_retries: u32,
) -> std::result::Result<Value, ProviderTransportError> {
    let mut attempt = 0u32;
    loop {
        let response = client
            .post(url)
            .headers(headers.clone())
            .json(body)
            .send()
            .await
            .map_err(|err| ProviderTransportError::Http(err.to_string()));
        match response {
            Ok(response) => {
                let status = response.status();
                let retry_after_ms = response
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(parse_retry_after_millis);
                let body = response
                    .json::<Value>()
                    .await
                    .map_err(|err| ProviderTransportError::Http(err.to_string()))?;
                if status.is_success() {
                    return Ok(body);
                }
                if (status.is_server_error() || status.as_u16() == 429) && attempt < max_retries {
                    attempt += 1;
                    tokio::time::sleep(backoff_for_attempt(attempt, retry_after_ms)).await;
                    continue;
                }
                return Err(ProviderTransportError::Http(format!(
                    "provider returned {status}: {body}"
                )));
            }
            Err(err) if attempt < max_retries => {
                attempt += 1;
                let _ = err;
                tokio::time::sleep(backoff_for_attempt(attempt, None)).await;
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

fn parse_retry_after_millis(value: &HeaderValue) -> Option<u64> {
    value
        .to_str()
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000))
}

fn backoff_for_attempt(attempt: u32, retry_after_ms: Option<u64>) -> Duration {
    if let Some(retry_after_ms) = retry_after_ms {
        return Duration::from_millis(retry_after_ms);
    }

    let base = 200u64.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
    Duration::from_millis(base.min(2_000))
}

fn synthetic_provider_response(
    request: &ModelRequest,
    cache_metrics: &CacheMetrics,
    workspace_root: &Path,
) -> Value {
    let output_text = if request
        .payload
        .task_type
        .to_ascii_lowercase()
        .contains("codemodification")
        || request
            .payload
            .task_type
            .to_ascii_lowercase()
            .contains("code_modification")
    {
        synthetic_patch_output(
            workspace_root,
            request.payload.target_files.first().map(|s| s.as_str()),
        )
        .unwrap_or_else(|err| {
            let file = request
                .payload
                .target_files
                .first()
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            format!(
                "<<<<<<< SEARCH {file}\n{err}\n=======\n{err}\n>>>>>>> REPLACE"
            )
        })
    } else {
        format!("Completed task: {}", request.payload.description)
    };

    let output_tokens = estimate_tokens(&output_text) as u64;
    let uncached_input = cache_metrics.uncached_input_tokens as u64;
    let cache_write = cache_metrics.cache_write_tokens as u64;
    let cache_read = cache_metrics.cache_read_tokens as u64;

    match request.provider {
        ProviderKind::Anthropic => json!({
            "id": format!("anthropic-{}", request.payload.task_id),
            "content": [{"type": "text", "text": output_text}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": uncached_input,
                "output_tokens": output_tokens,
                "cache_creation_input_tokens": cache_write,
                "cache_read_input_tokens": cache_read
            }
        }),
        ProviderKind::OpenAi => json!({
            "id": format!("openai-{}", request.payload.task_id),
            "output_text": output_text,
            "stop_reason": "stop",
            "usage": {
                "input_tokens": uncached_input + cache_write + cache_read,
                "output_tokens": output_tokens,
                "input_tokens_details": {
                    "cached_tokens": cache_read
                }
            }
        }),
    }
}

fn synthetic_patch_output(
    workspace_root: &Path,
    target_file: Option<&str>,
) -> std::result::Result<String, String> {
    let target_file = target_file.ok_or_else(|| "missing target file".to_string())?;
    let path = workspace_root.join(target_file);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("failed reading {}: {err}", path.display()))?;
    if content.is_empty() {
        return Err("refusing to build a no-op patch for an empty file".to_string());
    }
    Ok(format!(
        "<<<<<<< SEARCH {target_file}\n{content}\n=======\n{content}\n>>>>>>> REPLACE"
    ))
}

fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{CacheRetention, ChatMessage, ModelBoundaryPayload, ModelBoundaryPayloadType};

    fn sample_request(provider: ProviderKind) -> ModelRequest {
        ModelRequest {
            provider,
            model: "test-model".to_string(),
            system_instructions: vec!["Return unified diff only.".to_string()],
            tool_schemas: vec![],
            invariant_mission_context: vec![],
            payload: ModelBoundaryPayload {
                payload_type: ModelBoundaryPayloadType::TaskExecution,
                task_id: "task-1".to_string(),
                title: "Update src/auth.rs".to_string(),
                description: "Update src/auth.rs".to_string(),
                task_type: "CODE_MODIFICATION".to_string(),
                target_files: vec!["Cargo.toml".to_string()],
                target_symbols: vec![],
                context_spans: vec![],
                context_pack: None,
            },
            recent_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Focus only on login".to_string(),
            }],
            max_output_tokens: 512,
            temperature: Some(0.1),
            stream: false,
            cache_retention: CacheRetention::Extended,
        }
    }

    #[tokio::test]
    async fn synthetic_transport_returns_provider_parsed_response() {
        let transport = SyntheticTransport::new("/Users/noasantos/Fluri/axora");
        let response = transport
            .execute(&sample_request(ProviderKind::Anthropic))
            .await
            .unwrap();
        assert_eq!(response.provider, ProviderKind::Anthropic);
        assert!(!response.output_text.is_empty());
    }

    #[test]
    fn runtime_config_detects_credentials() {
        let config = ProviderRuntimeConfig {
            anthropic_api_key: Some(SecretString::new("x".to_string())),
            openai_api_key: None,
            ..ProviderRuntimeConfig::default()
        };
        assert!(config.has_credentials(ProviderKind::Anthropic));
        assert!(!config.has_credentials(ProviderKind::OpenAi));
    }

    #[test]
    fn default_transport_requires_explicit_opt_in_for_synthetic_fallback() {
        std::env::remove_var("AXORA_ALLOW_SYNTHETIC_PROVIDER_FALLBACK");
        match default_transport(ProviderKind::Anthropic, ".") {
            Err(ProviderTransportError::SyntheticFallbackDisabled(ProviderKind::Anthropic)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("synthetic fallback should be disabled by default"),
        }
    }
}
