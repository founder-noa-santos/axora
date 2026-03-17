# AXORA Architecture Ledger

**Last Updated:** 2026-03-17
**Maintained By:** Architect Agent
**Status:** Active — Living Document (auto-updated on sprint completion)

---

## 📋 Overview

This is the **AXORA Architecture Ledger** — a living document that tracks:
- Current agent assignments and status
- Active constraints (token budgets, concurrency limits)
- Execution graph (dependencies between sprints)
- Recent architectural changes
- Sprint history with metrics
- Performance metrics

**Purpose:** Provide architectural visibility and constraint enforcement.

---

## 👥 Current Agent Assignments

### Coordinator Agent
- **Role:** Decompose missions, dispatch tasks, validate outputs
- **State:** `planning/README.md` (structure + current assignments)
- **Constraints:**
  - Max 10 concurrent tasks
  - 50K token budget per day
  - Rate limit: 100 requests/minute

### Worker Agents

| Agent | Role | Current Sprint | Status | Focus Area |
|-------|------|----------------|--------|------------|
| **Agent A** | Documentation Specialist | — | ✅ **READY** | Available for New Mission |
| **Agent B** | Storage + Context Specialist | — | ✅ **READY** | Available for New Mission |
| **Agent C** | Implementation Specialist | — | ✅ **READY** | Available for New Mission |

---

## 🔒 Active Constraints

### Token Budget

| Level | Limit | Enforcement | Status |
|-------|-------|-------------|--------|
| **Per-Task** | 2,500 tokens | Hard stop (circuit breaker) | ✅ Enforced |
| **Per-Agent (Daily)** | 50,000 tokens | Hard stop + alert | ✅ Enforced |
| **Per-Session** | 100,000 tokens | Warning at 80% | ✅ Enforced |

**Enforcement Code:**
```rust
// Enforced by ContextManager
if context.estimate_tokens() > max_tokens {
    return Err(Error::TokenBudgetExceeded);
}
```

### Concurrency Limits

| Limit | Value | Enforcement | Status |
|-------|-------|-------------|--------|
| **Max Parallel Tasks** | 10 | Sliding-window semaphore | ✅ Enforced |
| **Rate Limit** | 100 req/min | Token bucket | ✅ Enforced |
| **Max Context Size** | 8,000 tokens | Hard limit | ✅ Enforced |

**Enforcement Code:**
```rust
// Enforced by ConcurrentExecutor
let permit = semaphore.acquire_owned().await?;
// Task executes...
drop(permit); // Release
```

### Duplicate Execution Prevention

| Mechanism | Pattern | Status |
|-----------|---------|--------|
| **Atomic Checkout** | Paperclip (FOR UPDATE SKIP LOCKED) | ✅ Implemented |
| **Single Assignee** | TaskQueue enforces | ✅ Implemented |
| **Idempotency Keys** | Per-task unique key | ✅ Implemented |

---

## 🔗 Execution Graph

```
Phase 2 Sprints (Token Optimization + Graph Workflow)
═══════════════════════════════════════════════════════

Agent A (Documentation):
  Sprint 3  ─┬─→ Sprint 6  ─→ Sprint 9  ─→ Sprint 11 ─→ Sprint 12 ─→ Sprint 18 ─→ Sprint 25
  (Minify)   │   (Docs)       (Benchmark)  (Pivot)     (ACONIC)    (Biz Rules)  (Ledger)
             │
Agent B (Storage/Context):   │
  Sprint 5  ─┴─→ Sprint 8  ─→ Sprint 10 ─→ Sprint 11 ─→ Sprint 12 ─→ Sprint 16 ─→ Sprint 17
  (TOON)       (Context)      (RAG)        (Graph)     (Blackboard) (SCIP)      (Influence)
                                                                  ─→ Sprint 20 ─→ Sprint 21 ─→ Sprint 22 ─→ Sprint 24
                                                                    (Pruning)   (Semaphore)  (Checkout)   (Repo Map)

Agent C (Implementation):
  Sprint 3B ─→ Sprint 7  ─→ Sprint 8  ─→ Sprint 9  ─→ Sprint 19 ─→ Sprint 23
  (Heartbeat)  (Decomp)     (Graph)      (ReAct)     (Traceability) (ACI)

Dependencies:
  Sprint 9 (A) requires: Sprint 3 (A), Sprint 6 (A)
  Sprint 11 (A) requires: R-10 validation
  Sprint 12 (A) requires: Sprint 11 (A)
  Sprint 18 (A) requires: Sprint 12 (A)
  Sprint 24 (B) requires: Sprint 22 (B)
  Sprint 25 (A) requires: Sprint 18 (A)
```

