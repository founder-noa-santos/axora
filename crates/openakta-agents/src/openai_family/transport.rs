//! OpenAI-family transport implementation.

use crate::openai_family::adapter::{NormalizedStream, SdkClient};
use crate::openai_family::capabilities::{
    ModelCapabilityRegistry, ProviderCapabilities, ResolvedCapabilities,
};
use crate::openai_family::config::OpenAiFamilyConfig;
use crate::openai_family::error::TransportError;
use crate::provider::{ModelRequest, ModelResponse, ProviderKind};
use crate::provider_transport::ProviderRuntimeConfig;

use async_openai::types::CreateChatCompletionRequest;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use secrecy::ExposeSecret;
use serde_json::Value;

/// OpenAI-family transport.
pub struct OpenAiFamilyTransport {
    client: SdkClient,
    provider_name: String,
    raw_base_url: Option<String>,
    raw_api_key: String,
    http_client: reqwest::Client,
}

impl OpenAiFamilyTransport {
    /// Create a new OpenAI-family transport.
    pub fn new(
        config: OpenAiFamilyConfig,
        runtime_config: ProviderRuntimeConfig,
    ) -> Result<Self, TransportError> {
        let provider_name = config.provider_name().to_string();
        let raw_base_url = config.base_url().map(str::to_string);
        let raw_api_key = config.api_key().expose_secret().to_string();
        let client = SdkClient::new(&config)?;
        let http_client = reqwest::Client::builder()
            .timeout(runtime_config.timeout)
            .build()
            .map_err(|err| TransportError::Http(err.to_string()))?;

        // Resolve capabilities from hardcoded defaults
        // (In production, this would come from the registry)
        Ok(Self {
            client,
            provider_name,
            raw_base_url,
            raw_api_key,
            http_client,
        })
    }

    /// Get the provider name.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Get the capabilities.
    pub fn capabilities(&self) -> &ResolvedCapabilities {
        unreachable!("capabilities are resolved per request")
    }

    /// Execute a non-streaming request.
    pub async fn execute(&self, request: &ModelRequest) -> Result<ModelResponse, TransportError> {
        if self.raw_base_url.is_some() {
            let raw_response = self.execute_raw_chat_completion(request).await?;
            return match raw_response {
                SdkExecution::Raw(raw_response) => {
                    crate::openai_family::adapter::parse_raw_chat_completion_response(
                        &raw_response,
                        ProviderKind::OpenAi,
                    )
                }
                SdkExecution::Structured(sdk_response) => {
                    crate::openai_family::types::parse_sdk_response(
                        &sdk_response,
                        ProviderKind::OpenAi,
                    )
                }
            };
        }

        let capabilities = self.resolve_capabilities(&request.model);
        // Build SDK request
        let sdk_request = crate::openai_family::types::build_sdk_request(request, &capabilities)?;

        // Execute with SDK
        let sdk_response = self
            .client
            .inner()
            .chat()
            .create(sdk_request)
            .await
            .map(SdkExecution::Structured)
            .map_err(|err| TransportError::Sdk(err.to_string()))?;

        // Parse SDK response
        match sdk_response {
            SdkExecution::Structured(sdk_response) => {
                crate::openai_family::types::parse_sdk_response(&sdk_response, ProviderKind::OpenAi)
            }
            SdkExecution::Raw(raw_response) => {
                crate::openai_family::adapter::parse_raw_chat_completion_response(
                    &raw_response,
                    ProviderKind::OpenAi,
                )
            }
        }
    }

    /// Execute a streaming request.
    pub async fn execute_stream(
        &self,
        request: &ModelRequest,
    ) -> Result<NormalizedStream, TransportError> {
        let capabilities = self.resolve_capabilities(&request.model);
        // Build SDK request
        let sdk_request = crate::openai_family::types::build_sdk_request(request, &capabilities)?;

        // Execute with SDK
        let stream = self
            .client
            .inner()
            .chat()
            .create_stream(sdk_request)
            .await
            .map_err(|err| TransportError::Sdk(err.to_string()))?;

        Ok(NormalizedStream::new(stream))
    }

    /// Build an SDK request for external use (testing, etc.).
    pub fn build_request(
        &self,
        request: &ModelRequest,
    ) -> Result<CreateChatCompletionRequest, TransportError> {
        let capabilities = self.resolve_capabilities(&request.model);
        crate::openai_family::types::build_sdk_request(request, &capabilities)
    }

    fn resolve_capabilities(&self, model: &str) -> ResolvedCapabilities {
        let registry = ModelCapabilityRegistry::builtin();
        let model_capabilities = registry.resolve_for_model(model);
        ResolvedCapabilities::resolve(
            &ProviderCapabilities::hardcoded_defaults(&self.provider_name),
            model_capabilities,
        )
    }

    async fn execute_raw_chat_completion(
        &self,
        request: &ModelRequest,
    ) -> Result<SdkExecution, TransportError> {
        let capabilities = self.resolve_capabilities(&request.model);
        let sdk_request = crate::openai_family::types::build_sdk_request(request, &capabilities)?;
        let body = serde_json::to_value(&sdk_request)
            .map_err(|err| TransportError::Serialization(err.to_string()))?;
        let base_url = self
            .raw_base_url
            .as_deref()
            .ok_or_else(|| TransportError::Configuration("missing compatible base URL".into()))?;
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let response = self
            .http_client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.raw_api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|err| TransportError::Http(err.to_string()))?;
        let status = response.status();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|err| TransportError::Http(err.to_string()))?;
        if !status.is_success() {
            let message = payload
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| payload.to_string());
            return Err(TransportError::Http(format!("{status}: {message}")));
        }
        Ok(SdkExecution::Raw(payload))
    }
}

enum SdkExecution {
    Structured(async_openai::types::CreateChatCompletionResponse),
    Raw(Value),
}
