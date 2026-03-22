//! Exa Search API (neural / semantic web search).
//!
//! API: <https://docs.exa.ai/reference/search>
//!
//! Authenticates with header `x-api-key`. Default `type` is `neural` for embedding-based retrieval.
//! Optional `category` (e.g. `github` for repositories, `research paper` for papers) and
//! `includeDomains` bias results toward technical documentation and code hosts.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::error::SearchError;
use crate::provider::SearchProvider;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

const PROVIDER: &str = "exa";

/// Canonical Exa API origin (never taken from untrusted config).
pub const EXA_CANONICAL_BASE_URL: &str = "https://api.exa.ai";

/// Exa category filter commonly used for **code and repositories** (Exa API value `github`).
pub const EXA_CATEGORY_GITHUB: &str = "github";

/// Exa HTTP client (BYOK). Accepts an injected [`reqwest::Client`] for tests and connection pooling.
pub struct ExaClient {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: String,
    /// `neural`, `auto`, `fast`, `instant`, etc.
    search_type: String,
    category: Option<String>,
    include_domains: Vec<String>,
}

impl ExaClient {
    pub fn new(
        http: reqwest::Client,
        api_key: SecretString,
        search_type: impl Into<String>,
        category: Option<String>,
        include_domains: Vec<String>,
    ) -> Self {
        Self {
            http,
            api_key,
            base_url: EXA_CANONICAL_BASE_URL.trim_end_matches('/').to_string(),
            search_type: search_type.into(),
            category,
            include_domains,
        }
    }

    #[cfg(any(test, feature = "search-provider-mock-endpoints"))]
    pub fn new_with_endpoint_for_tests(
        http: reqwest::Client,
        api_key: SecretString,
        base_url: impl AsRef<str>,
        search_type: impl Into<String>,
        category: Option<String>,
        include_domains: Vec<String>,
    ) -> Self {
        Self {
            http,
            api_key,
            base_url: base_url.as_ref().trim_end_matches('/').to_string(),
            search_type: search_type.into(),
            category,
            include_domains,
        }
    }

