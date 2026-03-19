# Multi-Agent API Cost Optimization

**Mission:** 90-95% reduction in API costs  
**Status:** 📋 Planning Complete — Ready to Start  
**Owners:** Agents B, C (Phase 1-3), Agent A (Phase 4)  
**Priority:** 🔴 CRITICAL  

---

## 🎯 Goal

Reduce multi-agent API costs by **90-95%** while improving latency by **30-50%** through:
1. ✅ Prefix caching (50-90% savings on input tokens)
2. ✅ Diff-only communication (89-98% savings on output tokens)
3. ✅ Graph-based context pruning (95-99% savings on context)
4. ✅ Binary agent protocol (eliminates negotiation tokens)

---

## 📁 Project Structure

```
multi-agent-optimization/
├── README.md                        ← This file (overview)
├── R-17-MULTI-AGENT-OPTIMIZATION.md ← Research findings (the "why")
└── IMPLEMENTATION-PLAN.md           ← Detailed plan (the "how")
```

---

## 🚀 Quick Start

### For Agent C (Phase 1: API Integration)

1. **Read Research:** [`R-17-MULTI-AGENT-OPTIMIZATION.md`](./R-17-MULTI-AGENT-OPTIMIZATION.md)
2. **Read Plan:** [`IMPLEMENTATION-PLAN.md#phase-1`](./IMPLEMENTATION-PLAN.md#phase-1-api-integration-week-1)
3. **Start Sprint C7:** API Client with Prefix Caching
4. **Implement:** Update `crates/axora-agents/src/api_client.rs`

### For Agent B (Phase 2: Graph-Based Context)

1. **Read Research:** [`R-17-MULTI-AGENT-OPTIMIZATION.md`](./R-17-MULTI-AGENT-OPTIMIZATION.md)
2. **Read Plan:** [`IMPLEMENTATION-PLAN.md#phase-2`](./IMPLEMENTATION-PLAN.md#phase-2-graph-based-context-week-2)
3. **Start Sprint B9:** SCIP Indexing (after Local-First RAG complete)
4. **Implement:** Update `crates/axora-indexing/src/scip.rs`

### For Agent A (Phase 4: Validation)

1. **Wait:** For Phase 1-3 completion
2. **Read Plan:** [`IMPLEMENTATION-PLAN.md#phase-4`](./IMPLEMENTATION-PLAN.md#phase-4-validation-documentation-week-4)
3. **Start Sprint A4:** Token Savings Benchmarking
4. **Implement:** Create benchmark suite + documentation

---

## 📊 Implementation Plan

### Phase 1: API Integration (Week 1) — Agent C

| Sprint | Title | Duration | Savings |
|--------|-------|----------|---------|
| **C7** | API Client with Prefix Caching | 2 days | 50-90% input |
| **C8** | Diff-Only Output Enforcement | 1-2 days | 89-98% output |

### Phase 2: Graph-Based Context (Week 2) — Agent B

| Sprint | Title | Duration | Savings |
|--------|-------|----------|---------|
| **B9** | SCIP Indexing | 3-4 days | Dependency tracking |
| **B10** | Context Pruning | 2-3 days | 95-99% context |

### Phase 3: Agent Protocol (Week 3) — Agent C

| Sprint | Title | Duration | Savings |
|--------|-------|----------|---------|
| **C11** | Agent Message Protocol | 2 days | Eliminates negotiation |
| **C12** | Graph Workflow Enforcement | 2 days | Prevents loops |

### Phase 4: Validation & Documentation (Week 4) — Agent A

| Sprint | Title | Duration | Purpose |
|--------|-------|----------|---------|
| **A4** | Token Savings Benchmarking | 2 days | Validate 90-95% savings |
| **A5** | Production Readiness & Docs | 2 days | User guides + migration |

**Total Duration:** 4 weeks (8 sprints)

---

## 📈 Success Metrics

| Metric | Current | Target | Reduction |
|--------|---------|--------|-----------|
| **Input Tokens** | 50,000/session | 2,500/session | 95% |
| **Output Tokens** | 10,000/session | 500/session | 95% |
| **Context Tokens** | 50,000/session | 500-2,500/session | 95-99% |
| **Cost per Session** | $4.80 | $0.39 | 92% |
| **Monthly Cost (100/day)** | $14,400 | $1,170 | 92% |

**Annual Savings:** $158,760/year

---

## 🔗 Related Work

### Existing Components (Already Implemented!)

| Component | Status | Location |
|-----------|--------|----------|
| PrefixCache | ✅ Implemented | `crates/axora-cache/src/prefix_cache.rs` |
| Diff | ✅ Implemented | `crates/axora-cache/src/diff.rs` |
| InfluenceGraph | ✅ Implemented | `crates/axora-indexing/src/influence.rs` |
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` |

**Key Insight:** This is **integration work**, not greenfield development!

### Current Agent Tasks

- **Agent A:** [`planning/agent-a/current_task.md`](../../planning/agent-a/current_task.md)
- **Agent B:** [`planning/agent-b/current_task.md`](../../planning/agent-b/current_task.md)
- **Agent C:** [`planning/agent-c/current_task.md`](../../planning/agent-c/current_task.md)

---

## 📚 Deep Dive

### Research Findings

**Three Critical Challenges:**
1. **Cost & Latency Tax** — Full code sent to API every time
2. **Blind Spot** — LLMs waste 80% of tokens reading directories
3. **Chaotic Orchestration** — Natural language negotiations

**Solutions:**
1. Prefix caching (50-90% savings)
2. Graph-based context pruning (95-99% savings)
3. Binary agent protocol (eliminates negotiations)

**Read:** [`R-17-MULTI-AGENT-OPTIMIZATION.md`](./R-17-MULTI-AGENT-OPTIMIZATION.md)

### Implementation Details

- **Sprint-by-sprint breakdown**
- **Code deliverables**
- **Success criteria**
- **Dependencies**

**Read:** [`IMPLEMENTATION-PLAN.md`](./IMPLEMENTATION-PLAN.md)

---

## 🎯 Four Optimization Pillars

### Pillar 1: Prefix Caching

- Cache static prompts (system instructions, code history)
- 50-90% reduction in input tokens
- Anthropic/OpenAI cache headers

### Pillar 2: Diff-Only Communication

- Agents output unified diffs (not full files)
- 89-98% reduction in output tokens
- Auto-enforcement + conversion

### Pillar 3: Graph-Based Context

- SCIP indexing for dependency tracking
- Context pruning via influence graph
- 95-99% reduction in context tokens

### Pillar 4: Binary Protocol

- Agents communicate via Protobuf/JSON
- No natural language negotiations
- <500 bytes per message (vs 5K+)

---

## ✅ Getting Started

**Agent C should:**

1. Read [`R-17-MULTI-AGENT-OPTIMIZATION.md`](./R-17-MULTI-AGENT-OPTIMIZATION.md)
2. Review [`IMPLEMENTATION-PLAN.md#sprint-c7`](./IMPLEMENTATION-PLAN.md#sprint-c7-api-client-with-prefix-caching)
3. Start Sprint C7 (API Client + Prefix Caching)

**Agent B should:**

1. Complete Local-First RAG first (Sprints B5-B8)
2. Then read [`IMPLEMENTATION-PLAN.md#sprint-b9`](./IMPLEMENTATION-PLAN.md#sprint-b9-scip-indexing-language-agnostic-parsing)
3. Start Sprint B9 (SCIP Indexing)

**Agent A should:**

1. Wait for Phase 1-3 completion
2. Read [`IMPLEMENTATION-PLAN.md#sprint-a4`](./IMPLEMENTATION-PLAN.md#sprint-a4-token-savings-benchmarking)
3. Start Sprint A4 (Benchmarking)

---

## 💰 ROI Calculation

**Current Monthly Cost (100 sessions/day):**
- 160,000 tokens/session × 100 × 30 = 480M tokens/month
- @ $0.03/1K input, $0.12/1K output = **$14,400/month**

**Target Monthly Cost:**
- 13,000 tokens/session × 100 × 30 = 39M tokens/month
- @ $0.03/1K input, $0.12/1K output = **$1,170/month**

**Monthly Savings: $13,230**  
**Annual Savings: $158,760**

---

**Ready to execute!** 🚀

**Last Updated:** 2026-03-18  
**Owners:** Agents A, B, C
