//! Type definitions for the catalog registry.
//!
//! Mirrors the JSON schema from the data-artifacts repository.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root object for providers file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersFile {
    pub schema_version: String,
    pub updated_at: String,
    pub providers: Vec<Provider>,
}

/// Root object for models file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsFile {
    pub schema_version: String,
    pub updated_at: String,
    pub models: Vec<Model>,
}

/// Provider metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Catalog slug (e.g., "azure-openai", "openai-direct")
    pub id: String,
    /// Canonical vendor for grouping (e.g., "openai", "anthropic", "google")
    pub vendor_slug: String,
    /// Human-readable label
    pub label: String,
    /// Description of the provider
    pub description: String,
    /// Current status
    #[serde(default)]
    pub status: ProviderStatus,
    /// Provider type classification
    #[serde(default)]
    pub provider_type: ProviderType,
    /// Documentation URL
    pub documentation_url: String,
    /// Optional homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage_url: Option<String>,
    /// API configuration
    pub api: ApiConfig,
    /// Authentication configuration
    pub authentication: AuthenticationConfig,
    /// Platform capability ceiling
    pub capabilities: ProviderCapabilities,
    /// List of supported model IDs (convenience index)
    pub supported_model_ids: Vec<String>,
    /// Optional default model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model_id: Option<String>,
    /// Optional deprecation date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_since: Option<String>,
    /// Optional supported regions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<Vec<String>>,
    /// Optional rate limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<RateLimits>,
    /// Optional pricing tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing_tier: Option<String>,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Forward-compatible vendor extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Provider status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    #[default]
    Active,
    Beta,
    Deprecated,
    Disabled,
}

/// Provider type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    #[default]
    Direct,
    Aggregator,
    Proxy,
    SelfHosted,
    CloudWrapper,
}

/// API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Base URL for API requests
    pub base_url: String,
    /// Compatibility information
    pub compatibility: ApiCompatibility,
    /// Optional default endpoint path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_endpoint_path: Option<String>,
    /// Optional notes about the API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// API compatibility information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCompatibility {
    /// Compatibility family
    pub family: CompatibilityFamily,
    /// API surface
    pub surface: ApiSurface,
    /// Strictness level
    pub strictness: CompatibilityStrictness,
}

/// Compatibility family.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityFamily {
    OpenAi,
    Anthropic,
    Google,
    Custom,
}

/// API surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiSurface {
    ChatCompletions,
    Responses,
    Messages,
    GenerateContent,
    Custom,
}

/// Compatibility strictness.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStrictness {
    #[default]
    Native,
    Compatible,
    Partial,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication scheme
    pub scheme: AuthScheme,
    /// Optional header name (e.g., "Authorization", "x-api-key")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Optional environment variable hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_var_hint: Option<String>,
    /// Optional query parameter name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_param: Option<String>,
    /// How the client should handle auth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_exposure: Option<ClientExposure>,
}

/// Authentication scheme.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthScheme {
    None,
    ApiKeyHeader,
    Bearer,
    OAuth2,
}

/// Client exposure for authentication.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientExposure {
    UserSupplied,
    ServerManaged,
    NotSupportedInClient,
}

/// Provider platform capabilities (ceiling).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    /// Supports chat completions
    #[serde(default)]
    pub chat: bool,
    /// Supports streaming
    #[serde(default)]
    pub streaming: bool,
    /// Supports embeddings
    #[serde(default)]
    pub embeddings: bool,
    /// Supports tool calls
    #[serde(default)]
    pub tool_calls: bool,
}

/// Rate limits.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests_per_minute: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_minute: Option<u32>,
}

/// Model metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model catalog ID
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Description
    pub description: String,
    /// Current status
    #[serde(default)]
    pub status: ModelStatus,
    /// Provider ID (primary link)
    pub provider_id: String,
    /// Modality classification
    #[serde(default)]
    pub modality: ModalityClass,
    /// Context window in tokens
    pub context_window_tokens: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Input modalities
    pub input_modalities: Vec<Modality>,
    /// Output modalities
    pub output_modalities: Vec<Modality>,
    /// Optional model family
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Optional deprecation date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_since: Option<String>,
    /// Optional tool calling details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calling: Option<ToolCallingConfig>,
    /// System prompt support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_support: Option<SystemPromptSupport>,
    /// Catalog visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_visibility: Option<CatalogVisibility>,
    /// Cost tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_tier: Option<String>,
    /// Documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    /// Notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Forward-compatible extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Model status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    #[default]
    Active,
    Beta,
    Deprecated,
    Disabled,
}

/// Modality classification for UX.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModalityClass {
    #[default]
    Text,
    Multimodal,
    Embedding,
    Audio,
    Image,
    Code,
}

/// Model capabilities.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    /// Supports streaming
    #[serde(default)]
    pub streaming: bool,
    /// Supports tool calls
    #[serde(default)]
    pub tool_calls: bool,
    /// Supports JSON mode
    #[serde(default)]
    pub json_mode: bool,
}

/// Modality.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Pdf,
    Code,
}

/// Tool calling configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolCallingConfig {
    #[serde(default)]
    pub parallel_calls: bool,
    #[serde(default)]
    pub forced_tool_choice: bool,
}

/// System prompt support level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptSupport {
    Native,
    Emulated,
    Unsupported,
    Unknown,
}

/// Catalog visibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogVisibility {
    Featured,
    #[default]
    Standard,
    Hidden,
}

/// Provider plan (e.g., "coding_plan", "pro", "default").
#[derive(Debug, Clone)]
pub struct ProviderPlan {
    pub name: String,
    pub model_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_status_serialization() {
        let status = ProviderStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");
    }

    #[test]
    fn test_compatibility_family_serialization() {
        let family = CompatibilityFamily::OpenAi;
        let json = serde_json::to_string(&family).unwrap();
        assert_eq!(json, "\"open_ai\"");
    }

    #[test]
    fn test_modality_serialization() {
        let modality = Modality::Image;
        let json = serde_json::to_string(&modality).unwrap();
        assert_eq!(json, "\"image\"");
    }
}
