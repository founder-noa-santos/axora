# Active Architecture Documentation

**Status:** ✅ Active & Enforced  
**Last Updated:** 2026-03-18  
**Owner:** Architect Agent  

---

## 🎯 Purpose

This folder contains the **Single Source of Truth** for OPENAKTA architecture.

All documents here are:
- ✅ **Aligned with strategic pivot** (Cloud APIs + Local RAG)
- ✅ **Implementation-ready** (not just research)
- ✅ **Actively maintained** (updated on each sprint)
- ✅ **White-listed concepts only** (no deprecated ideas)

---

## 📁 Document Structure

```
active_architecture/
├── 01_CORE_ARCHITECTURE.md       ← Blackboard, Dual-Thread ReAct, NATS, Protobuf
├── 02_LOCAL_RAG_AND_MEMORY.md    ← Jina, Qdrant/sqlite-vec, Tripartite Memory, AST Chunking
├── 03_CONTEXT_AND_TOKEN_OPTIMIZATION.md ← Prefix Caching, MetaGlyph, Diff-based comms, SCIP
└── README.md                     ← This file (navigation)
```

---

## 🏗️ Architecture Pillars

### Pillar 1: Core Architecture (`01_CORE_ARCHITECTURE.md`)

**Covers:**
- Blackboard Architecture (shared state, pub/sub)
- Dual-Thread ReAct Loops (Planning vs. Acting threads)
- Code Influence Graph (dependency-aware retrieval)
- NATS JetStream + Protobuf (binary protocol)
- State Machine Orchestration (deterministic execution)

**Key Components:**
- `crates/openakta-cache/src/blackboard/v2.rs`
- `crates/openakta-indexing/src/influence.rs`
- `crates/openakta-agents/src/worker.rs`

---

### Pillar 2: Local RAG & Memory (`02_LOCAL_RAG_AND_MEMORY.md`)

**Covers:**
- Jina Code Embeddings v2 (137M params, ~550MB RAM)
- Qdrant Embedded / sqlite-vec (local vector stores)
- Tripartite Memory (Semantic, Episodic, Procedural)
- AST-Based Chunking (Tree-sitter)
- Merkle Trees for incremental indexing

**Key Components:**
- `crates/openakta-embeddings/src/jina.rs`
- `crates/openakta-rag/src/vector_store.rs`
- `crates/openakta-indexing/src/chunker.rs`
- `crates/openakta-indexing/src/merkle.rs`

---

### Pillar 3: Context & Token Optimization (`03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`)

**Covers:**
- Prefix Caching (50-90% input savings)
- Diff-Based Communication (89-98% output savings)
- Graph-Based Context Pruning (95-99% context savings)
- MetaGlyph (symbolic operators)
- Q-Codes (abbreviation protocols)
- Cache-to-Cache / Latent Semantic Communication

**Key Components:**
- `crates/openakta-cache/src/prefix_cache.rs`
- `crates/openakta-cache/src/diff.rs`
- `crates/openakta-rag/src/graph_retriever.rs`

---

## 🚀 Strategic Pivot

### What Changed

| Before | After |
|--------|-------|
| Local LLM inference | Cloud APIs (Anthropic, OpenAI) |
| Cloud vector DBs (Turbopuffer) | Local vector stores (Qdrant Embedded) |
| Conversational agent swarms | Deterministic state machines |
| DDD agent teams | Blackboard + Graph workflow |

### What Stayed

- ✅ Token efficiency focus
- ✅ Local-first RAG
- ✅ Multi-agent coordination
- ✅ Business rule traceability

---

## 📚 Deprecated Concepts (Moved to `research/OUTDATED/`)

The following concepts have been **deprecated** and moved to `research/OUTDATED/`:

### Local LLM Inference
- ❌ Ollama
- ❌ llama.cpp
- ❌ vLLM
- ❌ Qwen 2.5 Coder local hosting
- ❌ Llama 3.3 local hosting

### Cloud Vector Databases
- ❌ Turbopuffer
- ❌ Pinecone
- ❌ Weaviate Cloud

### Conversational Agent Swarms
- ❌ AutoGen GroupChat
- ❌ Bag of Agents
- ❌ Unstructured agent negotiations

### Domain-Driven Design
- ❌ DDD Bounded Contexts for agents
- ❌ Anti-Corruption Layers
- ❌ Agent team organization by domain

---

## 🔗 Related Folders

| Folder | Purpose |
|--------|---------|
| `docs/active_architecture/` | ✅ **Current architecture** (Single Source of Truth) |
| `research/findings/` | ✅ **Active research** (Local-First RAG, Multi-Agent Optimization) |
| `research/OUTDATED/` | ❌ **Deprecated research** (moved from active folders) |
| `planning/` | 📋 **Sprint plans** (agent assignments) |

---

## 📊 Implementation Status

| Document | Components | Status | Next Review |
|----------|------------|--------|-------------|
| `01_CORE_ARCHITECTURE.md` | Blackboard, Influence Graph | ✅ 60% implemented | After MVP |
| `02_LOCAL_RAG_AND_MEMORY.md` | Jina, Qdrant, AST Chunking | 🔄 30% implemented | After Sprint B5 |
| `03_CONTEXT_AND_TOKEN_OPTIMIZATION.md` | Prefix Cache, Diff | ✅ 50% implemented | After Sprint C7 |

---

## 🎯 Getting Started

### For New Developers

1. **Read in order:**
   - `01_CORE_ARCHITECTURE.md` (foundation)
   - `02_LOCAL_RAG_AND_MEMORY.md` (RAG pipeline)
   - `03_CONTEXT_AND_TOKEN_OPTIMIZATION.md` (optimizations)

2. **Explore code:**
   - Follow component locations in each document
   - Run examples locally

3. **Contribute:**
   - Propose changes via PR
   - Update document if architecture changes

### For Agents (AI Contributors)

1. **Check your assignment:** `planning/agent-*/current_task.md`
2. **Read relevant architecture doc:** Depends on sprint
3. **Implement:** Follow specifications in docs
4. **Update doc:** If implementation differs from design

---

## ✅ Definition of Organized

- ✅ All active architecture in `docs/active_architecture/`
- ✅ All deprecated concepts in `research/OUTDATED/`
- ✅ All research findings in `research/findings/`
- ✅ Clear navigation (this README)
- ✅ Single Source of Truth (no duplication)

---

**This folder is the Single Source of Truth for OPENAKTA architecture.**

**Last Reviewed:** 2026-03-18  
**Next Review:** After MVP launch
