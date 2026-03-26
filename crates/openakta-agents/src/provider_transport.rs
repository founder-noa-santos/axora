//! Provider runtime transports for local execution.

use crate::openai_family::{CompatibleProviderConfig, OpenAiFamilyConfig, OpenAiFamilyTransport};
use crate::provider::{CacheMetrics, ModelRequest, ModelResponse, ProviderKind, ProviderUsage};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
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

    /// Local execution was requested without a configured local runtime.
    #[error("missing local configuration for {0:?}")]
    MissingLocalConfiguration(LocalProviderKind),

    /// Cloud execution failed because the network path is unavailable.
    #[error("cloud execution unavailable: {0}")]
    CloudExecutionUnavailable(String),

    /// The requested workflow step requires a cloud lane.
    #[error("cloud execution required: {0}")]
    CloudExecutionRequired(String),
}

/// Runtime configuration for HTTP-backed provider transports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRuntimeConfig {
    /// Timeout for provider requests.
    pub timeout: Duration,
    /// Retry budget for transient failures.
    pub max_retries: u32,
}

impl Default for ProviderRuntimeConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 2,
        }
    }
}

/// Stable provider instance identifier.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ProviderInstanceId(pub String);

impl ProviderInstanceId {
    /// Create a validated provider instance id.
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, String> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("provider instance id cannot be empty".to_string());
        }
        Ok(Self(value))
    }
}

impl std::fmt::Display for ProviderInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Provider wire-profile identifier.
///
/// ## Anthropic Removal Note
///
/// Anthropic support has been intentionally removed. Only OpenAI-compatible profiles remain.
/// Future provider integrations (including Anthropic) must be implemented behind openakta-api.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProfileId {
    /// OpenAI chat/responses-style API.
    OpenAiChatCompletions,
    /// OpenAI-compatible HTTP API.
    OpenAiCompatible,
}

impl ProviderProfileId {
    /// Map the profile to the wire protocol profile for transport selection.
    ///
    /// All providers now use OpenAI Chat Completions format.
    pub fn wire_profile(self) -> crate::wire_profile::WireProfile {
        crate::wire_profile::WireProfile::OpenAiChatCompletions
    }

    /// Map the profile to the telemetry provider kind for metrics/logging.
    pub fn provider_kind(self) -> ProviderKind {
        self.wire_profile().telemetry_kind()
    }
}

/// Inline or file-backed API key reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretRef {
    /// Inline API key value.
    pub api_key: Option<String>,
    /// File containing the API key.
    pub api_key_file: Option<PathBuf>,
}

/// Serde-backed provider instance configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderInstanceConfig {
    /// Wire profile for this instance.
    pub profile: ProviderProfileId,
    /// Base URL for the HTTP API.
    pub base_url: String,
    /// Secret material reference.
    #[serde(default)]
    pub secret: SecretRef,
    /// Whether this instance is considered local for routing.
    #[serde(default)]
    pub is_local: bool,
    /// Default model for this instance.
    pub default_model: Option<String>,
    /// Optional human-readable label.
    pub label: Option<String>,
}

/// Unified provider instance configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderInstancesConfig {
    /// All configured instances keyed by id.
    #[serde(default)]
    pub instances: HashMap<ProviderInstanceId, ProviderInstanceConfig>,
    /// Default cloud instance id.
    pub default_cloud_instance: Option<ProviderInstanceId>,
    /// Default local instance id.
    pub default_local_instance: Option<ProviderInstanceId>,
    /// Deterministic model routing priority.
    #[serde(default)]
    pub model_instance_priority: Vec<ProviderInstanceId>,
}

/// Optional remote model-registry configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteRegistryConfig {
    /// Remote JSON endpoint.
    pub url: String,
    /// Poll interval in seconds.
    pub poll_interval_secs: Option<u64>,
    /// HTTP timeout in seconds.
    pub http_timeout_secs: Option<u32>,
}

/// Resolved runtime provider instance with secrets loaded.
#[derive(Debug, Clone)]
pub struct ResolvedProviderInstance {
    /// Instance id.
    pub id: ProviderInstanceId,
    /// Wire profile.
    pub profile: ProviderProfileId,
    /// Base URL.
    pub base_url: String,
    /// API key if configured.
    pub api_key: Option<SecretString>,
    /// Whether this is a local lane.
    pub is_local: bool,
    /// Default model, if one was configured.
    pub default_model: Option<String>,
    /// Optional label for logs.
    pub label: Option<String>,
}

impl ResolvedProviderInstance {
    /// Coarse provider kind derived from the wire profile (for telemetry).
    pub fn provider_kind(&self) -> ProviderKind {
        self.profile.provider_kind()
    }

