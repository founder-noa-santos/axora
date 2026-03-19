# Context Compacting Research

**Mission:** Enable long-running multi-agent systems (100+ turns) with 60-80% cost reduction  
**Status:** ✅ Research Complete — Implementation Planning Required  
**Priority:** 🔴 CRITICAL  

---

## 🎯 Goal

Solve the **Context Escalation Crisis** in multi-agent systems:
- Context window saturates within 10-20 turns
- Token costs explode ($0.03-0.12 per 1K tokens)
- Latency increases (O(n²) attention computation)
- Cognitive degradation (accuracy: 98.1% → <64%)

**Target:** 10:1 to 30:1 compression ratios while preserving reasoning quality

---

## 📁 Research Structure

```
context-compacting/
├── README.md                        ← This file (overview)
├── R-15-CONTEXT-COMPACTING.md       ← Full research (10 pages)
└── IMPLEMENTATION-PLAN.md           ← Implementation plan (TBD)
```

---

## 🚀 Quick Start

### For Decision Makers

1. **Read Research:** [`R-15-CONTEXT-COMPACTING.md`](./R-15-CONTEXT-COMPACTING.md)
2. **Key Findings:**
   - CRDT Blackboard (not centralized locking)
   - Diff-Based Event Bus (89-98% savings)
   - Hierarchical Memory Structure (bounded ~5K-8K tokens)
3. **Approve:** Implementation plan (to be created)

### For Implementers

1. **Wait for:** Implementation plan approval
2. **Then read:** `IMPLEMENTATION-PLAN.md` (to be created)
3. **Start:** Phase 1 (CRDT Blackboard foundation)

---

## 📊 Key Findings

### Core Compacting Techniques

| Technique | Compression | Preservation | Complexity |
|-----------|-------------|--------------|------------|
| **Summarization** | 5x-10x | Moderate (semantic drift risk) | Low-Moderate |
| **Memory-Based (Letta)** | Infinite (fixed window) | High (requires retrieval) | High |
| **Structural (TOON)** | 2x-5x | **Perfect (lossless)** | Low |
| **Latent Compilation** | **16x-32x** | **Very High** | Very High |

### Recommended Architecture

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

## 🏗️ AXORA Worker Agent Memory Structure

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

## 📋 Implementation Priorities

### Phase 1: Foundation (Week 1-2)

| Component | Priority | Effort |
|-----------|----------|--------|
| CRDT Blackboard (Yjs) | 🔴 CRITICAL | 3-4 days |
| Diff-Based Event Bus | 🔴 CRITICAL | 2-3 days |
| Hierarchical Memory Structure | 🔴 CRITICAL | 2 days |

### Phase 2: Compaction Engines (Week 2-3)

| Component | Priority | Effort |
|-----------|----------|--------|
| TOON Serializer | 🟡 HIGH | 1-2 days |
| Rolling Summary | 🟡 HIGH | 2 days |
| Semantic Memory (Vector DB) | 🟡 HIGH | 3 days |

### Phase 3: Advanced Optimization (Week 3-4)

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

- **Full Research:** `R-15-CONTEXT-COMPACTING.md` (10 pages)
- **Core Architecture:** `docs/active_architecture/01_CORE_ARCHITECTURE.md`
- **Token Optimization:** `docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`
- **Multi-Agent Optimization:** `../multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md`

---

## ✅ Next Steps

1. **Review:** `R-15-CONTEXT-COMPACTING.md`
2. **Create:** Implementation plan (detailed sprint breakdown)
3. **Assign:** Agent B (Storage/Context Specialist) for Phase 1
4. **Integrate:** With existing Blackboard v2 implementation

---

**Research Status:** ✅ **Complete**  
**Priority:** 🔴 **CRITICAL** (enables long-running agents)  
**Last Updated:** 2026-03-18