---

## 📝 Recent Changes

### 2026-03-17
- ✅ **ALL PHASES 3 & 4 COMPLETE** — 100% done, all agents ready for next mission
- ✅ **Sprint A3 Complete** — Progress Monitoring (Rust backend: ProgressTracker, BlockerDetector, StatusReporter, 18 tests)
- ✅ **Sprint A5 Complete** — Progress Dashboard (TypeScript frontend: ProgressPanel, WebSocket, 42 tests)
- ✅ **Full-Stack Progress System** — Backend (Rust) + Frontend (TypeScript) ready for integration
- ✅ **Phase 3 Complete (Agent A)** — All Agent A Phase 3 sprints done (A1, A2, A3)
- ✅ **Phase 4 UI Complete (Agent A)** — All Agent A Phase 4 sprints done (A4, A5)
- ✅ **Sprint C6 Complete** — Phase 4 Integration (E2E tests, performance opt, release builds)
- ✅ **Sprint C3 Complete** — Result Merging with conflict detection + auto-resolution (16 tests)
- ✅ **Sprint A2 Complete** — Blackboard v2 with versioned context + subscribe/notify (12 tests)
- ✅ **Sprint B2 Complete** — Task Queue Management (priority, DAG, load balancing, 15 tests)

### 2026-03-16
- ✅ **Adopted Repository Map** (Aider pattern) — 90%+ token reduction for file discovery
- ✅ **Adopted ACI Formatting** (SWE-Agent pattern) — Standardized code blocks
- ✅ **Adopted Atomic Checkout** (Paperclip pattern) — Prevents duplicate execution
- ✅ **Adopted Sliding-Window Semaphores** (Dify pattern) — Resource throttling
- ✅ **Created AGENTS.md Ledger** (Industry standard) — Architectural visibility

### 2026-03-15
- ✅ **Graph-Based Workflow Pivot** (R-10 validation) — DDD rejected, Graph adopted
- ✅ **Implemented Dual-Thread ReAct Loops** — Reasoning + acting in parallel
- ✅ **Implemented Snapshot Blackboard** — TOCTOU prevention

### 2026-03-14
- ✅ **Implemented Context Pruning** — 95-99% token reduction
- ✅ **Implemented Influence Vector** — Code dependency tracking

---

## 📊 Sprint History

### Agent A (Documentation Specialist)

#### Phase 3 Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| **A1** | Context Compacting | ✅ Complete | 2026-03-17 | 60-80% |
| **A2** | Blackboard v2 | ✅ Complete | 2026-03-17 | N/A |
| **A3** | **Progress Monitoring** | ✅ **Complete** | **2026-03-17** | N/A |

#### Phase 4 Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| **A4** | UI Components | ✅ Complete | 2026-03-17 | N/A |
| **A5** | **Progress Dashboard** | ✅ **Complete** | **2026-03-17** | N/A |

#### Legacy Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| 3 | Code Minification | ✅ Complete | 2026-03-14 | 62.7% |
| 6 | Documentation Management | ✅ Complete | 2026-03-14 | N/A |
| 9 | Integration & Benchmarking | ✅ Complete | 2026-03-14 | Validated 88.8% |
| 11 | Graph Workflow Design | ✅ Complete | 2026-03-15 | N/A |
| 12 | ACONIC Decomposition Docs | ✅ Complete | 2026-03-15 | N/A |
| 18 | Business Rule Documentation | ✅ Complete | 2026-03-16 | N/A |
| 25 | AGENTS.md Living Document | ✅ Complete | 2026-03-16 | N/A |

### Agent B (Storage + Context Specialist)

| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| 5 | TOON Serialization | ✅ Complete | 2026-03-14 | 50-60% |
| 8 | Context Distribution | ✅ Complete | 2026-03-14 | N/A |
| 10 | RAG Integration | ✅ Complete | 2026-03-14 | N/A |
| 11 | Graph Workflow Implementation | ✅ Complete | 2026-03-15 | N/A |
| 12 | Snapshot Blackboard | ✅ Complete | 2026-03-15 | N/A |
| 16 | SCIP Indexing | ✅ Complete | 2026-03-15 | N/A |
| 17 | Influence Vector | ✅ Complete | 2026-03-15 | N/A |
| 20 | Context Pruning | ✅ Complete | 2026-03-16 | 95-99% |
| 21 | Sliding-Window Semaphores | ✅ Complete | 2026-03-16 | N/A |
| 22 | Atomic Checkout | ✅ Complete | 2026-03-16 | N/A |
| 24 | Repository Map | ✅ Complete | 2026-03-16 | 90%+ |

### Agent C (Implementation Specialist)

#### Phase 3 Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| C1 | Coordinator Core | ✅ Complete | 2026-03-17 | N/A |
| C2 | Task Decomposition | ✅ Complete | 2026-03-17 | N/A |
| C3 | Result Merging | ✅ Complete | 2026-03-17 | N/A |

#### Phase 4 Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| C4 | Tauri v2 Setup | ✅ Complete | 2026-03-17 | N/A |
| C5 | Chat Interface | ✅ Complete | 2026-03-17 | N/A |
| C6 | Integration + Polish | ✅ Complete | 2026-03-17 | N/A |

#### Legacy Sprints
| Sprint | Title | Status | Date | Token Reduction |
|--------|-------|--------|------|-----------------|
| 3B | Heartbeat System | ✅ Complete | 2026-03-14 | N/A |
| 7 | ACONIC Implementation | ✅ Complete | 2026-03-15 | N/A |
| 8 | Graph Workflow | ✅ Complete | 2026-03-15 | N/A |
| 9 | Dual-Thread ReAct | ✅ Complete | 2026-03-15 | N/A |
| 19 | Bidirectional Traceability | ✅ Complete | 2026-03-16 | N/A |
| 23 | ACI Formatting | ✅ Complete | 2026-03-16 | N/A |

---

## 📈 Performance Metrics

| Metric | Target | Current | Status | Measured In |
|--------|--------|---------|--------|-------------|
| **Token Reduction** | 90%+ | 95-99% | ✅ Exceeded | Sprint 9, 20, 24 |
| **Build Performance** | <1s | 342ms | ✅ Exceeded | Sprint C6 (Vite 8) |
| **E2E Test Coverage** | 10+ tests | 15+ tests | ✅ Exceeded | Sprint C6 |
| **Integration Tests** | 10+ tests | 76+ tests | ✅ Exceeded | Sprint A3, A5, C3, C6 |
| **Progress Monitoring** | 10+ tests | 18 tests | ✅ Exceeded | Sprint A3 |
| **Progress Dashboard** | 10+ tests | 42 tests | ✅ Exceeded | Sprint A5 |
| **Concurrency Speedup** | 3-5x | TBD | 🔄 Pending | Sprint 21 |
| **Context Allocation** | <10ms | TBD | 🔄 Pending | Sprint 20 |
| **Race Conditions** | 0 | 0 | ✅ Pass | Sprint 21, 22 |
| **Duplicate Execution** | 0% | 0% | ✅ Pass | Sprint 22 |
| **Code Minification** | ≥20% | 62.7% | ✅ Exceeded | Sprint 3 |
| **Prefix Caching** | 50-90% | TBD | 🔄 Pending | Sprint 1 |
| **Diff Communication** | 89-98% | TBD | 🔄 Pending | Sprint 2 |

---

## 🏛️ Architecture Decision Records (ADRs)

| ADR | Title | Status | Date | Sprint |
|-----|-------|--------|------|--------|
| **ADR-042** | Graph-Based Workflow | ✅ Active & Enforced | 2026-03-15 | 11 |
| **ADR-043** | Sliding-Window Semaphores | ✅ Active & Enforced | 2026-03-16 | 21 |
| **ADR-044** | Atomic Checkout | ✅ Active & Enforced | 2026-03-16 | 22 |
| **ADR-045** | Repository Map | ✅ Active & Enforced | 2026-03-16 | 24 |
| **ADR-046** | AGENTS.md Ledger | ✅ Active & Enforced | 2026-03-16 | 25 |

**See:** [`docs/ARCHITECTURE-LEDGER.md`](./docs/ARCHITECTURE-LEDGER.md) for detailed ADRs.

---

## 🔧 Pattern Adoptions

