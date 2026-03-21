# OPENAKTA Technical Architecture: Inter-Agent Communication

**Created:** 2026-03-16  
**Status:** ✅ Approved  
**Based On:** R-02 Research, ADR-009, ADR-016, ADR-017, ADR-018, ADR-019, ADR-020

---

## System Overview

OPENAKTA's inter-agent communication system implements a **hybrid architecture** combining deterministic state machine orchestration with asynchronous pub/sub for utility agents.

```
┌─────────────────────────────────────────────────────────────────┐
│              OPENAKTA Communication Stack                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  Application Layer                                               │
│  ┌─────────────────┐  ┌─────────────────┐                       │
│  │  State Machine  │  │  Semantic       │                       │
│  │  Orchestrator   │  │  Compression    │                       │
│  └─────────────────┘  └─────────────────┘                       │
├─────────────────────────────────────────────────────────────────┤
│  Message Layer                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Protocol Buffers (AgentMessage)                        │    │
│  │  - Strict typing, 45% smaller than JSON                 │    │
│  │  - Cryptographic signatures                             │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  Transport Layer                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  NATS JetStream                                          │    │
│  │  - Pub/Sub + Queue Groups                                │    │
│  │  - Exactly-once delivery                                 │    │
│  │  - Dead Letter Queues                                    │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  Tool Interface                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Model Context Protocol (MCP) over gRPC                 │    │
│  │  - Sandboxed tool execution                             │    │
│  │  - File system, terminal, git, etc.                     │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Communication Topology

### Hierarchical (Federated) Topology

```
                    ┌─────────────────┐
                    │   Orchestrator  │
                    │   (State Graph) │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼────┐        ┌────▼────┐        ┌────▼────┐
    │  Lead   │        │  Lead   │        │  Lead   │
    │  (FE)   │        │  (BE)   │        │  (QA)   │
    └────┬────┘        └────┬────┘        └────┬────┘
         │                   │                   │
    ┌────┴────┐        ┌────┴────┐        ┌────┴────┐
    │ Workers │        │ Workers │        │ Workers │
    └─────────┘        └─────────┘        └─────────┘
    
    NATS JetStream (Pub/Sub + Queue Groups)
```

**Topology Properties:**
- **Star (Level 1):** Orchestrator → Team Leads
- **Federated (Level 2):** Team Leads → Workers
- **Pub/Sub (Utility):** Background agents (linter, security scanner)

**Scalability:**
- 10 agents: Simple orchestrator sufficient
- 100 agents: Hierarchical topology required
- 1000+ agents: Horizontal scaling via NATS queue groups

---

## Message Schema

### Protobuf Definition

```protobuf
// openakta/proto/agent_message.proto
syntax = "proto3";

package openakta.v1;

enum MessageType {
  TASK_ASSIGN = 0;
  TASK_RESULT = 1;
  STATE_DELTA = 2;
  LOCK_REQUEST = 3;
  CAPABILITY_AD = 4;
}

message AgentMessage {
  // Conversation Management
  string message_id = 1;              
  string conversation_thread_id = 2;  
  MessageType type = 3;
  
  // Routing
  string sender_id = 4;
  string recipient_id = 5;            
  
  // Semantic Context (Optional, for future C2C)
  bytes latent_context = 6;           
  
  // Content Payload
  string payload_mime_type = 7;       // e.g., "application/json", "text/x-diff"
  bytes content = 8;                  
  
  // Observability & Security
  int64 timestamp = 9;
  int32 ttl_seconds = 10;
  bytes cryptographic_signature = 11; 
}

message AgentCard {
  string agent_id = 1;
  string role = 2;
  repeated Capability capabilities = 3;
  repeated string allowed_tools = 4;
  repeated string denied_tools = 5;
}

