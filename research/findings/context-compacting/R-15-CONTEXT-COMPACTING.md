# R-15: Context Compacting for Multi-Agent Systems

**Priority:** 🔴 CRITICAL (enables long-running agents, reduces costs)  
**Status:** ✅ Research Complete — Ready for Implementation  
**Date:** 2026-03-18  
**Source:** Comprehensive research on context escalation crisis  

---

## 🎯 Problem Statement: Context Escalation Crisis

### The Crisis

As agents operate over dozens or hundreds of sequential turns, they accumulate:
- Conversational history
- Tool execution logs
- Retrieved document segments
- Intermediate reasoning traces

**Without active intervention:**
- Context window saturates within **10-20 turns**
- Token costs explode ($0.03-0.12 per 1K tokens)
- Latency increases (O(n²) attention computation)
- **Cognitive degradation:** Accuracy plummets from 98.1% to <64%

### The Goal

**Context Compacting:** Algorithmic reduction of LLM operational memory to maximum information density

**Target Compression Ratios:**
- **10:1 to 30:1** compression
- Decouple functional memory from raw token footprint
- Preserve reasoning quality while accelerating inference

---

## 📊 Core Compacting Techniques

### Technique Comparison Matrix

| Category | Mechanism | Compression | Reasoning Preservation | Complexity |
|----------|-----------|-------------|----------------------|------------|
| **Summarization** | Hierarchical / Rolling | 5x-10x | Moderate (semantic drift risk) | Low-Moderate |
| **Memory-Based** | OS-Style Paging (Letta) | Infinite (fixed window) | High (requires robust retrieval) | High |
| **Structural** | TOON / JSON Patch | 2x-5x | **Perfect (lossless)** | Low |
| **Attention (Hard)** | Entropy Pruning (LLMLingua) | 3x-10x | Moderate (syntax degradation) | High |
| **Attention (Soft)** | Latent Compilation (KV Cache) | **16x-32x** | **Very High** | Very High |

---

### 1. Summarization-Based Compacting

#### Rolling Summary
- Maintain sliding window of recent N turns verbatim
- Oldest turns recursively summarized
- **Risk:** Semantic drift over hundreds of iterations

#### Hierarchical Summary (H-MEM / HiMem)
- Separate episodic memory from semantic knowledge
- Topic-Aware Event-Surprise Dual-Channel Segmentation
- Extract key events into higher-level knowledge nodes

#### Key-Point (Extractive) Pruning
- Retain only critical factual sentences
- Preserve exact syntax (beneficial for code)
- **Advantage:** No abstractive rewriting

---

### 2. Memory-Based Compacting

**Inspired by operating systems:**

#### Working Memory
- Fixed-size, strictly managed
- Active task variables only

#### Episodic Memory
- Raw chronological interaction logs
- Flushed from context immediately
- Stored externally, retrieved on-demand

#### Semantic Memory
- Facts, rules, world-states
- Embedded and stored in vector databases
- Injected only when semantically relevant

**Implementation Example (Letta):**
- Queue Manager monitors token pressure
- When context approaches saturation → trigger "flush"
- Evict oldest messages, recursively summarize
- Move raw data to external PostgreSQL/pgvector

---

### 3. Structural Compacting

#### Schema-Based Pruning
- Trim JSON payloads by removing null fields
- Remove deeply nested irrelevant arrays
- Remove verbose keys

#### TOON (Token-Oriented Object Notation)
- **Problem:** JSON is token-heavy (repeated keys, brackets, quotes)
- **Solution:** Define schema once, stream comma-separated values only
- **Savings:** Up to **80% token reduction** (lossless)

#### Diff-Based Context
- Provide only diffs (RFC 6902 JSON Patch)
- Not full state rewrites
- **Savings:** 89-98% for state updates

---

### 4. Attention-Based Compression

