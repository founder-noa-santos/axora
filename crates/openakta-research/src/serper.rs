//! Serper.dev Google Search API (primary provider).
//!
//! API: <https://serper.dev/>

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::error::SearchError;
use crate::provider::SearchProvider;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

const PROVIDER: &str = "serper";

/// Canonical Serper API origin (never taken from untrusted config — SSRF / BYOK exfiltration guard).
pub const SERPER_CANONICAL_BASE_URL: &str = "https://google.serper.dev";

/// Serper HTTP client.
pub struct SerperClient {
    http: reqwest::Client,
    api_key: SecretString,
    /// Only [`SERPER_CANONICAL_BASE_URL`] in production; tests may inject a mock origin via
    /// [`SerperClient::new_with_endpoint_for_tests`].
    base_url: String,
}

impl SerperClient {
    /// Production constructor: requests always go to [`SERPER_CANONICAL_BASE_URL`].
    pub fn new(http: reqwest::Client, api_key: SecretString) -> Self {
        Self {
            http,
            api_key,
            base_url: SERPER_CANONICAL_BASE_URL.trim_end_matches('/').to_string(),
        }
    }

    /// Wiremock / local integration tests only — never call with user-controlled URLs in production.
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
        SERPER_CANONICAL_BASE_URL
    }
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    /// `null` or missing → treated as empty (no panic).
    #[serde(default)]
    organic: Option<Vec<SerperOrganic>>,
}

#[derive(Debug, Deserialize)]
struct SerperOrganic {
    #[serde(default)]
    title: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    snippet: String,
}

/// Parse a **successful** (2xx) Serper JSON body into [`SearchResult`] rows. Unit-testable without HTTP.
pub fn parse_serper_response_body(body: &str) -> Result<Vec<SearchResult>, SearchError> {
    let parsed: SerperResponse = serde_json::from_str(body).map_err(|e| SearchError::Parse {
        provider: PROVIDER,
        message: e.to_string(),
    })?;
    let organic = parsed.organic.unwrap_or_default();
    Ok(organic
        .into_iter()
        .map(|o| SearchResult {
            title: o.title,
            url: o.link,
            snippet: o.snippet,
        })
        .collect())
}

#[async_trait]
impl SearchProvider for SerperClient {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let url = format!("{}/search", self.base_url);
        let body = serde_json::json!({ "q": query.q, "num": opts.max_results.min(20) });
        let mut headers = HeaderMap::new();
        let key_header = HeaderValue::from_str(self.api_key.expose_secret()).map_err(|e| {
            SearchError::Transport {
                provider: PROVIDER,
                message: format!("invalid X-API-KEY header: {e}"),
            }
        })?;
        headers.insert("X-API-KEY", key_header);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let resp = self
            .http
            .post(&url)
            .headers(headers)
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

        parse_serper_response_body(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_malformed_json_returns_parse_error() {
        let err = parse_serper_response_body("not json {{{").unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "serper", .. }));
    }

    #[test]
    fn parse_empty_object_yields_empty_hits() {
        assert!(parse_serper_response_body("{}").unwrap().is_empty());
    }

    #[test]
    fn parse_organic_null_yields_empty() {
        let j = r#"{"organic":null}"#;
        assert!(parse_serper_response_body(j).unwrap().is_empty());
    }

    #[test]
    fn parse_organic_missing_yields_empty() {
        assert!(parse_serper_response_body(r#"{"other":1}"#).unwrap().is_empty());
    }

    #[test]
    fn parse_one_hit() {
        let j = r#"{"organic":[{"title":"T","link":"https://x","snippet":"S"}]}"#;
        let v = parse_serper_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "T");
        assert_eq!(v[0].url, "https://x");
        assert_eq!(v[0].snippet, "S");
    }

    #[test]
    fn parse_partial_hit_uses_defaults() {
        let j = r#"{"organic":[{}]}"#;
        let v = parse_serper_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].title.is_empty());
        assert!(v[0].url.is_empty());
        assert!(v[0].snippet.is_empty());
    }
}