message Capability {
  string resource = 1;
  repeated string actions = 2;
  repeated string scopes = 3;
}
```

### Message Types

| Type | Purpose | Payload Example |
|------|---------|-----------------|
| `TASK_ASSIGN` | Delegate work | `{task: "implement login", priority: "high"}` |
| `TASK_RESULT` | Return results | `{status: "success", diff: "...", tests_passed: true}` |
| `STATE_DELTA` | State update | `{file: "src/auth.rs", diff: "@@ -10,7 +10,9 @@"}` |
| `LOCK_REQUEST` | Resource lock | `{resource: "src/main.rs", operation: "write"}` |
| `CAPABILITY_AD` | Advertise skills | `{role: "tester", tools: ["pytest", "jest"]}` |

### Conversation Threading

Every message includes:
- `message_id`: Unique identifier (UUID v4)
- `conversation_thread_id`: Groups related messages
- `timestamp`: Unix timestamp (nanoseconds)
- `ttl_seconds`: Message expiration (prevents stale processing)

**Causality Tracking:**
```rust
struct ConversationThread {
    thread_id: String,
    parent_message_id: Option<String>,
    messages: Vec<MessageId>,
    state: ThreadState,
}
```

---

## Transport Layer: NATS JetStream

### Why NATS JetStream?

| Feature | NATS JetStream | Kafka | Redis Pub/Sub |
|---------|---------------|-------|---------------|
| Latency (p50) | <1ms | 10-50ms | <1ms |
| Throughput | 1M+ msg/sec | 10M+ msg/sec | 5M+ msg/sec |
| Persistence | RAFT-based | Log-based | Streams (optional) |
| Queue Groups | ✅ Native | ✅ Consumer Groups | ❌ Custom |
| Exactly-Once | ✅ | ✅ | ❌ |
| Operational Complexity | Low | High | Medium |
| Rust Client | ✅ Mature | ✅ Good | ✅ Mature |

### NATS Subject Hierarchy

```
openakta.orchestrator.lead           // Orchestrator → Team Leads
openakta.team.frontend.worker        // Frontend team workers
openakta.team.backend.worker         // Backend team workers
openakta.team.qa.worker              // QA team workers
openakta.utility.linter              // Linter (pub/sub)
openakta.utility.security-scanner    // Security scanner (pub/sub)
openakta.dlq                         // Dead Letter Queue
```

### Queue Groups (Horizontal Scaling)

```rust
// All frontend workers share the same queue group
// Messages are load-balanced across available workers
let worker = nats_client
    .queue_subscribe("openakta.team.frontend.worker", "frontend-pool")
    .await?;
```

**Properties:**
- Messages distributed round-robin within queue group
- Automatic failover if worker crashes
- Scale by adding more workers to queue group

### Stream Configuration

```rust
use async_nats::jetstream::stream;

let stream_config = stream::Config {
    name: "OPENAKTA_TASKS".to_string(),
    subjects: vec!["openakta.*".to_string()],
    retention: stream::Retention::WorkQueue,
    max_messages_per_subject: 1000,
    storage: stream::Storage::File,
    replicas: 1, // Single node for local-first
    ..Default::default()
};
```

---

## Orchestration: State Machine

### State Graph Architecture

```rust
use std::collections::HashMap;

pub enum AgentState {
    Idle,
    Thinking,
    Executing,
    WaitingForReview,
    Blocked,
    Completed,
}

pub struct StateGraph {
    nodes: HashMap<StateId, State>,
    edges: Vec<Edge>,
    current_state: StateId,
    checkpoint: Option<Checkpoint>,
}

pub struct Edge {
    from: StateId,
    to: StateId,
    condition: TransitionCondition,
    error_handler: Option<ErrorHandler>,
}
```

### Transition Conditions

```rust
pub enum TransitionCondition {
    // Automatic transitions
    Always,
    OnSuccess,
    OnFailure,
    
    // Conditional transitions
    Condition(Box<dyn Fn(&SharedState) -> bool>),
    
    // Time-based
    Timeout(Duration),
    
    // External events
    MessageReceived(MessageType),
}
```

### Checkpointing (Time-Travel Debugging)

```rust
pub struct Checkpoint {
    id: String,
    timestamp: SystemTime,
    state_snapshot: SharedState,
    conversation_history: Vec<AgentMessage>,
    git_commit: Option<String>,
}

impl StateGraph {
    pub fn checkpoint(&mut self) -> CheckpointId {
        let checkpoint = Checkpoint {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            state_snapshot: self.state.clone(),
            conversation_history: self.history.clone(),
            git_commit: self.get_current_commit(),
        };
        
        self.checkpoints.push(checkpoint);
        checkpoint.id
    }
    
    pub fn restore(&mut self, checkpoint_id: &CheckpointId) -> Result<()> {
        let checkpoint = self.checkpoints
            .iter()
            .find(|c| c.id == *checkpoint_id)
            .ok_or(Error::CheckpointNotFound)?;
        
        self.state = checkpoint.state_snapshot.clone();
        self.current_state = checkpoint.initial_state;
        
        Ok(())
    }
}
```

---

## Token Efficiency

### State Delta Encoding

Instead of sending full files, send only diffs:

```rust
// Bad: Full file (3500 tokens)
pub struct FullFileUpdate {
    file_path: String,
    content: String,  // Entire file content
}

// Good: Diff (400 tokens)
pub struct StateDelta {
    file_path: String,
    unified_diff: String,  // @@ -10,7 +10,9 @@
    ast_mutations: Vec<AstMutation>,
}

// Example diff
let delta = StateDelta {
    file_path: "src/auth.rs".to_string(),
    unified_diff: r#"
@@ -10,7 +10,9 @@
 use crate::error::AuthError;
+use crate::error::TokenError;
 
 pub struct AuthManager {
+    token_expiry: Duration,
     secret: String,
 }
"#.to_string(),
};
```

**Token Savings:** 88% reduction (3500 → 400 tokens)

### Shared Context Store

Instead of passing conversation history, use shared vector store:

```rust
pub struct SharedContextStore {
    vector_db: LanceDB,
}

