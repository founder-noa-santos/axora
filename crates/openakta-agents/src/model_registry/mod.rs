//! Runtime model registry helpers.

use crate::provider_transport::{
    ModelRegistryEntry, ModelRegistrySnapshot, ProviderInstanceId, RegistryProvenance,
    TomlModelRegistryEntry,
};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// Authoritative runtime metadata for a single model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicModelMetadata {
    /// Model identifier.
    pub name: String,
    /// Maximum context window.
    pub max_context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Preferred provider instance for this model.
    pub preferred_instance: Option<ProviderInstanceId>,
}

/// Authoritative dynamic model registry used by routing and budgeting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicModelRegistry {
    /// Model metadata keyed by model name.
    pub models: HashMap<String, DynamicModelMetadata>,
    /// Snapshot provenance.
    pub sources: RegistryProvenance,
}

#[derive(Debug, Deserialize)]
struct RemoteRegistryBody {
    version: Option<u64>,
    #[serde(default)]
    models: Vec<RemoteRegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct RemoteRegistryEntry {
    name: String,
    max_context_window: u32,
    max_output_tokens: u32,
    preferred_instance: Option<ProviderInstanceId>,
}

/// Builtin catalog of known models.
pub fn builtin_catalog() -> HashMap<String, ModelRegistryEntry> {
    [
        ModelRegistryEntry {
            name: "claude-sonnet-4-5".to_string(),
            max_context_window: 200_000,
            max_output_tokens: 8_192,
            preferred_instance: None,
        },
        ModelRegistryEntry {
            name: "gpt-5.4".to_string(),
            max_context_window: 128_000,
            max_output_tokens: 8_192,
            preferred_instance: None,
        },
        ModelRegistryEntry {
            name: "qwen2.5-coder:7b".to_string(),
            max_context_window: 32_768,
            max_output_tokens: 4_096,
            preferred_instance: None,
        },
    ]
    .into_iter()
    .map(|entry| (entry.name.clone(), entry))
    .collect()
}

/// Parse a remote registry JSON payload.
pub fn parse_remote_json(bytes: &[u8]) -> anyhow::Result<HashMap<String, ModelRegistryEntry>> {
    let body: RemoteRegistryBody = serde_json::from_slice(bytes)?;
    if let Some(version) = body.version {
        if version > 1 {
            return Err(anyhow!(
                "unsupported remote model-registry version {}",
                version
            ));
        }
    }
    Ok(body
        .models
        .into_iter()
        .map(|entry| {
            (
                entry.name.clone(),
                ModelRegistryEntry {
                    name: entry.name,
                    max_context_window: entry.max_context_window,
                    max_output_tokens: entry.max_output_tokens,
                    preferred_instance: entry.preferred_instance,
                },
            )
        })
        .collect())
}

/// Apply TOML model extensions.
pub fn apply_toml_extensions(
    entries: &[TomlModelRegistryEntry],
) -> HashMap<String, ModelRegistryEntry> {
    entries
        .iter()
        .map(|entry| {
            (
                entry.name.clone(),
                ModelRegistryEntry {
                    name: entry.name.clone(),
                    max_context_window: entry.max_context_window,
                    max_output_tokens: entry.max_output_tokens,
                    preferred_instance: entry.preferred_instance.clone(),
                },
            )
        })
        .collect()
}

/// Merge builtin, remote, and TOML extension layers.
pub fn merge_layers(
    builtin: HashMap<String, ModelRegistryEntry>,
    remote: HashMap<String, ModelRegistryEntry>,
    toml: HashMap<String, ModelRegistryEntry>,
) -> ModelRegistrySnapshot {
    let mut models = builtin;
    models.extend(remote);
    models.extend(toml);
    ModelRegistrySnapshot {
        models,
        sources: RegistryProvenance {
            builtin_version: Some("builtin-v1".to_string()),
            remote_version: Some("remote-v1".to_string()),
            extensions_version: Some("toml-v1".to_string()),
        },
    }
}

/// Fetch remote registry JSON.
pub async fn fetch_remote(url: &str, timeout: Duration) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("failed to build remote registry client")?;
    let response = client.get(url).send().await?;
    let status = response.status();
    let body = response.bytes().await?;
    if !status.is_success() {
        return Err(anyhow!("remote registry returned {}", status));
    }
    Ok(body.to_vec())
}
