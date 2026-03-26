# 01_CORE_ARCHITECTURE

**Status:** Active (this doc is maintained; not every subsection is “enforced” in code—see below)  
**Last Updated:** 2026-03-26  
**Owner:** Architect Agent  

---

## 🎯 Overview

OPENAKTA uses a **hybrid architecture**:
- **Cloud APIs** for reasoning (OpenAI-family) — No local LLM inference
- **Local infrastructure** for indexing, RAG, and memory — Zero cloud costs for embeddings
- **Deterministic orchestration** — State machines, not conversational swarms

**Mission Operating Layer (MOL) — today vs target**

- **Today:** Data model and APIs for story intake, preparation, requirements, verification, and closure live primarily in **`openakta-api`** (Postgres migrations such as `openakta-api/migrations/0005_mission_operating_layer.sql`, handlers in `openakta-api/src/work_management.rs`). The **daemon** (`aktacode/crates/openakta-daemon`, e.g. `background/work_management_service.rs`, `background/work_plan_compiler.rs`) mirrors read models locally and drives execution. **Not every transition or invariant is enforced uniformly** across API, daemon, and agents yet; roadmap work adds authoritative gates.
- **Target:** Preparation and closure behave as **state machines with hard gates** (no “rich JSON but false-done”). Depends on ongoing MOL implementation (validation in API, compiler, coordinator).
- **Legacy:** Execution may still use **raw work items** or paths that predate strict MOL; see `docs/aios/mission-operating-layer.md`.

Intended product shape for MOL:

- **Preparation** — stories are clarified, profiled, and compiled into prepared packets; **target** is that Balanced+ profiles require readiness before execution.
- **Closure** — **target** is that mission success moves work toward **`closure_pending`** and only authoritative closure (coverage, claims, verification, handoffs, gates) yields **`closed`**—not task completion alone.

---

## 🏗️ Blackboard Architecture

### Central Coordination Pattern

```
┌─────────────────────────────────────────────────────────────────┐
│                    OPENAKTA Blackboard                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐   │
│  │  Coordinator │────▶│  Blackboard  │◀────│ Worker Agent │   │
│  │  (State      │     │  (Shared     │     │  (Planning   │   │
│  │   Machine)   │     │   State)     │     │   Thread)    │   │
│  └──────────────┘     └──────────────┘     └──────────────┘   │
│                              │                                  │
│                              ▼                                  │
│                     ┌──────────────┐                           │
│                     │ Worker Agent │                           │
│                     │ (Acting      │                           │
│                     │  Thread)     │                           │
│                     └──────────────┘                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **No Direct Agent Communication** — Agents publish/subscribe to Blackboard
2. **State Machine Orchestration** — Deterministic execution (no loops)
3. **Typed Blackboard Artifacts** — Runtime publication carries namespaces and schema labels for mission, verification, and result channels
4. **Binary Protocol** — Protobuf for inter-agent messages (not JSON)
5. **Snapshot-Based Consistency** — Prevents TOCTOU bugs

### Implementation

```rust
pub struct Blackboard {
    state: DashMap<String, Value>,
    version: AtomicU64,
    subscribers: DashMap<String, Sender<Update>>,
}

impl Blackboard {
    pub fn publish(&self, key: &str, value: Value) {
        // Update state
        // Increment version
        // Notify subscribers
    }
    
    pub fn subscribe(&self, key: &str) -> Receiver<Update> {
        // Return channel for updates
    }
}
```

**Location:** `crates/openakta-cache/src/blackboard/v2.rs`

---

## 🧠 Dual-Thread ReAct Loops

### Planning vs. Acting Threads

Each worker agent runs **two parallel threads**:

```
┌─────────────────────────────────────────────────────────────┐
│                    Worker Agent                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Planning Thread           Acting Thread                     │
│  (Reasoning)               (Execution)                       │
│                                                              │
│  ┌────────────────┐        ┌────────────────┐               │
│  │ • Parse goal   │        │ • Execute tool │               │
│  │ • Generate plan│        │ • Write code   │               │
│  │ • Check constraints    │ • Run tests    │               │
│  │ • Validate     │        │ • Report result│               │
│  └────────────────┘        └────────────────┘               │
│           │                          │                       │
│           └──────────┬───────────────┘                       │
│                      ▼                                       │
│           ┌──────────────────┐                              │
│           │  Shared Context  │                              │
│           │  (Snapshot)      │                              │
│           └──────────────────┘                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Benefits

- **No Blocking** — Planning continues while acting executes
- **Parallel Validation** — Planning validates while acting runs
- **Snapshot Safety** — Both threads read from consistent snapshot

### Implementation

```rust
pub struct WorkerAgent {
    planning_thread: PlanningThread,
    acting_thread: ActingThread,
    shared_context: Arc<RwLock<ContextSnapshot>>,
}

impl WorkerAgent {
    pub async fn run(&self, goal: &str) -> Result<Outcome> {
        // Spawn planning thread
        let planning = tokio::spawn(self.planning_thread.run(goal));
        
        // Spawn acting thread
        let acting = tokio::spawn(self.acting_thread.run());
        
        // Wait for both
        let (plan, outcome) = tokio::join!(planning, acting);
        
        Ok(outcome?)
    }
}
```

