# AXORA Architectural Decisions

This document records all significant architectural decisions made during AXORA development.

## Decision Log Format

Each decision follows this format:
- **ID:** Unique identifier
- **Date:** When the decision was made
- **Status:** Proposed | Accepted | Deprecated | Superseded
- **Context:** The problem we're solving
- **Decision:** What we decided
- **Consequences:** Implications (positive and negative)
- **Research:** Links to supporting research
- **Review Date:** When to revisit

---

## Business Decisions (Foundation)

### [ADR-BUS-001] Target Audience

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need to define primary target audience  
**Decision:** Individual developers (freelancers, hobbyists, professional devs)  
**Consequences:**
- ✅ Price must be accessible (~$10-20/month)
- ✅ Simplicity > enterprise features
- ✅ Local-first is a differentiator
- ⚠️ Need volume for significant revenue
**Research:** [BUSINESS-ALIGNMENT.md](./BUSINESS-ALIGNMENT.md)  
**Review Date:** After MVP launch

---

### [ADR-BUS-002] Product Differentiation

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need to differentiate from Cursor, Copilot, etc.  
**Decision:** Triple differentiation: Multi-agent + Pre-configured specialists + Configurable  
**Consequences:**
- ✅ Complex system to build
- ✅ Clear marketing message
- ✅ Flexibility attracts diverse users
- ⚠️ More complex than single-assistant products
**Research:** [BUSINESS-ALIGNMENT.md](./BUSINESS-ALIGNMENT.md)  
**Review Date:** N/A

---

### [ADR-BUS-003] Monetization Model

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need sustainable revenue model  
**Decision:** Subscription + Usage (BYOK supported)  
**Consequences:**
- ✅ Users can choose cost model
- ✅ Recurring revenue from subscriptions
- ✅ Additional revenue from usage markup
- ⚠️ Need to track usage accurately
**Research:** [BUSINESS-ALIGNMENT.md](./BUSINESS-ALIGNMENT.md)  
**Review Date:** After 100 users

---

### [ADR-BUS-004] Token Efficiency Focus

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Token costs are a major concern for users  
**Decision:** Multi-layer token optimization as core innovation  
**Consequences:**
- ✅ Lower costs for users
- ✅ Competitive advantage
- ✅ More complex implementation
- ⚠️ Need to prove efficiency gains
**Research:** [BUSINESS-ALIGNMENT.md](./BUSINESS-ALIGNMENT.md), R-03 (pending)  
**Review Date:** After R-03 research

---

### [ADR-BUS-005] Agent Hierarchy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Risk of multi-agent communication chaos  
**Decision:** Clear hierarchy (Arquiteto > Coder/Tester/Debugger > Browser Specialist)  
**Consequences:**
- ✅ Predictable behavior
- ✅ Easier debugging
- ✅ Clear escalation paths
- ⚠️ Less flexible than peer-to-peer
**Research:** [BUSINESS-ALIGNMENT.md](./BUSINESS-ALIGNMENT.md)  
**Review Date:** After agent implementation

---

## Technical Decisions

### [ADR-012] Context Management & RAG Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need production-grade RAG architecture for code retrieval in multi-agent system  
**Decision:** Implement Modular RAG with Hybrid Retrieval, AST-based chunking, and context reordering  

**Architecture:**
```
Query → [Query Reformulation] → [Hybrid Retrieval: BM25 + Dense] → [RRF Fusion]
      → [Cross-Encoder Re-rank] → [Knapsack Selection] → [Context Reordering] → LLM
```

**Key Components:**
1. **Hybrid Retrieval:** BM25 (lexical) + Jina-code-embeddings-1.5b (semantic)
2. **Fusion:** Reciprocal Rank Fusion (RRF)
3. **Re-ranking:** Cross-encoder with Knapsack optimization
4. **Chunking:** AST-based (cAST algorithm) via Tree-sitter
5. **Context Optimization:** "Lost in the Middle" reordering algorithm
6. **Sync:** Merkle tree for incremental state synchronization

**Consequences:**
- ✅ SOTA retrieval accuracy for code
- ✅ Sub-second latency for large codebases
- ✅ Handles lexical gap (variable names) + semantic queries
- ⚠️ Complex implementation (AST parsing, multiple indices)
- ⚠️ Requires Tree-sitter integration for all supported languages
- ⚠️ Merkle tree sync adds engineering overhead

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After MVP testing (2026-09)

---

### [ADR-006] Embedding Model for Code

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal embedding model for code retrieval  
**Decision:** Use Jina-code-embeddings-1.5b with Matryoshka Representation Learning  

**Model Specs:**
- **Parameters:** 1.54B
- **Context:** 32K tokens
- **Dimensions:** 1536 (truncatable to 256-1536 via MRL)
- **Backbone:** Qwen2.5
- **Pooling:** Last-token

**Benchmarks:**
- CodeChefX: 99.44%
- SWE-Bench: 86.33%
- MTEB Code avg: 78.94%

**Alternatives Considered:**
- Qwen3-Embedding-0.6B: Smaller, multilingual, but lower accuracy
- CodeXEmbed: Good on CoIR, less validated

**Consequences:**
- ✅ Best-in-class code retrieval accuracy
- ✅ Matryoshka allows dimension truncation (storage optimization)
- ✅ Sub-second latency profiling
- ⚠️ 1.54B params = ~3GB model size (FP16)
- ⚠️ Requires GPU for fast inference, or accept slower CPU

**Implementation:**
- Use ONNX Runtime or Candle (Rust) for inference
- Store embeddings at reduced dimensions (512-768) for storage efficiency
- Full 1536-dim only for query embeddings

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After benchmarking with real codebases (2026-06)

---

### [ADR-007] Vector Database

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need vector database for local-first code indexing with cloud sync capability  
**Decision:** Dual-layer architecture: LanceDB (local) + Qdrant (cloud sync)  

**Architecture:**
```
┌─────────────────────────────────────────┐
│  Local Layer (LanceDB)                  │
│  - Embedded, zero-copy Arrow format     │
│  - Disk-based, low RAM footprint        │
│  - Offline-first operations             │
└─────────────────────────────────────────┘
              │
              │ Merkle Tree Sync
              ▼
┌─────────────────────────────────────────┐
│  Cloud Layer (Qdrant)                   │
│  - HNSW indexing, payload filtering     │
│  - Multi-tenant security                │
│  - Cross-repository analytics           │
└─────────────────────────────────────────┘
```

**Local Layer (LanceDB):**
- Compute-storage separation
- Zero parameter tuning
- Native hybrid search support
- Low RAM usage (critical for local-first)

**Cloud Layer (Qdrant):**
- Rust-native (matches our stack)
- Rich JSON payload filtering
- Real-time search at scale
- Secure multi-tenant access control

**Alternatives Considered:**
- SQLite-vec: Simpler, but lacks advanced features
- FAISS: Fast, but no built-in persistence
- ChromaDB: Easier, but less performant at scale

**Consequences:**
- ✅ Best of both worlds (local privacy + cloud scale)
- ✅ LanceDB avoids RAM tax of HNSW-based DBs
- ✅ Qdrant matches Cursor's Turbopuffer deployment pattern
- ⚠️ Two databases to maintain
- ⚠️ Sync logic adds complexity

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After performance benchmarking (2026-06)

---

### [ADR-013] Code Chunking Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal chunking strategy for code indexing  
**Decision:** Implement cAST (Chunking via Abstract Syntax Trees) algorithm  

**Algorithm:**
```
1. Parse code with Tree-sitter → AST
2. Top-down traversal of AST nodes
3. Attempt to fit largest nodes (classes, functions) into chunk
4. If node exceeds budget, recursively split into children
5. Greedy merge of small sibling nodes
6. Budget based on non-whitespace character count (not tokens/lines)
```

**Chunk Metadata:**
- Absolute file path
- Symbol definitions (function names, class names)
- Incoming call graph edges
- Outgoing call graph edges
- Language identifier
- Git hash (for invalidation)

**Budget Configuration:**
- Target: 512-1024 non-whitespace characters per chunk
- Overlap: 50-100 characters (for context continuity)
- Max chunk: 2048 characters (hard limit)

**Alternatives Considered:**
- Line-based (fixed N lines): Simple but destroys structure
- Function-based (one chunk per function): Good but variable sizes
- ParentDocumentRetriever (LangChain): Index small, retrieve large
- File-based: Too large for most files

**Consequences:**
- ✅ Preserves syntactic integrity
- ✅ 20-30% retrieval accuracy improvement vs line-based
- ✅ Handles multi-language codebases
- ⚠️ Requires Tree-sitter parsers for all languages
- ⚠️ Slower indexing (AST parsing overhead)
- ⚠️ More complex implementation

**Implementation:**
- Use `tree-sitter` Rust crate
- Pre-built grammars for supported languages
- Incremental re-indexing on file changes

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After indexing benchmarks (2026-05)

---

### [ADR-014] Context Reordering Algorithm

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** LLMs ignore middle context ("Lost in the Middle" phenomenon)  
**Decision:** Implement deterministic reordering algorithm before prompt assembly  

**Algorithm:**
```
Given retrieved chunks sorted by relevance: [d1, d2, d3, d4, d5, d6]

Reorder to: [d1, d6, d2, d5, d3, d4]
            ↑    ↑    ↑    ↑    ↑    ↑
           Pos1 End  Pos2 End-1 Pos3 ...

Highest scoring → Beginning
Second highest → End
Third highest → Position 2
Fourth highest → Position End-1
...continue alternating inward
```

**Implementation:**
```rust
fn reorder_context(chunks: Vec<RetrievedChunk>) -> Vec<RetrievedChunk> {
    let mut reordered = Vec::with_capacity(chunks.len());
    let mut left = 0;
    let mut right = chunks.len() - 1;
    let mut pick_left = true;
    
    while left <= right {
        if pick_left {
            reordered.push(chunks[left].clone());
            left += 1;
        } else {
            reordered.push(chunks[right].clone());
            right -= 1;
        }
        pick_left = !pick_left;
    }
    
    reordered
}
```

**Alternatives Considered:**
- No reordering (descending score): Worst performance
- Random shuffle: Unpredictable
- LLM-based reordering: Too slow, expensive

**Consequences:**
- ✅ Training-free (no model fine-tuning needed)
- ✅ 15-25% improvement in information extraction
- ✅ Deterministic, reproducible
- ⚠️ Adds ~1ms latency per query
- ⚠️ Requires careful integration with token budgeting

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After A/B testing (2026-07)

---

### [ADR-015] State Synchronization (Merkle Tree)

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need sub-second incremental sync for large codebases  
**Decision:** Implement Merkle tree-based state synchronization  

**Architecture:**
```
Local Filesystem
       │
       ▼
┌──────────────────────────┐
│  Compute File Hashes     │ (SHA-256 per file)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Build Merkle Tree       │ (Recursive folder hashes)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Compare with Server     │ (Find divergent branches)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Sync Only Changed Files │ (Differential payload)
└──────────────────────────┘
```

**Hash Computation:**
- File hash: SHA-256 of file content
- Folder hash: SHA-256 of concatenated child hashes (sorted by name)
- Root hash: Single hash representing entire repository state

