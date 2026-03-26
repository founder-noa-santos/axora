#![cfg(feature = "sim-tests")]

//! Mocked API Client Integration Tests - Phase 6.3
//!
//! Integration tests using WireMock-style mock server for:
//! - Full coordinator flow with API client
//! - Mocked API server responses
//! - Controlled failure scenarios
//! - Performance testing without network dependencies

// Note: This test file requires the sim_mock_server.rs module to be present
// Run with: cargo test -p openakta-agents --features sim-tests --test sim_api_client_mocked

use openakta_api_client::{ApiClient, ApiError, ClientConfig};
use openakta_proto::provider_v1::{Message as ProtoMessage, ProviderRequest, Usage};
use std::time::Duration;
use tokio::time::timeout;

// Include mock server implementation
#[path = "sim_mock_server.rs"]
mod mock_server;

use mock_server::*;

/// Test: Basic mocked API execution
#[tokio::test]
async fn test_mocked_api_basic_execution() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock response
    let request_id = "test-basic-123";
    {
        let mut state_guard = state.lock().await;
        state_guard.add_completion_response(create_success_response(request_id));
    }

    // Make request
    let request = ProviderRequest {
        request_id: request_id.to_string(),
        tenant_id: "test-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![ProtoMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    };

    let result = client.execute(request).await;

    // Verify response
    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.response_id, request_id);
    assert!(!response.content.is_empty());

    // Verify request was recorded
    let state_guard = state.lock().await;
    assert_eq!(state_guard.request_count(), 1);
}

/// Test: Mocked API with tenant ID
#[tokio::test]
async fn test_mocked_api_with_tenant_id() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock response
    let request_id = "test-tenant-456";
    {
        let mut state_guard = state.lock().await;
        state_guard.add_completion_response(create_success_response(request_id));
    }

    // Make request with tenant ID
    let mut request = ProviderRequest {
        request_id: request_id.to_string(),
        tenant_id: "specific-tenant-789".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![ProtoMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    };

    let result = client.execute(request).await;

    // Verify response
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.response_id, request_id);
}

/// Test: Mocked streaming execution
#[tokio::test]
async fn test_mocked_api_streaming_execution() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock stream configuration
    let request_id = "test-stream-789";
    {
        let mut state_guard = state.lock().await;
        state_guard.add_stream_config(create_stream_chunks(
            request_id,
            "This is a streaming test response",
        ));
    }

    // Make streaming request
    let mut request = ProviderRequest {
        request_id: request_id.to_string(),
        tenant_id: "test-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![ProtoMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: true,
        ..Default::default()
    };

    let result = client.execute_stream(request).await;

    // Verify stream
    assert!(
        result.is_ok(),
        "Expected stream to start, got: {:?}",
        result
    );
    let mut stream = result.unwrap();

    // Consume stream
    use tokio_stream::StreamExt;
    let mut chunk_count = 0;
    let mut total_content = String::new();

    while let Some(chunk_result) = timeout(Duration::from_secs(5), stream.next())
        .await
        .ok()
        .flatten()
    {
        if let Ok(chunk) = chunk_result {
            chunk_count += 1;
            if let Some(delta) = &chunk.delta {
                total_content.push_str(&delta.content);
            }
        }
    }

    // Verify chunks received
    assert!(chunk_count > 0, "Expected at least one chunk");
    assert!(!total_content.is_empty(), "Expected non-empty content");
}

/// Test: Mocked embedding execution
#[tokio::test]
async fn test_mocked_api_embedding_execution() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock embed response
    {
        let mut state_guard = state.lock().await;
        state_guard.add_embed_response(MockEmbedResponse {
            response: openakta_proto::provider_v1::EmbedResponse {
                embeddings: vec![vec![0.1; 1536]],
                usage: Some(Usage {
                    input_tokens: 10,
                    output_tokens: 0,
                    total_tokens: 10,
                }),
            },
            delay_ms: 50,
        });
    }

    // Make embedding request
    let text = "This is a test text for embedding.".to_string();
    let result = client
        .embed(
            text,
            Some("text-embedding-3-small".to_string()),
            openakta_api_client::ExecutionStrategy::HostedOnly,
        )
        .await;

    // Verify response
    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.embeddings.len(), 1);
    assert_eq!(response.embeddings[0].len(), 1536);
}

/// Test: Mocked batch embedding execution
#[tokio::test]
async fn test_mocked_api_batch_embedding_execution() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock batch embed response
    {
        let mut state_guard = state.lock().await;
        state_guard.add_batch_embed_response(MockBatchEmbedResponse {
            response: openakta_proto::provider_v1::BatchEmbedResponse {
                embeddings: vec![vec![0.1; 1536], vec![0.2; 1536], vec![0.3; 1536]],
                usage: Some(Usage {
                    input_tokens: 30,
                    output_tokens: 0,
                    total_tokens: 30,
                }),
            },
            delay_ms: 100,
        });
    }

    // Make batch embedding request
    let texts = vec![
        "First text.".to_string(),
        "Second text.".to_string(),
        "Third text.".to_string(),
    ];

    let result = client
        .embed_batch(
            texts,
            Some("text-embedding-3-small".to_string()),
            openakta_api_client::ExecutionStrategy::HostedOnly,
        )
        .await;

    // Verify response
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.embeddings.len(), 3);
    for (i, embedding) in response.embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), 1536);
    }
}

