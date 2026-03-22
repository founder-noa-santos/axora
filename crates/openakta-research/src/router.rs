//! Ordered provider chain with automatic fallback.

use std::sync::Arc;

use crate::error::SearchError;
use crate::normalize::normalize_results;
use crate::provider::SearchProvider;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

/// Executes search against a tiered provider list (Serper → Tavily → …).
pub struct SearchRouter {
    chain: Vec<Arc<dyn SearchProvider>>,
}

impl SearchRouter {
    pub fn new(chain: Vec<Arc<dyn SearchProvider>>) -> Self {
        Self { chain }
    }

    /// Returns an empty router (always errors with [`SearchError::NoProvidersInChain`]).
    pub fn empty() -> Self {
        Self { chain: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Run query with fallback on retryable errors (429, 5xx, transport, parse).
    pub async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        if self.chain.is_empty() {
            return Err(SearchError::NoProvidersInChain);
        }

        let mut last_err: Option<SearchError> = None;
        for provider in &self.chain {
            match provider.search(query, opts).await {
                Ok(v) => return Ok(normalize_results(v, opts)),
                Err(e) if e.is_retryable() => {
                    tracing::debug!(
                        provider = provider.name(),
                        error = %e,
                        "research provider failed; trying next"
                    );
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }

        let last = last_err.unwrap_or(SearchError::NoProvidersInChain);
        Err(SearchError::AllProvidersExhausted(Box::new(last)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct OkProvider(Vec<SearchResult>);
    struct FailHttp(u16);
    struct FailFatal;

    #[async_trait]
    impl SearchProvider for OkProvider {
        fn name(&self) -> &'static str {
            "ok"
        }
        async fn search(
            &self,
            _query: &SearchQuery,
            _opts: &SearchOptions,
        ) -> Result<Vec<SearchResult>, SearchError> {
            Ok(self.0.clone())
        }
    }

    #[async_trait]
    impl SearchProvider for FailHttp {
        fn name(&self) -> &'static str {
            "http_fail"
        }
        async fn search(
            &self,
            _query: &SearchQuery,
            _opts: &SearchOptions,
        ) -> Result<Vec<SearchResult>, SearchError> {
            Err(SearchError::Http {
                status: Some(self.0),
                provider: "http_fail",
                message: "simulated".into(),
            })
        }
    }

    #[async_trait]
    impl SearchProvider for FailFatal {
        fn name(&self) -> &'static str {
            "fatal"
        }
        async fn search(
            &self,
            _query: &SearchQuery,
            _opts: &SearchOptions,
        ) -> Result<Vec<SearchResult>, SearchError> {
            Err(SearchError::Http {
                status: Some(400),
                provider: "fatal",
                message: "nope".into(),
            })
        }
    }

    #[tokio::test]
    async fn falls_back_on_503() {
        let hit = SearchResult {
            title: "t".into(),
            url: "u".into(),
            snippet: "s".into(),
        };
        let router = SearchRouter::new(vec![
            Arc::new(FailHttp(503)),
            Arc::new(OkProvider(vec![hit.clone()])),
        ]);
        let out = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "t");
    }

    #[tokio::test]
    async fn falls_back_on_429_rate_limit() {
        let hit = SearchResult {
            title: "r".into(),
            url: "u".into(),
            snippet: "s".into(),
        };
        let router = SearchRouter::new(vec![
            Arc::new(FailHttp(429)),
            Arc::new(OkProvider(vec![hit])),
        ]);
        let out = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(out[0].title, "r");
    }

    #[tokio::test]
    async fn falls_back_on_500() {
        let hit = SearchResult {
            title: "five".into(),
            url: "u".into(),
            snippet: "s".into(),
        };
        let router = SearchRouter::new(vec![
            Arc::new(FailHttp(500)),
            Arc::new(OkProvider(vec![hit])),
        ]);
        let out = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(out[0].title, "five");
    }

    #[tokio::test]
    async fn parse_error_is_retryable_and_falls_back() {
        let router = SearchRouter::new(vec![
            Arc::new(AlwaysParseFail),
            Arc::new(OkProvider(vec![SearchResult {
                title: "p".into(),
                url: "u".into(),
                snippet: "s".into(),
            }])),
        ]);
        let out = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(out[0].title, "p");
    }

    struct AlwaysParseFail;

    #[async_trait]
    impl SearchProvider for AlwaysParseFail {
        fn name(&self) -> &'static str {
            "parse_fail"
        }
        async fn search(
            &self,
            _query: &SearchQuery,
            _opts: &SearchOptions,
        ) -> Result<Vec<SearchResult>, SearchError> {
            Err(SearchError::Parse {
                provider: "parse_fail",
                message: "bad json".into(),
            })
        }
    }

    #[tokio::test]
    async fn stops_on_fatal() {
        let router = SearchRouter::new(vec![Arc::new(FailFatal), Arc::new(OkProvider(vec![]))]);
        let err = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn all_providers_retryable_exhausted() {
        let router = SearchRouter::new(vec![
            Arc::new(FailHttp(429)),
            Arc::new(FailHttp(500)),
        ]);
        let err = router
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap_err();
        match err {
            SearchError::AllProvidersExhausted(inner) => {
                assert!(inner.is_retryable() || matches!(*inner, SearchError::Http { .. }));
            }
            _ => panic!("expected AllProvidersExhausted, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn empty_chain_errors() {
        let router = SearchRouter::empty();
        assert!(matches!(
            router
                .search(
                    &SearchQuery { q: "q".into() },
                    &SearchOptions::default(),
                )
                .await,
            Err(SearchError::NoProvidersInChain)
        ));
    }
}