#### Hard Compression (Token Pruning)
- **Framework:** LLMLingua
- Calculate information entropy of every token
- Remove low-entropy (predictable, redundant) tokens
- **Risk:** Can destroy long-range semantic dependencies

#### Soft Compression (Latent Context Compilation)
- **The current frontier of compacting**
- Compile contexts directly into latent embeddings (KV cache tensors)
- **Techniques:** In-context Autoencoders (ICAE), disposable LoRAs
- **Compression:** **16x-32x** with near-zero loss
- **Advantage:** Bypasses text-tokenization bottleneck

---

## 🏗️ The Size-Fidelity Paradox & ACON

### The Paradox

**Problem:** Large foundation models hallucinate when compressing context
- Models overwrite source facts with pre-trained priors (semantic drift)

**Solution:** Shift to smaller, distilled models optimized for compression

### Agent Context Optimization (Acon)

**Leading framework for safe compression:**
- Utilizes failure-driven, task-aware guidelines
- Distills histories safely
- **Results:**
  - 26%-54% peak token reduction
  - **46% improvement** in downstream reasoning accuracy
  - Prevents semantic drift

---

## 📡 Context Sharing & Distribution

### Multi-Agent Context Architectures

#### A. Centralized Blackboard

```
Coordinator → Blackboard (shared state) → Workers read/write
```

**Characteristics:**
- Shared, highly observable knowledge base
- Hosted on in-memory data store (Redis, real-time DB)
- Agents monitor for state changes matching preconditions

