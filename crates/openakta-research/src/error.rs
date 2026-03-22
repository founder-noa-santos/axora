//! Search pipeline errors and retry classification.

use thiserror::Error;

/// HTTP status codes that trigger fallback to the next provider in the search router chain.
///
/// Includes **401** and **403** so an invalid or unauthorized key on one BYOK provider does not
/// block the rest of the chain. Excludes **400** (bad request) and **404** so callers can fail fast
/// on clearly malformed queries when a provider returns them.
#[inline]
pub fn is_retryable_http_status(status: u16) -> bool {
    status == 401
        || status == 403
        || status == 429
        || (500..=599).contains(&status)
}

/// Errors from providers or the router.
#[derive(Debug, Error)]
pub enum SearchError {
    /// HTTP failure with optional status (provider response).
    #[error("{provider} HTTP error: status={status:?} {message}")]
    Http {
        status: Option<u16>,
        provider: &'static str,
        message: String,
    },
    /// No API key configured for a named provider.
    #[error("missing API key for {provider}")]
    MissingApiKey { provider: &'static str },
    /// Response body could not be interpreted.
    #[error("{provider} parse error: {message}")]
    Parse {
        provider: &'static str,
        message: String,
    },
    /// Network / TLS / timeout (no HTTP status).
    #[error("{provider} transport: {message}")]
    Transport {
        provider: &'static str,
        message: String,
    },
    /// Every provider in the chain failed; last error preserved.
    #[error("all search providers failed: {0}")]
    AllProvidersExhausted(Box<SearchError>),
    /// Router built with an empty provider list.
    #[error("no search providers configured in router chain")]
    NoProvidersInChain,
}

impl SearchError {
    /// Whether the router should try the next provider in the chain.
    pub fn is_retryable(&self) -> bool {
        match self {
            SearchError::Transport { .. } => true,
            SearchError::Parse { .. } => true,
            SearchError::Http { status: None, .. } => true,
            SearchError::Http {
                status: Some(s), ..
            } => is_retryable_http_status(*s),
            SearchError::MissingApiKey { .. } => false,
            SearchError::AllProvidersExhausted(_) => false,
            SearchError::NoProvidersInChain => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_http_status_classification() {
        assert!(is_retryable_http_status(401));
        assert!(is_retryable_http_status(403));
        assert!(is_retryable_http_status(429));
        assert!(is_retryable_http_status(500));
        assert!(is_retryable_http_status(599));
        assert!(!is_retryable_http_status(428));
        assert!(!is_retryable_http_status(400));
        assert!(!is_retryable_http_status(404));
    }
}
