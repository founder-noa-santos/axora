//! ProviderTransport wrapper for OpenAiFamilyTransport.
//!
//! This allows the new SDK-based transport to be used interchangeably
//! with the legacy HTTP transport.

use crate::openai_family::{OpenAiFamilyConfig, OpenAiFamilyTransport, TransportError};
use crate::provider::{ModelRequest, ModelResponse, ProviderKind};
use crate::provider_transport::{ProviderRuntimeConfig, ProviderTransport, ProviderTransportError};
use tonic::async_trait;

/// Wrapper that implements ProviderTransport for OpenAiFamilyTransport.
pub struct OpenAiFamilyTransportWrapper {
    inner: OpenAiFamilyTransport,
}

impl OpenAiFamilyTransportWrapper {
    /// Create a new wrapper from OpenAI-family config.
    pub fn new(
        config: OpenAiFamilyConfig,
        runtime_config: ProviderRuntimeConfig,
    ) -> Result<Self, ProviderTransportError> {
        let inner = OpenAiFamilyTransport::new(config, runtime_config)
            .map_err(|e| ProviderTransportError::Build(e.to_string()))?;

        Ok(Self { inner })
    }
}

#[async_trait]
impl ProviderTransport for OpenAiFamilyTransportWrapper {
    async fn execute(
        &self,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ProviderTransportError> {
        self.inner.execute(request).await.map_err(|e| match e {
            TransportError::AuthenticationFailed => {
                ProviderTransportError::MissingCredentials(ProviderKind::OpenAi)
            }
            TransportError::Timeout(_) => ProviderTransportError::Http("timeout".into()),
            _ => ProviderTransportError::Http(e.to_string()),
        })
    }

    fn mode(&self) -> &'static str {
        "sdk"
    }
}