**Strengths:**
- ✅ Exceptional for opportunistic, non-linear problem solving
- ✅ Decouples agents (don't need to know each other)

**Weaknesses:**
- ❌ Susceptible to race conditions
- ❌ Read/write conflicts without strict concurrency management

---

#### B. Publish-Subscribe (Pub-Sub)

```
Agent publishes event → Message bus (NATS, Redis, Kafka) → Subscribers receive
```

**Characteristics:**
- Reactive, event-driven mesh
- No central intelligence
- Agents subscribe to topic-specific events

**Strengths:**
- ✅ Highly scalable
- ✅ Minimizes token transfer (targeted events only)
- ✅ Accommodates late-joining agents (event stream replay)

**Weaknesses:**
- ⚠️ Message ordering complexity
- ⚠️ Observability challenges

---

#### C. Coordinator-Mediated (Hierarchical)

```
All agents → Coordinator → Routes context
```

**Characteristics:**
- Central Supervisor/Coordinator manages state flow
- Workers receive slice of state, execute, return modified state

**Strengths:**
- ✅ Strict, deterministic workflows
- ✅ No consistency risk (sequential execution)

**Weaknesses:**
- ❌ Coordinator context bloat
- ❌ Scaling bottleneck (need nested sub-hierarchies for 100+ agents)

---

### Context Distribution Patterns

| Pattern | Token Cost | When to Use |
|---------|------------|-------------|
| **Full Broadcast** | O(N × context_size) | ❌ Micro-swarms only (2-3 agents) |
| **Diff-Based Push** | O(N × diff_size) | ✅ **Optimal** (89-98% savings) |
| **Pull-Based** | O(requests × context_size) | ⚠️ Token-efficient but high latency |
| **Versioned Context** | O(mismatches × diff_size) | ✅ Best for large-scale systems |

---

## 🔐 Context Consistency & Versioning

### Concurrency Strategies

#### A. Lock-Based Consistency

**Mechanism:** Mutual exclusion (Mutex) locks
- Agent requests lock → reads → modifies → writes → unlocks
- Other agents blocked until unlock

**Trade-off:**
- ✅ Prevents race conditions
- ❌ Destroys parallelization (O(N × L) contention)

---

#### B. Snapshot-Based (MVCC) Consistency

**Mechanism:** Multiversion Concurrency Control
- Agents read snapshot at specific timestamp
- Execute work based on local snapshot
- Conflicts detected at commit phase

**Trade-off:**
- ✅ Reduces read-blocking
- ❌ "Wasted work" (stale reads rejected at commit)

---

#### C. CRDT (Conflict-Free Replicated Data Types) — RECOMMENDED

**Mechanism:** Strong Eventual Consistency (SEC)
- Every agent maintains local replica
- Read/write locally without locks
- Mathematical guarantees of deterministic convergence

**Implementation (CodeCRDT):**
- Yjs library + WebSockets
- Y.Text for code, Y.Map for task claiming
- Last-Write-Wins (LWW) + logical clocks

**Caveat:**
- ✅ 100% syntactic convergence (no text corruption)
- ⚠️ 5-10% semantic conflicts still require evaluator agent

---

### Message History Propagation

| Pattern | Description | Verdict |
|---------|-------------|---------|
| **Full History** (OpenAI) | Entire message array for every request | ❌ Anti-pattern (context explosion) |
| **System + Messages** (Anthropic) | System prompt (cached) + truncated messages | ✅ Better, but still limited |
| **Hierarchical Memory** (AXORA) | Bounded size (~5K-8K tokens) regardless of turns | ✅ **Optimal** |

**Hierarchical Memory Structure:**
```
System: Immutable architecture, control flow, tool schemas
Working Context: Key-value pairs (current_goal, critical_files)
Recent Events: Rolling window of last N actions
Semantic Summaries: Compressed knowledge from vector DB
```

---

## 🏆 Competitive Analysis

### Framework Comparison

| Framework | Topology | Native Compaction | Consistency Model | Target Use Case |
|-----------|----------|-------------------|-------------------|-----------------|
| **Letta** | OS Memory Paging | Memory Block Self-Editing | Database Transactions | Long-term memory assistants |
| **LangGraph** | Graph / State Machine | Manual (Reducers/Trimming) | Functional Reducers | Deterministic enterprise workflows |
| **AutoGen** | Group Conversation | Message Truncation | Sequential Round-Robin | Multi-agent debate & consensus |
| **CrewAI** | Role-based Delegation | Semantic Extraction (RAG) | Cognitive Consolidation | Team-based task pipelines |
| **OpenHands** | Event-Sourced Log | Rolling Condenser | Deterministic Event Replay | Deep software engineering |
| **Dify** | Visual RAG Pipeline | Summary Index | Pipeline Variable State | Knowledge base QA apps |

---

### Key Takeaways from Each Framework

#### Letta
- ✅ Memory block self-editing
- ✅ Queue Manager for token pressure monitoring
- ✅ External archival storage (PostgreSQL/pgvector)

#### LangGraph
- ✅ Explicit global state (TypedDict/Pydantic)
- ✅ Reducer functions for safe state aggregation
- ⚠️ No native auto-compact (manual implementation required)

#### AutoGen
- ✅ ContextVariables (shared dictionary)
- ⚠️ Simplistic list truncation (FIFO)
- ⚠️ Highly susceptible to context bloat

#### CrewAI
- ✅ Entity extraction + RAG
- ✅ Cognitive consolidation (detect similar memories, merge/delete)
- ✅ Isolated private context scopes per agent

#### OpenHands
- ✅ Event-sourced state architecture
- ✅ LLMSummarizingCondenser (aggressive middle-log condensation)
- ✅ Deterministic event replay (S_t = f(S_0, E_1, E_2, ..., E_t))

#### Dify
- ✅ Summary Index (semantic summaries for chunk clusters)
- ✅ Holistic structured context retrieval
- ⚠️ Basic token pruning for conversations

---

## 🏗️ AXORA Architecture Recommendations

### Recommended Topology: Coordinator + CRDT Blackboard

```
┌─────────────────────────────────────────────────────────────────┐
│              AXORA Context Architecture                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Global Semantic State (Coordinator)                            │
│  • Lightweight graph of active tasks                            │
│  • Agent health metrics                                         │
│  • High-level milestones                                        │
│                                                                  │
│  Shared State (CRDT Blackboard)                                 │
│  • In-memory blackboard (Yjs-based)                            │
│  • Y.Map for key-value state                                   │
│  • Y.Text for text generation                                  │
│  • Strong Eventual Consistency (SEC)                           │
│                                                                  │
│  Diff-Based Event Bus (Redis Pub/Sub)                          │
│  • RFC 6902 JSON Patch generation                              │
│  • Topic-specific subscriptions                                │
│  • O(N × diff_size) token cost                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

### Context Compaction Integration

**AXORA Worker Agent Prompt Structure:**

```
┌─────────────────────────────────────────────────────────────────┐
│  Working Memory Block (Strictly Bounded: ~5K-8K tokens)         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. System Prompt (Immutable, Cached)                          │
│     • Architecture instructions                                 │
│     • Control flow rules                                        │
│     • Tool schemas                                              │
│                                                                  │
│  2. Working Context (Key-Value Pairs)                          │
│     • current_goal                                              │
│     • critical_files                                            │
│     • Updated via JSON patches only                            │
│                                                                  │
│  3. Recent Events (Rolling Window: Last N=10 Actions)          │
│     • Strict FIFO eviction                                     │
│     • No summarization (verbatim)                              │
│                                                                  │
│  4. Semantic Summaries (Dynamic Injection)                     │
│     • Retrieved from vector DB on relevance                    │
│     • Highly compressed knowledge                              │
│     • Evicted when no longer relevant                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

### Implementation Priorities

#### Phase 1: Foundation (Week 1-2)

| Component | Priority | Effort |
|-----------|----------|--------|
| CRDT Blackboard (Yjs) | 🔴 CRITICAL | 3-4 days |
| Diff-Based Event Bus | 🔴 CRITICAL | 2-3 days |
| Hierarchical Memory Structure | 🔴 CRITICAL | 2 days |

#### Phase 2: Compaction Engines (Week 2-3)

| Component | Priority | Effort |
|-----------|----------|--------|
| TOON Serializer | 🟡 HIGH | 1-2 days |
| Rolling Summary | 🟡 HIGH | 2 days |
| Semantic Memory (Vector DB) | 🟡 HIGH | 3 days |

#### Phase 3: Advanced Optimization (Week 3-4)

| Component | Priority | Effort |
|-----------|----------|--------|
| Latent Compilation (KV Cache) | 🟡 MEDIUM | 4-5 days |
| ACON Integration | 🟡 MEDIUM | 3 days |
| Performance Benchmarking | 🟡 MEDIUM | 2 days |

---

## 📈 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Compression Ratio** | 10:1 to 30:1 | Tokens before / after |
| **Reasoning Accuracy** | >95% (no degradation) | Task success rate |
| **Context Window Size** | <8K tokens (bounded) | Max tokens per agent |
| **Token Cost Reduction** | 60-80% | $/session comparison |
| **Latency Reduction** | 30-50% | Time-to-first-token |
| **Concurrency Speedup** | 3-5x | Parallel vs sequential |

---

## 🔗 Related Documents

- [`01_CORE_ARCHITECTURE.md`](../docs/active_architecture/01_CORE_ARCHITECTURE.md) — Blackboard, orchestration
- [`03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`](../docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md) — Prefix caching, diff communication
- [R-17: Multi-Agent Optimization](../multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md) — API cost reduction

---

## 📚 Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` |
| TOON Serializer | 📋 Planned | Next sprint |
| CRDT Integration | 📋 Planned | Next sprint |
| Diff-Based Event Bus | 📋 Planned | Next sprint |
| Hierarchical Memory | 📋 Designed | Research complete |

---

**Research Status:** ✅ **Complete**  
**Priority:** 🔴 **CRITICAL** (enables long-running agents)  
**Next Step:** Create implementation plan
