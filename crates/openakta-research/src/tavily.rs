//! Tavily Search API (fallback / AI-oriented RAG-friendly results).
//!
//! API: <https://docs.tavily.com/>

use async_trait::async_trait;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::error::SearchError;
use crate::provider::SearchProvider;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

const PROVIDER: &str = "tavily";

/// Canonical Tavily API origin (never taken from untrusted config).
pub const TAVILY_CANONICAL_BASE_URL: &str = "https://api.tavily.com";

/// Tavily HTTP client.
pub struct TavilyClient {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: String,
}

impl TavilyClient {
    pub fn new(http: reqwest::Client, api_key: SecretString) -> Self {
        Self {
            http,
            api_key,
            base_url: TAVILY_CANONICAL_BASE_URL.trim_end_matches('/').to_string(),
        }
    }

    #[cfg(any(test, feature = "search-provider-mock-endpoints"))]
    pub fn new_with_endpoint_for_tests(
        http: reqwest::Client,
        api_key: SecretString,
        base_url: impl AsRef<str>,
    ) -> Self {
        Self {
            http,
            api_key,
            base_url: base_url.as_ref().trim_end_matches('/').to_string(),
        }
    }

    pub fn default_base_url() -> &'static str {
        TAVILY_CANONICAL_BASE_URL
    }
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Option<Vec<TavilyHit>>,
}

#[derive(Debug, Deserialize)]
struct TavilyHit {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    content: String,
}

/// Parse a **successful** (2xx) Tavily JSON body into [`SearchResult`] rows. Unit-testable without HTTP.
pub fn parse_tavily_response_body(body: &str) -> Result<Vec<SearchResult>, SearchError> {
    let parsed: TavilyResponse = serde_json::from_str(body).map_err(|e| SearchError::Parse {
        provider: PROVIDER,
        message: e.to_string(),
    })?;
    let results = parsed.results.unwrap_or_default();
    Ok(results
        .into_iter()
        .map(|h| SearchResult {
            title: h.title,
            url: h.url,
            snippet: h.content,
        })
        .collect())
}

#[async_trait]
impl SearchProvider for TavilyClient {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let url = format!("{}/search", self.base_url);
        let max = opts.max_results.clamp(1, 20) as u32;
        let body = serde_json::json!({
            "api_key": self.api_key.expose_secret(),
            "query": query.q,
            "max_results": max,
        });

        let resp = self
            .http
            .post(&url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::http_util::map_reqwest(PROVIDER, e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| SearchError::Transport {
                provider: PROVIDER,
                message: e.to_string(),
            })?;

        if !status.is_success() {
            return Err(SearchError::Http {
                status: Some(status.as_u16()),
                provider: PROVIDER,
                message: crate::http_util::truncate_body(&text),
            });
        }

        parse_tavily_response_body(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_malformed_json_returns_parse_error() {
        let err = parse_tavily_response_body("%%%").unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "tavily", .. }));
    }

    #[test]
    fn parse_empty_object_yields_empty() {
        assert!(parse_tavily_response_body("{}").unwrap().is_empty());
    }

    #[test]
    fn parse_results_null_yields_empty() {
        assert!(parse_tavily_response_body(r#"{"results":null}"#)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parse_one_hit() {
        let j = r#"{"results":[{"title":"T","url":"https://u","content":"C"}]}"#;
        let v = parse_tavily_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet, "C");
    }
}
