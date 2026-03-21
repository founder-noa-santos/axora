//! Catalog registry for LLM providers and models.
//!
//! Consumes static JSON catalogs from a configurable base URL (e.g., GitHub Pages).
//! Handles fetch → validate → cross-validate → normalize → query.
//!
//! ## Origin URLs
//! - Base URL: configurable (e.g., `https://openakta.github.io/data-artifacts`)
//! - Providers: `{base}/providers/v1.json`
//! - Models: `{base}/models/v1.json`
//!
//! ## Versioning
//! - URL major version: `v1.json`, `v2.json`, etc.
//! - Payload `schema_version`: semver string inside JSON (e.g., "1.2.0")

// serde imported via types module
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

pub mod types;
pub use types::*;

/// Errors that can occur during registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("HTTP fetch failed: {0}")]
    FetchFailed(String),
    #[error("JSON parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Cross-validation error: {0}")]
    CrossValidationError(String),
    #[error("Incompatible schema version: {0}")]
    IncompatibleVersion(String),
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Unknown compatibility family: {0}")]
    UnknownCompatibilityFamily(String),
}

/// Result type for registry operations.
pub type Result<T> = std::result::Result<T, RegistryError>;

/// Configuration for the catalog registry.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Base URL without trailing slash (e.g., "https://openakta.github.io/data-artifacts")
    pub base_url: String,
    /// HTTP timeout for fetch operations
    pub timeout: Duration,
    /// Cache TTL before re-fetching
    pub cache_ttl: Duration,
    /// Whether to accept partial data (some invalid providers/models)
    pub allow_partial: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            base_url: "https://openakta.github.io/data-artifacts".to_string(),
            timeout: Duration::from_secs(10),
            cache_ttl: Duration::from_secs(86400), // 24 hours
            allow_partial: true,
        }
    }
}

/// Raw catalog files fetched from remote.
#[derive(Debug, Clone)]
pub struct RawCatalog {
    pub providers: ProvidersFile,
    pub models: ModelsFile,
}

/// Diagnostics from validation and cross-validation.
#[derive(Debug, Clone, Default)]
pub struct RegistryDiagnostics {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub providers_valid: usize,
    pub providers_invalid: usize,
    pub models_valid: usize,
    pub models_invalid: usize,
}

/// Normalized and validated registry snapshot.
#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    pub schema_version: String,
    pub updated_at: String,
    pub providers_by_id: HashMap<String, Provider>,
    pub models_by_id: HashMap<String, Model>,
    pub models_by_provider_id: HashMap<String, Vec<String>>,
    pub diagnostics: RegistryDiagnostics,
}

impl RegistrySnapshot {
    /// Get a provider by its catalog ID.
    pub fn get_provider(&self, id: &str) -> Option<&Provider> {
        self.providers_by_id.get(id)
    }

    /// Get a model by its catalog ID.
    pub fn get_model(&self, id: &str) -> Option<&Model> {
        self.models_by_id.get(id)
    }

