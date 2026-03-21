# Master Task List — All Agents

**Date:** 2026-03-18  
**Status:** ✅ Active  
**Missions:** Local-First RAG + Multi-Agent API Optimization  

---

## 🎯 Complete Task Sequence

### Phase 1: Local-First RAG Core (Week 1-2)

| Sprint | Owner | Title | Duration | Status | Dependencies |
|--------|-------|-------|----------|--------|--------------|
| **B5** | Agent B | Jina Code Embeddings | 2-3 days | ⏳ **NEXT** | None |
| **B6** | Agent B | Qdrant Embedded Setup | 2-3 days | ⏳ Pending | B5 |
| **B7** | Agent B | AST-Based Code Chunking | 3 days | ⏳ Pending | B5 |
| **B8** | Agent B | Merkle Tree + Change Detection | 3 days | ⏳ Pending | B7 |

**Agent A Status:** ⏸️ **IDLE** — Waiting for Phase 4 (Benchmarking)  
**Agent C Status:** ⏸️ **IDLE** — Working on Multi-Agent Phase 1 (parallel track)

---

### Phase 2: Multi-Agent API Optimization (Week 1-3)

| Sprint | Owner | Title | Duration | Status | Dependencies |
|--------|-------|-------|----------|--------|--------------|
| **C7** | Agent C | API Client with Prefix Caching | 2 days | ⏳ **NEXT** | None |
| **C8** | Agent C | Diff-Only Output Enforcement | 1-2 days | ⏳ Pending | C7 |
| **B9** | Agent B | SCIP Indexing | 3-4 days | ⏳ Pending | B8 (Local-First RAG complete) |
| **B10** | Agent B | Context Pruning (Graph Retrieval) | 2-3 days | ⏳ Pending | B9 |
| **C11** | Agent C | Agent Message Protocol (Protobuf) | 2 days | ⏳ Pending | C8 |
| **C12** | Agent C | Graph Workflow Enforcement | 2 days | ⏳ Pending | C11 |

**Agent A Status:** ⏸️ **IDLE** — Waiting for Phase 4 (Benchmarking)  
**Agent B Status:** 🔄 **BUSY** — After Local-First RAG, starts Multi-Agent Phase 2

---

### Phase 3: Validation & Documentation (Week 4)

| Sprint | Owner | Title | Duration | Status | Dependencies |
|--------|-------|-------|----------|--------|--------------|
| **A4** | Agent A | Token Savings Benchmarking | 2 days | ⏳ Pending | All Phase 1-3 sprints |
| **A5** | Agent A | Production Readiness & Docs | 2 days | ⏳ Pending | A4 |

**Agent B Status:** ✅ **COMPLETE** — All sprints done  
**Agent C Status:** ✅ **COMPLETE** — All sprints done

---

## 📊 Agent Workload Overview

### Agent A (Documentation Specialist)

| Phase | Sprints | Status | Timing |
|-------|---------|--------|--------|
| Phase 1 | — | ⏸️ IDLE | Weeks 1-3 |
| Phase 2 | — | ⏸️ IDLE | Weeks 1-3 |
| Phase 3 | A4, A5 | ⏳ Pending | Week 4 |

**Total:** 2 sprints (4 days)  
**Utilization:** 20% (Weeks 1-3 idle, Week 4 active)

---

### Agent B (Storage/Context Specialist)

| Phase | Sprints | Status | Timing |
|-------|---------|--------|--------|
| Phase 1 (Local-First RAG) | B5, B6, B7, B8 | ⏳ **NEXT: B5** | Weeks 1-2 |
| Phase 2 (Multi-Agent) | B9, B10 | ⏳ Pending | Week 2-3 |
| Phase 3 | — | ✅ COMPLETE | — |

**Total:** 6 sprints (13-16 days)  
**Utilization:** 100% (Weeks 1-3 full)

---

### Agent C (Implementation Specialist)

| Phase | Sprints | Status | Timing |
|-------|---------|--------|--------|
| Phase 1 (Multi-Agent) | C7, C8 | ⏳ **NEXT: C7** | Week 1 |
| Phase 2 (Multi-Agent) | C11, C12 | ⏳ Pending | Week 3 |
| Phase 3 | — | ✅ COMPLETE | — |

**Total:** 4 sprints (7-8 days)  
**Utilization:** 70% (Week 2 may be light)

---

## 🔗 Critical Path

```
Week 1:
├─ Agent B: Sprint B5 (Jina Embeddings) ────────┐
├─ Agent C: Sprint C7 (API Client + Caching) ───┤
└─ Agent A: ⏸️ IDLE                              │
                                                   │
Week 2:                                            │
├─ Agent B: Sprints B6-B8 (Qdrant, AST, Merkle) ──┼──┐
├─ Agent C: Sprint C8 (Diff Enforcement) ─────────┼──┤
└─ Agent A: ⏸️ IDLE                                │  │
                                                    │  │
Week 3:                                            │  │
├─ Agent B: Sprints B9-B10 (SCIP, Context Pruning)┼──┼──┐
├─ Agent C: Sprints C11-C12 (Protocol, Workflow) ─┼──┼──┤
└─ Agent A: ⏸️ IDLE                                │  │  │
                                                    │  │  │
Week 4:                                            │  │  │
└─ Agent A: Sprints A4-A5 (Benchmarking, Docs) ───┴──┴──┘
```

