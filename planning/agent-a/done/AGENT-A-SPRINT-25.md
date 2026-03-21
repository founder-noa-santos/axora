# Agent A — Sprint 25: AGENTS.md Living Document

**Phase:** 2  
**Sprint:** 25 (Documentation)  
**File:** `AGENTS.md` (root) + `docs/ARCHITECTURE-LEDGER.md`  
**Priority:** HIGH (architectural visibility)  
**Estimated Tokens:** ~60K output  

---

## 🎯 Task

Create **AGENTS.md Living Document** (industry standard) for architectural visibility and constraint enforcement.

### Context

Competitive analysis validates industry pattern:
- **AGENTS.md** — Standard architectural ledger (living document)
- **Auto-update** — Updated on sprint completion
- **Constraint Enforcement** — Token budgets, concurrency limits

**Your job:** Create AGENTS.md ledger (provides architectural visibility).

---

## 📋 Deliverables

### 1. Create AGENTS.md (Root)

**File:** `AGENTS.md` (repository root)

**Structure:**
```markdown
# OPENAKTA Architecture Ledger

**Last Updated:** 2026-03-16  
**Maintained By:** Architect Agent

## Current Architecture

### Coordinator Agent
- **Role:** Decompose missions, dispatch tasks, validate outputs
- **State:** `planning/coordinator/COORDINATION-BOARD.md`
- **Constraints:** Max 10 concurrent tasks, 50K token budget

### Worker Agents
- **Agent A:** Documentation Specialist
  - Current Sprint: 25 (AGENTS.md Ledger)
  - Status: In Progress
- **Agent B:** Storage + Context Specialist
  - Current Sprint: 24 (Repository Map)
  - Status: In Progress
- **Agent C:** Implementation Specialist
  - Current Sprint: 23 (ACI Formatting)
  - Status: In Progress

## Active Constraints

### Token Budget
- **Per-Task Limit:** 2,500 tokens (pruned context)
- **Per-Agent Limit:** 50,000 tokens (daily budget)
- **Enforcement:** Hard stop (circuit breaker)

### Concurrency Limits
- **Max Parallel Tasks:** 10 (sliding-window semaphore)
- **Rate Limit:** 100 requests/minute (API throttle)

## Execution Graph

```
Sprint 18 (A) → Sprint 19 (C)
Sprint 16 (B) → Sprint 17 (B) → Sprint 20 (B) → Sprint 21 (B) → Sprint 22 (B) → Sprint 24 (B)
Sprint 23 (C)
Sprint 25 (A)
```

## Recent Changes

### 2026-03-16
- Adopted sliding-window semaphores (Dify pattern)
- Adopted atomic checkout semantics (Paperclip pattern)
- Adopted ACI formatting (SWE-Agent pattern)
- Adopted repository map (Aider pattern — 90% token reduction)

### 2026-03-15
- Graph-Based Workflow pivot (R-10 validation)
- Implemented dual-thread ReAct loops
- Implemented snapshot blackboard (TOCTOU prevention)

## Sprint History

| Sprint | Agent | Title | Status | Token Reduction |
|--------|-------|-------|--------|-----------------|
| 11 | A | Documentation Pivot | ✅ Complete | N/A |
| 12 | A | ACONIC Docs | ✅ Complete | N/A |
| 18 | A | Business Rules | ✅ Complete | N/A |
| 25 | A | AGENTS.md Ledger | 🔄 In Progress | N/A |
| 11 | B | Context + RAG | ✅ Complete | 60-80% |
| 12 | B | Snapshot Blackboard | ✅ Complete | N/A |
| 16 | B | SCIP Indexing | ✅ Complete | N/A |
| 17 | B | Influence Vector | ✅ Complete | N/A |
| 20 | B | Context Pruning | ✅ Complete | 95-99% |
| 21 | B | Sliding-Window Semaphores | ✅ Complete | N/A |
| 22 | B | Atomic Checkout | ✅ Complete | N/A |
| 24 | B | Repository Map | ✅ Complete | 90%+ |
| 8 | C | Graph Workflow | ✅ Complete | N/A |
| 9 | C | Dual-Thread ReAct | ✅ Complete | N/A |
| 19 | C | Bidirectional Traceability | ✅ Complete | N/A |
| 23 | C | ACI Formatting | ✅ Complete | N/A |

## Performance Metrics

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Token Reduction | 90%+ | 95-99% | ✅ Exceeded |
| Concurrency Speedup | 3-5x | TBD | 🔄 Measuring |
| Context Allocation | <10ms | TBD | 🔄 Measuring |
| Race Conditions | 0 | 0 | ✅ Pass |
| Duplicate Execution | 0% | 0% | ✅ Pass |
```

