//! OPENAKTA Backend Integration Tests
//!
//! These tests verify the backend works correctly without the frontend.
//!
//! Run with:
//! ```bash
//! cargo test -p openakta-core --test integration
//! ```

use openakta_core::{CollectiveServer, CoreConfig};
use openakta_proto::collective::v1::{
    collective_service_client::CollectiveServiceClient, ListAgentsRequest, SubmitTaskRequest,
};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, timeout, Duration, Instant};

async fn wait_for_collective_client(
    endpoint: &str,
) -> CollectiveServiceClient<tonic::transport::Channel> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_error = None;

    loop {
        match CollectiveServiceClient::connect(endpoint.to_string()).await {
            Ok(client) => return client,
            Err(err) if Instant::now() < deadline => {
                last_error = Some(err.to_string());
                sleep(Duration::from_millis(25)).await;
            }
            Err(err) => {
                panic!(
                    "collective server at {endpoint} did not become ready: {}",
                    last_error.unwrap_or_else(|| err.to_string())
                );
            }
        }
    }
}

/// Test the collective server
#[tokio::test]
async fn test_collective_server_startup() {
    println!("🚀 Testing CollectiveServer startup...");

    let config = CoreConfig::default();
    let _server = CollectiveServer::new(config);

    println!("✅ Server startup test passed");
}

/// Test task submission via gRPC
#[tokio::test]
async fn test_submit_task_grpc() {
    println!("📝 Testing task submission via gRPC...");

    // Start server in background
    let config = CoreConfig {
        port: 50052, // Use different port for tests
        ..Default::default()
    };

    let server = CollectiveServer::new(config.clone());
    let server_handle = tokio::spawn(async move {
        server.serve().await.unwrap();
    });

    let mut client = wait_for_collective_client("http://127.0.0.1:50052").await;

    // Submit task
    let request = SubmitTaskRequest {
        title: "Test Task".to_string(),
        description: "Testing from integration test".to_string(),
        assignee_id: "agent-1".to_string(),
    };

    let response = client
        .submit_task(request)
        .await
        .expect("Failed to submit task");

    // Verify response
    assert!(
        response.into_inner().task.is_some(),
        "Task should be created"
    );

    // Cleanup
    server_handle.abort();

    println!("✅ Task submission test passed");
}

/// Test agent listing via gRPC
#[tokio::test]
async fn test_list_agents_grpc() {
    println!("📋 Testing agent listing via gRPC...");

    // Start server in background
    let config = CoreConfig {
        port: 50053, // Use different port for tests
        ..Default::default()
    };

    let server = CollectiveServer::new(config.clone());
    let server_handle = tokio::spawn(async move {
        server.serve().await.unwrap();
    });

    let mut client = wait_for_collective_client("http://127.0.0.1:50053").await;

    // List agents
    let request = ListAgentsRequest {
        filter_status: 0, // All statuses
    };

    let response = client
        .list_agents(request)
        .await
        .expect("Failed to list agents");

    // Verify response (should have at least empty list)
    let agents = response.into_inner().agents;
    let _ = agents.len();

    // Cleanup
    server_handle.abort();

    println!("✅ Agent listing test passed");
}

/// Test configuration loading
#[tokio::test]
async fn test_config_from_toml() {
    println!("⚙️  Testing configuration loading from TOML...");

    // Create temporary config file
    let config_content = r#"
        bind_address = "127.0.0.1"
        port = 50054
        database_path = "/tmp/test_openakta.db"
        max_concurrent_agents = 5
        frame_duration_ms = 32
        debug = true
    "#;

    let config_path = "/tmp/test_openakta_config.toml";
    std::fs::write(config_path, config_content).expect("Failed to write config file");

    // Load config
    let config = CoreConfig::from_file(&config_path.into()).expect("Failed to load config");

    // Verify config
    assert_eq!(config.bind_address, "127.0.0.1");
    assert_eq!(config.port, 50054);
    assert_eq!(config.max_concurrent_agents, 5);
    assert_eq!(config.frame_duration_ms, 32);
    assert!(config.debug);

    // Cleanup
    std::fs::remove_file(config_path).ok();

    println!("✅ Configuration loading test passed");
}

/// Graceful shutdown path used by the daemon (`serve_with_shutdown` + `Notify`).
///
/// Run manually: `cargo test -p openakta-core --test integration collective_graceful_shutdown_serve_with_shutdown -- --ignored`
#[tokio::test]
#[ignore = "binds an ephemeral port; run with --ignored"]
async fn collective_graceful_shutdown_serve_with_shutdown() {
    let shutdown = Arc::new(Notify::new());
    let notify = shutdown.clone();
    let config = CoreConfig {
        port: 0,
        ..Default::default()
    };
    let server = CollectiveServer::new(config);
    let handle = tokio::spawn(async move {
        server
            .serve_with_shutdown(async move {
                notify.notified().await;
            })
            .await
    });

    tokio::task::yield_now().await;
    shutdown.notify_waiters();

    let joined = timeout(Duration::from_secs(5), handle)
        .await
        .expect("timed out waiting for collective shutdown");
    joined
        .expect("task join failed")
        .expect("serve_with_shutdown failed");
}

/// Test frame executor
#[tokio::test]
async fn test_frame_executor() {
    println!("🎬 Testing frame executor...");

    use openakta_core::{FrameContext, FrameExecutor};
    use std::sync::Arc;
    use tokio::sync::{Notify, RwLock};
    use tokio::time::{timeout, Duration};

    let mut executor = FrameExecutor::new(60); // 60 FPS

    let frames_processed = Arc::new(RwLock::new(0));
    let frames_clone = frames_processed.clone();
    let done = Arc::new(Notify::new());
    let done_clone = done.clone();

    tokio::spawn(async move {
        executor
            .run(move |ctx: FrameContext| {
                let frames = frames_clone.clone();
                let done = done_clone.clone();
                async move {
                    let mut count = frames.write().await;
                    *count += 1;
                    if ctx.frame.number >= 10 {
                        done.notify_waiters();
                    }
                }
            })
            .await;
    });

    timeout(Duration::from_secs(2), done.notified())
        .await
        .expect("frame executor did not process 10 frames");

    let count = *frames_processed.read().await;
    assert!(count >= 10);

    println!("✅ Frame executor test passed");
}
