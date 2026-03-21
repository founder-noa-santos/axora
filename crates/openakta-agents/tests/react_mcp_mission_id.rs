//! Integration: ReAct → MCP must carry coordinator-trusted `mission_id` on the wire, ignoring
//! LLM-supplied `mission_id` in tool arguments when `ToolCallRequest.mission_id` is set.

use openakta_agents::hitl::{HitlConfig, MissionHitlGate};
use openakta_agents::{Action, ToolSet};
use openakta_mcp_server::{McpService, McpServiceConfig};
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
use openakta_proto::mcp::v1::tool_service_server::ToolServiceServer;
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

#[derive(Deserialize)]
struct CheckpointHeader {
    mission_id: String,
}

#[tokio::test]
async fn react_mcp_call_carries_mission_id() {
    const TRUSTED: &str = "trusted-m-1";
    const EVIL: &str = "evil-from-llm-hallucination";

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path()).unwrap();

    let checkpoint_dir = tmp.path().join(".openakta/checkpoints");
    let (_bus_tx, bus_rx) =
        tokio::sync::broadcast::channel::<openakta_proto::collective::v1::Message>(8);
    let gate = Arc::new(MissionHitlGate::new(
        HitlConfig {
            checkpoint_dir,
            ..Default::default()
        },
        Some((_bus_tx, bus_rx)),
    ));
    gate.register_mission_start(TRUSTED).unwrap();

    let cfg = McpServiceConfig {
        workspace_root: tmp.path().to_path_buf(),
        dense_store_path: tmp.path().join("v.db"),
        hitl_gate: Some(Arc::clone(&gate)),
        ..Default::default()
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);
    let mcp = McpService::with_config(cfg);
    let server_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GraphRetrievalServiceServer::new(mcp.clone()))
            .add_service(ToolServiceServer::new(mcp))
            .serve_with_incoming(incoming)
            .await
    });

    let endpoint = format!("http://{}", addr);
    let tools = ToolSet::with_mcp_endpoint(endpoint, "test-model")
        .with_workspace_root(tmp.path().to_string_lossy().to_string())
        .with_mcp_runtime_context("sess-origin", "worker", Some(TRUSTED.into()))
        .with_hitl(Arc::clone(&gate), TRUSTED);

    let options_json = r#"[{"id":"a","label":"A","description":"","is_default":true},{"id":"b","label":"B","description":"","is_default":false}]"#;
    let action = Action::new(
        "request_user_input",
        serde_json::json!({
            "mission_id": EVIL,
            "turn_index": "1",
            "text": "Pick one",
            "kind": "single",
            "options_json": options_json,
            "constraints_json": r#"{"min_selections":1,"max_selections":1}"#,
        }),
    );

    tools.execute(&action).await.unwrap();

    let cp_path = tmp
        .path()
        .join(".openakta/checkpoints")
        .join(format!("{TRUSTED}.json"));
    let raw = std::fs::read_to_string(&cp_path).unwrap();
    let header: CheckpointHeader = serde_json::from_str(&raw).unwrap();
    assert_eq!(header.mission_id, TRUSTED);

    let evil_path = tmp
        .path()
        .join(".openakta/checkpoints")
        .join(format!("{EVIL}.json"));
    assert!(
        !evil_path.exists(),
        "hallucinated mission_id must not create a checkpoint"
    );

    server_task.abort();
}
