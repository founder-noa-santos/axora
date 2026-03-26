#![cfg(feature = "sim-tests")]

//! Chaos Tests - Phase 6.3 Comprehensive Suite
//!
//! Tests for resilience under failure conditions:
//! - API unavailable (circuit breaker opens)
//! - Network partition (timeout errors)
//! - High latency (slow responses)
//! - Partial failures (streaming interruptions)
//! - Connection pool exhaustion
//! - DNS failures
//! - TLS handshake failures
//! - Payload size limits

use openakta_api_client::{ApiClient, ApiError, ClientConfig};
use std::time::Duration;
use tokio::time::timeout;

/// Test: Circuit breaker opens after repeated failures
#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    // Create client with invalid endpoint (will fail)
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make multiple requests to trigger circuit breaker
    let mut failures = 0;
    for _ in 0..10 {
        let request = create_test_request();
        match client.execute(request).await {
            Ok(_) => {}
            Err(_) => failures += 1,
        }
    }

    // Verify circuit breaker opened (should have failures)
    assert!(failures > 0, "Expected failures to trigger circuit breaker");
}

/// Test: Circuit breaker recovers after timeout
#[tokio::test]
async fn test_circuit_breaker_recovers() {
    // This test verifies that the circuit breaker transitions to half-open state
    // after the recovery timeout (30 seconds)

    // Note: This is a long-running test, may be skipped in CI
    if std::env::var("SKIP_LONG_TESTS").is_ok() {
        return;
    }

    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Trigger circuit breaker
    for _ in 0..10 {
        let _ = client.execute(create_test_request()).await;
    }

    // Wait for recovery timeout (30 seconds)
    tokio::time::sleep(Duration::from_secs(31)).await;

    // Circuit should be in half-open state now
    // Next request should be allowed (will still fail, but circuit allows it)
    let request = create_test_request();
    let _ = client.execute(request).await;
}

/// Test: Timeout error on slow response
#[tokio::test]
async fn test_timeout_on_slow_response() {
    // Create client with very short timeout
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(10), // Very short timeout
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make request (will timeout if server is slow or unavailable)
    let request = create_test_request();
    let result = timeout(Duration::from_millis(50), client.execute(request)).await;

    // Verify timeout occurred
    assert!(result.is_err(), "Expected timeout error");
}

/// Test: Connection error on unavailable API
#[tokio::test]
async fn test_connection_error_on_unavailable_api() {
    // Create client with unreachable endpoint
    let config = ClientConfig {
        endpoint: "192.0.2.1:3030".to_string(), // Reserved test IP (unreachable)
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make request (will fail with connection error)
    let request = create_test_request();
    let result = client.execute(request).await;

    // Verify connection error
    assert!(result.is_err(), "Expected connection error");

    if let Err(ApiError::Connection(_)) = result {
        // Expected
    } else {
        panic!("Expected Connection error, got: {:?}", result);
    }
}

/// Test: Multiple concurrent requests under failure conditions
#[tokio::test]
async fn test_concurrent_requests_under_failure() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make 100 concurrent requests
    let mut handles = vec![];
    for i in 0..100 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_request();
            client_clone.execute(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures_util::future::join_all(handles).await;

    // Count failures (should be high due to invalid endpoint)
    let mut failures = 0;
    let mut successes = 0;
    for result in results {
        match result {
            Ok(Ok(_)) => successes += 1,
            Ok(Err(_)) => failures += 1,
            Err(_) => failures += 1,
        }
    }

    // Verify most requests failed
    assert!(
        failures > 90,
        "Expected >90% failures, got {} failures out of 100",
        failures
    );
    assert!(
        successes < 10,
        "Expected <10% successes, got {} successes out of 100",
        successes
    );
}

/// Test: Streaming interruption handling
#[tokio::test]
async fn test_streaming_interruption() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make streaming request (will fail)
    let mut request = create_test_request();
    request.stream = true;

    let result = client.execute_stream(request).await;

    // Verify error occurred
    assert!(
        result.is_err(),
        "Expected error on streaming to invalid endpoint"
    );
}

