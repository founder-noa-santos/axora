//! SDK client wrapper.

use async_openai::{config::OpenAIConfig, Client};
use secrecy::ExposeSecret;

use crate::openai_family::config::OpenAiFamilyConfig;
use crate::openai_family::error::TransportError;

/// SDK client wrapper that owns the async-openai Client.
pub struct SdkClient {
    inner: Client<OpenAIConfig>,
    provider_name: String,
}

impl SdkClient {
    /// Create a new SDK client from configuration.
    pub fn new(config: &OpenAiFamilyConfig) -> Result<Self, TransportError> {
        let provider_name = config.provider_name().to_string();

        let openai_config = match config {
            OpenAiFamilyConfig::Official(cfg) => {
                let mut openai_config =
                    OpenAIConfig::new().with_api_key(cfg.api_key.expose_secret());

                if let Some(org_id) = &cfg.organization {
                    openai_config = openai_config.with_org_id(org_id);
                }

                if let Some(project_id) = &cfg.project {
                    openai_config = openai_config.with_project_id(project_id);
                }

                openai_config
            }
            OpenAiFamilyConfig::Compatible(cfg) => OpenAIConfig::new()
                .with_api_key(cfg.api_key.expose_secret())
                .with_api_base(&cfg.base_url),
        };

        let client = Client::with_config(openai_config);

        Ok(Self {
            inner: client,
            provider_name,
        })
    }

    /// Get the inner client.
    pub fn inner(&self) -> &Client<OpenAIConfig> {
        &self.inner
    }

    /// Get the provider name.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }
}
