//! Brave Search API (Web Search).
//!
//! API: <https://api-dashboard.search.brave.com/app/documentation/web-search/query>
//!
//! Authenticates with header `X-Subscription-Token`. Responses expose `web.results[]` with
//! `title`, `url`, and `description` (snippet).

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::error::SearchError;
use crate::provider::SearchProvider;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

const PROVIDER: &str = "brave";

/// Canonical Brave Search API origin (never taken from untrusted config).
pub const BRAVE_CANONICAL_BASE_URL: &str = "https://api.search.brave.com";

/// Brave Web Search HTTP client (BYOK). Accepts an injected [`reqwest::Client`] for tests and
/// connection pooling.
pub struct BraveClient {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: String,
}

impl BraveClient {
    pub fn new(http: reqwest::Client, api_key: SecretString) -> Self {
        Self {
            http,
            api_key,
            base_url: BRAVE_CANONICAL_BASE_URL.trim_end_matches('/').to_string(),
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
        BRAVE_CANONICAL_BASE_URL
    }
}

#[derive(Debug, Deserialize)]
struct BraveResponse {
    #[serde(default)]
    web: Option<BraveWeb>,
}

#[derive(Debug, Deserialize)]
struct BraveWeb {
    #[serde(default)]
    results: Option<Vec<Option<BraveWebResult>>>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResult {
    #[serde(default, deserialize_with = "crate::serde_util::lenient_string_or_default")]
    title: String,
    #[serde(default, deserialize_with = "crate::serde_util::lenient_string_or_default")]
    url: String,
    #[serde(default, deserialize_with = "crate::serde_util::lenient_string_or_default")]
    description: String,
}

/// Parse a **successful** (2xx) Brave JSON body into [`SearchResult`] rows.
pub fn parse_brave_response_body(body: &str) -> Result<Vec<SearchResult>, SearchError> {
    let parsed: BraveResponse = serde_json::from_str(body).map_err(|e| SearchError::Parse {
        provider: PROVIDER,
        message: e.to_string(),
    })?;
    let results = parsed
        .web
        .and_then(|w| w.results)
        .unwrap_or_default();
    Ok(results
        .into_iter()
        .filter_map(|row| {
            row.map(|r| SearchResult {
                title: r.title,
                url: r.url,
                snippet: r.description,
            })
        })
        .collect())
}

#[async_trait]
impl SearchProvider for BraveClient {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let count = opts.max_results.clamp(1, 20) as u32;
        let url = format!("{}/res/v1/web/search", self.base_url);

        let mut headers = HeaderMap::new();
        let token = HeaderValue::from_str(self.api_key.expose_secret()).map_err(|e| {
            SearchError::Transport {
                provider: PROVIDER,
                message: format!("invalid X-Subscription-Token: {e}"),
            }
        })?;
        headers.insert("X-Subscription-Token", token);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let resp = self
            .http
            .get(&url)
            .headers(headers)
            .query(&[("q", query.q.as_str()), ("count", &count.to_string())])
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

        parse_brave_response_body(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parse_malformed_json_returns_parse_error() {
        let err = parse_brave_response_body("not json").unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "brave", .. }));
    }

    #[test]
    fn parse_empty_object_yields_empty() {
        assert!(parse_brave_response_body("{}").unwrap().is_empty());
    }

    #[test]
    fn parse_web_null_yields_empty() {
        let j = r#"{"web":null}"#;
        assert!(parse_brave_response_body(j).unwrap().is_empty());
    }

    #[test]
    fn parse_results_null_yields_empty() {
        let j = r#"{"web":{"results":null}}"#;
        assert!(parse_brave_response_body(j).unwrap().is_empty());
    }

    #[test]
    fn parse_web_not_object_errors() {
        let j = r#"{"web":"oops"}"#;
        let err = parse_brave_response_body(j).unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "brave", .. }));
    }

    #[test]
    fn parse_results_not_array_errors() {
        let j = r#"{"web":{"results":"bad"}}"#;
        let err = parse_brave_response_body(j).unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "brave", .. }));
    }

    #[test]
    fn parse_null_row_skipped() {
        let j = r#"{"web":{"results":[null,{"title":"A","url":"https://a","description":""}]}}"#;
        let v = parse_brave_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "A");
    }

    #[test]
    fn parse_one_hit_maps_description_to_snippet() {
        let j = r#"{"web":{"results":[{"title":"T","url":"https://x","description":"snippet text"}]}}"#;
        let v = parse_brave_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "T");
        assert_eq!(v[0].url, "https://x");
        assert_eq!(v[0].snippet, "snippet text");
    }

    #[test]
    fn parse_partial_hit_uses_defaults() {
        let j = r#"{"web":{"results":[{}]}}"#;
        let v = parse_brave_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].title.is_empty());
        assert!(v[0].url.is_empty());
        assert!(v[0].snippet.is_empty());
    }

    #[test]
    fn parse_null_title_url_description_become_empty_strings() {
        let j = r#"{"web":{"results":[{"title":null,"url":null,"description":null}]}}"#;
        let v = parse_brave_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].title.is_empty());
        assert!(v[0].url.is_empty());
        assert!(v[0].snippet.is_empty());
    }

    #[test]
    fn parse_lenient_coerces_numeric_title_to_string() {
        let j = r#"{"web":{"results":[{"title":404,"url":"https://x","description":1}]}}"#;
        let v = parse_brave_response_body(j).unwrap();
        assert_eq!(v[0].title, "404");
        assert_eq!(v[0].snippet, "1");
    }

    #[test]
    fn http_error_maps_to_search_error_http() {
        let e = SearchError::Http {
            status: Some(401),
            provider: PROVIDER,
            message: "nope".into(),
        };
        assert!(e.is_retryable());
        assert!(crate::error::is_retryable_http_status(401));
    }

    #[tokio::test]
    async fn brave_client_401_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/res/v1/web/search"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid subscription"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let brave =
            BraveClient::new_with_endpoint_for_tests(client, SecretString::new("k".into()), srv.uri());

        let err = brave
            .search(
                &SearchQuery { q: "q".into() },
                &SearchOptions::default(),
            )
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(401),
                provider,
                ..
            } => {
                assert_eq!(*provider, "brave");
                assert!(err.is_retryable());
            }
            _ => panic!("expected Http 401, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn brave_client_429_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/res/v1/web/search"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let brave =
            BraveClient::new_with_endpoint_for_tests(client, SecretString::new("k".into()), srv.uri());

        let err = brave
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(429),
                provider,
                ..
            } => {
                assert_eq!(*provider, "brave");
                assert!(err.is_retryable());
                assert!(crate::error::is_retryable_http_status(429));
            }
            _ => panic!("expected Http 429, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn brave_client_403_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/res/v1/web/search"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let brave =
            BraveClient::new_with_endpoint_for_tests(client, SecretString::new("k".into()), srv.uri());

        let err = brave
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(403),
                provider,
                ..
            } => {
                assert_eq!(*provider, "brave");
                assert!(err.is_retryable());
            }
            _ => panic!("expected Http 403, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn brave_client_200_empty_json_ok() {
        let srv = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/res/v1/web/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let brave =
            BraveClient::new_with_endpoint_for_tests(client, SecretString::new("k".into()), srv.uri());

        let out = brave
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .expect("ok");
        assert!(out.is_empty());
    }
}
