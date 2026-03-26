//! API client error types

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, ApiError>;

/// API error types
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("API unavailable: {0}")]
    Unavailable(String),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Circuit breaker open")]
    CircuitOpen,

    #[error("Quota exceeded")]
    QuotaExceeded { reset_at: Option<String> },

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Authentication required: {0}")]
    AuthRequired(String),

    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    #[error("Unauthenticated: {0}")]
    Unauthenticated(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Stream interrupted: {reason}")]
    StreamInterrupted { reason: String },

    #[error("Content filtered: {0}")]
    ContentFilter(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Transport error: {0}")]
    Transport(tonic::Status),

    #[error("Connection error: {0}")]
    Connection(tonic::transport::Error),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

impl From<tonic::Status> for ApiError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::Unauthenticated => ApiError::Unauthenticated(status.message().to_string()),
            tonic::Code::ResourceExhausted => ApiError::RateLimited(status.message().to_string()),
            tonic::Code::InvalidArgument => ApiError::InvalidRequest(status.message().to_string()),
            tonic::Code::Unavailable => ApiError::Unavailable(status.message().to_string()),
            tonic::Code::DeadlineExceeded => ApiError::Timeout(status.message().to_string()),
            _ => ApiError::Transport(status),
        }
    }
}

impl From<tonic::transport::Error> for ApiError {
    fn from(err: tonic::transport::Error) -> Self {
        ApiError::Connection(err)
    }
}

impl ApiError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ApiError::Unavailable(_)
                | ApiError::Timeout(_)
                | ApiError::ConnectionRefused(_)
                | ApiError::ProviderUnavailable(_)
        )
    }

    /// Check if fallback should be triggered
    pub fn should_trigger_fallback(&self) -> bool {
        matches!(
            self,
            ApiError::CircuitOpen | ApiError::Timeout(_) | ApiError::ConnectionRefused(_)
        )
    }
}