    pub fn default_base_url() -> &'static str {
        EXA_CANONICAL_BASE_URL
    }

    pub fn default_search_type_neural() -> &'static str {
        "neural"
    }

    /// Build the JSON request body (same payload as [`SearchProvider::search`] POST). Used for tests
    /// and to verify category / domain filters without network I/O.
    pub(crate) fn build_search_request<'a>(
        &'a self,
        query: &'a SearchQuery,
        opts: &'a SearchOptions,
    ) -> ExaSearchRequest<'a> {
        let num_results = opts.max_results.clamp(1, 100) as u32;
        ExaSearchRequest {
            query: query.q.as_str(),
            search_type: self.search_type.as_str(),
            num_results,
            category: self.category.as_deref(),
            include_domains: self.include_domains.clone(),
            contents: ExaContents {
                text: ExaTextLimit {
                    max_characters: opts.max_snippet_chars.min(8000) as u32,
                },
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaSearchRequest<'a> {
    pub(crate) query: &'a str,
    #[serde(rename = "type")]
    pub(crate) search_type: &'a str,
    pub(crate) num_results: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) include_domains: Vec<String>,
    pub(crate) contents: ExaContents,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaContents {
    pub(crate) text: ExaTextLimit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaTextLimit {
    /// Bound snippet size before [`crate::normalize::normalize_results`].
    pub(crate) max_characters: u32,
}

#[derive(Debug, Deserialize)]
struct ExaResponse {
    #[serde(default)]
    results: Option<Vec<Option<ExaResult>>>,
}

#[derive(Debug, Deserialize)]
struct ExaResult {
    #[serde(default, deserialize_with = "crate::serde_util::lenient_string_or_default")]
    title: String,
    #[serde(default, deserialize_with = "crate::serde_util::lenient_string_or_default")]
    url: String,
    #[serde(default)]
    text: serde_json::Value,
    #[serde(default)]
    summary: serde_json::Value,
    /// Omitted or `null` in JSON becomes `None` (avoids serde failing on `highlights: null`).
    #[serde(default)]
    highlights: Option<Vec<serde_json::Value>>,
}

fn opt_string_from_json_value(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    }
}

/// Parse a **successful** (2xx) Exa JSON body into [`SearchResult`] rows.
pub fn parse_exa_response_body(body: &str) -> Result<Vec<SearchResult>, SearchError> {
    let parsed: ExaResponse = serde_json::from_str(body).map_err(|e| SearchError::Parse {
        provider: PROVIDER,
        message: e.to_string(),
    })?;
    let results = parsed.results.unwrap_or_default();
    Ok(results
        .into_iter()
        .filter_map(|row| {
            row.map(|r| {
                let snippet = exa_snippet(&r);
                SearchResult {
                    title: r.title,
                    url: r.url,
                    snippet,
                }
            })
        })
        .collect())
}

fn exa_snippet(r: &ExaResult) -> String {
    if let Some(t) = opt_string_from_json_value(&r.text) {
        if !t.is_empty() {
            return t;
        }
    }
    if let Some(s) = opt_string_from_json_value(&r.summary) {
        if !s.is_empty() {
            return s;
        }
    }
    if let Some(ref hl) = r.highlights {
        if !hl.is_empty() {
            let joined = hl
                .iter()
                .filter_map(|v| opt_string_from_json_value(v))
                .filter(|s| !s.is_empty())
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            if !joined.is_empty() {
                return joined;
            }
        }
    }
    String::new()
}

#[async_trait]
impl SearchProvider for ExaClient {
    fn name(&self) -> &'static str {
        PROVIDER
    }

    async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let body = self.build_search_request(query, opts);
        let url = format!("{}/search", self.base_url);

        let mut headers = HeaderMap::new();
        let key = HeaderValue::from_str(self.api_key.expose_secret()).map_err(|e| {
            SearchError::Transport {
                provider: PROVIDER,
                message: format!("invalid x-api-key header: {e}"),
            }
        })?;
        headers.insert("x-api-key", key);
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

        parse_exa_response_body(&text)
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
        let err = parse_exa_response_body("%%%").unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "exa", .. }));
    }

    #[test]
    fn parse_empty_object_yields_empty() {
        assert!(parse_exa_response_body("{}").unwrap().is_empty());
    }

    #[test]
    fn parse_results_null_yields_empty() {
        assert!(parse_exa_response_body(r#"{"results":null}"#)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parse_results_not_array_errors() {
        let err = parse_exa_response_body(r#"{"results":"nope"}"#).unwrap_err();
        assert!(matches!(err, SearchError::Parse { provider: "exa", .. }));
    }

    #[test]
    fn parse_null_result_row_skipped() {
        let j = r#"{"results":[null,{"title":"T","url":"https://u","text":"x"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "T");
    }

    #[test]
    fn parse_one_hit_prefers_text() {
        let j = r#"{"results":[{"title":"T","url":"https://u","text":"body","summary":"sum"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet, "body");
    }

    #[test]
    fn parse_text_as_number_coerced() {
        let j = r#"{"results":[{"title":"T","url":"https://u","text":42}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "42");
    }

    #[test]
    fn parse_falls_back_to_summary() {
        let j = r#"{"results":[{"title":"T","url":"https://u","summary":"S"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "S");
    }

    #[test]
    fn parse_falls_back_to_highlights() {
        let j = r#"{"results":[{"title":"T","url":"https://u","highlights":["a","b"]}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "a b");
    }

    #[test]
    fn parse_highlights_skips_null_keeps_strings_and_coerces_numbers() {
        let j = r#"{"results":[{"title":"T","url":"https://u","highlights":[null,"x",42,"z"]}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "x 42 z");
    }

    #[test]
    fn parse_highlights_null_field_falls_through_to_summary() {
        let j = r#"{"results":[{"title":"T","url":"https://u","highlights":null,"summary":"S"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "S");
    }

    #[test]
    fn parse_partial_hit_empty_snippet() {
        let j = r#"{"results":[{"title":"T","url":"https://u"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert_eq!(v[0].snippet, "");
    }

    #[test]
    fn parse_null_title_url_become_empty() {
        let j = r#"{"results":[{"title":null,"url":null,"text":"ok"}]}"#;
        let v = parse_exa_response_body(j).unwrap();
        assert!(v[0].title.is_empty());
        assert!(v[0].url.is_empty());
        assert_eq!(v[0].snippet, "ok");
    }

    #[test]
    fn build_request_includes_github_category_and_domains() {
        let client = reqwest::Client::new();
        let exa = ExaClient::new(
            client,
            SecretString::new("k".into()),
            "neural",
            Some(EXA_CATEGORY_GITHUB.into()),
            vec!["docs.rs".into(), "github.com".into()],
        );
        let q = SearchQuery {
            q: "async trait".into(),
        };
        let opts = SearchOptions::default();
        let body = exa.build_search_request(&q, &opts);
        assert_eq!(body.search_type, "neural");
        assert_eq!(body.category, Some("github"));
        assert_eq!(body.include_domains, vec!["docs.rs", "github.com"]);
        let v = serde_json::to_value(&body).expect("serialize");
        assert_eq!(v["type"], "neural");
        assert_eq!(v["category"], "github");
        assert_eq!(v["includeDomains"], serde_json::json!(["docs.rs", "github.com"]));
    }

    #[test]
    fn build_request_omits_category_when_none() {
        let client = reqwest::Client::new();
        let exa = ExaClient::new(
            client,
            SecretString::new("k".into()),
            "neural",
            None,
            vec![],
        );
        let q = SearchQuery { q: "q".into() };
        let opts = SearchOptions::default();
        let body = exa.build_search_request(&q, &opts);
        assert!(body.category.is_none());
        let v = serde_json::to_value(&body).expect("serialize");
        assert!(v.get("category").is_none());
    }

    #[tokio::test]
    async fn exa_client_401_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(401).set_body_string("bad key"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let exa = ExaClient::new_with_endpoint_for_tests(
            client,
            SecretString::new("k".into()),
            srv.uri(),
            "neural",
            None,
            vec![],
        );

        let err = exa
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(401),
                provider,
                ..
            } => {
                assert_eq!(*provider, "exa");
                assert!(err.is_retryable());
            }
            _ => panic!("expected Http 401, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn exa_client_429_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let exa = ExaClient::new_with_endpoint_for_tests(
            client,
            SecretString::new("k".into()),
            srv.uri(),
            "neural",
            None,
            vec![],
        );

        let err = exa
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(429),
                provider,
                ..
            } => {
                assert_eq!(*provider, "exa");
                assert!(err.is_retryable());
            }
            _ => panic!("expected Http 429, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn exa_client_403_bubbles_retryable_http() {
        let srv = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let exa = ExaClient::new_with_endpoint_for_tests(
            client,
            SecretString::new("k".into()),
            srv.uri(),
            "neural",
            None,
            vec![],
        );

        let err = exa
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .unwrap_err();

        match &err {
            SearchError::Http {
                status: Some(403),
                provider,
                ..
            } => {
                assert_eq!(*provider, "exa");
                assert!(err.is_retryable());
            }
            _ => panic!("expected Http 403, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn exa_client_200_empty_results_ok() {
        let srv = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&srv)
            .await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        let exa = ExaClient::new_with_endpoint_for_tests(
            client,
            SecretString::new("k".into()),
            srv.uri(),
            "neural",
            None,
            vec![],
        );

        let out = exa
            .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
            .await
            .expect("ok");
        assert!(out.is_empty());
    }
}
