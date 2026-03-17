# Phase 2 PIVOT: DDD Agents REJECTED — Graph-Based Workflow ADOPTED

**Date:** 2026-03-16  
**Source:** R-10 Research Findings (Skeptical Validation of DDD Agents)  
**Impact:** MAJOR PIVOT — DDD Agents DEFERRED, Graph-Based Workflow ADOPTED  

---

## 🚨 Executive Summary

**DDD Agents Status:** ❌ **REJECTED** for AXORA's target audience (individual developers)

**Key Findings:**
1. **DDD is enterprise over-engineering** — solves problems individual devs don't have
2. **Coordination overhead grows quadratically** — N(N-1)/2 communication paths
3. **"Expertise accumulation" is anthropomorphism** — it's just RAG, not team learning
4. **Cross-domain routing is a bottleneck** — 20-40% token overhead for translation
5. **Flat + Deterministic + RAG outperforms DDD** for 95% of individual dev use cases

**New Architecture:** **Graph-Based Deterministic Workflow** (LangGraph-style)

---

## 📊 Research Validation Summary

### What the Research Proved

| Claim | Evidence | Verdict |
|-------|----------|---------|
| DDD yields higher quality | ✅ True for ENTERPRISE, ❌ False for individual devs | **Contextual** |
| Agents accumulate expertise | ❌ Anthropomorphism — it's just RAG | **FALSE** |
| DDD scales better | ✅ For 20+ agents, ❌ For 5-10 agents | **Contextual** |
| Coordination overhead is manageable | ❌ Quadratic growth, 3-15x token inflation | **TRUE (negative)** |
| Cross-domain routing works | ❌ 20-40% overhead, high failure rate | **TRUE (negative)** |

### Critical Insights

**1. Specialist vs Generalist Performance:**
- ✅ Specialists win on **parallelizable** tasks (80.9% improvement)
- ❌ Specialists LOSE on **sequential** tasks (39-70% degradation)
- **Individual dev work is mostly sequential** → Generalists win

**2. Anthropomorphism Fallacy:**
> "Agents do not learn interactively like human engineers; their efficacy relies entirely on RAG architectures and state externalization."

**Translation:** "Domain expertise" = Better retrieval, not team structure

**3. Coordination Overhead Mathematics:**
```
N agents → N(N-1)/2 communication paths

3 agents → 3 paths
5 agents → 10 paths
10 agents → 45 paths  ← Token inflation explodes
```

**4. Industry Reality Check:**
- **AutoGen:** Flat specialization (avoids DDD)
- **CrewAI:** Role-based, NOT domain-based (avoids DDD)
- **LangGraph:** State machine, NO team metaphor (avoids DDD)

**Why?** DDD is too complex for most use cases.

---

## 🔄 Architecture Pivot

### OLD Plan (REJECTED)
```
User Request → DDD Orchestrator → Domain Teams → Anti-Corruption Layers → Merge
                 ↓
            High latency, high token cost, complex routing
```

### NEW Plan (ADOPTED)
```
User Request → Deterministic Graph → Generalist Agents + Domain RAG → Output
                 ↓
            Low latency, low token cost, simple routing
```

---

## ✅ What Changes

### 1. Agent Organization

**OLD (DDD):**
```
Auth Domain Team:
  - Auth Coder
  - Auth Reviewer
  - Auth Tester

Billing Domain Team:
  - Billing Coder
  - Billing Reviewer
  - Billing Tester
```

**NEW (Graph-Based):**
```
Deterministic Workflow:
  Node 1: Planner (generalist)
  Node 2: Executor (generalist)
  Node 3: Reviewer (generalist)
  
Domain Knowledge:
  - Auth Vector Store (RAG)
  - Billing Vector Store (RAG)
  - Dynamically retrieved when needed
```

**Key Difference:** Agents are NOT domain-specialized. Domain knowledge is in RAG, not agent structure.

---

### 2. Expertise Accumulation

**OLD (Anthropomorphic):**
> "Auth team will accumulate expertise over time through collaboration"

**NEW (Engineering Reality):**
> "Generalist agent + Auth RAG retrieves past successful patterns"

**Implementation:**
```rust
// OLD: Team-based memory (complex, anthropomorphic)
auth_team.memory.add_experience(task, result);

// NEW: RAG-based retrieval (simple, proven)
let auth_patterns = rag.retrieve("auth", query).await?;
let past_successes = rag.retrieve("past_wins", query).await?;
```

---

### 3. Cross-Domain Tasks

