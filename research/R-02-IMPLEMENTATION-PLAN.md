# R-02 Research Summary & Implementation Plan

**Date:** 2026-03-16  
**Status:** ✅ Research Complete → 🔄 Ready for Implementation  
**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Architecture:** [architecture-communication.md](../docs/architecture-communication.md)

---

## Executive Summary

R-02 research on **Inter-Agent Communication** is complete. Six architectural decisions have been made (ADR-009, ADR-016, ADR-017, ADR-018, ADR-019, ADR-020) defining a production-grade communication system for multi-agent coordination.

**Key Outcome:** AXORA will implement a **hybrid architecture** with:
- NATS JetStream (transport layer)
- Protocol Buffers (message serialization)
- Hierarchical state machine (orchestration)
- Model Context Protocol (tool interface)
- Capability-based security (zero-trust)

**Competitive Advantage:** This architecture surpasses AutoGen (too chaotic) and CrewAI (too rigid) by combining LangGraph's determinism with Devin's context engineering.

---

## Decisions Made

| ADR | Decision | Impact |
|-----|----------|--------|
| ADR-009 | NATS JetStream + Protobuf + State Machine | Foundation for all agent communication |
| ADR-016 | Protobuf AgentMessage schema | 45% smaller payload, type safety |
| ADR-017 | Multi-layer token optimization | 88% reduction in token costs |
| ADR-018 | Hierarchical state machine | Deterministic execution, no hallucination loops |
| ADR-019 | Capability-based security | Zero-trust architecture |
| ADR-020 | Model Context Protocol (MCP) | Standardized tool interface |

---

## Implementation Roadmap

### Sprint 0: Foundation (Week 1-2)

**Goal:** Set up NATS infrastructure and Protobuf schemas