    /// List all models for a given provider.
    pub fn list_models_for_provider(&self, provider_id: &str) -> Vec<&Model> {
        self.models_by_provider_id
            .get(provider_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.models_by_id.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get effective capabilities for a provider-model pair.
    /// Effective = provider ceiling ∧ model capability
    pub fn get_effective_capabilities(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> EffectiveCapabilities {
        let provider = self.get_provider(provider_id);
        let model = self.get_model(model_id);

        EffectiveCapabilities {
            streaming: provider.map(|p| p.capabilities.streaming).unwrap_or(false)
                && model.map(|m| m.capabilities.streaming).unwrap_or(false),
            tool_calls: provider.map(|p| p.capabilities.tool_calls).unwrap_or(false)
                && model.map(|m| m.capabilities.tool_calls).unwrap_or(false),
            json_mode: provider.map(|p| p.capabilities.chat).unwrap_or(false)
                && model.map(|m| m.capabilities.json_mode).unwrap_or(false),
            accepts_images: model
                .map(|m| m.input_modalities.contains(&Modality::Image))
                .unwrap_or(false),
            accepts_audio: model
                .map(|m| m.input_modalities.contains(&Modality::Audio))
                .unwrap_or(false),
        }
    }

    /// Resolve adapter hint for a provider.
    pub fn resolve_adapter_hint(&self, provider_id: &str) -> AdapterHint {
        let Some(provider) = self.get_provider(provider_id) else {
            return AdapterHint::Unknown {
                reason: format!("Provider '{}' not found", provider_id),
            };
        };

        match provider.api.compatibility.family {
            CompatibilityFamily::OpenAi => AdapterHint::Supported {
                adapter_id: "openai".to_string(),
                surface: provider.api.compatibility.surface.clone(),
            },
            CompatibilityFamily::Anthropic => AdapterHint::Supported {
                adapter_id: "anthropic".to_string(),
                surface: provider.api.compatibility.surface.clone(),
            },
            CompatibilityFamily::Google => AdapterHint::Supported {
                adapter_id: "google".to_string(),
                surface: provider.api.compatibility.surface.clone(),
            },
            CompatibilityFamily::Custom => AdapterHint::Unknown {
                reason: "Custom compatibility family".to_string(),
            },
        }
    }
}

/// Effective capabilities after intersecting provider and model.
#[derive(Debug, Clone, Copy, Default)]
pub struct EffectiveCapabilities {
    pub streaming: bool,
    pub tool_calls: bool,
    pub json_mode: bool,
    pub accepts_images: bool,
    pub accepts_audio: bool,
}

/// Adapter resolution result.
#[derive(Debug, Clone)]
pub enum AdapterHint {
    Supported {
        adapter_id: String,
        surface: ApiSurface,
    },
    Unsupported {
        reason: String,
    },
    Unknown {
        reason: String,
    },
}

/// Cached registry with TTL.
#[derive(Debug)]
pub struct CatalogRegistry {
    config: RegistryConfig,
    cache: Option<(RegistrySnapshot, Instant)>,
    client: reqwest::Client,
}

impl CatalogRegistry {
    /// Create a new registry with the given configuration.
    pub fn new(config: RegistryConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            cache: None,
            client,
        }
    }

    /// Fetch catalog from remote (bypassing cache).
    pub async fn fetch_catalog(&self) -> Result<RegistrySnapshot> {
        let providers_url = format!(
            "{}/providers/v1.json",
            self.config.base_url.trim_end_matches('/')
        );
        let models_url = format!(
            "{}/models/v1.json",
            self.config.base_url.trim_end_matches('/')
        );

        // Fetch both in parallel
        let (providers_res, models_res) = tokio::join!(
            self.fetch_json::<ProvidersFile>(&providers_url),
            self.fetch_json::<ModelsFile>(&models_url),
        );

        let providers = providers_res?;
        let models = models_res?;

        // Validate schema versions are compatible
        self.validate_schema_version(&providers.schema_version)?;
        self.validate_schema_version(&models.schema_version)?;

        // Build normalized registry
        let snapshot = self.build_registry(providers, models)?;

        Ok(snapshot)
    }

    /// Get registry (from cache if valid, otherwise fetch).
    pub async fn get_registry(&mut self) -> Result<RegistrySnapshot> {
        // Check cache
        if let Some((snapshot, fetched_at)) = &self.cache {
            if fetched_at.elapsed() < self.config.cache_ttl {
                return Ok(snapshot.clone());
            }
        }

        // Fetch fresh
        let snapshot = self.fetch_catalog().await?;
        self.cache = Some((snapshot.clone(), Instant::now()));
        Ok(snapshot)
    }

    /// Force refresh the cache.
    pub async fn refresh(&mut self) -> Result<RegistrySnapshot> {
        self.cache = None;
        self.get_registry().await
    }

    /// Internal: fetch JSON from URL.
    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| RegistryError::FetchFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::FetchFailed(format!(
                "HTTP {} for {}",
                response.status(),
                url
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| RegistryError::FetchFailed(e.to_string()))?;

        serde_json::from_str(&text).map_err(|e| RegistryError::ParseError(e.to_string()))
    }

    /// Validate schema version is compatible (major version 1).
    fn validate_schema_version(&self, version: &str) -> Result<()> {
        // Accept any 1.x.x version
        if version.starts_with("1.") || version == "1" {
            Ok(())
        } else {
            Err(RegistryError::IncompatibleVersion(format!(
                "Expected schema version 1.x.x, got {}",
                version
            )))
        }
    }

    /// Build normalized registry from raw files.
    fn build_registry(
        &self,
        providers_file: ProvidersFile,
        models_file: ModelsFile,
    ) -> Result<RegistrySnapshot> {
        let mut diagnostics = RegistryDiagnostics::default();
        let mut providers_by_id = HashMap::new();
        let mut models_by_id = HashMap::new();
        let mut models_by_provider_id: HashMap<String, Vec<String>> = HashMap::new();

        // Validate and deduplicate providers
        for provider in providers_file.providers {
            if providers_by_id.contains_key(&provider.id) {
                diagnostics.warnings.push(format!(
                    "Duplicate provider ID '{}', keeping first",
                    provider.id
                ));
                diagnostics.providers_invalid += 1;
                continue;
            }

            // Validate provider
            if let Err(e) = self.validate_provider(&provider) {
                diagnostics.errors.push(format!(
                    "Provider '{}' validation failed: {}",
                    provider.id, e
                ));
                diagnostics.providers_invalid += 1;
                if !self.config.allow_partial {
                    return Err(RegistryError::ValidationError(e));
                }
                continue;
            }

            providers_by_id.insert(provider.id.clone(), provider);
            diagnostics.providers_valid += 1;
        }

        // Validate and deduplicate models
        for model in models_file.models {
            if models_by_id.contains_key(&model.id) {
                diagnostics
                    .warnings
                    .push(format!("Duplicate model ID '{}', keeping first", model.id));
                diagnostics.models_invalid += 1;
                continue;
            }

            // Cross-validation: provider must exist
            if !providers_by_id.contains_key(&model.provider_id) {
                diagnostics.errors.push(format!(
                    "Model '{}' references unknown provider '{}'",
                    model.id, model.provider_id
                ));
                diagnostics.models_invalid += 1;
                if !self.config.allow_partial {
                    return Err(RegistryError::CrossValidationError(format!(
                        "Orphan model '{}'",
                        model.id
                    )));
                }
                continue;
            }

            // Validate model
            if let Err(e) = self.validate_model(&model) {
                diagnostics
                    .errors
                    .push(format!("Model '{}' validation failed: {}", model.id, e));
                diagnostics.models_invalid += 1;
                if !self.config.allow_partial {
                    return Err(RegistryError::ValidationError(e));
                }
                continue;
            }

            // Build models_by_provider_id index
            models_by_provider_id
                .entry(model.provider_id.clone())
                .or_default()
                .push(model.id.clone());

            models_by_id.insert(model.id.clone(), model);
            diagnostics.models_valid += 1;
        }

        // Cross-validate supported_model_ids against models
        for (provider_id, provider) in &providers_by_id {
            for model_id in &provider.supported_model_ids {
                if let Some(model) = models_by_id.get(model_id) {
                    // Preferred truth: model's provider_id must match
                    if model.provider_id != *provider_id {
                        diagnostics.warnings.push(format!(
                            "Provider '{}' claims model '{}' but model lists provider '{}' (preferring model truth)",
                            provider_id, model_id, model.provider_id
                        ));
                    }
                } else {
                    diagnostics.warnings.push(format!(
                        "Provider '{}' lists unknown model '{}' in supported_model_ids",
                        provider_id, model_id
                    ));
                }
            }

            // Validate default_model_id if present
            if let Some(ref default_id) = provider.default_model_id {
                if !models_by_id.contains_key(default_id) {
                    diagnostics.warnings.push(format!(
                        "Provider '{}' default_model_id '{}' not found",
                        provider_id, default_id
                    ));
                } else {
                    let model = models_by_id.get(default_id).unwrap();
                    if model.provider_id != *provider_id {
                        diagnostics.warnings.push(format!(
                            "Provider '{}' default_model_id '{}' belongs to different provider '{}'",
                            provider_id, default_id, model.provider_id
                        ));
                    }
                }
            }
        }

        Ok(RegistrySnapshot {
            schema_version: providers_file.schema_version,
            updated_at: providers_file.updated_at,
            providers_by_id,
            models_by_id,
            models_by_provider_id,
            diagnostics,
        })
    }

    /// Validate a single provider.
    fn validate_provider(&self, provider: &Provider) -> std::result::Result<(), String> {
        if provider.id.is_empty() {
            return Err("Provider ID cannot be empty".to_string());
        }
        if provider.supported_model_ids.is_empty() {
            return Err("Provider must have at least one supported model".to_string());
        }
        if provider.api.base_url.is_empty() {
            return Err("Provider API base_url cannot be empty".to_string());
        }
        Ok(())
    }

    /// Validate a single model.
    fn validate_model(&self, model: &Model) -> std::result::Result<(), String> {
        if model.id.is_empty() {
            return Err("Model ID cannot be empty".to_string());
        }
        if model.provider_id.is_empty() {
            return Err("Model provider_id cannot be empty".to_string());
        }
        if model.context_window_tokens == 0 {
            return Err("Model context_window_tokens must be > 0".to_string());
        }
        if model.input_modalities.is_empty() {
            return Err("Model must have at least one input modality".to_string());
        }
        if model.output_modalities.is_empty() {
            return Err("Model must have at least one output modality".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_join_without_trailing_slash() {
        let config = RegistryConfig {
            base_url: "https://example.com/data".to_string(),
            ..Default::default()
        };
        let url = format!(
            "{}/providers/v1.json",
            config.base_url.trim_end_matches('/')
        );
        assert_eq!(url, "https://example.com/data/providers/v1.json");
    }

    #[test]
    fn test_url_join_with_trailing_slash() {
        let config = RegistryConfig {
            base_url: "https://example.com/data/".to_string(),
            ..Default::default()
        };
        let url = format!(
            "{}/providers/v1.json",
            config.base_url.trim_end_matches('/')
        );
        assert_eq!(url, "https://example.com/data/providers/v1.json");
    }
}
