//! Configuration types for OpenAI-family transport.

use secrecy::SecretString;

/// OpenAI-family configuration.
#[derive(Debug, Clone)]
pub enum OpenAiFamilyConfig {
    /// Official OpenAI API.
    Official(OfficialOpenAiConfig),
    /// OpenAI-compatible provider with custom base URL.
    Compatible(CompatibleProviderConfig),
}

impl OpenAiFamilyConfig {
    /// Get the provider name for this configuration.
    pub fn provider_name(&self) -> &str {
        match self {
            OpenAiFamilyConfig::Official(_) => "openai",
            OpenAiFamilyConfig::Compatible(cfg) => &cfg.provider_name,
        }
    }

    /// Get the API key.
    pub fn api_key(&self) -> &SecretString {
        match self {
            OpenAiFamilyConfig::Official(cfg) => &cfg.api_key,
            OpenAiFamilyConfig::Compatible(cfg) => &cfg.api_key,
        }
    }

    /// Get the base URL (None for official OpenAI).
    pub fn base_url(&self) -> Option<&str> {
        match self {
            OpenAiFamilyConfig::Official(_) => None,
            OpenAiFamilyConfig::Compatible(cfg) => Some(&cfg.base_url),
        }
    }
}

/// Official OpenAI API configuration.
#[derive(Debug, Clone)]
pub struct OfficialOpenAiConfig {
    /// API key.
    pub api_key: SecretString,
    /// Organization ID (optional).
    pub organization: Option<String>,
    /// Project ID (optional).
    pub project: Option<String>,
}

/// OpenAI-compatible provider configuration.
#[derive(Debug, Clone)]
pub struct CompatibleProviderConfig {
    /// API key.
    pub api_key: SecretString,
    /// Base URL for the compatible provider.
    pub base_url: String,
    /// Provider name (e.g., "deepseek", "qwen", "moonshot").
    pub provider_name: String,
}
