# HITL final polish (H4, H9, integration test)

## H4 — Checkpoint vs. publish (transactional boundary)

**Problem:** `broadcast::Sender::send` returns an error when there are no active receivers. A successful checkpoint followed by a failed publish left missions stuck in `pending_answer` with on-disk state.

**Refactors:**

1. **`MissionHitlGate::new(config, bus)`** now takes `Option<(broadcast::Sender<Message>, broadcast::Receiver<Message>)>`. The gate **retains** the `Receiver` in `_broadcast_hold` so at least one subscriber always exists and `send` is reliable for “publisher side” transactional semantics (or fails only if the bus is closed).
2. **`raise_question`** already: persists checkpoint → `publish_question` → on either failure, **`rollback_raise`** and removes checkpoint when publish fails after checkpoint.
3. **`publish_question`** — keeps **strict** `send`: errors are **not** swallowed; callers get `Err` and rollback runs.

**Call-site rule:** Replace `MissionHitlGate::new(cfg, Some(tx))` with:

```rust
let (tx, rx) = tokio::sync::broadcast::channel(1024);
let gate = Arc::new(MissionHitlGate::new(cfg, Some((tx.clone(), rx))));
// pass `tx` to CollectiveServer / any other fan-out as today
```

Files updated: `openakta-agents/src/hitl.rs`, `openakta-core/src/bootstrap.rs`, `openakta-daemon/src/main.rs`, `openakta-core/src/server.rs` (tests).

---

## H9 — Sensitive routing + cryptographic answer token

### Sensitive routing

1. **Blackboard global publish on answer:** `MissionHitlGate::submit_answer` returns `HitlSubmitAnswerOutcome { suppress_global_blackboard }` when `QuestionEnvelope.sensitive` was set at raise time. **`CollectiveServer::ingest_hitl_answer`** skips `SharedBlackboard::publish` when that flag is set.
2. **Sensitive answers on the collective bus:** For the same flag, **`submit_answer` does not `send` the answer** on the tokio `broadcast` channel (session handles answers via gRPC / waiters, not global watcher fan-out).
3. **Wildcard `stream_messages`:** If `StreamMessagesRequest.agent_id` is empty (admin/tap), **sensitive `Question` messages** (`human_question.sensitive`) are **never** yielded — prevents global snooping.

### Cryptographic `expiry_token`

When `HitlConfig.answer_hmac_secret` is set (e.g. via `OPENAKTA_HITL_HMAC_SECRET` hex through `with_hmac_secret_from_env`):

- **`raise_question`** requires `expires_at` and sets `expiry_token` to `v1.<hex(HMAC-SHA256)>` over a canonical payload: `mission_id \\0 question_id \\0 expires_at (seconds, nanos BE)`.
- **`submit_answer`** verifies the MAC with the same payload; **wall-clock expiry alone is not sufficient** when HMAC mode is enabled.

Implementation: `sign_hitl_token`, `verify_hitl_token`, `hitl_hmac_payload` in `crates/openakta-agents/src/hitl.rs`.

---

## Integration test — `react_mcp_call_carries_mission_id`

**Location:** `crates/openakta-agents/tests/react_mcp_mission_id.rs`

**Contract under test:** `ToolSet` sets trusted `mission_id` on `ToolCallRequest`. `request_user_input` in `openakta-mcp-server` **must** prefer `ctx.request.mission_id` over any `mission_id` inside tool JSON when the trusted id is non-empty. The checkpoint file on disk must be keyed only by the trusted ID.

**Supporting API:** `ToolSet::with_workspace_root` (`crates/openakta-agents/src/react.rs`) so tests (and embedders) can set `workspace_root` without changing process CWD.

### Source — `react_mcp_call_carries_mission_id`

```rust
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
    let (_bus_tx, bus_rx) = tokio::sync::broadcast::channel::<openakta_proto::collective::v1::Message>(
        8,
    );
    let gate = Arc::new(MissionHitlGate::new(
        HitlConfig {
            checkpoint_dir,
            ..Default::default()
        },
        Some((_bus_tx, bus_rx)),
    ));
    gate.register_mission_start(TRUSTED).unwrap();

    let mut cfg = McpServiceConfig::default();
    cfg.workspace_root = tmp.path().to_path_buf();
    cfg.dense_store_path = tmp.path().join("v.db");
    cfg.hitl_gate = Some(Arc::clone(&gate));

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

    let cp_path = tmp.path().join(".openakta/checkpoints").join(format!("{TRUSTED}.json"));
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
```

---

## Re-exports

`HitlSubmitAnswerOutcome` is exported from `openakta-agents` (`src/lib.rs`) for downstream servers and tests.