    /// Wire protocol profile for request building (for transport).
    pub fn wire_profile(&self) -> crate::wire_profile::WireProfile {
        self.profile.wire_profile()
    }
}

/// Runtime bundle for all configured provider instances.
#[derive(Debug, Clone, Default)]
pub struct ProviderRuntimeBundle {
    /// Resolved instances.
    pub instances: HashMap<ProviderInstanceId, ResolvedProviderInstance>,
    /// Shared HTTP client policy.
    pub http: ProviderRuntimeConfig,
}

/// Runtime model registry entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRegistryEntry {
    /// Model identifier.
    pub name: String,
    /// Maximum context window.
    pub max_context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Preferred provider instance.
    pub preferred_instance: Option<ProviderInstanceId>,
}

/// Provenance metadata for a registry snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryProvenance {
    /// Builtin catalog version.
    pub builtin_version: Option<String>,
    /// Remote version marker.
    pub remote_version: Option<String>,
    /// Local extension digest or version.
    pub extensions_version: Option<String>,
}

/// Runtime model registry snapshot.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistrySnapshot {
    /// Model metadata keyed by model name.
    pub models: HashMap<String, ModelRegistryEntry>,
    /// Snapshot provenance.
    pub sources: RegistryProvenance,
}

/// TOML model registry extension entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TomlModelRegistryEntry {
    /// Model identifier.
    pub name: String,
    /// Maximum context window.
    pub max_context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Preferred provider instance.
    pub preferred_instance: Option<ProviderInstanceId>,
}

/// Optional routing hint attached at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoutingHint {
    /// Model to route.
    pub model: String,
    /// Optional explicit target instance.
    pub instance: Option<ProviderInstanceId>,
}

/// Routing reason classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingReason {
    /// Explicit user override.
    UserOverride,
    /// Registry preferred instance.
    RegistryPreferred,
    /// Priority-list resolution.
    PriorityList,
    /// Local-first heuristic.
    LocalFirst,
    /// Configured default lane.
    DefaultLane,
    /// Only one candidate exists.
    SingleCandidate,
    /// Unknown model routed to defaults.
    UnknownModelDefault,
}

/// Final routing decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingResolution {
    /// Selected instance.
    pub instance: ProviderInstanceId,
    /// Selected model.
    pub model: String,
    /// Resolution reason.
    pub reason: RoutingReason,
}

/// Stable reference to a cloud instance/model execution lane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudModelRef {
    /// Provider instance backing the lane.
    pub instance_id: ProviderInstanceId,
    /// Model requested from the lane.
    pub model: String,
    /// Wire protocol profile for request building.
    pub wire_profile: crate::wire_profile::WireProfile,
    /// Derived provider kind for logs and telemetry.
    pub telemetry_kind: ProviderKind,
}

/// Supported local provider backends.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalProviderKind {
    /// Ollama HTTP runtime.
    Ollama,
    /// OpenAI-compatible HTTP API with API key auth.
    OpenAiCompatible,
}

/// Runtime fallback policy for cloud failures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Never try a local fallback automatically.
    Never,
    /// Offer local recovery explicitly, but do not downgrade automatically.
    #[default]
    Explicit,
    /// Downgrade to local automatically when available.
    Automatic,
}

/// Local runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalProviderConfig {
    /// Local provider kind.
    pub provider: LocalProviderKind,
    /// Base URL for the local runtime.
    pub base_url: String,
    /// API key for authenticated local-compatible providers.
    pub api_key: Option<String>,
    /// Default model identifier.
    pub default_model: String,
    /// Provider name for capability defaults and telemetry.
    pub provider_name: String,
    /// Task classes allowed to run locally.
    pub enabled_for: Vec<String>,
}

impl Default for LocalProviderConfig {
    fn default() -> Self {
        Self {
            provider: LocalProviderKind::Ollama,
            base_url: "http://127.0.0.1:11434".to_string(),
            api_key: None,
            default_model: "qwen2.5-coder:7b".to_string(),
            provider_name: "ollama".to_string(),
            enabled_for: vec![
                "syntax_fix".to_string(),
                "docstring".to_string(),
                "autocomplete".to_string(),
                "small_edit".to_string(),
            ],
        }
    }
}

/// Stable reference to the default local execution lane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelRef {
    /// Provider instance backing the lane.
    pub instance_id: ProviderInstanceId,
    /// Model requested from the local lane.
    pub model: String,
    /// Wire protocol profile for request building.
    pub wire_profile: crate::wire_profile::WireProfile,
    /// Derived provider kind for logs and telemetry.
    pub telemetry_kind: ProviderKind,
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

/// Local inference transport abstraction.
#[async_trait]
pub trait LocalProviderTransport: Send + Sync {
    /// Execute the request against a local model.
    async fn execute_local(
        &self,
        request: &ModelRequest,
        model: &str,
    ) -> std::result::Result<ModelResponse, ProviderTransportError>;