**Location:** `crates/openakta-agents/src/worker.rs`

---

## 🔗 Code Influence Graph

### Dependency-Aware Context Retrieval

Instead of sending entire codebase to LLM, we send only the **influence slice**:

```
Query: "Fix auth token refresh bug"
     │
     ▼
┌─────────────────────────────────────────┐
│  Influence Graph Traversal              │
│                                         │
│  1. Find affected file (auth.rs)        │
│  2. Get direct dependencies (5 files)   │
│  3. Get reverse dependencies (3 files)  │
│  4. Calculate transitive closure        │
│  5. Apply token budget (max 2.5K tokens)│
│                                         │
│  Result: 12 files, 2.3K tokens          │
│  (vs 50K tokens for full codebase)      │
└─────────────────────────────────────────┘
```

### Influence Vector

Each file has a pre-calculated influence vector:

```rust
pub struct InfluenceVector {
    pub file_id: FileId,
    pub direct_dependencies: Vec<FileId>,      // Files this file depends on
    pub reverse_dependencies: Vec<FileId>,     // Files that depend on this file
    pub call_graph_depth: usize,               // Max depth of call chain
    pub business_rule_count: usize,            // Linked business rules
    pub transitive_closure: Vec<FileId>,       // All affected files
}
```

### SCIP Protocol Integration

We use **SCIP (Sourcegraph Code Intelligence Protocol)** for language-agnostic indexing:

- **Protobuf format** (not JSON) — Compact, typed
- **Human-readable identifiers** — Not opaque numeric IDs
- **Package ownership** — (manager, name, version, symbol)

**Location:** `crates/openakta-indexing/src/influence.rs`

---

## 📡 Communication Protocol

### NATS JetStream + Protobuf

**Transport Layer:** NATS JetStream
- **Async message passing** — Decoupled agents
- **Persistent streams** — Survive restarts
- **At-least-once delivery** — No lost messages

**Message Format:** Protobuf
```protobuf
message AgentMessage {
  string task_id = 1;
  MessageType type = 2;
  bytes payload = 3;  // Protobuf-encoded
  uint64 timestamp = 4;
}

enum MessageType {
  TASK_ASSIGNED = 0;
  PROGRESS_UPDATE = 1;
  RESULT_SUBMITTED = 2;
  BLOCKER_ALERT = 3;
}
```

### Message Types

| Type | Purpose | Size Target |
|------|---------|-------------|
| `TASK_ASSIGNED` | Coordinator → Worker | <500 bytes |
| `PROGRESS_UPDATE` | Worker → Blackboard | <200 bytes |
| `RESULT_SUBMITTED` | Worker → Coordinator | <1KB (diff) |
| `BLOCKER_ALERT` | Worker → Coordinator | <300 bytes |

**Key:** All messages are **binary Protobuf**, not natural language.

---

## 🔄 State Machine Orchestration

### Deterministic Execution

```rust
pub enum AgentState {
    Pending,
    InProgress,
    WaitingForInput,
    Completed,
    Failed,
}

pub struct StateMachine {
    current_state: AgentState,
    transitions: Vec<Transition>,
}

impl StateMachine {
    pub fn transition(&mut self, event: Event) -> Result<()> {
        // Validate transition (no loops)
        // Execute transition
        // Update state
        Ok(())
    }
}
```

### Benefits

- **No Infinite Loops** — Graph validation prevents cycles
- **Predictable Execution** — Same input → same output
- **Debuggable** — State transitions are logged

---

## 📊 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Blackboard Publish | <10ms | P95 latency |
| State Transition | <5ms | P95 latency |
| Influence Graph Traversal | <50ms | For 10K files |
| Message Size (avg) | <500 bytes | Protobuf-encoded |
| Retrieval Latency | <100ms | End-to-end query |

---

## 🔗 Related Documents

- [`02_LOCAL_RAG_AND_MEMORY.md`](./02_LOCAL_RAG_AND_MEMORY.md) — RAG, embeddings, memory
- [`03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`](./03_CONTEXT_AND_TOKEN_OPTIMIZATION.md) — Caching, diffs, SCIP

---

## 📚 Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| Blackboard v2 | ✅ Implemented | `crates/openakta-cache/src/blackboard/v2.rs` |
| Influence Graph | ✅ Implemented | `crates/openakta-indexing/src/influence.rs` |
| Dual-Thread ReAct | ✅ Designed | Research complete |
| NATS + Protobuf | 📋 Planned | Next sprint |
| State Machine | ✅ Designed | Graph workflow |

---

**Scope:** This document is the **primary architecture narrative** for core runtime concepts (blackboard, indexing, protocols). It is **not** a guarantee that every box in every diagram is production-complete—compare with the implementation ledger and `docs/aios/*` for MOL specifics.

**Last Reviewed:** 2026-03-26  
**Next Review:** After MVP launch or major MOL gate milestones