**Tasks:**
- [ ] Add dependencies to `Cargo.toml`:
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
  ```

- [ ] Create `crates/axora-communication/` crate structure
- [ ] Define Protobuf schemas (`proto/agent_message.proto`):
  ```protobuf
  syntax = "proto3";
  
  enum MessageType {
    TASK_ASSIGN = 0;
    TASK_RESULT = 1;
    STATE_DELTA = 2;
    LOCK_REQUEST = 3;
    CAPABILITY_AD = 4;
  }
  
  message AgentMessage {
    string message_id = 1;
    string conversation_thread_id = 2;
    MessageType type = 3;
    string sender_id = 4;
    string recipient_id = 5;
    bytes latent_context = 6;
    string payload_mime_type = 7;
    bytes content = 8;
    int64 timestamp = 9;
    int32 ttl_seconds = 10;
    bytes cryptographic_signature = 11;
  }
  ```

- [ ] Set up embedded NATS server for development
- [ ] Create test fixtures (sample messages)

**Deliverable:** Empty crate with dependencies, Protobuf compilation working

---

### Sprint 1: NATS Transport Layer (Week 3-4)

**Goal:** Implement NATS JetStream integration

**Tasks:**
- [ ] Implement NATS client wrapper:
  ```rust
  pub struct NatsTransport {
      client: Client,
      jetstream: Context,
  }
  
  impl NatsTransport {
      pub async fn publish(&self, subject: &str, message: &AgentMessage) -> Result<()>;
      pub async fn subscribe(&self, subject: &str) -> Result<MessageStream>;
      pub async fn queue_subscribe(&self, subject: &str, queue: &str) -> Result<MessageStream>;
  }
  ```

- [ ] Configure JetStream streams:
  ```rust
  let stream_config = stream::Config {
      name: "AXORA_TASKS".to_string(),
      subjects: vec!["axora.*".to_string()],
      retention: stream::Retention::WorkQueue,
      storage: stream::Storage::File,
      replicas: 1,
      ..Default::default()
  };
  ```

- [ ] Implement subject hierarchy:
  - `axora.orchestrator.lead`
  - `axora.team.frontend.worker`
  - `axora.team.backend.worker`
  - `axora.team.qa.worker`
  - `axora.utility.linter`
  - `axora.dlq`

- [ ] Implement queue groups for horizontal scaling
- [ ] Benchmark: message latency, throughput

**Deliverable:** `axora-transport` crate with NATS integration

**Success Criteria:**
- Message latency (p50): <1ms
- Message latency (p95): <5ms
- Throughput: >10K msg/sec

---

### Sprint 2: Message Serialization (Week 5)

**Goal:** Implement Protobuf serialization with signing

**Tasks:**
- [ ] Generate Rust code from Protobuf schemas
- [ ] Implement message signing:
  ```rust
  pub struct SecureMessage {
      message: AgentMessage,
      signature: Signature,
  }
  
  impl SecureMessage {
      pub fn sign(message: &AgentMessage, private_key: &SigningKey) -> Self;
      pub fn verify(&self, public_key: &VerifyingKey) -> Result<bool>;
  }
  ```

- [ ] Implement conversation threading:
  ```rust
  pub struct ConversationThread {
      thread_id: String,
      parent_message_id: Option<String>,
      messages: Vec<MessageId>,
  }
  ```

- [ ] Implement TTL for message expiration
- [ ] Benchmark: serialization/deserialization latency

**Deliverable:** `axora-message` crate with Protobuf + signing

**Success Criteria:**
- Serialization latency: <100μs
- Signature verification: <2ms
- Payload size: 45% smaller than JSON

---

### Sprint 3: State Machine Engine (Week 6-7)

**Goal:** Implement deterministic orchestration

**Tasks:**
- [ ] Define state graph data structures:
  ```rust
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
  ```

- [ ] Implement transition conditions:
  ```rust
  pub enum TransitionCondition {
      Always,
      OnSuccess,
      OnFailure,
      Condition(Box<dyn Fn(&SharedState) -> bool>),
      Timeout(Duration),
      MessageReceived(MessageType),
  }
  ```

- [ ] Implement checkpointing (time-travel debugging):
  ```rust
  pub struct Checkpoint {
      id: String,
      timestamp: SystemTime,
      state_snapshot: SharedState,
      conversation_history: Vec<AgentMessage>,
      git_commit: Option<String>,
  }
  ```

- [ ] Implement error handlers and retry edges
- [ ] Write unit tests for state transitions

**Deliverable:** `axora-orchestrator` crate with state machine

**Success Criteria:**
- State transition latency: <10ms
- Checkpoint restore: <100ms
- No hallucination loops in testing

---

### Sprint 4: Token Efficiency (Week 8)

**Goal:** Implement state delta encoding and shared context

**Tasks:**
- [ ] Implement unified diff generation:
  ```rust
  pub struct StateDelta {
      file_path: String,
      unified_diff: String,
      ast_mutations: Vec<AstMutation>,
  }
  
  impl StateDelta {
      pub fn from_files(original: &str, modified: &str) -> Self;
      pub fn token_count(&self) -> usize;
  }
  ```

- [ ] Implement shared context store:
  ```rust
  pub struct SharedContextStore {
      vector_db: LanceDB,
      embedder: JinaEmbedder,
  }
  
  impl SharedContextStore {
      pub async fn write_finding(&self, finding: &Finding) -> Result<FindingId>;
      pub async fn get_context(&self, query: &str, thread_id: &str) -> Result<Vec<Finding>>;
  }
  ```

- [ ] Implement abbreviation protocol for common phrases
- [ ] Measure token savings vs naive NL

**Deliverable:** `axora-compression` crate with token optimization

**Success Criteria:**
- State delta encoding: 88% token reduction
- Shared context retrieval: <100ms latency
- No context pollution in agent conversations

---

### Sprint 5: Security Layer (Week 9)

**Goal:** Implement capability-based security

**Tasks:**
- [ ] Define capability schema:
  ```rust
  pub struct Capability {
      resource: String,
      actions: Vec<String>,
      scopes: Vec<String>,
  }
  
  impl Capability {
      pub fn can_access(&self, resource: &str, action: &str, path: &Path) -> bool;
  }
  ```

- [ ] Implement agent key management:
  ```rust
  pub struct AgentKeys {
      private_key: SigningKey,
      public_key: VerifyingKey,
  }
  
  impl AgentKeys {
      pub fn generate() -> Self;
      pub fn load_from_keychain(agent_id: &str) -> Result<Self>;
      pub fn store_in_keychain(&self, agent_id: &str) -> Result<()>;
  }
  ```

- [ ] Implement capability validation at tool boundary
- [ ] Implement message signature verification
- [ ] Write security tests (unauthorized access attempts)

**Deliverable:** `axora-security` crate with capability-based auth

**Success Criteria:**
- Unauthorized access blocked: 100%
- Signature verification latency: <2ms
- Key management: Secure (OS keychain)

---

### Sprint 6: MCP Server (Week 10-11)

**Goal:** Implement Model Context Protocol server

**Tasks:**
- [ ] Define MCP service in Protobuf:
  ```protobuf
  service ToolService {
    rpc ListTools(ListToolsRequest) returns (ListToolsResponse);
    rpc ExecuteTool(ToolRequest) returns (ToolResponse);
  }
  ```

- [ ] Implement MCP server with tonic:
  ```rust
  #[tonic::async_trait]
  impl ToolService for McpServer {
      async fn list_tools(&self, request: Request<ListToolsRequest>)
          -> Result<Response<ListToolsResponse>, Status>;
      
      async fn execute_tool(&self, request: Request<ToolRequest>)
          -> Result<Response<ToolResponse>, Status>;
  }
  ```

- [ ] Implement tools:
  - File System (read, write, delete)
  - Terminal (execute commands)
  - Git (commit, diff, log)
  - HTTP (API calls)
  - Database (queries)

- [ ] Implement capability validation for each tool
- [ ] Write integration tests for tool execution

**Deliverable:** `axora-mcp` crate with MCP server

**Success Criteria:**
- Tool execution latency: <100ms (excluding LLM)
- Capability validation: 100% accurate
- Interoperable with MCP clients

---

### Sprint 7: Dead Letter Queue & Fault Tolerance (Week 12)

**Goal:** Implement fault tolerance mechanisms

**Tasks:**
- [ ] Implement DLQ handler:
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
      ) -> Result<()>;
  }
  ```

