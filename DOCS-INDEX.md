# AXORA Documentation Index

**Last Updated:** 2026-03-18  
**Status:** ✅ Active & Enforced  

---

## 🎯 Quick Navigation

### For Developers

| Document | Purpose | Location |
|----------|---------|----------|
| **Active Architecture** | Single Source of Truth | [`docs/active_architecture/`](./docs/active_architecture/) |
| **Agent Tasks** | Current sprint assignments | [`planning/agent-*/current_task.md`](./planning/) |
| **Status Dashboard** | Visual project status | [`planning/STATUS-DASHBOARD.md`](./planning/STATUS-DASHBOARD.md) |
| **Research Findings** | Active research | [`research/findings/`](./research/findings/) |

---

## 📁 Key Folders

```
axora/
├── docs/active_architecture/     ← 📖 START HERE (Single Source of Truth)
│   ├── 01_CORE_ARCHITECTURE.md
│   ├── 02_LOCAL_RAG_AND_MEMORY.md
│   ├── 03_CONTEXT_AND_TOKEN_OPTIMIZATION.md
│   └── README.md
│
├── planning/
│   ├── MASTER-TASK-LIST.md       ← 📋 ALL tasks (12 sprints)
│   ├── agent-a/current_task.md   ← Agent A: IDLE Weeks 1-3, START Week 4
│   ├── agent-b/current_task.md   ← Agent B: STARTING B5 (Jina)
│   ├── agent-c/current_task.md   ← Agent C: STARTING C7 (API Client)
│   └── STATUS-DASHBOARD.md       ← Visual status
│
├── research/
│   ├── findings/                 ← Active research
│   │   ├── local-first-rag/
│   │   ├── multi-agent-optimization/
│   │   └── cli-vs-mcp/           ← NEW: Architecture decision (CLI vs MCP)
│   └── OUTDATED/                 ← Deprecated (archived)
│
└── crates/                       ← Implementation
    ├── axora-cache/              ← Blackboard, PrefixCache, Diff
    ├── axora-indexing/           ← Influence Graph, AST Chunking
    ├── axora-rag/                ← RAG pipeline
    └── axora-agents/             ← Worker agents, Coordinator
```

---

## 🚀 Getting Started

### Step 1: Read Active Architecture

**Start here:** [`docs/active_architecture/README.md`](./docs/active_architecture/README.md)

**Read in order:**
1. [`01_CORE_ARCHITECTURE.md`](./docs/active_architecture/01_CORE_ARCHITECTURE.md) — Blackboard, Dual-Thread ReAct, Influence Graph
2. [`02_LOCAL_RAG_AND_MEMORY.md`](./docs/active_architecture/02_LOCAL_RAG_AND_MEMORY.md) — Jina, Qdrant Embedded, Tripartite Memory
3. [`03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`](./docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md) — Prefix Caching, Diff, SCIP

### Step 2: Check Your Assignment

**Agents:** Read your current task:
- **Agent A:** [`planning/agent-a/current_task.md`](./planning/agent-a/current_task.md)
- **Agent B:** [`planning/agent-b/current_task.md`](./planning/agent-b/current_task.md)
- **Agent C:** [`planning/agent-c/current_task.md`](./planning/agent-c/current_task.md)

### Step 3: Review Research (Optional)

**Active Research:**
- **Local-First RAG:** [`research/findings/local-first-rag/`](./research/findings/local-first-rag/)
- **Multi-Agent Optimization:** [`research/findings/multi-agent-optimization/`](./research/findings/multi-agent-optimization/)

---

## 📊 Current Status

### Active Missions

| Mission | Owner | Status | Progress |
|---------|-------|--------|----------|
| **Local-First RAG** | Agent B | 🔄 In Progress | Sprint B5 (Jina Embeddings) |
| **Multi-Agent Optimization** | Agents B, C, A | 🔄 In Progress | Sprint C7 (API Client) |

### Completed Work

| Component | Status | Location |
|-----------|--------|----------|
| Theme System | ✅ Complete | `apps/desktop/` |
| Blackboard v2 | ✅ Complete | `crates/axora-cache/src/blackboard/v2.rs` |
| PrefixCache | ✅ Complete | `crates/axora-cache/src/prefix_cache.rs` |
| Diff | ✅ Complete | `crates/axora-cache/src/diff.rs` |
| InfluenceGraph | ✅ Complete | `crates/axora-indexing/src/influence.rs` |

---

## 🏗️ Architecture Pillars

### Pillar 1: Core Architecture

**Blackboard + Dual-Thread ReAct + Influence Graph**

- Agents communicate via Blackboard (not direct messages)
- Planning and Acting run in parallel threads
- Context retrieval via influence graph (not brute force)

**Read:** [`docs/active_architecture/01_CORE_ARCHITECTURE.md`](./docs/active_architecture/01_CORE_ARCHITECTURE.md)

### Pillar 2: Local RAG & Memory

**Jina Code v2 + Qdrant Embedded + Tripartite Memory**

- 100% local indexing (no cloud vector DBs)
- Lightweight embeddings (137M params, ~550MB RAM)
- Three memory types: Semantic, Episodic, Procedural

**Read:** [`docs/active_architecture/02_LOCAL_RAG_AND_MEMORY.md`](./docs/active_architecture/02_LOCAL_RAG_AND_MEMORY.md)

### Pillar 3: Context & Token Optimization

**Prefix Caching + Diff Communication + SCIP**

- 90-95% token cost reduction
- Prefix caching (50-90% input savings)
- Diff-only communication (89-98% output savings)
- Graph-based context pruning (95-99% context savings)

**Read:** [`docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`](./docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md)

---

## 🚨 Deprecated Concepts (Archived)

The following concepts have been **deprecated** and moved to [`research/OUTDATED/`](./research/OUTDATED/):

### Local LLM Inference
- ❌ Ollama
- ❌ llama.cpp
- ❌ vLLM
- ❌ Local Qwen/Llama hosting

### Cloud Vector Databases
- ❌ Turbopuffer
- ❌ Pinecone
- ❌ Weaviate Cloud

### Conversational Agent Swarms
- ❌ AutoGen GroupChat
- ❌ Bag of Agents
- ❌ Unstructured negotiations

### Domain-Driven Design
- ❌ DDD Bounded Contexts for agents
- ❌ Anti-Corruption Layers
- ❌ Agent teams by domain

---

## 📞 Contact & Support

**Questions?**
- Check [`docs/active_architecture/README.md`](./docs/active_architecture/README.md)
- Review your agent task: `planning/agent-*/current_task.md`
- Consult architecture docs: `docs/active_architecture/`

**Need to propose changes?**
- Create PR with updated architecture doc
- Include rationale and trade-offs
- Get Architect Agent approval

---

## ✅ Definition of Organized

- ✅ Single Source of Truth (`docs/active_architecture/`)
- ✅ Deprecated concepts archived (`research/OUTDATED/`)
- ✅ Clear navigation (this file)
- ✅ Agent tasks up-to-date (`planning/agent-*/current_task.md`)

---

**Welcome to AXORA! Start with `docs/active_architecture/README.md`.** 🚀

**Last Reviewed:** 2026-03-18  
**Maintained By:** Architect Agent