---

## 📋 Detailed Task Descriptions

### Sprint B5: Jina Code Embeddings

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Download Jina Code v2 weights from HuggingFace
- [ ] Convert model to Candle Safetensors format
- [ ] Implement `JinaEmbedder` struct in `openakta-embeddings`
- [ ] Add CPU-only inference (AVX2 acceleration)
- [ ] Implement embedding normalization
- [ ] Add embedding cache (disk-based)
- [ ] Write benchmarks (target: <25ms per block)

**Deliverables:**
- `crates/openakta-embeddings/src/jina.rs`
- `crates/openakta-embeddings/src/cache.rs`
- `benches/embed_bench.rs`

**Reference:** `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md#sprint-1`

---

### Sprint B6: Qdrant Embedded Setup

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Add `qdrant-client` crate (embedded mode)
- [ ] Implement `VectorStore` initialization
- [ ] Define payload schema (file path, language, block type)
- [ ] Implement CRUD operations (insert, delete, update, search)
- [ ] Add hybrid search (BM25 + vectors)
- [ ] Implement payload filtering
- [ ] Add persistence (survive restarts)

**Deliverables:**
- `crates/openakta-rag/src/vector_store.rs`
- `crates/openakta-rag/src/hybrid_search.rs`
- `crates/openakta-rag/src/schema.rs`

**Reference:** `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md#sprint-2`

---

### Sprint B7: AST-Based Code Chunking

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Add `tree-sitter` + language grammars (Rust, TS, Python)
- [ ] Implement `CodeChunker` struct
- [ ] Extract semantic units (functions, classes, modules)
- [ ] Implement chunk size normalization
- [ ] Add chunk metadata (file path, language, location)
- [ ] Handle edge cases (malformed code, mixed languages)

**Deliverables:**
- `crates/openakta-indexing/src/chunker.rs`
- `crates/openakta-indexing/src/languages/`
- `crates/openakta-indexing/src/metadata.rs`

**Reference:** `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md#sprint-3`

---

### Sprint B8: Merkle Tree + Change Detection

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 HIGH

**Tasks:**
- [ ] Add `blake3` crate for fast hashing
- [ ] Implement `MerkleIndex` struct
- [ ] Integrate with file watcher (`notify` crate)
- [ ] Implement change detection algorithm
- [ ] Add hash persistence (survive restarts)
- [ ] Implement garbage collection

**Deliverables:**
- `crates/openakta-indexing/src/merkle.rs`
- `crates/openakta-indexing/src/watcher.rs`
- `crates/openakta-indexing/src/gc.rs`

**Reference:** `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md#sprint-4`

---

### Sprint C7: API Client with Prefix Caching

**Owner:** Agent C  
**Duration:** 2 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Add `PrefixCache` field to `ApiClient` struct
- [ ] Implement `extract_static_prefix(messages)` function
- [ ] Add cache key computation (SHA256 of prefix)
- [ ] Integrate with Anthropic cache headers
- [ ] Integrate with OpenAI prefix caching
- [ ] Add metrics tracking
- [ ] Write integration tests

**Deliverables:**
- `crates/openakta-agents/src/api_client.rs`
- `crates/openakta-agents/src/cache_integration.rs`
- `crates/openakta-agents/src/metrics.rs`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-c7`

---

### Sprint C8: Diff-Only Output Enforcement

**Owner:** Agent C  
**Duration:** 1-2 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Create `DiffEnforcer` struct
- [ ] Implement `validate_output(output: &AgentOutput) -> Result<()>`
- [ ] Add system prompt (`prompts/diff_only.md`)
- [ ] Add auto-conversion (full write → diff)
- [ ] Add metrics (diff size vs full file size)
- [ ] Write tests

**Deliverables:**
- `crates/openakta-agents/src/diff_enforcer.rs`
- `crates/openakta-agents/src/prompts/diff_only.md`
- `crates/openakta-agents/src/converter.rs`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-c8`

---

### Sprint B9: SCIP Indexing

**Owner:** Agent B  
**Duration:** 3-4 days  
**Priority:** 🟡 HIGH

**Tasks:**
- [ ] Define SCIP Protobuf schema
- [ ] Add `rust-analyzer` for Rust parsing
- [ ] Add `scip-typescript` for TS/JS parsing
- [ ] Add `scip-python` for Python parsing
- [ ] Integrate with `InfluenceGraph` (replace simple parsing)
- [ ] Write tests (verify symbol extraction)

