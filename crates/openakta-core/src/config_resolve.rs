//! Config merge and runtime-resolution helpers.

use crate::CoreConfig;
use anyhow::{anyhow, Context};
use openakta_agents::{
    ModelRegistryEntry, ModelRegistrySnapshot, ProviderInstanceId, ProviderInstancesConfig,
    ProviderRuntimeBundle, ResolvedProviderInstance, SecretRef, TomlModelRegistryEntry,
};
use secrecy::SecretString;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Partial config patch sourced from a workspace-global file.
pub type CoreConfigPatch = toml::Value;

/// Load project config from a specific path (`workspace_root/openakta.toml`).
pub fn load_project_config(path: &Path) -> anyhow::Result<CoreConfig> {
    CoreConfig::from_project_file(&path.to_path_buf())
}

/// Load an optional workspace-global config overlay.
pub fn load_workspace_overlay() -> anyhow::Result<Option<CoreConfigPatch>> {
    let path = workspace_overlay_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read workspace overlay at {}", path.display()))?;
    let patch = toml::from_str(&content)?;
    Ok(Some(patch))
}

/// Merge defaults, optional workspace patch, and project config.
pub fn merge_config_layers(
    defaults: CoreConfig,
    workspace: Option<CoreConfigPatch>,
    project: CoreConfig,
) -> anyhow::Result<CoreConfig> {
    let mut merged = toml::Value::try_from(defaults)?;
    if let Some(workspace) = workspace {
        merge_toml(&mut merged, workspace);
    }
    merge_toml(&mut merged, toml::Value::try_from(project)?);
    Ok(merged.try_into()?)
}

/// Resolve provider secrets from config and project-root-relative files.
pub fn resolve_secrets(
    project_root: &Path,
    instances: &ProviderInstancesConfig,
) -> anyhow::Result<HashMap<ProviderInstanceId, SecretString>> {
    let mut secrets = HashMap::new();
    for (instance_id, instance) in &instances.instances {
        if let Some(secret) = resolve_secret_ref(project_root, &instance.secret)? {
            secrets.insert(instance_id.clone(), secret);
        }
    }
    Ok(secrets)
}

/// Build the resolved provider runtime bundle.
pub fn build_provider_bundle(
    core: &CoreConfig,
    secrets: &HashMap<ProviderInstanceId, SecretString>,
) -> anyhow::Result<ProviderRuntimeBundle> {
    let mut instances = HashMap::new();
    for (id, instance) in &core.providers.instances {
        validate_instance_id(id)?;
        instances.insert(
            id.clone(),
            ResolvedProviderInstance {
                id: id.clone(),
                profile: instance.profile,
                base_url: instance.base_url.clone(),
                api_key: secrets.get(id).cloned(),
                is_local: instance.is_local,
                default_model: instance.default_model.clone(),
                label: instance.label.clone(),
            },
        );
    }
    Ok(ProviderRuntimeBundle {
        instances,
        http: core.provider_runtime.clone(),
    })
}

/// Build the runtime model-registry snapshot from builtin, remote, and TOML layers.
pub async fn build_model_registry_snapshot(
    core: &CoreConfig,
) -> anyhow::Result<ModelRegistrySnapshot> {
    let builtin = openakta_agents::model_registry::builtin_catalog();
    tracing::info!("Loaded {} builtin models", builtin.len());

    let remote = if let Some(remote) = &core.remote_registry {
        tracing::info!("Fetching remote registry from: {}", remote.url);
        let timeout = Duration::from_secs(remote.http_timeout_secs.unwrap_or(5) as u64);
        let body = openakta_agents::model_registry::fetch_remote(&remote.url, timeout).await?;
        let remote_models = openakta_agents::model_registry::parse_remote_json(&body)?;
        tracing::info!("Loaded {} remote models from registry", remote_models.len());
        remote_models
    } else {
        tracing::warn!("No remote registry configured");
        HashMap::new()
    };

    let toml = apply_toml_extensions(&core.registry_models);
    if !toml.is_empty() {
        tracing::info!("Loaded {} TOML model extensions", toml.len());
    }

    let merged = openakta_agents::model_registry::merge_layers(builtin, remote, toml);
    tracing::info!("Total models in registry: {}", merged.models.len());
    Ok(merged)
}

fn resolve_secret_ref(
    project_root: &Path,
    secret: &SecretRef,
) -> anyhow::Result<Option<SecretString>> {
    if let Some(path) = &secret.api_key_file {
        let resolved = if path.is_absolute() {
            path.clone()
        } else {
            project_root.join(path)
        };

        if !resolved.exists() {
            anyhow::bail!(
                "API key file not found at: {}\n\n\
                 To fix this:\n\
                 1. Create the file: mkdir -p {}\n\
                 2. Add your API key: echo 'your-api-key-here' > {}\n\n\
                 Or configure the API key directly in openakta.toml using api_key instead of api_key_file.",
                resolved.display(),
                resolved.parent().unwrap_or(project_root).display(),
                resolved.display()
            );
        }

        let value = std::fs::read_to_string(&resolved)
            .with_context(|| format!("failed to read api key file {}", resolved.display()))?;
        return Ok(Some(SecretString::new(value.trim().to_string())));
    }
    Ok(secret.api_key.clone().map(SecretString::new))
}

fn validate_instance_id(id: &ProviderInstanceId) -> anyhow::Result<()> {
    if id.0.trim().is_empty() {
        return Err(anyhow!("provider instance id cannot be empty"));
    }
    Ok(())
}

fn apply_toml_extensions(
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

fn workspace_overlay_path() -> PathBuf {
    if let Ok(path) = std::env::var("OPENAKTA_CONFIG_DIR") {
        return PathBuf::from(path).join("config.toml");
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config/openakta/config.toml")
}

fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                match base_table.get_mut(&key) {
                    Some(existing) => merge_toml(existing, value),
                    None => {
                        base_table.insert(key, value);
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_agents::{ProviderInstanceConfig, ProviderProfileId, ProviderRuntimeConfig};
    use secrecy::ExposeSecret;

    #[test]
    fn empty_instance_id_is_rejected() {
        assert!(validate_instance_id(&ProviderInstanceId(String::new())).is_err());
    }

    #[test]
    fn secret_file_wins_over_inline_secret() {
        let tempdir = tempfile::tempdir().unwrap();
        let secret_path = tempdir.path().join("provider.key");
        std::fs::write(&secret_path, "from-file\n").unwrap();
        let secret = SecretRef {
            api_key: Some("inline".to_string()),
            api_key_file: Some(secret_path.file_name().unwrap().into()),
        };
        let resolved = resolve_secret_ref(tempdir.path(), &secret).unwrap();
        assert_eq!(resolved.unwrap().expose_secret(), "from-file");
    }

    #[test]
    fn bundle_builds_resolved_instances() {
        let mut config = CoreConfig {
            provider_runtime: ProviderRuntimeConfig::default(),
            ..Default::default()
        };
        config.providers.instances.insert(
            ProviderInstanceId("cloud".to_string()),
            ProviderInstanceConfig {
                profile: ProviderProfileId::OpenAiChatCompletions,
                base_url: "https://api.openai.com/v1".to_string(),
                secret: SecretRef::default(),
                is_local: false,
                default_model: Some("gpt-4o".to_string()),
                label: None,
            },
        );
        let secrets = HashMap::new();
        let bundle = build_provider_bundle(&config, &secrets).unwrap();
        assert!(bundle
            .instances
            .contains_key(&ProviderInstanceId("cloud".to_string())));
    }
}