**Sync Protocol:**
1. Client computes local Merkle tree
2. Client sends root hash to server
3. Server compares with stored tree
4. Server identifies divergent branches
5. Client uploads only changed files
6. Server re-indexes changed chunks only

**Performance:**
- 50,000 files → ~3.2 MB hash metadata
- Changed file detection: O(log n) tree traversal
- Sync payload: Only changed files (not full scan)

**Alternatives Considered:**
- Full re-scan on every change: Too slow
- File watcher only: Misses external changes
- Git-based diff: Doesn't handle uncommitted changes

**Consequences:**
- ✅ Sub-second change detection
- ✅ Minimal bandwidth usage
- ✅ Works with uncommitted changes
- ⚠️ SHA-256 computation overhead on large files
- ⚠️ Complex tree management logic
- ⚠️ Need to handle file renames/moves

**Implementation:**
- Use `sha2` crate for hashing
- Store tree in SQLite for persistence
- Background sync thread (non-blocking)

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After large repo testing (2026-08)

---

## Inter-Agent Communication Decisions (R-02)

### [ADR-009] Inter-Agent Communication Protocol

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal communication protocol for multi-agent coordination  
**Decision:** Implement hybrid architecture: NATS JetStream (transport) + Protobuf (serialization) + State Machine (orchestration)  

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│              AXORA Communication Stack                   │
├─────────────────────────────────────────────────────────┤
│  Application Layer                                       │
│  ┌─────────────────┐  ┌─────────────────┐               │
│  │  State Machine  │  │  Semantic       │               │
│  │  (Orchestrator) │  │  Compression    │               │
│  └─────────────────┘  └─────────────────┘               │
├─────────────────────────────────────────────────────────┤
│  Message Layer                                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Protocol Buffers (AgentMessage schema)         │    │
│  │  - Task assignments, results, state deltas      │    │
│  │  - Cryptographic signatures                     │    │
│  └─────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────┤
│  Transport Layer                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  NATS JetStream                                  │    │
│  │  - Pub/Sub + Queue Groups                        │    │
│  │  - Exactly-once delivery                         │    │
│  │  - Dead Letter Queues                            │    │
│  └─────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────┤
│  Tool Interface                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Model Context Protocol (MCP) over gRPC         │    │
│  │  - Sandboxed tool execution                     │    │
│  │  - File system, terminal access                 │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

**Key Components:**
1. **Transport:** NATS JetStream (async-nats crate)
2. **Serialization:** Protocol Buffers (prost crate)
3. **Orchestration:** Hierarchical state machine (LangGraph-inspired)
4. **Tool Interface:** MCP over gRPC (tonic crate)
5. **Security:** Capability-based auth + JWS signatures

**Consequences:**
- ✅ Sub-millisecond message latency
- ✅ Exactly-once delivery semantics
- ✅ Horizontal scaling via queue groups
- ✅ Deterministic execution (state machine)
- ✅ Token-efficient communication
- ⚠️ Complex implementation (multiple layers)
- ⚠️ NATS operational overhead (embedded server)
- ⚠️ Protobuf schema management required

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After MVP testing (2026-09)

---

### [ADR-016] Message Schema Design

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need formalized message structure for agent communication  
**Decision:** Implement Protobuf-based AgentMessage schema with layered envelope  

**Schema:**
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
  // Conversation Management
  string message_id = 1;              
  string conversation_thread_id = 2;  
  MessageType type = 3;
  
  // Routing
  string sender_id = 4;
  string recipient_id = 5;            
  
  // Semantic Context (Optional)
  bytes latent_context = 6;           
  
  // Content Payload
  string payload_mime_type = 7;       
  bytes content = 8;                  
  
  // Observability & Security
  int64 timestamp = 9;
  int32 ttl_seconds = 10;
  bytes cryptographic_signature = 11; 
}
```

**Message Types:**
1. **Task-related:** TASK_ASSIGN, TASK_RESULT
2. **State Management:** STATE_DELTA (diffs, not full state)
3. **Coordination:** LOCK_REQUEST (for shared resources)
4. **Meta-communication:** CAPABILITY_AD (agent capabilities)

**Key Design Principles:**
- Strict conversation threading (causality tracking)
- MIME-typed payloads (flexible content types)
- Cryptographic signatures (message integrity)
- Latent context field (future-proof for C2C communication)
- TTL for message expiration

**Alternatives Considered:**
- JSON schema: Too verbose, no type safety
- Avro: Larger payload, schema registry overhead
- MessagePack: No schema evolution support

**Consequences:**
- ✅ 45% smaller payload vs JSON
- ✅ Strict type safety (compile-time checks)
- ✅ Schema evolution support
- ✅ Fast serialization/deserialization
- ⚠️ Requires Protobuf compilation step
- ⚠️ LLMs can't directly parse binary (need intermediary)

**Implementation:**
- Use `prost` crate for Rust
- Generate code from `.proto` files
- Intermediary layer converts Protobuf ↔ JSON for LLM parsing

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After message volume profiling (2026-07)

---

### [ADR-017] Token Efficiency for Agent Communication

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Natural language communication causes severe token waste  
**Decision:** Implement multi-layer token optimization strategy  

**Optimization Layers:**
```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: State Delta Encoding                          │
│  - Send only diffs (not full files)                     │
│  - Send only state transitions                          │
│  - Savings: ~88%                                        │
├─────────────────────────────────────────────────────────┤
│  Layer 2: Semantic Compression                          │
│  - Brevity codes & abbreviations                        │
│  - Structured note-taking (not full transcripts)        │
│  - Savings: ~65%                                        │
├─────────────────────────────────────────────────────────┤
│  Layer 3: Retrieval-Based Context                       │
│  - Shared vector store (blackboard)                     │
│  - Query-based context retrieval                        │
│  - Avoids passing full conversation history             │
├─────────────────────────────────────────────────────────┤
│  Layer 4: Latent Cache Transfer (Future)                │
│  - KV cache transfer (C2C)                              │
│  - Bypass tokenization entirely                         │
│  - Savings: ~100% (latent space)                        │
└─────────────────────────────────────────────────────────┘
```

**Token Savings Analysis:**

| Approach | Tokens/Message | Savings |
|----------|---------------|---------|
| Naive NL (Full Context) | ~3,500 | baseline |
| Brevity Codes | ~1,200 | ~65% |
| State Delta Encoding | ~400 | ~88% |
| Latent Cache Transfer | 0 (latent) | ~100% |

**Implementation Priority:**
1. State Delta Encoding (immediate, high impact)
2. Retrieval-Based Context (immediate, architectural)
3. Semantic Compression (phase 2)
4. Latent Cache Transfer (R&D, future)

**Alternatives Considered:**
- Full context transfer: Prohibitively expensive
- Summarized context: Lossy, hallucination drift
- Pure latent transfer: Requires homogeneous models

**Consequences:**
- ✅ 88% reduction in token costs
- ✅ Faster agent communication
- ✅ Reduced context pollution
- ⚠️ Complexity in delta computation
- ⚠️ Need shared vector store infrastructure
- ⚠️ Latent transfer requires model compatibility

**Implementation:**
- Unified diff generation for code changes
- AST mutation tracking
- Shared LanceDB for context storage
- Abbreviation protocol for common phrases

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After token cost analysis (2026-06)

---

### [ADR-018] Agent Orchestration Pattern

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need deterministic orchestration to avoid hallucination loops  
**Decision:** Implement hierarchical state machine (LangGraph-inspired) with pub/sub for utility agents  

**Architecture:**
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

**Orchestration Model:**
- **Central Orchestrator:** Governs state transitions, macro-task delegation
- **Team Leads:** Manage specialized pools (Frontend, Backend, QA)
- **Workers:** Execute specific tasks within scope
- **Utility Agents:** Async pub/sub (linter, security scanner, etc.)

**State Machine Properties:**
- Directed Acyclic Graph (DAG) for workflows
- Explicit state mutations (not message passing)
- Checkpointing for time-travel debugging
- Error edges for fault tolerance

**Alternatives Considered:**
- Pure message passing (AutoGen): Too chaotic, hallucination loops
- Sequential handoffs (CrewAI): Too rigid, no cyclic flows
- Stateless routines (Swarm): No persistence, manual recovery

**Consequences:**
- ✅ Deterministic execution
- ✅ Prevents "telephone game" context loss
- ✅ Built-in fault tolerance
- ✅ Horizontal scaling via queue groups
- ⚠️ More complex than pure message passing
- ⚠️ Central orchestrator can be bottleneck
- ⚠️ Requires careful state schema design

**Implementation:**
- Custom Rust state machine engine
- NATS for async utility agents
- Checkpointing to SQLite
- Error handling with retry edges

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After workflow complexity analysis (2026-08)

---

### [ADR-019] Security Model for Agent Communication

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Multi-agent system requires zero-trust security  
**Decision:** Implement capability-based security with cryptographic message signing  

**Security Layers:**
```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: Authentication                                │
│  - W3C Decentralized Identifiers (DIDs)                 │
│  - Cryptographic agent identity                         │
├─────────────────────────────────────────────────────────┤
│  Layer 2: Authorization                                 │
│  - Capability-based tokens (Agent Cards)                │
│  - Scoped permissions per agent                         │
│  - RBAC at tool execution layer                         │
├─────────────────────────────────────────────────────────┤
│  Layer 3: Message Integrity                             │
│  - JSON Web Signatures (JWS) / Protobuf signatures      │
│  - Prevents MITM prompt injection                       │
│  - Audit trail for all messages                         │
└─────────────────────────────────────────────────────────┘
```

**Agent Capabilities:**
```protobuf
message AgentCard {
  string agent_id = 1;
  string role = 2;
  repeated Capability capabilities = 3;
  repeated string allowed_tools = 4;
  repeated string denied_tools = 5;
}

message Capability {
  string resource = 1;      // e.g., "filesystem", "terminal"
  repeated string actions = 2;  // e.g., ["read", "write"]
  repeated string scopes = 3;   // e.g., ["./src/**", "!./secrets/**"]
}
```

**Message Signing:**
- All messages signed with agent's private key
- Recipient verifies signature before processing
- Signature includes timestamp (prevents replay attacks)

**Alternatives Considered:**
- API keys: Too coarse-grained, no scoping
- OAuth: Overkill for internal agent communication
- No security: Unacceptable for code execution

**Consequences:**
- ✅ Zero-trust architecture
- ✅ Prevents rogue agent actions
- ✅ Audit trail for compliance
- ✅ Scoped permissions (least privilege)
- ⚠️ Key management overhead
- ⚠️ Signature verification latency (~1-2ms)
- ⚠️ Need secure key storage

**Implementation:**
- `ed25519` crate for signing
- Keys stored in OS keychain (macOS Keychain, Windows Credential Manager)
- Capability validation at tool execution boundary

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After security audit (2026-09)

---

### [ADR-020] Tool Interface (Model Context Protocol)

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need standardized interface for agent tool execution  
**Decision:** Adopt Anthropic's Model Context Protocol (MCP) over gRPC  

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│  Agent (LLM)                                            │
│  - Sends tool requests via MCP                          │
└─────────────────────────────────────────────────────────┘
       │
       │ MCP over gRPC (tonic)
       ▼
┌─────────────────────────────────────────────────────────┐
│  MCP Server (Rust)                                      │
│  - Validates capabilities                               │
│  - Sandboxes tool execution                             │
│  - Returns structured results                           │
├─────────────────────────────────────────────────────────┤
│  Available Tools:                                       │
│  - File System (read, write, delete)                    │
│  - Terminal (execute commands)                          │
│  - Git (commit, diff, log)                              │
│  - HTTP (API calls)                                     │
│  - Database (queries)                                   │
└─────────────────────────────────────────────────────────┘
```

