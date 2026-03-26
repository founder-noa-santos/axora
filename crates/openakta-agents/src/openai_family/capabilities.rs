//! Provider capability matrix for OpenAI-family providers.

/// Provider-level capabilities (apply to all models from this provider).
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    /// Provider name.
    pub provider_name: String,
    /// Supports tool calling.
    pub supports_tools: bool,
    /// Supports JSON mode / structured output.
    pub supports_json_mode: bool,
    /// Supports streaming.
    pub supports_streaming: bool,
    /// Supports vision/multimodal.
    pub supports_vision: bool,
    /// Supports prompt caching.
    pub supports_prompt_cache: bool,
    /// Supports seed parameter.
    pub supports_seed: bool,
    /// Maximum context window size.
    pub max_context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Rate limit in requests per minute (if known).
    pub rate_limit_rpm: Option<u32>,
    /// Rate limit in tokens per minute (if known).
    pub rate_limit_tpm: Option<u32>,
}

/// Model-level capabilities (override provider defaults).
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    /// Model name.
    pub model_name: String,
    /// Override provider context window.
    pub max_context_window: Option<u32>,
    /// Override provider output tokens.
    pub max_output_tokens: Option<u32>,
    /// Override provider tools support.
    pub supports_tools: Option<bool>,
    /// Override provider vision support.
    pub supports_vision: Option<bool>,
    /// Model is deprecated.
    pub deprecated: bool,
}

/// Resolved capabilities at request time (provider + model merged).
#[derive(Debug, Clone)]
pub struct ResolvedCapabilities {
    /// Supports tool calling.
    pub supports_tools: bool,
    /// Supports JSON mode.
    pub supports_json_mode: bool,
    /// Supports streaming.
    pub supports_streaming: bool,
    /// Supports vision.
    pub supports_vision: bool,
    /// Supports prompt cache.
    pub supports_prompt_cache: bool,
    /// Maximum context window.
    pub max_context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
}

/// Built-in model capability registry.
#[derive(Debug, Clone)]
pub struct ModelCapabilityRegistry {
    entries: Vec<ModelCapabilities>,
}

impl ModelCapabilityRegistry {
    /// Conservative built-in model capability registry keyed by known ids/prefixes.
    pub fn builtin() -> Self {
        Self {
            entries: vec![
                ModelCapabilities {
                    model_name: "gpt-4".to_string(),
                    max_context_window: Some(128_000),
                    max_output_tokens: Some(16_384),
                    supports_tools: Some(true),
                    supports_vision: Some(true),
                    deprecated: false,
                },
                ModelCapabilities {
                    model_name: "gpt-3.5".to_string(),
                    max_context_window: Some(16_385),
                    max_output_tokens: Some(4_096),
                    supports_tools: Some(true),
                    supports_vision: Some(false),
                    deprecated: false,
                },
                ModelCapabilities {
                    model_name: "qwen".to_string(),
                    max_context_window: Some(131_072),
                    max_output_tokens: Some(8_192),
                    supports_tools: Some(true),
                    supports_vision: Some(true),
                    deprecated: false,
                },
                ModelCapabilities {
                    model_name: "deepseek".to_string(),
                    max_context_window: Some(128_000),
                    max_output_tokens: Some(8_192),
                    supports_tools: Some(true),
                    supports_vision: Some(false),
                    deprecated: false,
                },
            ],
        }
    }

    pub fn resolve_for_model(&self, model_name: &str) -> Option<&ModelCapabilities> {
        self.entries.iter().find(|entry| {
            model_name == entry.model_name || model_name.starts_with(&entry.model_name)
        })
    }
}

impl ResolvedCapabilities {
    /// Resolve capabilities by merging provider and model-level capabilities.
    /// Model-level overrides take precedence.
    pub fn resolve(provider: &ProviderCapabilities, model: Option<&ModelCapabilities>) -> Self {
        let max_context = model
            .and_then(|m| m.max_context_window)
            .unwrap_or(provider.max_context_window);

        let max_output = model
            .and_then(|m| m.max_output_tokens)
            .unwrap_or(provider.max_output_tokens);

        Self {
            supports_tools: model
                .and_then(|m| m.supports_tools)
                .unwrap_or(provider.supports_tools),
            supports_json_mode: provider.supports_json_mode,
            supports_streaming: provider.supports_streaming,
            supports_vision: model
                .and_then(|m| m.supports_vision)
                .unwrap_or(provider.supports_vision),
            supports_prompt_cache: provider.supports_prompt_cache,
            max_context_window: max_context,
            max_output_tokens: max_output,
        }
    }
}

impl ProviderCapabilities {
    /// Get hardcoded defaults for known providers.
    pub fn hardcoded_defaults(provider_name: &str) -> Self {
        match provider_name {
            "openai" => Self {
                provider_name: "openai".into(),
                supports_tools: true,
                supports_json_mode: true,
                supports_streaming: true,
                supports_vision: true,
                supports_prompt_cache: true,
                supports_seed: true,
                max_context_window: 128_000,
                max_output_tokens: 64_000,
                rate_limit_rpm: Some(10_000),
                rate_limit_tpm: Some(1_000_000),
            },
            "deepseek" => Self {
                provider_name: "deepseek".into(),
                supports_tools: true,
                supports_json_mode: true,
                supports_streaming: true,
                supports_vision: false,
                supports_prompt_cache: false,
                supports_seed: false,
                max_context_window: 128_000,
                max_output_tokens: 64_000,
                rate_limit_rpm: Some(1_000),
                rate_limit_tpm: Some(100_000),
            },
            "qwen" => Self {
                provider_name: "qwen".into(),
                supports_tools: true,
                supports_json_mode: true,
                supports_streaming: true,
                supports_vision: true,
                supports_prompt_cache: false,
                supports_seed: false,
                max_context_window: 256_000,
                max_output_tokens: 64_000,
                rate_limit_rpm: Some(500),
                rate_limit_tpm: Some(50_000),
            },
            "moonshot" => Self {
                provider_name: "moonshot".into(),
                supports_tools: true,
                supports_json_mode: true,
                supports_streaming: true,
                supports_vision: false,
                supports_prompt_cache: false,
                supports_seed: false,
                max_context_window: 128_000,
                max_output_tokens: 32_000,
                rate_limit_rpm: Some(500),
                rate_limit_tpm: Some(50_000),
            },
            "openrouter" => Self {
                provider_name: "openrouter".into(),
                supports_tools: true,
                supports_json_mode: true,
                supports_streaming: true,
                supports_vision: true,
                supports_prompt_cache: false,
                supports_seed: false,
                max_context_window: 128_000,
                max_output_tokens: 64_000,
                rate_limit_rpm: Some(1_000),
                rate_limit_tpm: Some(100_000),
            },
            // Conservative defaults for unknown providers
            _ => Self {
                provider_name: provider_name.into(),
                supports_tools: false,
                supports_json_mode: false,
                supports_streaming: true,
                supports_vision: false,
                supports_prompt_cache: false,
                supports_seed: false,
                max_context_window: 32_000,
                max_output_tokens: 4_096,
                rate_limit_rpm: None,
                rate_limit_tpm: None,
            },
        }
    }
}
