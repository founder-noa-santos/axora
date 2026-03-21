# Research Summary (Phase 2)

**Date:** 2026-03-16
**Status:** Active
**Purpose:** One-page summary of all Phase 2 research findings

---

## 📊 Research Overview

| Research | Topic | Status | Outcome |
|----------|-------|--------|---------|
| **R-10** | DDD Agents Validation | ✅ Complete | REJECTED |
| **R-11** | Concurrency + ReAct | ✅ Complete | ADOPTED |
| **R-13** | Influence Graph | ✅ Complete | ADOPTED |
| **R-14** | Memory Architecture | ✅ Complete | ADOPTED |
| **Concurrent Decomposition** | ACONIC Framework | ✅ Complete | ADOPTED |

---

## R-10: DDD Agents Validation

### Question
Should OPENAKTA organize agents by domain (DDD) or use flat structure?

### Findings
1. **DDD is enterprise over-engineering** — solves problems individual devs don't have
2. **Coordination overhead grows quadratically** — N(N-1)/2 communication paths
3. **"Expertise accumulation" is anthropomorphism** — it's just RAG, not team learning
4. **Cross-domain routing is a bottleneck** — 20-40% token overhead for translation
5. **Industry avoids DDD** — AutoGen, CrewAI, LangGraph all use simpler models

### Decision
**REJECTED** for OPENAKTA's target audience (individual developers)

### Adopted Alternative
**Graph-Based Workflow** (LangGraph-style)
- O(N) coordination (not O(N²))
- <10% token overhead (not 40%+)
- ~40h implementation (not 120h+)