**MCP Benefits:**
- Industry standard (Anthropic, many adopters)
- Secure sandboxing
- Structured tool definitions
- Automatic schema generation

**Alternatives Considered:**
- Custom tool interface: Reinventing the wheel
- OpenAPI: Too HTTP-centric, not agent-focused
- Direct function calling: No sandboxing, security risks

**Consequences:**
- ✅ Standardized interface
- ✅ Secure execution sandbox
- ✅ Interoperable with other MCP tools
- ✅ Automatic documentation
- ⚠️ Learning curve for team
- ⚠️ gRPC overhead for local tools
- ⚠️ Need to implement MCP server in Rust

**Implementation:**
- Use `tonic` for gRPC server
- Implement MCP specification
- Sandbox tools with capability checks
- Structured error handling

**Research:** [R-02 Findings](./findings/inter-agent-communication/R-02-result.md)  
**Review Date:** After tool integration testing (2026-07)

---

## Token Efficiency Decisions (R-03)

### [ADR-021] Multi-Layer Token Optimization Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Token costs scale exponentially in multi-agent systems without optimization  
**Decision:** Implement 5-layer token optimization pipeline  

**Optimization Layers (in priority order):**

| Layer | Technique | Savings | Implementation |
|-------|-----------|---------|----------------|
| 1 | Prefix Prompt Caching | 50-90% cost, 13-79% latency | Critical (Week 1-2) |
| 2 | Diff-Based Code Communication | 89-98% reduction | Critical (Week 3-4) |
| 3 | Code Minification | 24-42% reduction | High (Week 3-4) |
| 4 | TOON Serialization | 50-60% reduction | High (Week 5-6) |
| 5 | Semantic Caching (L3) | 40-70% cache hit rate | Medium (Week 5-6) |

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: Provider Prefix Caching                       │
│  - Static prompts at beginning                          │
│  - 90% discount on cached reads                         │
├─────────────────────────────────────────────────────────┤
│  Layer 2: L1/L2 Cache (Exact Match)                     │
│  - dashmap (in-memory)                                  │
│  - RocksDB (disk-backed)                                │
├─────────────────────────────────────────────────────────┤
│  Layer 3: Semantic Cache (Fuzzy Match)                  │
│  - Qdrant vector DB                                     │
│  - Cosine similarity threshold: 0.98 (code), 0.92 (NL)  │
├─────────────────────────────────────────────────────────┤
│  Layer 4: Code Minification                             │
│  - Whitespace removal (24-42% savings)                  │
│  - Identifier compression                               │
│  - Diff-only communication (89-98% savings)             │
├─────────────────────────────────────────────────────────┤
│  Layer 5: LLMLingua-2 Compression                       │
│  - Token classification for pruning                     │
│  - 50-80% context compression                           │
└─────────────────────────────────────────────────────────┘
```

**Cost Impact Analysis:**

**Before Optimization** (100M input + 10M output tokens/day):
- System prompts: $40/day (uncached)
- Context: $75/day
- Messages/Output: $140/day
- **Total: ~$255/day** ($7,650/month)

**After Optimization:**
- System prompts: $4/day (90% cached)
- Context: $7.50/day (minified)
- Messages/Output: $14/day (diff-based)
- **Total: ~$25.50/day** ($765/month)

**Savings: 90% cost reduction**

**Consequences:**
- ✅ 90% reduction in token costs
- ✅ 13-79% latency reduction
- ✅ Viable unit economics at scale
- ⚠️ Implementation complexity (multiple layers)
- ⚠️ Cache invalidation logic required
- ⚠️ Need to handle cache misses gracefully

**Implementation:**
- Rust crates: `dashmap`, `rocksdb`, `qdrant-rust`
- Prompt reorganization (static first, dynamic last)
- Diff generation/validation pipeline
- TOON encoder for tool outputs

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After cost analysis at scale (2026-06)

---

### [ADR-022] Prefix Prompt Caching

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Provider caching offers 90% discounts but requires specific prompt structure  
**Decision:** Reorganize all agent prompts to maximize prefix cache hits  

**Prompt Structure:**
```
┌─────────────────────────────────────────┐
│  STATIC PREFIX (Cached - 90% discount)  │
│  - System instructions                  │
│  - Agent role definition                │
│  - Capabilities & constraints           │
│  - Abbreviation dictionary              │
│  - Tool schemas                         │
├─────────────────────────────────────────┤
│  DYNAMIC SUFFIX (Uncached - full price) │
│  - Conversation history                 │
│  - Current task context                 │
│  - User query                           │
└─────────────────────────────────────────┘
```

**Implementation:**
```rust
pub struct PromptBuilder {
    static_prefix: String,  // Computed once, reused
    dynamic_suffix: String,
}

impl PromptBuilder {
    pub fn new(agent_type: AgentType) -> Self {
        let static_prefix = Self::build_static_prefix(agent_type);
        Self {
            static_prefix,
            dynamic_suffix: String::new(),
        }
    }
    
    pub fn build(&mut self, context: &str, query: &str) -> String {
        self.dynamic_suffix = format!("{}\n{}", context, query);
        format!("{}\n\n{}", self.static_prefix, self.dynamic_suffix)
    }
}
```

**Cache Hit Optimization:**
- Group all static content at absolute beginning
- Use identical formatting across all agent calls
- Avoid variable whitespace/indentation in prefix
- Keep abbreviation dictionary in prefix (benefits from 90% discount)

**Expected Performance:**
- Cache hit rate: >90% for system prompts
- Cost reduction: 90% on prefix tokens
- Latency reduction: 13-79% on TTFT

**Alternatives Considered:**
- No caching: 10x more expensive
- Full prompt caching (Anthropic): Requires specific API usage
- Semantic caching only: Doesn't capture provider discounts

**Consequences:**
- ✅ 90% cost reduction on static content
- ✅ Lower TTFT (cached prefill)
- ✅ Simple implementation (prompt reorganization)
- ⚠️ Requires discipline in prompt structure
- ⚠️ Cache invalidated if prefix changes

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After cache hit rate analysis (2026-05)

---

### [ADR-023] Diff-Based Code Communication

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Full file transfers between agents waste 89-98% of tokens  
**Decision:** All code communication must use diff/patch format  

**Communication Protocol:**
```rust
// Bad: Full file transfer (5000 tokens)
pub struct CodeUpdate {
    file_path: String,
    content: String,  // Entire file
}

// Good: Diff transfer (150 tokens)
pub struct CodeDiff {
    file_path: String,
    unified_diff: String,  // @@ -10,7 +10,9 @@
    hunk_count: usize,
}

// Example diff
let diff = CodeDiff {
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
    hunk_count: 1,
};
```

**Implementation:**
```rust
pub struct DiffEngine {
    repo_path: PathBuf,
}

impl DiffEngine {
    pub fn generate_diff(&self, file_path: &str, new_content: &str) -> Result<CodeDiff> {
        let original = std::fs::read_to_string(file_path)?;
        let diff = unified_diff::generate(&original, new_content, file_path);
        
        Ok(CodeDiff {
            file_path: file_path.to_string(),
            unified_diff: diff,
            hunk_count: count_hunks(&diff),
        })
    }
    
    pub fn apply_diff(&self, diff: &CodeDiff) -> Result<()> {
        // Apply patch with validation
        patch::apply(&diff.unified_diff, &self.repo_path)
    }
}
```

**Token Savings:**

| Scenario | Full File | Diff | Savings |
|----------|-----------|------|---------|
| Single line change | 5000 tokens | 50 tokens | 99% |
| Function edit | 5000 tokens | 150 tokens | 97% |
| Multi-file refactor | 50000 tokens | 2000 tokens | 96% |

**Validation:**
- All diffs must be validated before applying
- Failed diffs trigger fallback (full file review)
- Unit tests run after each diff application

**Alternatives Considered:**
- Full file transfer: Prohibitively expensive
- AST deltas: More complex, requires model fine-tuning
- SEARCH/REPLACE blocks: Good alternative, similar savings

**Consequences:**
- ✅ 89-98% token reduction
- ✅ Faster agent communication
- ✅ Clear change tracking
- ⚠️ Diff application can fail
- ⚠️ Need robust error handling
- ⚠️ Multi-file changes require coordination

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After diff success rate analysis (2026-06)

---

### [ADR-024] Code Minification Pipeline

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Human-readable formatting consumes 24-42% of code tokens without semantic value  
**Decision:** Implement code minification before sending to LLM  

**Minification Layers:**
```rust
pub struct CodeMinifier {
    language: Language,
    identifier_map: HashMap<String, String>,
}

impl CodeMinifier {
    /// Layer 1: Whitespace removal (24-42% savings)
    pub fn strip_whitespace(&self, code: &str) -> String {
        code.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Layer 2: Identifier compression
    pub fn compress_identifiers(&mut self, code: &str) -> String {
        let mut result = code.to_string();
        for (original, alias) in &self.identifier_map {
            result = result.replace(original, alias);
        }
        result
    }
    
    /// Layer 3: Comment stripping
    pub fn strip_comments(&self, code: &str) -> String {
        // Remove single-line and multi-line comments
        comment_strip_regex.replace_all(code, "").to_string()
    }
    
    /// Reverse: Restore original identifiers
    pub fn decompress(&self, minified: &str) -> String {
        let mut result = minified.to_string();
        for (original, alias) in &self.identifier_map {
            result = result.replace(alias, original);
        }
        result
    }
}
```

**Identifier Mapping:**
```rust
// Example mapping
let identifier_map = hashmap! {
    "calculateMonthlyRevenueMetrics" => "a1",
    "userRepository" => "a2",
    "authenticationService" => "a3",
};

// Before: 45 tokens
calculateMonthlyRevenueMetrics(userRepository, authenticationService);

// After: 12 tokens (73% savings)
a1(a2, a3);
```

**Quality Impact:**
- LLMs maintain 98%+ accuracy on minified code (Fill-in-the-Middle benchmarks)
- Models rely on syntactic tokens, not visual formatting
- Identifier compression is lossless (bidirectional mapping)

**Alternatives Considered:**
- No minification: 24-42% token waste
- AST serialization: More complex, requires fine-tuned models
- Partial minification: Less savings, similar complexity

**Consequences:**
- ✅ 24-42% token reduction
- ✅ No quality degradation
- ✅ Reversible (identifier mapping)
- ⚠️ Adds processing overhead (~10ms)
- ⚠️ Need to maintain identifier map
- ⚠️ Debugging minified code harder

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After quality benchmark (2026-06)

---

### [ADR-025] TOON Serialization for Tool Outputs

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** JSON format wastes 50-60% of tokens on structural redundancy  
**Decision:** Implement Token-Optimized Object Notation (TOON) for all tool outputs  

**TOON Format:**
```rust
// JSON (11,842 tokens for large response)
[
  {
    "user_id": 12345,
    "username": "john_doe",
    "email": "john@example.com",
    "role": "admin",
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "user_id": 12346,
    "username": "jane_doe",
    "email": "jane@example.com",
    "role": "user",
    "created_at": "2024-01-16T11:45:00Z"
  }
]

// TOON (4,617 tokens - 61% reduction)
Schema: {0:user_id, 1:username, 2:email, 3:role, 4:created_at}
0:12345 1:john_doe 2:john@example.com 3:admin 4:2024-01-15T10:30:00Z | 
0:12346 1:jane_doe 2:jane@example.com 3:user 4:2024-01-16T11:45:00Z
```

**Implementation:**
```rust
pub struct ToonSerializer {
    schema: HashMap<String, u8>,  // field_name -> field_id
}

impl ToonSerializer {
    pub fn serialize(&self, data: &Value) -> Result<String> {
        let mut output = String::new();
        
        // Schema header
        output.push_str("Schema: {");
        for (field, id) in &self.schema {
            output.push_str(&format!("{}:{}, ", id, field));
        }
        output.push_str("}\n");
        
        // Data rows (space-separated, pipe-delimited)
        if let Value::Array(objects) = data {
            for obj in objects {
                let row = self.serialize_object(obj)?;
                output.push_str(&row);
                output.push_str(" | \n");
            }
        }
        
        Ok(output)
    }
    
