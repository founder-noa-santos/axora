# Agent B — Current Task

**Date:** 2026-03-18  
**Status:** 🔄 **STARTING SPRINT B5**  
**Priority:** 🔴 CRITICAL  

---

## 📋 Your Complete Sequence

| Phase | Sprints | Status | Timing |
|-------|---------|--------|--------|
| Phase 1 (Local-First RAG) | B5, B6, B7, B8 | ⏳ **STARTING B5** | Weeks 1-2 |
| Phase 2 (Multi-Agent) | B9, B10 | ⏳ Pending | Week 2-3 |
| Phase 3 | — | ✅ COMPLETE | — |

**Total:** 6 sprints (13-16 days of work)  
**Utilization:** 100% (Weeks 1-3 full)

---

## 🎯 Your Missions

### Mission 1: Local-First RAG (Weeks 1-2)

**Goal:** Zero cloud embedding costs, <1GB RAM usage

**Your Sprints:**
- **B5:** Jina Code Embeddings (2-3 days)
- **B6:** Qdrant Embedded Setup (2-3 days)
- **B7:** AST-Based Code Chunking (3 days)
- **B8:** Merkle Tree + Change Detection (3 days)

### Mission 2: Multi-Agent API Optimization (Week 2-3)

**Goal:** 95-99% context token reduction

**Your Sprints:**
- **B9:** SCIP Indexing (3-4 days)
- **B10:** Context Pruning / Graph Retrieval (2-3 days)

---

## 📊 Complete Task List

**See:** [`planning/MASTER-TASK-LIST.md`](../planning/MASTER-TASK-LIST.md)

**All Your Sprints:**
- B5: Jina Code Embeddings
- B6: Qdrant Embedded Setup
- B7: AST-Based Code Chunking
- B8: Merkle Tree + Change Detection
- B9: SCIP Indexing
- B10: Context Pruning (Graph Retrieval)

---

## 🚀 Starting NOW: Sprint B5

**Sprint B5: Jina Code Embeddings Integration**

**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL (blocks all other RAG sprints)

**Tasks:**
1. [ ] Download Jina Code v2 weights from HuggingFace
2. [ ] Convert model to Candle Safetensors format
3. [ ] Implement `JinaEmbedder` struct in `openakta-embeddings`
4. [ ] Add CPU-only inference (AVX2 acceleration)
5. [ ] Implement embedding normalization
6. [ ] Add embedding cache (disk-based, avoid re-computation)
7. [ ] Write benchmarks (target: <25ms per block)

**Deliverables:**
- `crates/openakta-embeddings/src/jina.rs` — Jina model wrapper
- `crates/openakta-embeddings/src/cache.rs` — Embedding cache
- `benches/embed_bench.rs` — Performance benchmarks

**Success Criteria:**
- [ ] Can embed 100 code blocks in <3 seconds
- [ ] Single embedding takes <25ms on CPU
- [ ] Cache hit reduces latency to <5ms
- [ ] Memory usage <600MB during inference

**Reference:** `planning/MASTER-TASK-LIST.md#sprint-b5-jina-code-embeddings`

---

## 📈 What Comes Next

### After B5 (Week 1)

| Sprint | Title | Duration | Priority |
|--------|-------|----------|----------|
| **B6** | Qdrant Embedded Setup | 2-3 days | 🔴 CRITICAL |
| **B7** | AST-Based Code Chunking | 3 days | 🔴 CRITICAL |
| **B8** | Merkle Tree + Change Detection | 3 days | 🔴 HIGH |

### After Local-First RAG (Week 2-3)

| Sprint | Title | Duration | Priority |
|--------|-------|----------|----------|
| **B9** | SCIP Indexing | 3-4 days | 🟡 HIGH |
| **B10** | Context Pruning | 2-3 days | 🔴 CRITICAL |

---

## 📚 Reference Files

- **Master Task List:** `planning/MASTER-TASK-LIST.md` (ALL tasks for all agents)
- **Local-First RAG Plan:** `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md`
- **RAG Research:** `research/findings/local-first-rag/R-16-LOCAL-FIRST-RAG.md`
- **Multi-Agent Plan:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md`
- **Multi-Agent Research:** `research/findings/multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md`
- **Your Status:** `planning/agent-b/AGENT-B-STATUS.md`
- **Dashboard:** `planning/STATUS-DASHBOARD.md`

---

## ✅ Definition of Ready for Sprint B5

Agent B is ready when:
- [x] All Phase 3 sprints complete (B1, B2)
- [x] All Phase 4 settings sprints complete (B4)
- [x] Status files updated
- [x] No pending tasks

**Status:** ✅ **ALL CRITERIA MET** — START B5 NOW

---

## 🚀 Next Steps

**Today:**
1. Read plan: `research/findings/local-first-rag/IMPLEMENTATION-PLAN.md#sprint-1`
2. Download Jina Code v2 model from HuggingFace
3. Start implementing in `crates/openakta-embeddings/src/jina.rs`

**This Week:**
- Complete Sprint B5 (Jina Embeddings)
- Start Sprint B6 (Qdrant Embedded)

---

**Agent B is STARTING Sprint B5 (Jina Code Embeddings)!** 🚀