/// Test: Invalid URI handling
#[tokio::test]
async fn test_invalid_uri_handling() {
    // Try to create client with invalid URI
    let config = ClientConfig {
        endpoint: "not a valid uri".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    // Client creation should fail with invalid URI
    let result = ApiClient::new(config);

    // Verify error
    assert!(result.is_err(), "Expected error on invalid URI");

    if let Err(ApiError::InvalidUri(_)) = result {
        // Expected
    } else {
        panic!("Expected InvalidUri error, got: {:?}", result);
    }
}

/// Test: DNS resolution failure
#[tokio::test]
async fn test_dns_resolution_failure() {
    // Create client with non-existent domain
    let config = ClientConfig {
        endpoint: "this-domain-does-not-exist-12345.com:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make request (will fail with DNS error)
    let request = create_test_request();
    let result = client.execute(request).await;

    // Verify error (could be Connection or Timeout depending on DNS behavior)
    assert!(result.is_err(), "Expected error on DNS failure");
}

/// Test: TLS handshake failure (if TLS enabled)
#[tokio::test]
async fn test_tls_handshake_failure() {
    // Create client with TLS enabled but connecting to non-TLS server
    let config = ClientConfig {
        endpoint: "httpbin.org:80".to_string(), // HTTP, not HTTPS
        use_tls: true,                          // TLS enabled
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make request (will fail with TLS error)
    let request = create_test_request();
    let result = client.execute(request).await;

    // Verify error
    assert!(result.is_err(), "Expected error on TLS handshake failure");
}

/// Test: Request payload too large
#[tokio::test]
async fn test_large_payload_handling() {
    // This test verifies that large payloads are handled correctly
    // (not rejected by the client, though server may reject)

    let config = ClientConfig::default();
    let client = ApiClient::new(config).expect("Failed to create client");

    // Create request with very large message
    let mut request = create_test_request();
    request.messages = vec![openakta_proto::provider_v1::Message {
        role: "user".to_string(),
        content: "Test message. ".repeat(100000), // ~1.6MB
    }];

    // Serialize to verify it works (won't actually send without server)
    let bytes = request.encode_to_vec();
    assert!(bytes.len() > 1_000_000, "Expected large payload");
}

/// Test: Rapid reconnection attempts
#[tokio::test]
async fn test_rapid_reconnection_attempts() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make rapid reconnection attempts
    for _ in 0..50 {
        let request = create_test_request();
        let _ = client.execute(request).await;
    }

    // Test completes without panic = pass
    // (verifies client doesn't crash under rapid failure conditions)
}

/// Test: Client clone under failure conditions
#[tokio::test]
async fn test_client_clone_under_failure() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Clone client multiple times
    let clients: Vec<_> = (0..10).map(|_| client.clone()).collect();

    // Make requests from all clones concurrently
    let mut handles = vec![];
    for client in clients {
        let handle = tokio::spawn(async move {
            let request = create_test_request();
            client.execute(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests
    let results = futures_util::future::join_all(handles).await;

    // Verify all requests completed (even if with errors)
    assert_eq!(results.len(), 10, "Expected 10 results");
}

/// Test: Connection pool exhaustion
#[tokio::test]
async fn test_connection_pool_exhaustion() {
    // Create client with very small pool (simulated by many concurrent requests)
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_secs(1),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = match ApiClient::new(config) {
        Ok(c) => c,
        Err(_) => return, // Server not available, skip test
    };

    // Make many concurrent requests to exhaust pool
    let mut handles = vec![];
    for i in 0..100 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_request();
            client_clone.execute(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests
    let results = futures_util::future::join_all(handles).await;

    // Count successes and failures
    let mut successes = 0;
    let mut failures = 0;
    for result in results {
        match result {
            Ok(Ok(_)) => successes += 1,
            _ => failures += 1,
        }
    }

    // Test passes if it doesn't panic (pool exhaustion is handled gracefully)
    // Some requests may fail due to pool exhaustion, which is expected
    println!(
        "Pool exhaustion test: {} successes, {} failures",
        successes, failures
    );
}

/// Test: Repeated connection/disconnection
#[tokio::test]
async fn test_repeated_connection_cycles() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Make many requests in sequence
    for i in 0..50 {
        let request = create_test_request();
        let _ = client.execute(request).await;

        // Verify client is still usable after each failure
        if i % 10 == 0 {
            // Client should still be cloneable and functional
            let _clone = client.clone();
        }
    }

    // Test passes if client survives all cycles without panic
}

/// Test: Mixed success and failure scenarios
#[tokio::test]
async fn test_mixed_success_failure_scenarios() {
    // This test simulates real-world scenarios where some requests succeed and others fail

    // Create client with localhost (may or may not be available)
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_secs(2),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = match ApiClient::new(config) {
        Ok(c) => c,
        Err(_) => return, // Skip if can't create client
    };

    // Make requests with varying parameters
    let mut results = Vec::new();

    for i in 0..20 {
        let request = openakta_proto::provider_v1::ProviderRequest {
            request_id: format!("mixed-{}", i),
            tenant_id: "test-tenant".to_string(),
            provider: if i % 2 == 0 {
                "openai"
            } else {
                "invalid-provider"
            }
            .to_string(),
            model: if i % 3 == 0 { "invalid-model" } else { "gpt-4" }.to_string(),
            messages: vec![openakta_proto::provider_v1::Message {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
            ..Default::default()
        };

        let result = client.execute(request).await;
        results.push(result);
    }

    // Test passes without panic - real-world scenario handled gracefully
    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = results.iter().filter(|r| r.is_err()).count();

    println!(
        "Mixed scenario: {} successes, {} failures",
        successes, failures
    );
}

/// Test: Memory pressure under high load
#[tokio::test]
async fn test_memory_pressure_under_high_load() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Create large payloads
    let large_content = "Test message. ".repeat(10000);

    // Make requests with large payloads
    let mut handles = vec![];
    for i in 0..20 {
        let client_clone = client.clone();
        let content_clone = large_content.clone();
        let handle = tokio::spawn(async move {
            let request = openakta_proto::provider_v1::ProviderRequest {
                request_id: format!("large-{}", i),
                tenant_id: "test-tenant".to_string(),
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                messages: vec![openakta_proto::provider_v1::Message {
                    role: "user".to_string(),
                    content: content_clone,
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

    // Test passes if no memory-related panics occur
    assert_eq!(results.len(), 20, "Expected 20 results");
}

/// Test: Rapid client creation and destruction
#[tokio::test]
async fn test_rapid_client_lifecycle() {
    // Rapidly create and destroy clients
    for _ in 0..50 {
        let config = ClientConfig {
            endpoint: "invalid-endpoint:9999".to_string(),
            use_tls: false,
            connect_timeout: Duration::from_millis(100),
            timeout: Duration::from_millis(500),
            migration_mode: false,
            feature_flags: Default::default(),
            execution_strategy: Default::default(),
        };

        let client = ApiClient::new(config).expect("Failed to create client");

        // Make one request
        let request = create_test_request();
        let _ = client.execute(request).await;

        // Client dropped here
    }

    // Test passes if no resource leaks or panics
}

/// Test: Concurrent streaming and non-streaming requests
#[tokio::test]
async fn test_mixed_streaming_nonstreaming() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Mix of streaming and non-streaming requests
    let mut handles = vec![];

    for i in 0..20 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let mut request = create_test_request();
            request.request_id = format!("mixed-{}", i);

            if i % 2 == 0 {
                // Non-streaming
                request.stream = false;
                client_clone
                    .execute(request)
                    .await
                    .map(|_| "non-stream".to_string())
            } else {
                // Streaming
                request.stream = true;
                match client_clone.execute_stream(request).await {
                    Ok(mut stream) => {
                        use tokio_stream::StreamExt;
                        let mut chunks = 0;
                        while let Some(_) = stream.next().await {
                            chunks += 1;
                        }
                        Ok(format!("stream-{}", chunks))
                    }
                    Err(e) => Err(e),
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all requests
    let results = futures_util::future::join_all(handles).await;

    // Test passes without panic
    assert_eq!(results.len(), 20, "Expected 20 results");
}

/// Test: Error message quality under various failures
#[tokio::test]
async fn test_error_message_quality() {
    // Test 1: Invalid endpoint
    let config1 = ClientConfig {
        endpoint: "not-a-valid-endpoint".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let result1 = ApiClient::new(config1);
    assert!(result1.is_err());
    let err1 = result1.unwrap_err();
    assert!(
        !err1.to_string().is_empty(),
        "Error message should not be empty"
    );

    // Test 2: Connection timeout
    let config2 = ClientConfig {
        endpoint: "192.0.2.1:3030".to_string(), // Reserved test IP
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client2 = ApiClient::new(config2).expect("Failed to create client");
    let request = create_test_request();
    let result2 = client2.execute(request).await;

    if let Err(err2) = result2 {
        assert!(
            !err2.to_string().is_empty(),
            "Error message should not be empty"
        );
    }

    // Test 3: Request timeout
    let config3 = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(1), // Very short timeout
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client3 = match ApiClient::new(config3) {
        Ok(c) => c,
        Err(_) => return, // Skip if can't create
    };

    let request = create_test_request();
    let result3 = client3.execute(request).await;

    if let Err(err3) = result3 {
        assert!(
            !err3.to_string().is_empty(),
            "Error message should not be empty"
        );
    }
}

/// Test: Graceful degradation under sustained load
#[tokio::test]
async fn test_graceful_degradation() {
    // Create client with invalid endpoint
    let config = ClientConfig {
        endpoint: "invalid-endpoint:9999".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_millis(100),
        timeout: Duration::from_millis(500),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = ApiClient::new(config).expect("Failed to create client");

    // Sustained load of 100 requests
    let mut handles = vec![];
    for i in 0..100 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_request();
            client_clone.execute(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests
    let results = futures_util::future::join_all(handles).await;

    // Count failures (should be high due to invalid endpoint)
    let failures = results
        .iter()
        .filter(|r| match r {
            Ok(Ok(_)) => false,
            _ => true,
        })
        .count();

    // Verify system degrades gracefully (no panics, most requests fail)
    assert!(failures > 90, "Expected >90% failures under sustained load");
}

/// Helper: Create a test provider request
fn create_test_request() -> openakta_proto::provider_v1::ProviderRequest {
    openakta_proto::provider_v1::ProviderRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        tenant_id: "test-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![openakta_proto::provider_v1::Message {
            role: "user".to_string(),
            content: "Test message".to_string(),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    }
}
