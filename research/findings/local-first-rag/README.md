# Local-First RAG Project

**Mission:** Zero cloud embedding costs, <1GB RAM usage  
**Status:** 📋 Planning Complete — Ready to Start  
**Owner:** Agent B  
**Priority:** 🔴 CRITICAL  

---

## 🎯 Goal

Implement a complete RAG system that:
- ✅ Runs 100% locally (no cloud embedding API calls)
- ✅ Uses <1GB RAM (works on any dev machine)
- ✅ Achieves <100ms retrieval latency
- ✅ Supports incremental indexing (no full re-index on every change)
- ✅ Pure Rust implementation (no Python dependencies)

---

## 📁 Project Structure

```
local-first-rag/
├── README.md                        ← This file (overview)
├── R-16-LOCAL-FIRST-RAG.md          ← Research findings (the "why")
└── IMPLEMENTATION-PLAN.md           ← Detailed plan (the "how")
```

---

## 🚀 Quick Start

### For Agent B (Implementer)

1. **Read Research:** [`R-16-LOCAL-FIRST-RAG.md`](./R-16-LOCAL-FIRST-RAG.md)
2. **Read Plan:** [`IMPLEMENTATION-PLAN.md`](./IMPLEMENTATION-PLAN.md)
3. **Start Sprint B5:** Download Jina Code v2 from HuggingFace
4. **Implement:** Update `crates/axora-embeddings/src/jina.rs`
5. **Test:** Ensure <25ms latency target

---

## 📊 Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)

| Sprint | Title | Duration | Status |
|--------|-------|----------|--------|
| **B5** | Jina Code Embeddings | 2-3 days | ⏳ NEXT |
| **B6** | Qdrant Embedded Setup | 2-3 days | ⏳ Pending |
| **B7** | AST-Based Code Chunking | 3 days | ⏳ Pending |
| **B8** | Merkle Tree + Change Detection | 3 days | ⏳ Pending |

### Phase 2: Integration & Optimization (Week 3-4)

| Sprint | Title | Duration | Status |
|--------|-------|----------|--------|
| **B9** | RAG Pipeline Integration | 3 days | ⏳ Pending |
| **B10** | Performance Optimization | 3 days | ⏳ Pending |
| **B11** | Developer Experience | 2 days | ⏳ Pending |
| **B12** | Testing & Validation | 3 days | ⏳ Pending |

**Total Duration:** 4 weeks (8 sprints)

---

## 📈 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| RAM Usage (peak) | <1GB | `htop` during embedding |
| RAM Usage (idle) | <300MB | `htop` at rest |
| Retrieval Latency (P95) | <100ms | End-to-end query time |
| Embedding Speed | >100 blocks/sec | Batch initial scan |
| Cloud Costs | $0/month | Monthly bill |

---

## 🔗 Related Work

### Completed Components

| Component | Status | Location |
|-----------|--------|----------|
| PrefixCache | ✅ Implemented | `crates/axora-cache/src/prefix_cache.rs` |
| Diff | ✅ Implemented | `crates/axora-cache/src/diff.rs` |
| InfluenceGraph | ✅ Implemented | `crates/axora-indexing/src/influence.rs` |
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` |

### Current Agent Task

**Agent B:** [`planning/agent-b/current_task.md`](../../planning/agent-b/current_task.md)

---

## 📚 Deep Dive

### Research Findings

- **Problem:** Cloud embeddings cost $$$ per query
- **Solution:** 4-pillar local-first architecture
- **Impact:** $0 cloud costs, <1GB RAM, <100ms latency

**Read:** [`R-16-LOCAL-FIRST-RAG.md`](./R-16-LOCAL-FIRST-RAG.md)

### Implementation Details

- **Sprint-by-sprint breakdown**
- **Code deliverables**
- **Success criteria**
- **Dependencies**

**Read:** [`IMPLEMENTATION-PLAN.md`](./IMPLEMENTATION-PLAN.md)

---

## 🎯 Four Pillars

### Pillar 1: Jina Code Embeddings v2

- 137M parameters (~550MB RAM)
- CPU-only inference (~15ms/query)
- 97% accuracy of giant models

### Pillar 2: Qdrant Embedded

- Zero background servers
- ~200MB RAM for 100K vectors
- <5ms retrieval latency

### Pillar 3: Candle Framework

- Pure Rust (no Python)
- AVX2 CPU acceleration
- ~15-25ms per block

### Pillar 4: Surgical Indexing

- Tree-sitter for semantic chunking
- BLAKE3 hashes for change detection
- 80-95% reduction in re-indexing

---

## ✅ Getting Started

**Agent B should:**

1. Read [`R-16-LOCAL-FIRST-RAG.md`](./R-16-LOCAL-FIRST-RAG.md)
2. Review [`IMPLEMENTATION-PLAN.md#sprint-1`](./IMPLEMENTATION-PLAN.md#sprint-1-jina-code-embeddings-integration)
3. Download Jina Code v2 from HuggingFace
4. Start implementing Sprint B5

---

**Ready to execute!** 🚀

**Last Updated:** 2026-03-18  
**Owner:** Agent B