/// Test: Mocked search execution
#[tokio::test]
async fn test_mocked_api_search_execution() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add mock search response
    {
        let mut state_guard = state.lock().await;
        state_guard.add_search_response(MockSearchResponse {
            response: openakta_proto::research_v1::SearchResponse {
                results: vec![
                    openakta_proto::research_v1::SearchResult {
                        title: "Test Result 1".to_string(),
                        url: "https://example.com/1".to_string(),
                        snippet: "First test result".to_string(),
                    },
                    openakta_proto::research_v1::SearchResult {
                        title: "Test Result 2".to_string(),
                        url: "https://example.com/2".to_string(),
                        snippet: "Second test result".to_string(),
                    },
                ],
                total_count: 2,
            },
            delay_ms: 50,
        });
    }

    // Make search request
    let request = openakta_proto::research_v1::SearchRequest {
        query: "test query".to_string(),
        limit: 10,
        ..Default::default()
    };

    let result = client.search(request).await;

    // Verify response
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.total_count, 2);
}

/// Test: Mocked concurrent requests
#[tokio::test]
async fn test_mocked_api_concurrent_requests() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add multiple mock responses
    {
        let mut state_guard = state.lock().await;
        for i in 0..10 {
            state_guard
                .add_completion_response(create_success_response(&format!("concurrent-{}", i)));
        }
    }

    // Make 10 concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = ProviderRequest {
                request_id: format!("concurrent-{}", i),
                tenant_id: "test-tenant".to_string(),
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                messages: vec![ProtoMessage {
                    role: "user".to_string(),
                    content: "Test message".to_string(),
                }],
                max_tokens: 100,
                temperature: 0.7,
                stream: false,
                ..Default::default()
            };
            client_clone.execute(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests
    let results = futures_util::future::join_all(handles).await;

    // Count successes
    let mut successes = 0;
    for result in results {
        if let Ok(Ok(_)) = result {
            successes += 1;
        }
    }

    // All requests should succeed
    assert_eq!(successes, 10, "Expected all 10 requests to succeed");
}

/// Test: Mocked failure handling
#[tokio::test]
async fn test_mocked_api_failure_handling() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add failing mock response
    {
        let mut state_guard = state.lock().await;
        state_guard.add_completion_response(create_failure_response());
    }

    // Make request
    let request = ProviderRequest {
        request_id: "test-failure".to_string(),
        tenant_id: "test-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![ProtoMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    };

    let result = client.execute(request).await;

    // Verify failure
    assert!(result.is_err(), "Expected error, got success");
}

/// Test: Mocked slow response handling
#[tokio::test]
async fn test_mocked_api_slow_response_handling() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_millis(100), // Short timeout
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add slow mock response
    {
        let mut state_guard = state.lock().await;
        state_guard.add_completion_response(create_slow_response("test-slow", 200));
        // 200ms delay
    }

    // Make request
    let request = ProviderRequest {
        request_id: "test-slow".to_string(),
        tenant_id: "test-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![ProtoMessage {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    };

    let result = client.execute(request).await;

    // Verify timeout
    assert!(result.is_err(), "Expected timeout error");
}

/// Test: Mocked circuit breaker behavior
#[tokio::test]
async fn test_mocked_api_circuit_breaker_behavior() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Add multiple failing responses to trigger circuit breaker
    {
        let mut state_guard = state.lock().await;
        for _ in 0..10 {
            state_guard.add_completion_response(create_failure_response());
        }
    }

    // Make multiple requests
    let mut failures = 0;
    for _ in 0..10 {
        let request = ProviderRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "test-tenant".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            messages: vec![ProtoMessage {
                role: "user".to_string(),
                content: "Test message".to_string(),
            }],
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
            ..Default::default()
        };

        if let Err(_) = client.execute(request).await {
            failures += 1;
        }
    }

    // Verify failures (circuit breaker should open)
    assert!(failures > 0, "Expected failures to trigger circuit breaker");
}

/// Test: Mocked different models
#[tokio::test]
async fn test_mocked_api_different_models() {
    let (state, endpoint) = start_mock_server().await;

    let config = ClientConfig {
        endpoint: endpoint.clone(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    let models = vec!["gpt-4", "gpt-3.5-turbo", "claude-3-opus", "qwen-max"];

    for model in models {
        // Add mock response
        {
            let mut state_guard = state.lock().await;
            state_guard
                .add_completion_response(create_success_response(&format!("test-{}", model)));
        }

        // Make request
        let request = ProviderRequest {
            request_id: format!("test-{}", model),
            tenant_id: "test-tenant".to_string(),
            provider: "openai".to_string(),
            model: model.to_string(),
            messages: vec![ProtoMessage {
                role: "user".to_string(),
                content: "Test message".to_string(),
            }],
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
            ..Default::default()
        };

        let result = client.execute(request).await;

        // Verify success
        assert!(
            result.is_ok(),
            "Expected success for model {}, got: {:?}",
            model,
            result
        );
    }
}