---

### 2. Create Auto-Update Script

**File:** `scripts/update-agents-ledger.sh`

**Content:**
```bash
#!/bin/bash

# Auto-update AGENTS.md ledger on sprint completion

set -e

echo "Updating AGENTS.md ledger..."

# Get current date
DATE=$(date +%Y-%m-%d)

# Get completed sprints from done/ folders
for agent_dir in planning/agent-*/done; do
    agent_name=$(basename $(dirname $agent_dir) | sed 's/agent-//')
    
    for sprint_file in $agent_dir/AGENT-${agent_name}-SPRINT-*.md; do
        if [ -f "$sprint_file" ]; then
            sprint_num=$(basename $sprint_file | grep -oP 'SPRINT-\K[0-9]+')
            sprint_title=$(grep "^#.*Sprint $sprint_num" $sprint_file | sed 's/^# //')
            
            # Update ledger (append to sprint history)
            echo "| $sprint_num | $agent_name | $sprint_title | ✅ Complete | N/A |" >> AGENTS.md.tmp
        fi
    done
done

# Replace old sprint history with updated version
# (Use sed or awk to replace table section)

echo "Ledger updated successfully!"
```

---

### 3. Create docs/ARCHITECTURE-LEDGER.md

**File:** `docs/ARCHITECTURE-LEDGER.md`

**Content:**
```markdown
# Architecture Ledger (Detailed)

**Purpose:** Track architectural decisions, constraints, and enforcement mechanisms.

## Decision Log

### ADR-042: Graph-Based Workflow (2026-03-15)
- **Status:** Active & Enforced
- **Context:** DDD Agents rejected (over-engineering for individual devs)
- **Decision:** Adopt LangGraph-style deterministic graph workflow
- **Enforcement:** Coordinator agent enforces graph routing

### ADR-043: Sliding-Window Semaphores (2026-03-16)
- **Status:** Active & Enforced
- **Context:** Resource starvation in concurrent execution
- **Decision:** Adopt Dify pattern (semaphore-based throttling)
- **Enforcement:** ConcurrentExecutor enforces max_concurrent limit

### ADR-044: Atomic Checkout (2026-03-16)
- **Status:** Active & Enforced
- **Context:** Duplicate execution (multiple agents same task)
- **Decision:** Adopt Paperclip pattern (FOR UPDATE SKIP LOCKED)
- **Enforcement:** TaskQueue enforces single-assignee model

## Constraint Enforcement

### Token Budget Enforcement
```rust
// Enforced by ContextManager
if context.estimate_tokens() > max_tokens {
    return Err(Error::TokenBudgetExceeded);
}
```

### Concurrency Enforcement
```rust
// Enforced by ConcurrentExecutor
let permit = semaphore.acquire_owned().await?;
// Task executes...
drop(permit); // Release
```

### Duplicate Execution Prevention
```rust
// Enforced by TaskQueue
let task = checkout_task(agent_id).await?;
// Only one agent gets the task (atomic)
```

## Pattern Adoptions

| Pattern | Source | Status | Sprint |
|---------|--------|--------|--------|
| Sliding-Window Semaphores | Dify | ✅ Implemented | 21 |
| Atomic Checkout | Paperclip | ✅ Implemented | 22 |
| ACI Formatting | SWE-Agent | ✅ Implemented | 23 |
| Repository Map | Aider | ✅ Implemented | 24 |
| AGENTS.md Ledger | Industry Standard | ✅ Implemented | 25 |
```

---

## 📁 File Boundaries

**Create:**
- `AGENTS.md` (repository root)
- `docs/ARCHITECTURE-LEDGER.md` (NEW)
- `scripts/update-agents-ledger.sh` (NEW)

**Update:**
- None (new files only)

**DO NOT Edit:**
- `crates/` (implementation — Agents B and C's domain)

---

## ✅ Success Criteria

- [ ] `AGENTS.md` created (root level)
- [ ] `docs/ARCHITECTURE-LEDGER.md` created
- [ ] `scripts/update-agents-ledger.sh` created
- [ ] Auto-update script works
- [ ] Sprint history table populated
- [ ] Constraints documented
- [ ] Pattern adoptions documented
- [ ] Ledger is human-readable AND machine-parseable

---

## 🔗 References

- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](../shared/PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- Research document — AGENTS.md industry standard

---

**Start AFTER Sprint 18 (Business Rule Documentation) is complete.**

**Priority: HIGH — provides architectural visibility.**

**Dependencies:**
- Sprint 18 (Business Rule Documentation) — recommended but not required

**Blocks:**
- None (documentation improvement)