    pub fn deserialize(&self, toon: &str) -> Result<Value> {
        // Parse schema, then data rows
        // Reconstruct JSON
    }
}
```

**Schema Definition:**
```rust
let schema = hashmap! {
    "user_id".to_string() => 0,
    "username".to_string() => 1,
    "email".to_string() => 2,
    "role".to_string() => 3,
    "created_at".to_string() => 4,
};
```

**Token Savings:**

| Data Structure | JSON Tokens | TOON Tokens | Savings |
|----------------|-------------|-------------|---------|
| Small object (5 fields) | 50 | 20 | 60% |
| Array (100 objects) | 5000 | 2000 | 60% |
| Nested structure | 10000 | 4500 | 55% |

**Alternatives Considered:**
- JSON: Standard but verbose
- MessagePack: Binary, LLMs can't parse
- CBOR: Binary, same issue

**Consequences:**
- ✅ 50-60% token reduction
- ✅ LLM-readable (text-based)
- ✅ Schema validation
- ⚠️ Requires schema definition
- ⚠️ Less human-readable than JSON
- ⚠️ Serialization/deserialization overhead

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After tool output analysis (2026-07)

---

### [ADR-026] Multi-Tier Caching Architecture

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Redundant LLM calls waste 40-70% of token budget  
**Decision:** Implement 3-tier caching (L1 exact, L2 disk, L3 semantic)  

**Cache Architecture:**
```rust
pub struct MultiTierCache {
    l1: DashMap<String, CacheEntry>,      // In-memory, exact match
    l2: RocksDB,                          // Disk-backed, warm storage
    l3: QdrantClient,                     // Vector DB, semantic match
    embedder: ONNXEmbedder,               // Local embedding model
}

impl MultiTierCache {
    pub async fn get(&self, query: &str) -> Option<CacheEntry> {
        // L1: Exact match (microseconds)
        let query_hash = sha256(query);
        if let Some(entry) = self.l1.get(&query_hash) {
            return Some(entry.clone());
        }
        
        // L2: Disk lookup (milliseconds)
        if let Some(entry) = self.l2.get(&query_hash) {
            self.l1.insert(query_hash, entry.clone());
            return Some(entry);
        }
        
        // L3: Semantic match (tens of milliseconds)
        let query_embedding = self.embedder.embed(query).await?;
        let results = self.l3
            .search(query_embedding)
            .min_similarity(0.98)  // High threshold for code
            .top_k(1)
            .await?;
        
        if let Some(matched) = results.first() {
            if matched.score > 0.98 {
                return Some(matched.entry.clone());
            }
        }
        
        None  // Cache miss, call LLM
    }
    
    pub async fn set(&self, query: &str, response: &str) {
        let entry = CacheEntry {
            query: query.to_string(),
            response: response.to_string(),
            timestamp: SystemTime::now(),
        };
        
        let query_hash = sha256(query);
        self.l1.insert(query_hash, entry.clone());
        self.l2.insert(&query_hash, &entry);
        
        // L3: Store embedding
        let embedding = self.embedder.embed(query).await?;
        self.l3.upsert_point(embedding, entry).await;
    }
}
```

**Cache Invalidation:**
```rust
pub enum InvalidationStrategy {
    /// Time-based expiration
    TTL(Duration),
    
    /// Dependency-based (file hash changed)
    Dependency(PathBuf),
    
    /// Manual invalidation
    Manual(Vec<String>),
}

impl MultiTierCache {
    pub async fn invalidate(&self, strategy: InvalidationStrategy) {
        match strategy {
            InvalidationStrategy::TTL(duration) => {
                let cutoff = SystemTime::now() - duration;
                self.l1.retain(|_, v| v.timestamp > cutoff);
            }
            InvalidationStrategy::Dependency(path) => {
                // Invalidate all entries depending on file
                let file_hash = compute_file_hash(&path);
                self.l1.retain(|_, v| v.dependencies.contains(&file_hash));
            }
            InvalidationStrategy::Manual(keys) => {
                for key in keys {
                    self.l1.remove(&key);
                }
            }
        }
    }
}
```

**Performance Targets:**

| Cache Tier | Latency | Hit Rate | Use Case |
|------------|---------|----------|----------|
| L1 (DashMap) | <10μs | 30-40% | Exact repeats |
| L2 (RocksDB) | <5ms | 20-30% | Warm storage |
| L3 (Qdrant) | <50ms | 20-30% | Semantic match |
| **Total** | - | **70-80%** | - |

**Alternatives Considered:**
- Single-tier cache: Lower hit rate
- Semantic-only: Misses exact matches, higher latency
- No caching: 100% LLM calls, 10x cost

**Consequences:**
- ✅ 70-80% cache hit rate
- ✅ 40-70% cost reduction
- ✅ Microsecond latency for L1 hits
- ⚠️ Cache complexity
- ⚠️ Invalidation logic required
- ⚠️ Memory/disk usage

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After cache hit rate analysis (2026-06)

---

### [ADR-027] Symbolic Metalanguage (MetaGlyph)

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Natural language instructions waste 62-81% of prompt tokens  
**Decision:** Implement symbolic metalanguage for agent communication  

**MetaGlyph Operators:**
```rust
pub enum MetaGlyph {
    // Set operations
    Membership,      // ∈ (in)
    Exclusion,       // ∉ (not in)
    Intersection,    // ∩ (and)
    Union,           // ∪ (or)
    
    // Logic
    Implication,     // ⇒ (if-then)
    Equivalence,     // ⇔ (if and only if)
    Negation,        // ¬ (not)
    
    // Functions
    Composition,     // ∘ (function composition)
    Mapping,         // ↦ (maps to)
    
    // Quantifiers
    Universal,       // ∀ (for all)
    Existential,     // ∃ (exists)
}
```

**Token Savings:** 62-81% reduction in instructions

**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Review Date:** After fidelity testing (2026-07)

---

## Local Indexing & Embedding Decisions (R-04)

### [ADR-006] Embedding Model for Code

**Date:** 2026-03-16  
**Status:** ✅ Updated  
**Context:** Need optimal embedding model for local code retrieval  
**Decision:** Use Jina Code Embeddings v2 (default) or Nomic Embed Code (premium)  

**Model Comparison:**

| Model | Dimensions | MRR @10 | CPU Latency | Memory | License |
|-------|-----------|---------|-------------|--------|---------|
| **Jina Code v2** | 768 | 0.792 | **~15ms** | ~550MB | Apache 2.0 |
| **Nomic Embed Code** | 768 | **~0.81** | ~120ms (INT8) | ~7GB | Apache 2.0 |
| CodeBERT | 768 | 0.699 | ~45ms | 476MB | MIT |
| BGE-Code-v1 | 1024 | 0.795 | ~85ms | 2.2GB | BAAI |

**Decision:**
- **Default:** Jina Code Embeddings v2 (97% of SOTA quality, 2% of parameters)
- **Premium:** Nomic Embed Code (SOTA open model, requires 16GB+ RAM)

**Implementation:**
- Candle for native Rust inference
- INT8 quantization for Nomic
- LoRA adapters for domain-specific fine-tuning

**Performance Targets:**
- Default (Jina): <25ms CPU inference
- Premium (Nomic): <120ms with INT8
- Indexing throughput: >100 files/sec

**Consequences:**
- ✅ Apache 2.0 licensing (commercial use OK)
- ✅ Unified multi-language embedding space
- ✅ Local-first, no cloud dependency
- ⚠️ 7GB memory for Nomic (quantized)
- ⚠️ Quality gap vs cloud models (text-embedding-3-large)

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After benchmarking on real codebases (2026-06)

---

### [ADR-007] Vector Database

**Date:** 2026-03-16  
**Status:** ✅ Updated  
**Context:** Need vector database for local-first code indexing  
**Decision:** Qdrant Embedded (primary) or sqlite-vec (simplicity)  

**Evaluation:**

| Database | Query P95 | Memory (100K) | Rust Support | License |
|----------|-----------|---------------|--------------|---------|
| **Qdrant Embedded** | **1.6-3.5ms** | ~200MB | **Native** | Apache 2.0 |
| **sqlite-vec** | 12-17ms | **<100MB** | Extension | MIT |
| LanceDB | 25-30ms | ~150MB | Bindings | Apache 2.0 |
| ChromaDB | 5-10ms | ~300MB | Limited | Apache 2.0 |

**Decision:**
- **Primary:** Qdrant Embedded (sub-5ms queries, native Rust)
- **Alternative:** sqlite-vec (unified SQLite, simpler ops)

**Configuration:**
```rust
// Qdrant Embedded
let config = QdrantConfig::memory()
    .with_hnsw(M=16, ef=128)
    .with_scalar_quantization();
```

**Index Algorithm:** HNSW
- M=16 (connectivity)
- ef=128 (search depth)
- >95% recall @10
- O(log n) query complexity

**Consequences:**
- ✅ Sub-5ms query latency (Qdrant)
- ✅ Native Rust integration
- ✅ Filtered search (vector + metadata)
- ✅ ACID transactions
- ⚠️ Qdrant adds ~350MB binary overhead
- ⚠️ sqlite-vec has no HNSW yet (IVF only)

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After load testing (2026-06)

---

### [ADR-028] Code Chunking Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal chunking for code indexing  
**Decision:** AST-based function extraction via Tree-sitter with hierarchical relationships  

**Chunking Hierarchy:**
```
FileChunk (path, imports, doc)
  ├── ClassChunk (name, fields, methods)
  │     ├── MethodChunk (signature, body)
  │     └── MethodChunk (...)
  └── FunctionChunk (standalone)
```

**Implementation:**
```rust
use tree_sitter::{Parser, Query};

pub struct Chunker {
    parser: Parser,
    language_queries: HashMap<Language, Query>,
}

impl Chunker {
    pub fn extract_functions(&self, code: &str, language: Language) -> Vec<Chunk> {
        let tree = self.parser.parse(code, None).unwrap();
        let query = self.language_queries.get(&language).unwrap();
        
        // Tree-sitter S-expression query for functions
        let query_matches = query.matches(&tree.root_node(), code.as_bytes());
        
        query_matches.map(|m| Chunk::from_match(m)).collect()
    }
}
```

**Chunk Parameters:**
- Target size: 256-512 tokens
- Overlap: 50-100 tokens
- Maximum: 1024 tokens (hard limit)
- Minimum: 50 tokens (filter trivial)

**Metadata Schema:**
```rust
pub struct ChunkMetadata {
    // Core
    chunk_id: String,
    file_path: PathBuf,
    line_range: (usize, usize),
    language: Language,
    git_hash: String,
    