| Pattern | Source | Status | Sprint | Benefit |
|---------|--------|--------|--------|---------|
| **Playwright E2E Testing** | Microsoft | ✅ Implemented | C6 | Cross-browser E2E tests |
| **Code Splitting** | Rollup | ✅ Implemented | C6 | Better caching, faster loads |
| **Release Build Automation** | Tauri | ✅ Implemented | C6 | Cross-platform builds |
| **Sliding-Window Semaphores** | Dify | ✅ Implemented | 21 | Resource throttling |
| **Atomic Checkout** | Paperclip | ✅ Implemented | 22 | Duplicate prevention |
| **ACI Formatting** | SWE-Agent | ✅ Implemented | 23 | Standardized code blocks |
| **Repository Map** | Aider | ✅ Implemented | 24 | 90%+ token reduction |
| **AGENTS.md Ledger** | Industry Standard | ✅ Implemented | 25 | Architectural visibility |

---

## 📚 Knowledge Navigation

| Document | Location | Purpose |
|----------|----------|---------|
| **Architecture Ledger (Detailed)** | [`docs/ARCHITECTURE-LEDGER.md`](./docs/ARCHITECTURE-LEDGER.md) | Detailed ADRs, constraints |
| **Graph Workflow Design** | [`planning/shared/GRAPH-WORKFLOW-DESIGN.md`](./planning/shared/GRAPH-WORKFLOW-DESIGN.md) | Graph architecture |
| **ACONIC Decomposition** | [`planning/shared/ACONIC-DECOMPOSITION-DESIGN.md`](./planning/shared/ACONIC-DECOMPOSITION-DESIGN.md) | Task decomposition |
| **RAG Expertise Design** | [`planning/shared/RAG-EXPERTISE-DESIGN.md`](./planning/shared/RAG-EXPERTISE-DESIGN.md) | RAG-based expertise |
| **Business Rules** | [`docs/business_rules/`](./docs/business_rules/) | Business rule documentation |
| **Research Summary** | [`planning/shared/RESEARCH-SUMMARY.md`](./planning/shared/RESEARCH-SUMMARY.md) | Research findings |

---

## 🔄 Auto-Update Mechanism

This ledger is **auto-updated** on sprint completion:

```bash
# Update ledger after sprint completion
./scripts/update-agents-ledger.sh
```

**What gets updated:**
- Sprint status (In Progress → Complete)
- Sprint history table
- Recent changes section
- Performance metrics (if measured)

**What stays manual:**
- Architecture decisions (require ADR)
- Constraint changes (require approval)
- Pattern adoptions (require documentation)

---

## ✅ Validation

Run validation to ensure ledger consistency:

```bash
# Validate ledger format
./scripts/validate-ledger.sh

# Expected output:
# ✓ All sprints accounted for
# ✓ Dependencies valid
# ✓ Metrics consistent
```

---

**This ledger provides ARCHITECTURAL VISIBILITY for all AXORA agents.**

**Last Automated Update:** 2026-03-16  
**Next Scheduled Review:** 2026-03-17
| 11 | A | Architecture Documentation Pivot | ✅ Complete | N/A |
| 18 | A | Business Rule Documentation | ✅ Complete | N/A |
| 3 | A | Code Minification | ✅ Complete | N/A |
| 6 | A | Documentation Management System | ✅ Complete | N/A |
| 9 | A | Phase 2 Integration & Benchmarking | ✅ Complete | N/A |
| 10 | B | Phase 2 Consolidation & Documentation | ✅ Complete | N/A |
| 11 | B | Context Distribution Pivot | ✅ Complete | N/A |
| 12 | B | Snapshot Blackboard Implementation | ✅ Complete | N/A |
| 16 | B | SCIP Indexing Implementation | ✅ Complete | N/A |
| 17 | B | Influence Vector Calculation | ✅ Complete | N/A |
| 5 | B | TOON Serialization | ✅ Complete | N/A |
| 8 | B | Context Distribution System | ✅ Complete | N/A |
| 19 | C | Bidirectional Traceability | ✅ Complete | N/A |
| 7 | C | Task Decomposition & Concurrency | ✅ Complete | N/A |
| 7 | C | Task Decomposition & Concurrency | ✅ Complete | N/A |
| 9 | C | Dual-Thread ReAct Implementation | ✅ Complete | N/A |