**Deliverables:**
- `crates/openakta-indexing/src/scip.rs`
- `crates/openakta-indexing/src/parsers/`
- `crates/openakta-indexing/src/scip.proto`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-b9`

---

### Sprint B10: Context Pruning (Graph Retrieval)

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
- [ ] Create `GraphRetriever` struct
- [ ] Implement `retrieve_relevant_context()`
- [ ] Implement dependency graph traversal (BFS/DFS)
- [ ] Add token budget enforcement
- [ ] Integrate with existing RAG pipeline
- [ ] Add metrics
- [ ] Write tests

**Deliverables:**
- `crates/openakta-rag/src/graph_retriever.rs`
- `crates/openakta-rag/src/pruning.rs`
- `crates/openakta-rag/src/traversal.rs`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-b10`

---

### Sprint C11: Agent Message Protocol

**Owner:** Agent C  
**Duration:** 2 days  
**Priority:** 🟡 HIGH

**Tasks:**
- [ ] Define `AgentMessage` enum
- [ ] Implement Protobuf serialization
- [ ] Integrate with Blackboard v2 (publish/subscribe)
- [ ] Update agents to use protocol
- [ ] Add validation (schema enforcement)
- [ ] Write tests

**Deliverables:**
- `crates/openakta-agents/src/protocol.rs`
- `crates/openakta-agents/src/protocol.proto`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-c11`

---

### Sprint C12: Graph Workflow Enforcement

**Owner:** Agent C  
**Duration:** 2 days  
**Priority:** 🟡 HIGH

**Tasks:**
- [ ] Define workflow graph (states, transitions)
- [ ] Enforce deterministic execution (no loops)
- [ ] Add timeout enforcement
- [ ] Integrate with Coordinator
- [ ] Add metrics
- [ ] Write tests

**Deliverables:**
- `crates/openakta-agents/src/workflow.rs`
- `crates/openakta-agents/src/state_machine.rs`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-c12`

---

### Sprint A4: Token Savings Benchmarking

**Owner:** Agent A  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM

**Tasks:**
- [ ] Set up benchmark suite
- [ ] Measure prefix caching savings (target: 50-90%)
- [ ] Measure diff communication savings (target: 89-98%)
- [ ] Measure context pruning savings (target: 95-99%)
- [ ] Generate validation report

**Deliverables:**
- `benches/token_savings_bench.rs`
- `docs/TOKEN-SAVINGS-VALIDATION.md`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-a4`

---

### Sprint A5: Production Readiness & Documentation

**Owner:** Agent A  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM

**Tasks:**
- [ ] Write user guide
- [ ] Write developer guide
- [ ] Create migration guide
- [ ] Add troubleshooting guide
- [ ] Create demo video

**Deliverables:**
- `docs/MULTI-AGENT-OPTIMIZATION.md`
- `docs/API-COST-OPTIMIZATION.md`
- `docs/MIGRATION-GUIDE.md`

**Reference:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-a5`

---

## 📈 Expected Impact

| Metric | Current | Target | Reduction |
|--------|---------|--------|-----------|
| **Input Tokens** | 50,000/session | 2,500/session | 95% |
| **Output Tokens** | 10,000/session | 500/session | 95% |
| **Context Tokens** | 50,000/session | 500-2,500/session | 95-99% |
| **Cost per Session** | $4.80 | $0.39 | 92% |
| **Monthly Cost (100/day)** | $14,400 | $1,170 | 92% |
| **Annual Savings** | — | — | **$158,760** |

---

## ✅ Quick Reference

### Starting Next (Week 1)

| Agent | Sprint | Title | Priority |
|-------|--------|-------|----------|
| **Agent B** | B5 | Jina Code Embeddings | 🔴 CRITICAL |
| **Agent C** | C7 | API Client with Prefix Caching | 🔴 CRITICAL |
| **Agent A** | — | ⏸️ IDLE (Weeks 1-3) | — |

### After Week 1

| Agent | Sprint | Title | Priority |
|-------|--------|-------|----------|
| **Agent B** | B6-B8 | Qdrant, AST Chunking, Merkle | 🔴 CRITICAL |
| **Agent C** | C8 | Diff-Only Enforcement | 🔴 CRITICAL |
| **Agent A** | — | ⏸️ IDLE (Weeks 1-3) | — |

### After Week 2

| Agent | Sprint | Title | Priority |
|-------|--------|-------|----------|
| **Agent B** | B9-B10 | SCIP Indexing, Context Pruning | 🟡 HIGH / 🔴 CRITICAL |
| **Agent C** | C11-C12 | Protocol, Workflow | 🟡 HIGH |
| **Agent A** | — | ⏸️ IDLE (Weeks 1-3) | — |

### Week 4

| Agent | Sprint | Title | Priority |
|-------|--------|-------|----------|
| **Agent A** | A4-A5 | Benchmarking, Documentation | 🟡 MEDIUM |
| **Agent B** | — | ✅ COMPLETE | — |
| **Agent C** | — | ✅ COMPLETE | — |

---

**All tasks from both missions (Local-First RAG + Multi-Agent Optimization) are listed here. Nothing is lost.** 🚀

**Last Updated:** 2026-03-18  
**Maintained By:** Architect Agent