- [ ] Implement retry policy with exponential backoff:
  ```rust
  pub struct RetryPolicy {
      max_retries: u32,
      initial_delay: Duration,
      max_delay: Duration,
      multiplier: f64,
  }
  ```

- [ ] Implement poison pill detection
- [ ] Implement debugger agent notification
- [ ] Chaos testing: agent crashes, network partitions

**Deliverable:** Fault tolerance integrated into communication stack

**Success Criteria:**
- Automatic retry on transient failures
- Poison pills routed to DLQ
- System recovers from agent crashes

---

### Sprint 8: Integration & End-to-End (Week 13-14)

**Goal:** Integrate all components into unified communication stack

**Tasks:**
- [ ] Create `CommunicationStack` struct:
  ```rust
  pub struct CommunicationStack {
      transport: NatsTransport,
      serializer: ProtobufSerializer,
      orchestrator: StateGraph,
      security: CapabilityManager,
      mcp_server: McpServer,
  }
  ```

- [ ] Wire all components together
- [ ] Implement hierarchical agent topology:
  - Orchestrator → Team Leads → Workers
- [ ] End-to-end latency profiling
- [ ] Optimize bottlenecks
- [ ] Integration tests

**Deliverable:** Fully functional communication stack

**Success Criteria:**
- End-to-end latency (p50): <10ms
- End-to-end latency (p95): <50ms
- No message loss
- Deterministic execution

---

### Sprint 9: Benchmarking & Optimization (Week 15-16)

**Goal:** Validate performance against targets

**Tasks:**
- [ ] Run performance benchmarks:
  - Message latency
  - Throughput (msg/sec)
  - Serialization speed
  - State transition latency
- [ ] Profile memory usage
- [ ] Optimize hot paths
- [ ] Document performance results
- [ ] A/B test: old vs new communication

**Deliverable:** Performance report, optimized stack

**Success Criteria:**
- All performance targets met (see architecture doc)
- Message latency (p50): <1ms
- Throughput: >10K msg/sec

---

## Testing Strategy

### Unit Tests
- Protobuf serialization correctness
- Message signing and verification
- Capability validation logic
- State transition logic
- NATS subject routing

### Integration Tests
- End-to-end message flow
- Queue group load balancing
- DLQ handling
- Checkpoint restore

### Chaos Tests
- Agent crash during task execution
- Network partition (NATS unavailable)
- Poison pill messages
- Replay attack simulation

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| NATS operational complexity | High | Medium | Use embedded mode for local-first, document deployment |
| Protobuf schema management | Medium | High | Version schemas, backward compatibility tests |
| State machine too rigid | High | Medium | Add escape hatches for emergent behavior |
| Capability validation slow | Medium | Low | Cache validation results, optimize glob matching |
| MCP integration issues | Medium | Medium | Follow spec closely, test with existing MCP clients |

---

## Success Metrics

**After Implementation:**
- ✅ Message Latency (p50): <1ms
- ✅ Message Latency (p95): <5ms
- ✅ Serialization: <100μs
- ✅ Signature Verification: <2ms
- ✅ State Transition: <10ms
- ✅ Token Costs: 88% reduction vs naive NL

**Business Impact:**
- ✅ Deterministic agent execution (no hallucination loops)
- ✅ Token-efficient communication (88% savings)
- ✅ Secure, zero-trust architecture
- ✅ Horizontal scaling via queue groups
- ✅ Competitive advantage over AutoGen/CrewAI

---

## Next Steps

1. **Start Sprint 0** (Foundation) - Week 1-2
2. **Await R-06 research** (Agent Architecture) - May refine orchestration
3. **Parallel: R-03 research** (Token Efficiency) - Validate compression strategies
4. **Parallel: R-07 research** (Memory & State) - Shared context integration

---

## Related Documents

- [ADR-009: Inter-Agent Communication Protocol](../research/DECISIONS.md#adr-009)
- [ADR-016: Message Schema Design](../research/DECISIONS.md#adr-016)
- [ADR-017: Token Efficiency](../research/DECISIONS.md#adr-017)
- [ADR-018: Agent Orchestration Pattern](../research/DECISIONS.md#adr-018)
- [ADR-019: Security Model](../research/DECISIONS.md#adr-019)
- [ADR-020: Tool Interface (MCP)](../research/DECISIONS.md#adr-020)
- [Architecture: Communication](../docs/architecture-communication.md)
- [R-02 Research Findings](./findings/inter-agent-communication/R-02-result.md)