impl SharedContextStore {
    pub async fn write_finding(&self, finding: &Finding) -> Result<FindingId> {
        let embedding = self.embedder.embed(&finding.summary).await?;
        
        let doc = Document {
            id: Uuid::new_v4().to_string(),
            content: finding.summary.clone(),
            embedding,
            metadata: FindingMetadata {
                agent_id: finding.agent_id.clone(),
                thread_id: finding.thread_id.clone(),
                timestamp: SystemTime::now(),
                file_refs: finding.file_refs.clone(),
            },
        };
        
        self.vector_db.insert(doc).await
    }
    
    pub async fn get_context(&self, query: &str, thread_id: &str) -> Result<Vec<Finding>> {
        let query_embedding = self.embedder.embed(query).await?;
        
        let results = self.vector_db
            .search(query_embedding)
            .filter(|doc| doc.metadata.thread_id == thread_id)
            .top_k(10)
            .await?;
        
        Ok(results.into_iter().map(|d| d.into_finding()).collect())
    }
}
```

**Benefits:**
- No conversation history bloat
- Agents retrieve only relevant context
- Persistent across sessions
- Query-based, not linear

---

## Security Model

### Capability-Based Authorization

```rust
pub struct AgentCapabilities {
    agent_id: String,
    role: String,
    capabilities: Vec<Capability>,
}

pub struct Capability {
    resource: String,  // "filesystem", "terminal", "git"
    actions: Vec<String>,  // ["read", "write", "execute"]
    scopes: Vec<String>,  // ["./src/**", "!./secrets/**"]
}

impl Capability {
    pub fn can_access(&self, resource: &str, action: &str, path: &Path) -> bool {
        // Check resource type
        if self.resource != resource {
            return false;
        }
        
        // Check action
        if !self.actions.contains(&action.to_string()) {
            return false;
        }
        
        // Check path scope (glob matching)
        for scope in &self.scopes {
            if scope.starts_with('!') {
                // Exclusion pattern
                if glob_match(&scope[1..], path) {
                    return false;
                }
            } else {
                // Inclusion pattern
                if glob_match(scope, path) {
                    return true;
                }
            }
        }
        
        false
    }
}
```

### Message Signing

```rust
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};

pub struct SecureMessage {
    message: AgentMessage,
    signature: Signature,
}

impl SecureMessage {
    pub fn sign(message: &AgentMessage, private_key: &SigningKey) -> Self {
        let bytes = message.encode_to_vec();
        let signature = private_key.sign(&bytes);
        
        Self {
            message: message.clone(),
            signature,
        }
    }
    
    pub fn verify(&self, public_key: &VerifyingKey) -> Result<bool> {
        let bytes = self.message.encode_to_vec();
        public_key.verify(&bytes, &self.signature)
            .map_err(|_| Error::InvalidSignature)
    }
}
```

**Security Properties:**
- ✅ Message integrity (tamper detection)
- ✅ Authentication (sender verification)
- ✅ Non-repudiation (audit trail)
- ✅ Replay attack prevention (timestamp + TTL)

---

## Tool Interface: Model Context Protocol (MCP)

### MCP Server Architecture

```rust
use tonic::{Request, Response, Status};
use openakta_proto::mcp::v1::{
    tool_service_server::ToolService,
    ToolRequest, ToolResponse, ListToolsRequest, ToolDefinition,
};

pub struct McpServer {
    capabilities: AgentCapabilities,
    tools: HashMap<String, Box<dyn Tool>>,
}

#[tonic::async_trait]
impl ToolService for McpServer {
    async fn list_tools(
        &self,
        _request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        let tools = self.tools
            .values()
            .map(|t| t.definition())
            .collect();
        
        Ok(Response::new(ListToolsResponse { tools }))
    }
    
