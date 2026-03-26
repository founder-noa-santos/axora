//! Error types for OpenAI-family transport.

use thiserror::Error;

/// Transport errors for OpenAI-family providers.
#[derive(Debug, Error)]
pub enum TransportError {
    /// SDK error from async-openai.
    #[error("SDK error: {0}")]
    Sdk(String),

    /// SDK request build error.
    #[error("SDK request build failed: {0}")]
    SdkRequestBuild(String),

    /// SDK stream error.
    #[error("SDK stream error: {0}")]
    SdkStream(String),

    /// Capability not supported by provider.
    #[error("capability not supported: {0}")]
    CapabilityNotSupported(String),

    /// Max tokens exceeded provider limit.
    #[error("max tokens exceeded: requested {requested}, max {max}")]
    MaxTokensExceeded { requested: u32, max: u32 },

    /// Temperature out of valid range.
    #[error("invalid temperature: {value} (must be {min}-{max})")]
    InvalidTemperature { value: f32, min: f32, max: f32 },

    /// Message role not recognized.
    #[error("invalid message role: {0}")]
    InvalidMessageRole(String),

    /// Tool schema validation failed.
    #[error("invalid tool schema: {0}")]
    InvalidToolSchema(String),

    /// Response had no choices.
    #[error("no choices in response")]
    NoChoicesInResponse,

    /// Authentication failed.
    #[error("authentication failed")]
    AuthenticationFailed,

    /// Rate limit exceeded.
    #[error("rate limit exceeded")]
    RateLimitExceeded { retry_after_ms: Option<u64> },

    /// Server error from provider.
    #[error("server error: {0}")]
    ServerError(String),

    /// Request timeout.
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(String),
}

impl TransportError {
    /// Classify error for retry decision.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TransportError::ServerError(_)
                | TransportError::Timeout(_)
                | TransportError::RateLimitExceeded { .. }
                | TransportError::Http(_)
        )
    }

    /// Map to HTTP status code for observability.
    pub fn status_code(&self) -> u16 {
        match self {
            TransportError::AuthenticationFailed => 401,
            TransportError::RateLimitExceeded { .. } => 429,
            TransportError::CapabilityNotSupported(_) => 400,
            TransportError::MaxTokensExceeded { .. } => 400,
            TransportError::InvalidTemperature { .. } => 400,
            TransportError::InvalidMessageRole(_) => 400,
            TransportError::InvalidToolSchema(_) => 400,
            TransportError::ServerError(_) => 500,
            TransportError::Timeout(_) => 408,
            TransportError::SdkRequestBuild(_) => 400,
            _ => 500,
        }
    }

    /// Get error class for metrics.
    pub fn error_class(&self) -> ErrorClass {
        match self {
            TransportError::AuthenticationFailed => ErrorClass::Authentication,
            TransportError::RateLimitExceeded { .. } => ErrorClass::RateLimit,
            TransportError::ServerError(_) => ErrorClass::ServerError,
            TransportError::Timeout(_) => ErrorClass::Timeout,
            TransportError::CapabilityNotSupported(_)
            | TransportError::MaxTokensExceeded { .. }
            | TransportError::InvalidTemperature { .. }
            | TransportError::InvalidMessageRole(_)
            | TransportError::InvalidToolSchema(_)
            | TransportError::SdkRequestBuild(_) => ErrorClass::ClientError,
            TransportError::Serialization(_) => ErrorClass::Serialization,
            _ => ErrorClass::ServerError,
        }
    }

    /// Safe error message that doesn't leak secrets.
    pub fn safe_message(&self) -> String {
        match self {
            TransportError::AuthenticationFailed => "Authentication failed".into(),
            _ => self.to_string(),
        }
    }
}

/// Error classification for metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Authentication,
    RateLimit,
    ServerError,
    Timeout,
    ClientError,
    Serialization,
}
