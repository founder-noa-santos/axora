//! HTTP-level tests: Serper returns 429, Tavily returns 200 — router must fall back.

use std::sync::Arc;

use openakta_research::{SearchOptions, SearchQuery, SearchRouter, SerperClient, TavilyClient};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn serper_429_then_tavily_200_returns_tavily_hits() {
    let serper_srv = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&serper_srv)
        .await;

    let tavily_srv = MockServer::start().await;
    let body = serde_json::json!({
        "results": [{"title": "FromTavily", "url": "https://example.com", "content": "snippet"}]
    });
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&tavily_srv)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");
    let key = SecretString::new("test-key".into());

    let serper =
        SerperClient::new_with_endpoint_for_tests(client.clone(), key.clone(), serper_srv.uri());
    let tavily = TavilyClient::new_with_endpoint_for_tests(client, key, tavily_srv.uri());

    let router = SearchRouter::new(vec![Arc::new(serper), Arc::new(tavily)]);

    let out = router
        .search(
            &SearchQuery {
                q: "rust async".into(),
            },
            &SearchOptions::default(),
        )
        .await
        .expect("router should fall back to Tavily");

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "FromTavily");
    assert_eq!(out[0].url, "https://example.com");
}

#[tokio::test]
async fn serper_500_then_tavily_200() {
    let serper_srv = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&serper_srv)
        .await;

    let tavily_srv = MockServer::start().await;
    let body = serde_json::json!({"results": []});
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&tavily_srv)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");
    let key = SecretString::new("k".into());

    let router = SearchRouter::new(vec![
        Arc::new(SerperClient::new_with_endpoint_for_tests(
            client.clone(),
            key.clone(),
            serper_srv.uri(),
        )),
        Arc::new(TavilyClient::new_with_endpoint_for_tests(
            client,
            key,
            tavily_srv.uri(),
        )),
    ]);

    let out = router
        .search(&SearchQuery { q: "q".into() }, &SearchOptions::default())
        .await
        .expect("fallback");

    assert!(out.is_empty());
}
