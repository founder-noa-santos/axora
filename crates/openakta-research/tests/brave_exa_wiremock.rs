//! HTTP-level tests: Brave returns 401/429, Exa returns 200 — router must fall back.

use std::sync::Arc;

use openakta_research::{
    BraveClient, ExaClient, SearchOptions, SearchQuery, SearchRouter,
};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn brave_401_then_exa_200_returns_exa_hits() {
    let brave_srv = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid subscription"))
        .mount(&brave_srv)
        .await;

    let exa_srv = MockServer::start().await;
    let body = serde_json::json!({
        "results": [{"title": "FromExa", "url": "https://exa.example", "text": "snippet"}]
    });
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&exa_srv)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");
    let key = SecretString::new("test-key".into());

    let brave = BraveClient::new_with_endpoint_for_tests(client.clone(), key.clone(), brave_srv.uri());
    let exa = ExaClient::new_with_endpoint_for_tests(
        client,
        key,
        exa_srv.uri(),
        "neural",
        None,
        vec![],
    );

    let router = SearchRouter::new(vec![Arc::new(brave), Arc::new(exa)]);

    let out = router
        .search(
            &SearchQuery {
                q: "rust async".into(),
            },
            &SearchOptions::default(),
        )
        .await
        .expect("router should fall back to Exa");

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "FromExa");
    assert_eq!(out[0].url, "https://exa.example");
}

#[tokio::test]
async fn brave_429_then_exa_200() {
    let brave_srv = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&brave_srv)
        .await;

    let exa_srv = MockServer::start().await;
    let body = serde_json::json!({"results": []});
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&exa_srv)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");
    let key = SecretString::new("k".into());

    let brave = BraveClient::new_with_endpoint_for_tests(client.clone(), key.clone(), brave_srv.uri());
    let exa = ExaClient::new_with_endpoint_for_tests(
        client,
        key,
        exa_srv.uri(),
        "neural",
        None,
        vec![],
    );

    let router = SearchRouter::new(vec![Arc::new(brave), Arc::new(exa)]);

    let out = router
        .search(
            &SearchQuery { q: "q".into() },
            &SearchOptions::default(),
        )
        .await
        .expect("fallback");

    assert!(out.is_empty());
}

#[tokio::test]
async fn exa_403_then_brave_200() {
    let exa_srv = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&exa_srv)
        .await;

    let brave_srv = MockServer::start().await;
    let body = r#"{"web":{"results":[{"title":"BraveHit","url":"https://brave.example","description":"d"}]}}"#;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&brave_srv)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");
    let key = SecretString::new("k".into());

    let exa = ExaClient::new_with_endpoint_for_tests(
        client.clone(),
        key.clone(),
        exa_srv.uri(),
        "neural",
        None,
        vec![],
    );
    let brave = BraveClient::new_with_endpoint_for_tests(client, key, brave_srv.uri());

    let router = SearchRouter::new(vec![Arc::new(exa), Arc::new(brave)]);

    let out = router
        .search(
            &SearchQuery { q: "q".into() },
            &SearchOptions::default(),
        )
        .await
        .expect("fallback to Brave");

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "BraveHit");
}