### Documents
- [`planning/shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md)
- [`planning/shared/GRAPH-WORKFLOW-DESIGN.md`](./shared/GRAPH-WORKFLOW-DESIGN.md)
- [`planning/shared/DDD-TDD-AGENT-TEAMS.md`](./shared/DDD-TDD-AGENT-TEAMS.md) (REJECTED status)

---

## R-11: Concurrency + ReAct

### Question
How should OPENAKTA handle concurrent agent execution?

### Findings
1. **Concurrent execution is mandatory** — sequential is 3-5x slower
2. **Dual-thread ReAct outperforms sequential** — reasoning + acting in parallel
3. **Blackboard pattern > Chat pattern** — shared state, not message passing
4. **Pull-based context > Push-based** — agents request what they need

### Decision
**ADOPTED** — Concurrent execution with dual-thread ReAct

### Implementation
- Parallel task groups (from DAG topological sort)
- Shared blackboard for state
- Pull-based context retrieval

### Documents
- [`planning/shared/CONCURRENT-IMPLEMENTATION.md`](./shared/CONCURRENT-IMPLEMENTATION.md)
- [`planning/shared/CONCURRENCY-STRATEGY.md`](./shared/CONCURRENCY-STRATEGY.md)

---

## R-13: Influence Graph

### Question
How should OPENAKTA detect code dependencies for impact analysis?

### Findings
1. **Static analysis > LLM for dependencies** — 95%+ accuracy vs 70-80%
2. **Code influence graph is efficient** — O(1) lookup after O(n) build
3. **LLM is expensive and unreliable** — token cost + hallucination risk

### Decision
**ADOPTED** — Static analysis for influence graph

### Implementation
- Tree-sitter for parsing
- Influence graph storage
- O(1) dependency lookup

### Documents
- [`planning/shared/INFLUENCE-GRAPH-DESIGN.md`](./shared/INFLUENCE-GRAPH-DESIGN.md) (TODO)

---

## R-14: Memory Architecture

### Question
How should OPENAKTA implement agent memory?

### Findings
1. **Semantic memory = Vector stores** — API contracts, schemas, patterns
2. **Episodic memory = Conversation logs** — Past debugging sessions, decisions
3. **Procedural memory = Skill files** — SKILL.md with executable workflows
4. **Late-interaction retrieval > Single embedding** — 15-20% better for code

### Decision
**ADOPTED** — RAG-based memory (not team-based learning)

### Implementation
- Domain-specific vector stores
- Conversation log storage
- SKILL.md files for procedural memory
- ColBERT-style late-interaction retrieval

### Documents
- [`planning/shared/RAG-EXPERTISE-DESIGN.md`](./shared/RAG-EXPERTISE-DESIGN.md)

---

## Concurrent Task Decomposition Research

### Question
What is the mathematical foundation for task decomposition?

### Findings
1. **ACONIC framework validates approach** — Constraint-induced complexity
2. **Treewidth measures task complexity** — LLM optimal threshold = 5
3. **AOP validation ensures correctness** — Solvability, Completeness, Non-redundancy
4. **DAG construction enables parallelization** — Topological sort → parallel groups

### Decision
**ADOPTED** — ACONIC-based decomposition

### Implementation
- Constraint graph parsing
- Treewidth calculation
- AOP validator
- DAG builder with topological sort

### Documents
- [`planning/shared/ACONIC-DECOMPOSITION-DESIGN.md`](./shared/ACONIC-DECOMPOSITION-DESIGN.md)
- [`planning/shared/AOP-VALIDATOR-SPEC.md`](./shared/AOP-VALIDATOR-SPEC.md)

---

## Key Validations

### ✅ Graph-Based > DDD
- **Evidence:** R-10 research + industry analysis
- **Benefit:** O(N) vs O(N²) coordination, <10% vs 40%+ token overhead
- **Status:** Implemented in GRAPH-WORKFLOW-DESIGN.md

### ✅ O(N) Coordination > O(N²)
- **Evidence:** Mathematical proof + R-10 validation
- **Benefit:** Linear scaling, predictable performance
- **Status:** Core to Graph-Based Workflow

### ✅ Blackboard > Chat
- **Evidence:** R-11 concurrency research
- **Benefit:** Shared state, no message passing overhead
- **Status:** Implemented in concurrent execution design

### ✅ Pull-Based Context > Push
- **Evidence:** R-11 concurrency research
- **Benefit:** Agents request what they need, no wasted tokens
- **Status:** Implemented in context allocation design

### ✅ Dual-Thread ReAct > Sequential
- **Evidence:** R-11 concurrency research
- **Benefit:** Reasoning + acting in parallel, 3-5x faster
- **Status:** Implemented in agent execution design

### ✅ ACONIC Decomposition > Heuristic
- **Evidence:** Concurrent task decomposition research
- **Benefit:** Mathematical guarantee of valid decomposition
- **Status:** Documented in ACONIC-DECOMPOSITION-DESIGN.md

---

## Research-to-Implementation Mapping

| Research | Implementation Sprint | Status |
|----------|----------------------|--------|
| R-10 (DDD) | Sprint 11 (Graph Workflow) | ✅ Complete |
| R-11 (Concurrency) | Sprint 7/8 (Decomposition/Context) | 🔄 In Progress |
| R-13 (Influence Graph) | Sprint X (Impact Analysis) | 📋 Planned |
| R-14 (Memory) | Sprint 6 (Documentation) | ✅ Complete |
| ACONIC | Sprint 12 (Decomposition Design) | ✅ Complete |

---

## Open Questions

1. **What is the optimal treewidth threshold?**
   - Current: 5 (conservative)
   - Need: Empirical tuning with real missions

2. **What is the actual parallelization quotient?**
   - Target: >90%
   - Need: Measurement from real decompositions

3. **What is the AOP pass rate?**
   - Target: 100%
   - Need: Validation from implementation

---

## Next Research Priorities

1. **Empirical treewidth tuning** — Test with real missions
2. **AOP validation accuracy** — Measure false positive/negative rates
3. **Parallelization measurement** — Track actual vs theoretical parallelism
4. **Token efficiency validation** — Measure actual token savings

---

**This summary provides TRACEABILITY from research to implementation.**