    /// Returns the current operating mode.
    fn mode(&self) -> &'static str;

    /// Returns the local provider kind.
    fn kind(&self) -> LocalProviderKind;
}

/// Local Ollama transport.
pub struct OllamaTransport {
    config: LocalProviderConfig,
    client: reqwest::Client,
}

impl OllamaTransport {
    /// Create a new Ollama transport.
    pub fn new(
        config: LocalProviderConfig,
        timeout: Duration,
    ) -> std::result::Result<Self, ProviderTransportError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| ProviderTransportError::Http(err.to_string()))?;
        Ok(Self { config, client })
    }
}

#[async_trait]
impl LocalProviderTransport for OllamaTransport {
    async fn execute_local(
        &self,
        request: &ModelRequest,
        model: &str,
    ) -> std::result::Result<ModelResponse, ProviderTransportError> {
        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));
        let body = build_ollama_body(request, model);
        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .json(&body)
            .send()
            .await
            .map_err(classify_local_http_error)?;
        let status = response.status();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|err| ProviderTransportError::Http(err.to_string()))?;
        if !status.is_success() {
            return Err(ProviderTransportError::Http(format!(
                "local provider returned {status}: {payload}"
            )));
        }
        parse_ollama_response(&payload)
    }

    fn mode(&self) -> &'static str {
        "local"
    }

    fn kind(&self) -> LocalProviderKind {
        LocalProviderKind::Ollama
    }
}

/// Local transport backed by an OpenAI-compatible endpoint.
pub struct OpenAiCompatibleTransport {
    config: LocalProviderConfig,
    inner: OpenAiFamilyTransport,
}

impl OpenAiCompatibleTransport {
    /// Create a new OpenAI-compatible local transport.
    pub fn new(
        config: LocalProviderConfig,
        runtime: ProviderRuntimeConfig,
    ) -> std::result::Result<Self, ProviderTransportError> {
        let api_key = config
            .api_key
            .clone()
            .ok_or(ProviderTransportError::MissingCredentials(
                ProviderKind::OpenAi,
            ))?;
        let inner = OpenAiFamilyTransport::new(
            OpenAiFamilyConfig::Compatible(CompatibleProviderConfig {
                api_key: SecretString::new(api_key),
                base_url: config.base_url.clone(),
                provider_name: config.provider_name.clone(),
            }),
            runtime,
        )
        .map_err(|err| ProviderTransportError::Build(err.to_string()))?;
        Ok(Self { config, inner })
    }
}

#[async_trait]
impl LocalProviderTransport for OpenAiCompatibleTransport {
    async fn execute_local(
        &self,
        request: &ModelRequest,
        model: &str,
    ) -> std::result::Result<ModelResponse, ProviderTransportError> {
        let mut request = request.clone();
        request.model = model.to_string();
        self.inner
            .execute(&request)
            .await
            .map_err(|err| ProviderTransportError::Http(err.to_string()))
    }

    fn mode(&self) -> &'static str {
        "local-compatible"
    }

    fn kind(&self) -> LocalProviderKind {
        self.config.provider
    }
}

/// Build the default local transport stack for the configured provider.
pub fn default_local_transport(
    config: &LocalProviderConfig,
    timeout: Duration,
) -> std::result::Result<Box<dyn LocalProviderTransport>, ProviderTransportError> {
    match config.provider {
        LocalProviderKind::Ollama => Ok(Box::new(OllamaTransport::new(config.clone(), timeout)?)),
        LocalProviderKind::OpenAiCompatible => Ok(Box::new(OpenAiCompatibleTransport::new(
            config.clone(),
            ProviderRuntimeConfig {
                timeout,
                ..ProviderRuntimeConfig::default()
            },
        )?)),
    }
}

/// Build the runtime local-provider config from a resolved instance.
pub fn local_provider_config_from_instance(
    instance: &ResolvedProviderInstance,
    enabled_for: Vec<String>,
) -> LocalProviderConfig {
    let api_key = instance
        .api_key
        .as_ref()
        .map(|secret| secret.expose_secret().to_string());
    let provider = if api_key.is_some() {
        LocalProviderKind::OpenAiCompatible
    } else {
        LocalProviderKind::Ollama
    };

    LocalProviderConfig {
        provider,
        base_url: instance.base_url.clone(),
        api_key,
        default_model: instance
            .default_model
            .clone()
            .unwrap_or_else(|| "qwen2.5-coder:7b".to_string()),
        provider_name: infer_provider_name(instance),
        enabled_for,
    }
}