    // Semantic
    function_name: Option<String>,
    class_name: Option<String>,
    signature: String,
    docstring: Option<String>,
    
    // Dependencies
    imports: Vec<String>,
    callees: Vec<String>,
    type_references: Vec<String>,
}
```

**Consequences:**
- ✅ Preserves semantic units (functions, classes)
- ✅ 20-30% retrieval accuracy vs line-based
- ✅ Parent-child relationships for context expansion
- ⚠️ Requires Tree-sitter grammars (40+ languages)
- ⚠️ <1ms incremental parse for typical edits
- ⚠️ 85-90% parse rate (fallback for unparseable)

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After parse rate analysis (2026-05)

---

### [ADR-029] Incremental Indexing with Merkle Trees

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need efficient change detection for large codebases  
**Decision:** Implement Merkle tree-based change detection with content-addressed invalidation  

**Architecture:**
```rust
pub struct MerkleTree {
    root_hash: [u8; 32],
    nodes: HashMap<PathBuf, HashNode>,
}

pub struct HashNode {
    hash: [u8; 32],  // BLAKE3 or SHA-256
    children: Vec<PathBuf>,
    content_hash: Option<[u8; 32]>,  // For leaf nodes (files)
}

impl MerkleTree {
    pub fn compute_file_hash(&self, path: &Path) -> [u8; 32] {
        // BLAKE3 for speed (faster than SHA-256)
        blake3::hash(&std::fs::read(path).unwrap()).into()
    }
    
    pub fn find_changed(&self, old_tree: &MerkleTree) -> Vec<PathBuf> {
        // O(log n) comparison - only traverse divergent branches
        self.diff(old_tree)
    }
}
```

**Change Detection Pipeline:**
1. File system watches (`notify` crate) → real-time events
2. 500ms debouncing → batch rapid changes
3. Merkle tree comparison → identify changed files
4. Chunk-level hash check → invalidate only affected chunks
5. Selective re-embedding → 80-95% reduction vs full reindex

**Performance:**
- 50,000 files → ~3.2 MB hash metadata
- Changed file detection: O(log n) tree traversal
- Typical edit → 5-10 chunks invalidated (not full file)
- Incremental sync: <5s for typical changes

**Consequences:**
- ✅ O(log n) change detection vs O(n) full scan
- ✅ 80-95% reduction in re-embedding workload
- ✅ Content-addressed deduplication
- ✅ Git integration for historical context
- ⚠️ BLAKE3 computation overhead on large files
- ⚠️ Complex tree management (renames, moves)
- ⚠️ Need garbage collection for orphaned chunks

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After incremental sync benchmarking (2026-07)

---

### [ADR-030] Hybrid Retrieval Pipeline

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Single retrieval modality insufficient for code search  
**Decision:** Implement hybrid pipeline: HNSW vector + BM25 lexical + symbol exact + cross-encoder re-ranking  

**Pipeline Architecture:**
```
Query → Preprocessing → Embedding → Parallel Retrieval → Fusion → Re-rank → Results
                                    │
                                    ├── HNSW Vector (Top-100)
                                    ├── BM25 Lexical (Top-100)
                                    └── Symbol Exact (All)
```

**Implementation:**
```rust
pub struct HybridRetriever {
    vector_index: QdrantClient,
    bm25_index: TantivyIndex,  // or SQLite FTS
    symbol_index: HashMap<String, Vec<ChunkId>>,
    cross_encoder: MiniLM,
}

impl HybridRetriever {
    pub async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<Chunk>> {
        // Parallel retrieval
        let (vector_results, bm25_results, symbol_results) = tokio::join!(
            self.vector_search(query, 100),
            self.bm25_search(query, 100),
            Ok(self.symbol_search(query)),
        );
        
        // Reciprocal Rank Fusion
        let fused = rrf_fusion(vec![vector_results?, bm25_results?, symbol_results?]);
        
        // Cross-encoder re-ranking
        let reranked = self.cross_encoder.rerank(fused, query).await?;
        
        Ok(reranked.into_iter().take(limit).collect())
    }
}
```

**Latency Budget:**
| Stage | Target | Cumulative |
|-------|--------|------------|
| Preprocessing + Embedding | 10ms | 10ms |
| Parallel Retrieval | 20ms | 30ms |
| Score Fusion | 5ms | 35ms |
| Cross-Encoder Re-rank | 25ms | 60ms |
| Result Formatting | 5ms | 65ms |
| **Contingency (35%)** | 35ms | **<100ms P95** |

**Re-ranking Models:**
- MiniLM-L6 (22M params): 15-25ms, 5-10% MRR improvement
- Qwen2.5-1.5B (conditional): 50-100ms, 10-15% on complex queries

**Consequences:**
- ✅ <100ms P95 query latency
- ✅ >95% recall @10
- ✅ Handles semantic + lexical + exact queries
- ✅ Cross-encoder improves ranking quality
- ⚠️ Multiple index maintenance
- ⚠️ Re-ranking adds latency (conditional invocation)
- ⚠️ Score fusion tuning required

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After query latency profiling (2026-07)

---

### [ADR-031] Cross-File Dependency Tracking

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Code retrieval requires understanding cross-file relationships  
**Decision:** Build static call graph with Tree-sitter for co-retrieval of related definitions  

**Call Graph Construction:**
```rust
pub struct CallGraph {
    // callee → callers
    call_edges: HashMap<ChunkId, Vec<ChunkId>>,
    // caller → callees
    call_graph: HashMap<ChunkId, Vec<ChunkId>>,
    // import → imported symbols
    import_map: HashMap<PathBuf, Vec<Symbol>>,
}

impl CallGraph {
    pub fn build(&self, chunks: &[Chunk]) -> Self {
        // Tree-sitter resolves function calls to definitions
        // Handles language-specific module systems
    }
    