    async fn execute_tool(
        &self,
        request: Request<ToolRequest>,
    ) -> Result<Response<ToolResponse>, Status> {
        let req = request.into_inner();
        
        // Validate capability
        if !self.capabilities.can_access(
            &req.tool_name,
            "execute",
            Path::new(&req.parameters["path"])
        ) {
            return Err(Status::permission_denied("Insufficient capabilities"));
        }
        
        // Execute tool
        let tool = self.tools.get(&req.tool_name)
            .ok_or_else(|| Status::not_found("Tool not found"))?;
        
        let result = tool.execute(req.parameters).await?;
        
        Ok(Response::new(ToolResponse {
            result: Some(result),
            error: None,
        }))
    }
}
```

### Available Tools

| Tool | Actions | Scopes |
|------|---------|--------|
| File System | read, write, delete | `./**` (excludes `./secrets/**`) |
| Terminal | execute | Limited commands (git, cargo, npm) |
| Git | commit, diff, log, status | Full repo access |
| HTTP | GET, POST, PUT, DELETE | Whitelisted domains only |
| Database | query | Read-only replicas |

---

## Fault Tolerance

### Dead Letter Queue (DLQ)

```rust
pub struct DeadLetterQueue {
    nats_client: Client,
    max_retries: u32,
}

impl DeadLetterQueue {
    pub async fn handle_failure(
        &self,
        message: &AgentMessage,
        error: &AgentError,
        retry_count: u32,
    ) -> Result<()> {
        if retry_count >= self.max_retries {
            // Move to DLQ for human review
            self.nats_client
                .publish("openakta.dlq", message.encode_to_vec().into())
                .await?;
            
            // Notify debugger agent
            self.nats_client
                .publish("openakta.utility.debugger", DebuggerNotification {
                    message: message.clone(),
                    error: error.clone(),
                }.encode_to_vec().into())
                .await?;
        } else {
            // Let NATS JetStream redeliver (exponential backoff)
            // NACK the message
            return Err(Error::Nack);
        }
        
        Ok(())
    }
}
```

### Retry Policy

```rust
pub struct RetryPolicy {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl RetryPolicy {
    pub fn exponential_backoff(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay * (self.multiplier.powi(attempt as i32));
        delay.min(self.max_delay)
    }
}

// Default policy
let default_retry_policy = RetryPolicy {
    max_retries: 5,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
};
```

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Message Latency (p50) | <1ms | NATS publish → subscribe |
| Message Latency (p95) | <5ms | NATS publish → subscribe |
| Serialization (Protobuf) | <100μs | encode + decode |
| Signature Verification | <2ms | ed25519 verify |
| State Transition | <10ms | Graph traversal + validation |
| DLQ Processing | <100ms | Failure → DLQ publish |

---

## Dependencies (Rust Crates)

```toml
[dependencies]
# NATS client
async-nats = "0.36"

# Protocol Buffers
prost = "0.13"
tonic = "0.12"

# Cryptography
ed25519-dalek = "2.1"
jsonwebtoken = "9.3"

# State machine
async-graphql = "7.0"  # For state graph DSL

# Utilities
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
```

---

## Open Questions

### 1. MCP over NATS Extension
**Question:** Can MCP be extended to work over NATS instead of gRPC?

**Pros:**
- Unified transport (NATS for everything)
- Simpler architecture

**Cons:**
- MCP spec currently only supports stdio/SSE/HTTP
- Would require custom extension

**Decision:** Defer - use MCP over gRPC for tools, NATS for agent communication

---

### 2. Cache-to-Cache (C2C) Degradation
**Question:** What's the cross-model degradation rate for latent transfer?

**Unknown:**
- Accuracy loss when transferring KV cache between different model families
- Requires homogeneous models for best results

**Decision:** Implement latent_context field in schema, but use text-based transfer initially. Research C2C after MVP.

---

### 3. Central Orchestrator Bottleneck
**Question:** At what point does the orchestrator become a bottleneck?

**Unknown:**
- Max state transitions per second
- LLM TTFT remains the true bottleneck

**Decision:** Profile with 10, 50, 100 concurrent agents. Add horizontal scaling (queue groups) if needed.

---

## Testing Strategy

### Unit Tests
- Protobuf serialization/deserialization
- Message signing and verification
- Capability validation
- State transition logic
- NATS subject routing

### Integration Tests
- End-to-end message flow (publish → subscribe)
- Queue group load balancing
- DLQ handling
- Checkpoint restore

### Chaos Tests
- Agent crash during task execution
- Network partition (NATS unavailable)
- Poison pill messages
- Replay attack simulation

---

## Related Documents

- [ADR-009: Inter-Agent Communication Protocol](../research/DECISIONS.md#adr-009)
- [ADR-016: Message Schema Design](../research/DECISIONS.md#adr-016)
- [ADR-017: Token Efficiency](../research/DECISIONS.md#adr-017)
- [ADR-018: Agent Orchestration Pattern](../research/DECISIONS.md#adr-018)
- [ADR-019: Security Model](../research/DECISIONS.md#adr-019)
- [ADR-020: Tool Interface (MCP)](../research/DECISIONS.md#adr-020)
- [R-02 Research Findings](./findings/inter-agent-communication/R-02-result.md)

---

## Next Steps

1. **Prototype NATS transport** - Basic pub/sub with Protobuf
2. **Implement state machine** - Core orchestration engine
3. **Build MCP server** - Tool sandboxing
4. **Add security layer** - Capability validation, message signing
5. **Benchmark performance** - Validate latency targets
6. **Chaos testing** - Fault tolerance validation