fn infer_provider_name(instance: &ResolvedProviderInstance) -> String {
    let label = instance
        .label
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let base_url = instance.base_url.to_ascii_lowercase();

    if label.contains("openrouter") || base_url.contains("openrouter.ai") {
        "openrouter".to_string()
    } else if label.contains("deepseek") || base_url.contains("deepseek") {
        "deepseek".to_string()
    } else if label.contains("moonshot") || base_url.contains("moonshot") {
        "moonshot".to_string()
    } else if label.contains("qwen") || base_url.contains("dashscope") {
        "qwen".to_string()
    } else if instance.api_key.is_some() {
        "openai-compatible".to_string()
    } else {
        "ollama".to_string()
    }
}

fn classify_local_http_error(err: reqwest::Error) -> ProviderTransportError {
    ProviderTransportError::Http(err.to_string())
}

fn build_ollama_body(request: &ModelRequest, model: &str) -> Value {
    let mut messages = request
        .system_instructions
        .iter()
        .map(|instruction| {
            json!({
                "role": "system",
                "content": instruction,
            })
        })
        .collect::<Vec<_>>();

    if !request.invariant_mission_context.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": Value::Array(request.invariant_mission_context.clone()).to_string(),
        }));
    }

    if request.payload.task_type == "Ask" {
        if request.recent_messages.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": request.payload.description,
            }));
        } else {
            for message in &request.recent_messages {
                messages.push(json!({
                    "role": message.role,
                    "content": message.content,
                }));
            }
        }
    } else {
        messages.push(json!({
            "role": "user",
            "content": request.payload.to_toon().unwrap_or_else(|_| request.payload.description.clone()),
        }));

        for message in &request.recent_messages {
            messages.push(json!({
                "role": message.role,
                "content": message.content,
            }));
        }
    }

    json!({
        "model": model,
        "think": false,
        "stream": false,
        "messages": messages,
        "options": {
            "temperature": request.temperature.unwrap_or(0.0),
            "num_predict": request.max_output_tokens,
        }
    })
}

fn parse_ollama_response(
    response: &Value,
) -> std::result::Result<ModelResponse, ProviderTransportError> {
    let output_text = response
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .or_else(|| response.get("response").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    let prompt_tokens = number_field(response, "prompt_eval_count");
    let completion_tokens = number_field(response, "eval_count");

    Ok(ModelResponse {
        id: response
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string),
        // Local Ollama is OpenAI-compatible enough for the shared response model.
        provider: ProviderKind::OpenAi,
        content: output_text.clone(),
        output_text,
        tool_calls: Vec::new(),
        usage: ProviderUsage {
            input_tokens: prompt_tokens,
            output_tokens: completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cache_write_tokens: 0,
            cache_read_tokens: 0,
            uncached_input_tokens: prompt_tokens,
        },
        stop_reason: response
            .get("done_reason")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                response
                    .get("done")
                    .and_then(Value::as_bool)
                    .filter(|done| *done)
                    .map(|_| "stop".to_string())
            }),
        provider_request_id: None,
        raw: response.clone(),
    })
}

fn number_field(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{
        CacheRetention, ChatMessage, ModelBoundaryPayload, ModelBoundaryPayloadType,
    };

    fn sample_request(wire_profile: crate::wire_profile::WireProfile) -> ModelRequest {
        ModelRequest {
            provider: wire_profile,
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
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            max_output_tokens: 512,
            temperature: Some(0.1),
            stream: false,
            cache_retention: CacheRetention::Extended,
        }
    }

    #[test]
    fn runtime_config_is_flat_http_policy() {
        let config = ProviderRuntimeConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 2);
    }

    #[test]
    fn parse_ollama_response_maps_into_shared_model() {
        let response = parse_ollama_response(&json!({
            "model": "qwen2.5-coder:7b",
            "message": {
                "role": "assistant",
                "content": "patched output"
            },
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": 11,
            "eval_count": 7
        }))
        .unwrap();

        assert_eq!(response.output_text, "patched output");
        assert_eq!(response.usage.input_tokens, 11);
        assert_eq!(response.usage.output_tokens, 7);
    }

    #[test]
    fn default_local_transport_builds_ollama_lane() {
        let config = LocalProviderConfig::default();
        let transport = default_local_transport(&config, Duration::from_secs(5)).unwrap();

        assert_eq!(transport.mode(), "local");
        assert_eq!(transport.kind(), LocalProviderKind::Ollama);
    }

    #[test]
    fn local_provider_config_defaults_to_fast_path_model() {
        let config = LocalProviderConfig::default();

        assert_eq!(config.provider, LocalProviderKind::Ollama);
        assert_eq!(config.default_model, "qwen2.5-coder:7b");
        assert!(config.enabled_for.iter().any(|task| task == "syntax_fix"));
    }
}