**OLD (DDD with Anti-Corruption Layers):**
```
User: "Add paid OAuth tier"

Orchestrator:
  1. Parse intent
  2. Split: Auth Team + Billing Team
  3. Send to Auth Team (via ACL translation)
  4. Send to Billing Team (via ACL translation)
  5. Merge outputs (detect conflicts)
  6. Resolve conflicts (recursive handoffs)

Token Overhead: 40%+ for translation + merging
```

**NEW (Graph + RAG):**
```
User: "Add paid OAuth tier"

Graph Workflow:
  1. Planner: Decompose task
  2. Executor: Retrieve auth RAG + billing RAG
  3. Executor: Write code (full repo access)
  4. Reviewer: Validate

Token Overhead: <10% (just RAG retrieval)
```

---

## 📋 Immediate Actions

### This Week (High Priority)

**Agent A (Documentation):**
- [ ] Update `DDD-TDD-AGENT-TEAMS.md` with REJECTED status
- [ ] Create `GRAPH-WORKFLOW-DESIGN.md` (new architecture)
- [ ] Document RAG-based "expertise" (not team-based)

**Agent B (Implementation):**
- [ ] Update Sprint 8 (Context) to use RAG-based domain retrieval
- [ ] Remove DDD-specific code (bounded contexts, ACLs)
- [ ] Add LangGraph-style state machine primitives

**Agent C (Implementation):**
- [ ] Pivot Sprint 7 (Decomposition) to graph-based, not domain-based
- [ ] Update decomposer for sequential workflows (not parallel domains)
- [ ] Add deterministic routing (not semantic routing)

---

### Next Week (Medium Priority)

**New Sprint: Graph Workflow Engine**
- [ ] Create `crates/axora-graph/` (new crate)
- [ ] Implement state machine primitives
- [ ] Implement deterministic routing
- [ ] Integrate with RAG for domain knowledge

**New Sprint: Domain RAG Tools**
- [ ] Create domain-specific vector stores (auth, billing, etc.)
- [ ] Implement late-interaction retrieval (ColBERT-style)
- [ ] Add "Experience-as-Parameters" memory bank

---

## 🎯 Success Metrics (NEW Architecture)

| Metric | DDD (OLD) | Graph + RAG (NEW) | Target |
|--------|-----------|-------------------|--------|
| Coordination Overhead | O(N²) | O(N) | O(N) ✅ |
| Token Efficiency | 60% overhead | <10% overhead | <10% ✅ |
| Cross-Domain Latency | 20-40% penalty | <5% penalty | <5% ✅ |
| Implementation Complexity | 120+ hours | 40 hours | 40h ✅ |
| Individual Dev Fit | ❌ Poor | ✅ Excellent | ✅ ✅ |

---

## 📊 Decision Matrix (Re-Evaluated)

| Criteria | DDD Evaluation | Graph + RAG Evaluation |
|----------|----------------|------------------------|
| Evidence Strength | ⚠️ Mixed (enterprise only) | ✅ Strong (all contexts) |
| User Value | ❌ Solution in search of problem | ✅ Solves real pain point |
| Implementation Cost | ❌ >120 hours + high complexity | ✅ ~40 hours + medium complexity |
| Alignment (Individual Dev) | ❌ Misaligned | ✅ Perfectly aligned |
| **Final Verdict** | ❌ **REJECT** | ✅ **ADOPT** |

---

## 🔗 Updated Research Links

- [R-10: DDD Agents Validation](../research/prompts/10-ddd-agents-validation.md) — **FINDINGS: REJECT**
- [R-11: Concurrency + ReAct](../research/prompts/11-concurrency-react-loops.md) — **Still valid (graph-based)**
- [R-13: Influence Graph](../research/prompts/13-influence-graph-business-rules.md) — **Still valid (RAG-based)**
- [R-14: Memory Architecture](../research/prompts/14-memory-architecture.md) — **Pivot to Experience-as-Parameters**

---

## ✅ Validation Criteria

Pivot is successful when:
- [ ] DDD Agents formally REJECTED in documentation
- [ ] Graph-Based Workflow design complete
- [ ] Domain RAG tools implemented (not Domain Agents)
- [ ] Coordination overhead is O(N), not O(N²)
- [ ] Token overhead <10% (not 40%+)
- [ ] Implementation time <40 hours (not 120+)

---

**This pivot FUNDAMENTALLY CHANGES AXORA's architecture from enterprise DDD to individual-dev-friendly Graph + RAG.**

**All agents must read this and update their current sprints accordingly.**