    pub fn get_related(&self, chunk_id: &ChunkId, depth: usize) -> Vec<ChunkId> {
        // BFS traversal to depth (typically 2-3 levels)
        // Returns callers, callees, type dependencies
    }
}
```

**Co-Retrieval Strategy:**
- When function A matches → fetch callers, callees, type definitions
- Bounded depth (2-3 levels) to prevent explosion
- Dependency-aware ranking boost for well-connected nodes
- Import resolution for cross-file references

**Consequences:**
- ✅ Enables "find all usages" queries
- ✅ Type resolution for better understanding
- ✅ Architectural queries (impact analysis)
- ⚠️ Call graph construction overhead
- ⚠️ Dynamic languages (Python, JS) have incomplete resolution
- ⚠️ Graph maintenance on incremental updates

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After call graph accuracy analysis (2026-08)

---

### [ADR-032] Generated Code Handling

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Generated code pollutes index with low-value content  
**Decision:** Detect and exclude generated code via heuristics and configuration  

**Detection Heuristics:**
```rust
pub fn is_generated(path: &Path, content: &str) -> bool {
    content.contains("DO NOT EDIT") ||
    content.contains("@generated") ||
    path.ends_with(".gen.rs") ||
    path.starts_with("target/") ||
    path.starts_with("node_modules/") ||
    (content.lines().count() > 1000 && repetition_score(content) > 0.8)
}
```

**Consequences:**
- ✅ Cleaner index, better retrieval quality
- ✅ Reduced storage and embedding costs
- ✅ Faster indexing
- ⚠️ False positives possible
- ⚠️ Need `.axoraignore` configuration

**Research:** [R-04 Findings](./findings/local-indexing/R-04-result.md)  
**Review Date:** After false positive analysis (2026-06)

---

## Local Model Optimization Decisions (R-05)

### [ADR-008] Local LLM Model Selection

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal local LLM models for agent inference  
**Decision:** Multi-tier model strategy with Qwen 2.5 Coder as primary  

**Model Tiers:**

| Tier | Model | Quantization | Size | HumanEval | Tokens/sec (M3) | Use Case |
|------|-------|--------------|------|-----------|-----------------|----------|
| **Default** | Qwen 2.5 Coder 7B | Q4_K_M | ~4.7GB | ~76% | 40-50 | Routine code gen, autocomplete |
| **Quality** | Qwen 2.5 Coder 32B | Q5_K_M | ~22GB | ~65% | 8-10 | Complex refactoring, debugging |
| **Speed** | Llama 3.3 8B | Q4_K_M | ~6GB | 72.6% | ~100 | Latency-critical autocomplete |
| **Premium** | Qwen 2.5 Coder 32B | Q8_0 | ~32GB | ~88% | 6-8 | Critical code review |

**Decision Rationale:**
- Qwen 2.5 Coder 7B: 88.4% HumanEval (FP16), matches 32B sibling
- Qwen 2.5 Coder 32B: 88.4% HumanEval, surpasses GPT-4 (87.1%)
- Llama 3.3 8B: 2.5-3.5× faster than Qwen 7B for speed-critical paths
- All Apache 2.0 licensed (commercial use OK)

**Hardware Requirements:**

| Hardware | Viable Models | Experience |
|----------|---------------|------------|
| 16GB RAM, M2/RTX 3060 | Qwen 7B @ Q4_K_M | Functional (minimum) |
| 32GB RAM, M3 Max/RTX 4070 | Qwen 7B/32B @ Q4/Q5 | Smooth (recommended) |
| 64GB RAM, M3 Ultra/RTX 4090 | Qwen 32B @ Q5_K_M | Near-cloud quality (optimal) |

**Implementation:**
- Ollama for service management (recommended)
- llama-cpp-rs for embedded low-latency paths
- Candle for pure Rust fallback

**Consequences:**
- ✅ Tier 2 quality achievable on consumer hardware
- ✅ 128K context window for repository-level understanding
- ✅ Apache 2.0 licensing (no commercial restrictions)
- ⚠️ 20GB+ for 32B model (high-end hardware required)
- ⚠️ 6-10 tok/s for 32B on Apple Silicon (acceptable for quality tasks)
- ⚠️ Model obsolescence risk (6-month cycle)

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After quarterly model review (2026-06)

---

### [ADR-033] Quantization Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal quantization for local deployment  
**Decision:** GGUF Q4_K_M as default, Q5_K_M for quality-critical  

**Quantization Tiers:**

| Format | Size Reduction | Quality Loss (Code) | Speed Gain | Use Case |
|--------|---------------|---------------------|------------|----------|
| Q8_0 | 50% | ~1% | 1.5× | Critical code review |
| Q6_K | 57% | ~1.5% | 1.8× | Premium 32B deployment |
| **Q5_K_M** | 64% | ~2-3% | 2.0× | Quality path default |
| **Q4_K_M** | 75% | ~3-5% | 2.5× | **Recommended default** |
| Q3_K_M | 81% | ~5-8% | 3.0× | Emergency low-resource |
| Q2_K | 88% | ~10-15% | 3.5× | Not recommended |

**Code-Specific Sensitivity:**

| Quantization | Syntactic Validity | Semantic Accuracy | Runtime Success |
|--------------|-------------------|-------------------|-----------------|
| Q8_0 | >99% | ~99% | Excellent |
| Q6_K | >98% | ~97% | Very Good |
| Q5_K_M | >97% | ~95% | Good |
| **Q4_K_M** | **>95%** | **~92%** | **Good** |
| Q3_K_M | ~92% | ~85% | Degraded |

**Decision Rationale:**
- Q4_K_M achieves 75% size reduction with ~3-5% quality loss
- K-variants use importance matrix weighting for attention weights
- Below Q4, syntax errors increase non-linearly
- Code more sensitive than conversational tasks (precise syntax)

**Consequences:**
- ✅ 75% size reduction enables consumer hardware deployment
- ✅ ~92% semantic accuracy for typical coding workflows
- ✅ GGUF universal compatibility (CPU, GPU, Apple Silicon)
- ⚠️ Quality degradation in complex multi-step reasoning
- ⚠️ Below Q4: bracket mismatches, type errors, API hallucinations

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After quality validation (2026-07)

---

### [ADR-034] Inference Engine Selection

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal inference engine for local LLM deployment  
**Decision:** Ollama (default service) + llama-cpp-rs (embedded low-latency)  

**Engine Comparison:**

| Engine | Maturity | Rust Integration | Performance | Best For |
|--------|----------|------------------|-------------|----------|
| **Ollama** | ⭐⭐⭐⭐⭐ (120K+ stars) | HTTP API (`ollama-rs`) | ~90% baseline | **Recommended default** |
| **llama.cpp** | ⭐⭐⭐⭐⭐ (reference) | `llama-cpp-rs` | 100% baseline | **Maximum control, low-latency** |
| MLX | ⭐⭐⭐⭐☆ | `mlx-rs` (limited) | ~150% Apple | Apple-only optimization |
| vLLM | ⭐⭐⭐⭐☆ | Limited | 3-5× batched | GPU batching, cloud |
| Candle | ⭐⭐⭐☆☆ | Native | ~70% baseline | Pure Rust stack |

**Ollama Benefits:**
- Zero-configuration deployment (`ollama pull qwen2.5-coder:7b`)
- OpenAI-compatible API (drop-in cloud replacement)
- Hot model swapping without restart
- Cross-platform consistency (macOS, Linux, Windows)
- ~5-10% overhead vs raw llama.cpp (acceptable for operational benefits)

**llama-cpp-rs Benefits:**
- Complete GGUF feature support (all quantization variants)
- GPU offloading flexibility (CUDA, Metal, Vulkan)
- Custom scheduling and speculative decoding
- Embedded deployment without service boundaries
- ~5-10% performance advantage over Ollama

**Implementation:**
```rust
// Ollama integration (recommended)
use ollama_rs::Ollama;

pub struct InferenceClient {
    client: Ollama,
    default_model: String,
}

impl InferenceClient {
    pub async fn generate(&self, prompt: &str) -> Result<GenerationResponse> {
        let request = GenerationRequest::new(
            self.default_model.clone(), 
            prompt.to_string()
        )
        .options(GenerationOptions::default()
            .temperature(0.2)
            .num_ctx(8192)
            .num_predict(2048));
        
        self.client.generate_stream(request).await
    }
}
```

**Consequences:**
- ✅ Operational simplicity (Ollama)
- ✅ Maximum control (llama-cpp-rs)
- ✅ Clean separation of concerns
- ⚠️ Ollama service dependency
- ⚠️ ~5-10% performance overhead (Ollama)
- ⚠️ Integration complexity (llama-cpp-rs)

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After performance profiling (2026-06)

---

### [ADR-035] Multi-Model Task Routing

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Heterogeneous agent workloads require intelligent model selection  
**Decision:** Implement hybrid router: heuristic for clear cases, confidence-based for ambiguous  

**Task-Based Routing:**

| Task Category | Model | Target Latency | Quality Expectation |
|---------------|-------|---------------|---------------------|
| Autocomplete (single token) | Llama 3.3 8B | <50ms | Syntactic correctness |
| Inline suggestions | Qwen 7B | <100ms | Local context match |
| Simple function generation | Qwen 7B | <500ms | 80%+ test passage |
| Syntax error fixing | Qwen 7B | <200ms | Compilation success |
| Documentation | Qwen 7B | <1s | Accuracy, completeness |
| Multi-file refactoring | Qwen 32B | <5s | Semantic preservation |
| Architecture design | Qwen 32B | <10s | Pattern appropriateness |
| Complex algorithm | Qwen 32B | <5s | Correctness, efficiency |
| Subtle bug diagnosis | Qwen 32B | <10s | Root cause identification |
| Comprehensive tests | Qwen 32B | <5s | Coverage, edge cases |

**Router Implementation:**
```rust
pub enum RoutingStrategy {
    /// Token count, keywords, file complexity
    Heuristic,
    /// Small classifier on task embedding
    EmbeddingBased,
    /// Fast-path generation with quality check
    ConfidenceBased,
}

pub struct TaskRouter {
    strategy: RoutingStrategy,
    fast_model: String,
    quality_model: String,
}

impl TaskRouter {
    pub fn route(&self, task: &AgentTask) -> ModelSelection {
        match self.strategy {
            RoutingStrategy::Heuristic => {
                if task.token_count < 500 && task.complexity < Threshold::Medium {
                    ModelSelection::Fast(self.fast_model.clone())
                } else {
                    ModelSelection::Quality(self.quality_model.clone())
                }
            }
            RoutingStrategy::ConfidenceBased => {
                let (result, confidence) = self.fast_path.generate(task).await;
                if confidence > 0.85 {
                    ModelSelection::FastResult(result)
                } else {
                    ModelSelection::Quality(self.quality_model.clone())
                }
            }
            // ...
        }
    }
}
```

**Escalation Triggers:**
- Detected complexity (token count, AST depth, dependency graph size)
- Explicit user request ("use best model")
- Fast-path confidence below threshold (<85%)
- Hardware unavailable (battery mode, thermal throttling, OOM)
- Time-critical deadline (calendar integration)

**Consequences:**
- ✅ Optimal resource utilization
- ✅ 60%+ users accept quality trade-offs for speed
- ✅ Transparent escalation paths
- ⚠️ Router adds ~10ms latency (embedding-based)
- ⚠️ Requires labeled data for training (embedding-based)
- ⚠️ Doubles latency for escalated tasks (confidence-based)

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After A/B testing (2026-08)

---

### [ADR-036] Cloud Fallback Integration

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Cloud frontier models retain advantages for specific scenarios  
**Decision:** Implement seamless cloud fallback with cost management  

**Cloud Fallback Triggers:**

| Condition | Detection Method | Cloud Target |
|-----------|---------------|--------------|
| Local confidence < threshold | Perplexity, consistency checks | GPT-4o, Claude 4 Sonnet |
| Task complexity > threshold | AST metrics, dependency analysis | Claude 4 (reasoning), GPT-4o (generation) |
| Explicit user request | UI toggle, prompt directive | User preference |
| Hardware unavailable | Battery mode, thermal throttling | Default cloud |
| Time-critical deadline | Calendar integration | Fastest available |

**Integration Patterns:**

| Pattern | Description | User Experience |
|---------|-------------|---------------|
| **Shadow mode** | Local and cloud in parallel; cloud replaces if better | Transparent, maximum quality |
| **Upgrade stream** | Begin local, background check triggers cloud continuation | Responsive start, quality guarantee |
| **Explicit choice** | Clear UI indication of local/cloud trade-offs | Informed consent, educational |

**Cost Management:**
- Per-user quotas (e.g., $10/month included)
- Organizational budgets with alerts
- Transparent cost attribution per task
- Rate limiting for non-critical tasks

**Implementation:**
```rust
pub struct HybridInference {
    local_client: InferenceClient,
    cloud_client: OpenAIClient,
    cost_tracker: CostTracker,
    quota_manager: QuotaManager,
}

impl HybridInference {
    pub async fn infer(&self, task: &AgentTask) -> Result<InferenceResult> {
        // Try local first
        match self.local_client.generate(&task.prompt).await {
            Ok(result) if result.confidence > 0.85 => {
                Ok(InferenceResult::Local(result))
            }
            Ok(_) | Err(_) => {
                // Check quota before cloud fallback
                if self.quota_manager.check_quota(task.user_id).await? {
                    let cloud_result = self.cloud_client.create_completion(...).await?;
                    self.cost_tracker.record(task.user_id, cloud_result.usage).await;
                    Ok(InferenceResult::Cloud(cloud_result))
                } else {
                    Err(Error::QuotaExceeded)
                }
            }
        }
    }
}
```

**Consequences:**
- ✅ Best of both worlds (local speed + cloud quality)
- ✅ Cost transparency and control
- ✅ User choice and informed consent
- ⚠️ Cloud API costs scale with usage
- ⚠️ Privacy considerations for code transmission
- ⚠️ Network dependency for fallback

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After cost analysis (2026-07)

---

### [ADR-037] Performance Optimization Pipeline

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need to maximize inference throughput for interactive use  
**Decision:** Implement layered optimizations: KV caching, batching, speculative decoding  

**Optimization Techniques:**

| Technique | Implementation | Expected Gain | Complexity |
|-----------|---------------|-------------|------------|
| **KV Cache Persistence** | LRU cache with 60% compression | 30-50% latency reduction | Medium |
| **Request Batching** | Dynamic batch size based on latency SLO | 20-40% throughput | Low |
| **Speculative Decoding** | 1.5B draft model, verify with 7B | 2-3× for repetitive patterns | High |
| **Prefix Caching** | Shared attention state for common prompts | 40-60% prompt processing | Medium |

**Consequences:**
- ✅ 2-3× throughput improvement
- ✅ 30-50% latency reduction
- ⚠️ Implementation complexity (speculative decoding)
- ⚠️ Memory overhead for KV cache

**Research:** [R-05 Findings](./findings/local-models/R-05-result.md)  
**Review Date:** After performance profiling (2026-07)

---

## Agent Architecture Decisions (R-06)

### [ADR-010] Agent Orchestration Pattern

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need deterministic orchestration to avoid hallucination loops  
**Decision:** Implement hierarchical state machine (LangGraph-inspired) with CrewAI role-based patterns  

**Architecture Pattern:**
```
┌─────────────────────────────────────────────────────────┐
│              Hierarchical State Machine                  │
├─────────────────────────────────────────────────────────┤
│  Orchestrator (State Graph)                             │
│  ├── Nodes: Agent functions                             │
│  ├── Edges: Conditional transitions                     │
│  └── State: Shared context                              │
├─────────────────────────────────────────────────────────┤
│  Role-Based Agents (CrewAI-inspired)                    │
│  ├── Arquiteto (design, structure)                      │
│  ├── Coder (implementation)                             │
│  ├── Reviewer (code review)                             │
│  ├── Tester (test generation)                           │
│  └── Debugger (bug fixing)                              │
├─────────────────────────────────────────────────────────┤
│  Communication: NATS JetStream (ADR-009)                │
└─────────────────────────────────────────────────────────┘
```

**Key Components:**
- **State Graph:** Directed acyclic graph (DAG) for workflows
- **Nodes:** Agent functions with explicit inputs/outputs
- **Edges:** Conditional transitions based on state
- **Checkpointing:** Time-travel debugging
- **Error Edges:** Fault tolerance with retry logic

**Orchestration Patterns:**

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **Sequential** | Tasks execute in defined order | Linear workflows |
| **Hierarchical** | Manager coordinates subordinates | Complex multi-agent tasks |
| **Parallel** | Agents work concurrently | Independent subtasks |
| **Cyclic** | Feedback loops for iteration | Refinement cycles |

**Comparison with Alternatives:**

| Framework | Pattern | Pros | Cons |
|-----------|---------|------|------|
| **LangGraph** | State machine | Deterministic, checkpointing | More complex |
| **AutoGen** | Message passing | Flexible, conversational | Hallucination loops |
| **CrewAI** | Role-based hierarchy | Clear responsibilities | Rigid, sequential |
| **AXORA (Hybrid)** | State machine + roles | Best of both | Implementation complexity |

**Consequences:**
- ✅ Deterministic execution (no hallucination loops)
- ✅ Built-in fault tolerance (error edges)
- ✅ Time-travel debugging (checkpointing)
- ✅ Clear role separation (CrewAI-inspired)
- ⚠️ More complex than pure message passing
- ⚠️ Central orchestrator can be bottleneck
- ⚠️ Requires careful state schema design

**Research:** [R-06 Findings](./findings/agent-architecture/R-06-result.md)  
**Review Date:** After workflow complexity analysis (2026-08)

---

### [ADR-038] Task Assignment Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need optimal task assignment for multi-agent coordination  
**Decision:** Implement capability-based assignment with dynamic load balancing  

**Assignment Strategies:**

| Strategy | Description | Pros | Cons |
|----------|-------------|------|------|
| **Static** | Pre-defined roles | Simple, predictable | Inflexible |
| **Dynamic** | Based on load/availability | Flexible, efficient | Complex |
| **Capability-Based** | Match skills to requirements | Optimal quality | Requires capability registry |
| **Market-Based** | Auction/bidding | Optimal assignment | Overhead, complexity |

**Decision:** Hybrid approach:
1. **Capability-Based** for quality-critical tasks
2. **Dynamic Load Balancing** for homogeneous pools
3. **Static** for simple, well-defined roles

**Implementation:**
```rust
pub struct TaskAssigner {
    capability_registry: CapabilityRegistry,
    load_balancer: LoadBalancer,
}

impl TaskAssigner {
    pub fn assign(&self, task: &Task, agents: &[Agent]) -> Option<AgentId> {
        // Filter by capability
        let capable = agents.iter()
            .filter(|a| a.capabilities.matches(&task.requirements))
            .collect::<Vec<_>>();
        
        // Load balance among capable agents
        self.load_balancer.select(&capable)
    }
}

pub struct Capability {
    pub skills: Vec<Skill>,
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub max_concurrent_tasks: usize,
}
```

**Consequences:**
- ✅ Optimal task-agent matching
- ✅ Load balancing prevents bottlenecks
- ✅ Capability registry enables discovery
- ⚠️ Requires capability definition/maintenance
- ⚠️ Load balancer adds complexity

**Research:** [R-06 Findings](./findings/agent-architecture/R-06-result.md)  
**Review Date:** After assignment efficiency analysis (2026-08)

---

### [ADR-039] Conflict Resolution Mechanism

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Agents may disagree on implementations or approaches  
**Decision:** Implement multi-tier conflict resolution: voting → arbitration → human escalation  

**Conflict Scenarios:**
- Two agents suggest different implementations
- Reviewer rejects writer's code
- Disagreement on architecture decisions

**Resolution Strategies:**

| Strategy | Description | Use Case |
|----------|-------------|----------|
| **Voting** | Majority wins | Simple disagreements |
| **Arbitration** | Designated arbiter decides | Technical disputes |
| **Merge** | LLM merges suggestions | Partial agreement |
| **Human Escalation** | Escalate to user | Unresolvable conflicts |

**Implementation:**
```rust
pub enum ConflictResolution {
    Voting { threshold: f32 },
    Arbitration { arbiter: AgentId },
    Merge { merger: AgentId },
    HumanEscalation,
}

pub struct ConflictResolver {
    strategy: ConflictResolution,
}

impl ConflictResolver {
    pub async fn resolve(&self, conflict: &Conflict) -> Result<Resolution> {
        match self.strategy {
            ConflictResolution::Voting { threshold } => {
                // Collect votes from agents
                // Return majority decision
            }
            ConflictResolution::Arbitration { arbiter } => {
                // Arbiter reviews both proposals
                // Makes final decision
            }
            ConflictResolution::Merge { merger } => {
                // Merger combines best parts of both
                // Produces unified solution
            }
            ConflictResolution::HumanEscalation => {
                // Present both options to user
                // User makes final decision
            }
        }
    }
}
```

**Escalation Path:**
```
Agent Disagreement
       ↓
Voting (if >80% agreement)
       ↓ (no consensus)
Arbitration (senior agent)
       ↓ (still unresolved)
Merge (combine suggestions)
       ↓ (still unresolved)
Human Escalation (user decides)
```

**Consequences:**
- ✅ Clear escalation path
- ✅ Multiple resolution strategies
- ✅ Human oversight for critical decisions
- ⚠️ Voting can be slow (collect all votes)
- ⚠️ Arbiter bias possible
- ⚠️ Merge may produce suboptimal results

**Research:** [R-06 Findings](./findings/agent-architecture/R-06-result.md)  
**Review Date:** After conflict frequency analysis (2026-09)

---

## Memory & State Decisions (R-07)

### [ADR-011] Memory Architecture

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Agents need persistent memory across sessions  
**Decision:** Implement multi-tier memory: short-term (context), long-term (vector DB), shared (blackboard)  

**Memory Types:**

| Type | Storage | Retrieval | Use Case |
|------|---------|-----------|----------|
| **Short-term** | Context window | Direct access | Current session, immediate context |
| **Long-term Episodic** | Vector DB (Qdrant) | Similarity search | Past experiences, conversations |
| **Long-term Semantic** | Vector DB + Graph | Hybrid search | Facts, knowledge, relationships |
| **Procedural** | Database | Lookup | Skills, workflows, tool usage |
| **Shared Blackboard** | LanceDB/Qdrant | Publish/subscribe | Team knowledge, repository maps |

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│              Multi-Tier Memory System                    │
├─────────────────────────────────────────────────────────┤
│  Short-term (Context Window)                            │
│  - Current conversation                                 │
│  - Immediate task context                               │
├─────────────────────────────────────────────────────────┤
│  Long-term Episodic (Vector DB)                         │
│  - Past conversations                                   │
│  - Previous task outcomes                               │
│  - User interactions                                    │
├─────────────────────────────────────────────────────────┤
│  Long-term Semantic (Vector + Graph)                    │
│  - Codebase knowledge                                   │
│  - Architecture decisions                               │
│  - API documentation                                    │
├─────────────────────────────────────────────────────────┤
│  Shared Blackboard (Publish/Subscribe)                  │
│  - Repository maps                                      │
│  - Team conventions                                     │
│  - Shared learnings                                     │
└─────────────────────────────────────────────────────────┘
```

**Memory Operations:**
```rust
pub struct MemoryManager {
    short_term: ContextWindow,
    episodic: VectorStore,
    semantic: HybridStore,
    shared: Blackboard,
}

impl MemoryManager {
    pub async fn write(&self, memory: &Memory) -> Result<()> {
        match memory.type {
            MemoryType::Episodic => self.episodic.insert(memory).await,
            MemoryType::Semantic => self.semantic.insert(memory).await,
            MemoryType::Shared => self.shared.publish(memory).await,
        }
    }
    
    pub async fn retrieve(&self, query: &str, type: MemoryType) -> Result<Vec<Memory>> {
        match type {
            MemoryType::Episodic => self.episodic.search(query, 10).await,
            MemoryType::Semantic => self.semantic.hybrid_search(query, 10).await,
            MemoryType::Shared => self.shared.subscribe(query).await,
        }
    }
}
```

**Memory Consolidation:**
- **Summarization:** Compress detailed interactions into summaries
- **Embedding:** Convert experiences to vector representations
- **Integration:** Merge related memories, resolve conflicts
- **Prioritization:** Tag important memories for retention

**Forgetting Mechanisms:**
- **Decay:** Time-based expiration (TTL)
- **Capacity Limits:** LRU eviction when storage full
- **Relevance Filtering:** Remove low-utility memories
- **Privacy Compliance:** User-initiated deletion

**Consequences:**
- ✅ Persistent knowledge across sessions
- ✅ Multi-tier optimization (speed vs capacity)
- ✅ Shared knowledge enables collaboration
- ⚠️ Memory consistency challenges
- ⚠️ Forgetting logic complexity
- ⚠️ Privacy/security considerations

**Research:** [R-07 Findings](./findings/memory-state/R-07-result.md)  
**Review Date:** After memory usage analysis (2026-08)

---

### [ADR-040] Multi-Agent Memory Sharing

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Agents need to share knowledge without conflicts  
**Decision:** Implement shared blackboard with access control and consistency protocols  

**Sharing Patterns:**

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **Private** | Agent-specific memory | Sensitive data, agent state |
| **Shared Read** | All agents can read | Repository maps, conventions |
| **Shared Write** | All agents can write | Team learnings, discoveries |
| **Controlled** | Access control required | Sensitive knowledge |

**Implementation:**
```rust
pub struct SharedBlackboard {
    store: VectorStore,
    access_control: AccessControl,
    consistency: ConsistencyProtocol,
}

impl SharedBlackboard {
    pub async fn publish(&self, memory: &Memory, agent: &Agent) -> Result<()> {
        // Check write permissions
        if !self.access_control.can_write(agent, memory) {
            return Err(Error::AccessDenied);
        }
        
        // Check for conflicts
        if let Some(conflict) = self.consistency.check_conflict(memory).await {
            return self.consistency.resolve(conflict, memory).await;
        }
        
        // Publish to shared store
        self.store.insert(memory).await
    }
    
    pub async fn retrieve(&self, query: &str, agent: &Agent) -> Result<Vec<Memory>> {
        // Check read permissions
        if !self.access_control.can_read(agent, query) {
            return Err(Error::AccessDenied);
        }
        
        self.store.search(query, 10).await
    }
}
```

**Consistency Protocols:**
- **Event Sourcing:** Track all changes, enable replay
- **Version Vectors:** Detect concurrent modifications
- **Conflict Resolution:** Merge or escalate on conflicts

**Consequences:**
- ✅ Shared knowledge enables collaboration
- ✅ Access control prevents unauthorized access
- ✅ Consistency protocols prevent conflicts
- ⚠️ Complexity in conflict resolution
- ⚠️ Performance overhead for consistency
- ⚠️ Access control management

**Research:** [R-07 Findings](./findings/memory-state/R-07-result.md)  
**Review Date:** After sharing pattern analysis (2026-08)

---

## Evaluation & Benchmarking Decisions (R-08)

### [ADR-041] Evaluation Framework

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need comprehensive evaluation for agent quality  
**Decision:** Implement multi-dimensional evaluation: benchmarks, real-world tasks, user satisfaction  

**Evaluation Dimensions:**

| Dimension | Metrics | Tools | Frequency |
|-----------|---------|-------|-----------|
| **Code Quality** | HumanEval, MBPP, SWE-Bench | Standard benchmarks | Per model release |
| **Task Completion** | Success rate, time to completion | Custom test suite | Per feature release |
| **User Satisfaction** | Ratings, acceptance rate, edit distance | UI telemetry | Continuous |
| **Reliability** | Error rate, retry rate, crash rate | Monitoring | Continuous |
| **Efficiency** | Token usage, latency, cost | Cost tracking | Continuous |

**Benchmark Suite:**

| Benchmark | Purpose | Target (7B) | Target (32B) |
|-----------|---------|-------------|--------------|
| HumanEval | Function generation | >70% | >85% |
| MBPP | Python problems | >60% | >75% |
| SWE-Bench Verified | Real bug fixing | N/A | >40% |
| Aider | Multi-file editing | >50% | >65% |
| LiveCodeBench | Competitive programming | >50% | >70% |
| Custom AXORA Suite | Agent-specific tasks | Establish baseline | >90% of 7B |

**Implementation:**
```rust
pub struct EvaluationFramework {
    benchmarks: Vec<Benchmark>,
    real_world_tasks: Vec<Task>,
    user_feedback: FeedbackCollector,
}

impl EvaluationFramework {
    pub async fn evaluate(&self, model: &Model) -> EvaluationReport {
        let mut report = EvaluationReport::new();
        
        // Run benchmarks
        for benchmark in &self.benchmarks {
            let score = benchmark.run(model).await?;
            report.benchmark_scores.insert(benchmark.name, score);
        }
        
        // Run real-world tasks
        for task in &self.real_world_tasks {
            let result = task.execute(model).await?;
            report.task_success_rate += result.success as f32;
        }
        
        // Collect user feedback
        report.user_satisfaction = self.user_feedback.collect().await;
        
        report
    }
}
```

**Success Criteria:**
- HumanEval: >70% (7B), >85% (32B)
- Task success rate: >80%
- User satisfaction: >4.0/5.0
- Error rate: <5%
- Token efficiency: Within budget

**Consequences:**
- ✅ Comprehensive quality measurement
- ✅ Continuous improvement feedback
- ✅ User-centric evaluation
- ⚠️ Benchmark maintenance overhead
- ⚠️ Real-world task curation effort
- ⚠️ User feedback collection complexity

**Research:** [R-08 Findings](./findings/evaluation/R-08-result.md)  
**Review Date:** After quarterly evaluation (2026-09)

---

### [ADR-042] Production Monitoring Strategy

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Need real-time monitoring for production systems  
**Decision:** Implement multi-layer monitoring: metrics, logs, traces, alerts  

**Monitoring Layers:**

| Layer | Metrics | Tools | Alert Threshold |
|-------|---------|-------|-----------------|
| **Infrastructure** | CPU, memory, disk, network | Prometheus, Grafana | >80% utilization |
| **Model** | Token usage, latency, error rate | Custom metrics | >5% error rate |
| **Agent** | Task success, retry rate, conflicts | Event tracking | >10% failure rate |
| **User** | Satisfaction, acceptance, churn | Analytics | <3.5/5.0 rating |

**Key Metrics:**

| Category | Metric | Target | Alert |
|----------|--------|--------|-------|
| **Performance** | Query latency P95 | <100ms | >500ms |
| **Reliability** | Error rate | <5% | >10% |
| **Quality** | Task success rate | >80% | <60% |
| **Efficiency** | Token cost per task | <$0.01 | >$0.05 |
| **User** | Satisfaction score | >4.0/5.0 | <3.5/5.0 |

**Implementation:**
```rust
pub struct MonitoringSystem {
    metrics: MetricsCollector,
    logs: LogAggregator,
    traces: TraceCollector,
    alerts: AlertManager,
}

impl MonitoringSystem {
    pub async fn record(&self, event: &AgentEvent) {
        // Record metrics
        self.metrics.increment(&event.type);
        self.metrics.histogram(&event.latency);
        
        // Log event
        self.logs.log(&event);
        
        // Record trace
        self.traces.record(&event.trace_id, &event.span);
        
        // Check alerts
        if let Some(alert) = self.alerts.check(&event).await {
            self.alerts.send(alert).await;
        }
    }
}
```

**Alert Channels:**
- Email for critical alerts
- Slack for warnings
- Dashboard for informational

**Consequences:**
- ✅ Real-time visibility
- ✅ Proactive issue detection
- ✅ Data-driven optimization
- ⚠️ Monitoring overhead
- ⚠️ Alert fatigue risk
- ⚠️ Privacy considerations

**Research:** [R-08 Findings](./findings/evaluation/R-08-result.md)  
**Review Date:** After production deployment (2026-10)
Local Filesystem
       │
       ▼
┌──────────────────────────┐
│  Compute File Hashes     │ (SHA-256 per file)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Build Merkle Tree       │ (Recursive folder hashes)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Compare with Server     │ (Find divergent branches)
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  Sync Only Changed Files │ (Differential payload)
└──────────────────────────┘
```

**Hash Computation:**
- File hash: SHA-256 of file content
- Folder hash: SHA-256 of concatenated child hashes (sorted by name)
- Root hash: Single hash representing entire repository state

**Sync Protocol:**
1. Client computes local Merkle tree
2. Client sends root hash to server
3. Server compares with stored tree
4. Server identifies divergent branches
5. Client uploads only changed files
6. Server re-indexes changed chunks only

**Performance:**
- 50,000 files → ~3.2 MB hash metadata
- Changed file detection: O(log n) tree traversal
- Sync payload: Only changed files (not full scan)

**Alternatives Considered:**
- Full re-scan on every change: Too slow
- File watcher only: Misses external changes
- Git-based diff: Doesn't handle uncommitted changes

**Consequences:**
- ✅ Sub-second change detection
- ✅ Minimal bandwidth usage
- ✅ Works with uncommitted changes
- ⚠️ SHA-256 computation overhead on large files
- ⚠️ Complex tree management logic
- ⚠️ Need to handle file renames/moves

**Implementation:**
- Use `sha2` crate for hashing
- Store tree in SQLite for persistence
- Background sync thread (non-blocking)

**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Review Date:** After large repo testing (2026-08)

**Date:** 2026-03-16  
**Status:** ✅ Accepted  
**Context:** Project was pinned to Rust 1.75.0, but dependencies required newer versions  
**Decision:** Updated to Rust 1.94.0  
**Consequences:** 
- ✅ All dependencies compile
- ✅ Access to latest Rust features
- ⚠️ Users need recent Rust installation
**Research:** N/A (build fix)  
**Review Date:** N/A

---

### [ADR-002] Protocol Buffers for gRPC

**Date:** 2026-03-16  
**Status:** ✅ Accepted (inherited)  
**Context:** Need serialization format for agent communication  
**Decision:** Keep Protocol Buffers (prost/tonic)  
**Consequences:**
- ✅ Strong typing
- ✅ Schema evolution support
- ✅ Rust ecosystem support
- ⚠️ Requires protobuf compiler
**Research:** [R-02: Inter-Agent Communication](./prompts/02-inter-agent-communication.md)  
**Review Date:** After R-02 research complete

---

### [ADR-003] Tauri v2 for Desktop

**Date:** 2026-03-16  
**Status:** 🔄 Under Research  
**Context:** Need desktop application framework  
**Decision:** Continue with Tauri v2 (pending research)  
**Consequences:**
- ✅ Leverages Rust backend
- ✅ Small bundle size
- ⚠️ v2 is relatively new
**Research:** [R-03: Desktop App](../../planning/PHASE-003-desktop-app.md)  
**Review Date:** After R-03 research complete

---

### [ADR-004] SQLite for Storage

**Date:** 2026-03-16  
**Status:** ✅ Accepted (inherited)  
**Context:** Need persistent storage for agents, tasks, messages  
**Decision:** Use SQLite with rusqlite  
**Consequences:**
- ✅ Simple deployment (single file)
- ✅ Good Rust support
- ✅ Low overhead
- ⚠️ Limited concurrency
- ⚠️ Need vector extension for embeddings
**Research:** [R-04: Local Indexing](./prompts/04-local-indexing-embedding.md)  
**Review Date:** After R-04 research complete

---

### [ADR-005] gRPC for Desktop-Daemon Communication

**Date:** 2026-03-16  
**Status:** ✅ Accepted (inherited)  
**Context:** Need IPC between desktop app and daemon  
**Decision:** Use gRPC over localhost  
**Consequences:**
- ✅ Strong typing
- ✅ Streaming support
- ✅ Well-supported in Rust
- ⚠️ Requires protobuf compilation for frontend
**Research:** [R-02: Inter-Agent Communication](./prompts/02-inter-agent-communication.md)  
**Review Date:** After R-02 research complete

---

## Pending Decisions

These decisions await research completion:

| ID | Topic | Depends On | Target Date |
|----|-------|------------|-------------|
| ADR-006 | Embedding Model | R-04 | 2026-03-23 |
| ADR-007 | Vector Database | R-04 | 2026-03-23 |
| ADR-008 | Local LLM Model | R-05 | 2026-03-23 |
| ADR-009 | Agent Communication Protocol | R-02 | 2026-03-23 |
| ADR-010 | Agent Architecture | R-06 | 2026-03-30 |
| ADR-011 | Memory Architecture | R-07 | 2026-03-30 |
| ADR-012 | Context Management Strategy | R-01 | 2026-03-23 |

---

## Decision Making Process

1. **Research First:** No decision without supporting research
2. **Document:** Record decision in this log
3. **Review:** Set review date for complex decisions
4. **Communicate:** Ensure team knows about decisions

---

## Research Status

| Research | Status | Completion | Decision Impact |
|----------|--------|------------|-----------------|
| R-01: Context Management | 📋 Ready | 0% | ADR-012 |
| R-02: Inter-Agent Communication | 📋 Ready | 0% | ADR-002, ADR-005, ADR-009 |
| R-03: Token Efficiency | 📋 Ready | 0% | Multiple |
| R-04: Local Indexing | 📋 Ready | 0% | ADR-004, ADR-006, ADR-007 |
| R-05: Model Optimization | 📋 Ready | 0% | ADR-008 |
| R-06: Agent Architecture | 📋 Ready | 0% | ADR-010 |
| R-07: Memory & State | 📋 Ready | 0% | ADR-011 |
| R-08: Evaluation | 📋 Ready | 0% | Multiple |

---

## Notes

- Decisions marked "(inherited)" were made before this log was created
- Research must be completed before changing inherited decisions
- Review dates are targets, not deadlines

---

## Template for New Decisions

```markdown
### [ADR-XXX] Title

**Date:** YYYY-MM-DD  
**Status:** Proposed | Accepted | Deprecated | Superseded  
**Context:** The problem we're solving  
**Decision:** What we decided  
**Consequences:**
- ✅ Positive implications
- ⚠️ Negative implications
**Research:** [Link to research](./prompts/XX-...)
**Review Date:** YYYY-MM-DD
```
