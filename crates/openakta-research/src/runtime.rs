//! Configuration-driven construction of [`SearchRouter`] and default [`SearchOptions`].

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::brave::BraveClient;
use crate::exa::ExaClient;
use crate::provider::SearchProvider;
use crate::router::SearchRouter;
use crate::serper::SerperClient;
use crate::tavily::TavilyClient;
use crate::types::SearchOptions;

fn default_http_timeout_secs() -> u64 {
    30
}

fn default_max_results() -> u8 {
    5
}

fn default_max_snippet_chars() -> usize {
    280
}

fn default_max_title_chars() -> usize {
    120
}

fn default_provider_chain() -> Vec<String> {
    vec!["serper".to_string(), "tavily".to_string()]
}

/// TOML `[research]` section (BYOK secret file paths).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConfig {
    #[serde(default = "default_http_timeout_secs")]
    pub http_timeout_secs: u64,
    #[serde(default = "default_max_results")]
    pub max_results_default: u8,
    #[serde(default = "default_max_snippet_chars")]
    pub max_snippet_chars: usize,
    #[serde(default = "default_max_title_chars")]
    pub max_title_chars: usize,
    #[serde(default = "default_provider_chain")]
    pub provider_chain: Vec<String>,
    #[serde(default)]
    pub serper: Option<SerperProviderConfig>,
    #[serde(default)]
    pub tavily: Option<TavilyProviderConfig>,
    #[serde(default)]
    pub brave: Option<BraveProviderConfig>,
    #[serde(default)]
    pub exa: Option<ExaProviderConfig>,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            http_timeout_secs: default_http_timeout_secs(),
            max_results_default: default_max_results(),
            max_snippet_chars: default_max_snippet_chars(),
            max_title_chars: default_max_title_chars(),
            provider_chain: default_provider_chain(),
            serper: None,
            tavily: None,
            brave: None,
            exa: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerperProviderConfig {
    pub api_key_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavilyProviderConfig {
    pub api_key_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraveProviderConfig {
    pub api_key_file: PathBuf,
}

fn default_exa_search_type() -> String {
    "neural".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExaProviderConfig {
    pub api_key_file: PathBuf,
    /// Exa `type` field: `neural` (embedding search), `auto`, `fast`, `instant`, etc.
    #[serde(default = "default_exa_search_type")]
    pub search_type: String,
    /// Optional category filter (e.g. `research paper` for papers).
    #[serde(default)]
    pub category: Option<String>,
    /// Restrict to domains (e.g. `github.com`, `docs.rs`) for code and technical docs.
    #[serde(default)]
    pub include_domains: Vec<String>,
}

/// Resolved router + token defaults for agent/MCP callers.
pub struct ResearchRuntime {
    pub router: SearchRouter,
    pub default_options: SearchOptions,
}

impl ResearchRuntime {
    /// Build from workspace root and `[research]` config. Providers without a readable non-empty key file are skipped.
    pub fn from_workspace(project_root: &Path, config: &ResearchConfig) -> anyhow::Result<Self> {
        let timeout = Duration::from_secs(config.http_timeout_secs.max(1));
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("build reqwest client for research")?;

        let mut chain: Vec<Arc<dyn SearchProvider>> = Vec::new();

        for id in &config.provider_chain {
            match id.as_str() {
                "serper" => {
                    if let Some(ref pcfg) = config.serper {
                        if let Some(key) = read_secret_file(project_root, &pcfg.api_key_file)? {
                            chain.push(Arc::new(SerperClient::new(client.clone(), key)));
                        }
                    }
                }
                "tavily" => {
                    if let Some(ref pcfg) = config.tavily {
                        if let Some(key) = read_secret_file(project_root, &pcfg.api_key_file)? {
                            chain.push(Arc::new(TavilyClient::new(client.clone(), key)));
                        }
                    }
                }
                "brave" => {
                    if let Some(ref pcfg) = config.brave {
                        if let Some(key) = read_secret_file(project_root, &pcfg.api_key_file)? {
                            chain.push(Arc::new(BraveClient::new(client.clone(), key)));
                        }
                    }
                }
                "exa" => {
                    if let Some(ref pcfg) = config.exa {
                        if let Some(key) = read_secret_file(project_root, &pcfg.api_key_file)? {
                            let st = if pcfg.search_type.is_empty() {
                                ExaClient::default_search_type_neural().to_string()
                            } else {
                                pcfg.search_type.clone()
                            };
                            chain.push(Arc::new(ExaClient::new(
                                client.clone(),
                                key,
                                st,
                                pcfg.category.clone(),
                                pcfg.include_domains.clone(),
                            )));
                        }
                    }
                }
                other => {
                    tracing::warn!(
                        provider = other,
                        "unknown research provider id in provider_chain; skipping"
                    );
                }
            }
        }

        let default_options = SearchOptions {
            max_results: config.max_results_default as usize,
            max_snippet_chars: config.max_snippet_chars,
            max_title_chars: config.max_title_chars,
        };

        Ok(Self {
            router: SearchRouter::new(chain),
            default_options,
        })
    }
}

fn read_secret_file(project_root: &Path, path: &Path) -> anyhow::Result<Option<SecretString>> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    if !resolved.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&resolved)
        .with_context(|| format!("read research API key {}", resolved.display()))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(SecretString::new(trimmed.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_workspace_skips_when_no_key_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = ResearchConfig::default();
        let rt = ResearchRuntime::from_workspace(tmp.path(), &cfg).unwrap();
        assert!(rt.router.is_empty());
    }

    #[test]
    fn from_workspace_loads_serper_when_key_present() {
        let tmp = tempfile::tempdir().unwrap();
        let key_path = tmp.path().join("serper.key");
        std::fs::write(&key_path, "test-key-ascii-only\n").unwrap();
        let cfg = ResearchConfig {
            provider_chain: vec!["serper".to_string()],
            serper: Some(SerperProviderConfig {
                api_key_file: key_path,
            }),
            ..Default::default()
        };
        let rt = ResearchRuntime::from_workspace(tmp.path(), &cfg).unwrap();
        assert!(!rt.router.is_empty());
    }

    #[test]
    fn from_workspace_loads_brave_when_key_present() {
        let tmp = tempfile::tempdir().unwrap();
        let key_path = tmp.path().join(".openakta/secrets/brave.key");
        std::fs::create_dir_all(key_path.parent().unwrap()).unwrap();
        std::fs::write(&key_path, "brave-subscription-token\n").unwrap();
        let cfg = ResearchConfig {
            provider_chain: vec!["brave".to_string()],
            brave: Some(BraveProviderConfig {
                api_key_file: PathBuf::from(".openakta/secrets/brave.key"),
            }),
            ..Default::default()
        };
        let rt = ResearchRuntime::from_workspace(tmp.path(), &cfg).unwrap();
        assert!(!rt.router.is_empty());
    }

    #[test]
    fn from_workspace_loads_exa_when_key_present() {
        let tmp = tempfile::tempdir().unwrap();
        let key_path = tmp.path().join("exa.key");
        std::fs::write(&key_path, "exa-api-key\n").unwrap();
        let cfg = ResearchConfig {
            provider_chain: vec!["exa".to_string()],
            exa: Some(ExaProviderConfig {
                api_key_file: key_path,
                search_type: "neural".to_string(),
                category: None,
                include_domains: vec!["github.com".into()],
            }),
            ..Default::default()
        };
        let rt = ResearchRuntime::from_workspace(tmp.path(), &cfg).unwrap();
        assert!(!rt.router.is_empty());
    }
}
