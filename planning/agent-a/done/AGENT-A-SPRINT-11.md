# Agent A — Sprint 11: Architecture Documentation Pivot

**Sprint:** 11 of Phase 2  
**File:** `planning/shared/GRAPH-WORKFLOW-DESIGN.md` + documentation updates  
**Estimated Tokens:** ~50K output tokens  

---

## 🎯 Task

Update ALL architecture documentation to reflect the DDD → Graph-Based Workflow pivot.

### Context

R-10 research conclusively proved:
- DDD Agents are OVER-ENGINEERING for individual developers
- Coordination overhead grows quadratically (N(N-1)/2)
- "Expertise accumulation" is anthropomorphism (it's just RAG)
- Graph-Based Deterministic Workflow (LangGraph-style) is superior for 95% of use cases

**Your job:** Update documentation to reflect this PIVOT.

---

## 📋 Deliverables

### 1. Update DDD-TDD-AGENT-TEAMS.md

**File:** `planning/shared/DDD-TDD-AGENT-TEAMS.md`

**Add at top:**
```markdown
## ⚠️ STATUS: REJECTED (2026-03-16)

This document contains historical analysis of DDD Agent Teams.

**Decision:** DEFERRED indefinitely, Graph-Based Workflow ADOPTED instead.

**Reason:** DDD is enterprise over-engineering. Individual developers need:
- Low latency (not high coordination overhead)
- Simple architecture (not bounded contexts + ACLs)
- Holistic context (not siloed domain teams)

See: [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md)
```

**Keep:** Original analysis (for historical reference)

**Add:** "Lessons Learned" section with key takeaways from R-10.

---

### 2. Create GRAPH-WORKFLOW-DESIGN.md

**File:** `planning/shared/GRAPH-WORKFLOW-DESIGN.md`

**Structure:**
```markdown
# Graph-Based Deterministic Workflow

## Overview
- What: State machine with deterministic nodes
- Why: O(N) coordination (not O(N²)), <10% token overhead
- Who: Individual developers (AXORA's target)

## Architecture

### Nodes (Agent Roles)
- Planner: Decompose tasks, create execution graph
- Executor: Write code (full repo access, domain RAG)
- Reviewer: Validate code, run tests

### Edges (Deterministic Routing)
- Explicit state transitions (no semantic routing)
- Guard conditions (validate before transition)
- Error edges (retry or escalate)

### Domain Knowledge (RAG, not Agents)
- Auth Vector Store
- Billing Vector Store
- Past Successes Memory Bank

## Comparison: DDD vs Graph

| Aspect | DDD | Graph + RAG |
|--------|-----|-------------|
| Coordination | O(N²) | O(N) |
| Token Overhead | 40%+ | <10% |
| Cross-Domain | Complex routing | Direct access |
| Implementation | 120h+ | ~40h |

## Implementation Plan

### Phase 1: State Machine Primitives
- Node definition
- Edge definition
- Guard conditions

### Phase 2: Domain RAG Integration
- Vector stores per domain
- Late-interaction retrieval
- Experience-as-Parameters memory

### Phase 3: Integration
- Integrate with existing agents
- Migration path from DDD concepts
```

**Length:** 2000-3000 words (comprehensive but concise)

---

### 3. Update AGENTS.md

**File:** `AGENTS.md` (root level)

**Changes:**
1. Remove any DDD references
2. Add Graph-Based Workflow section
3. Update "Knowledge Navigation" table with new docs

**Example:**
```markdown
## 🧠 Core Mindset

### Graph-Based Execution (NEW)
- Agents are NOT domain-specialized
- Domain knowledge is in RAG, not agent structure
- Coordination is O(N), not O(N²)
- Deterministic routing (not semantic routing)
```

---

### 4. Create RAG-EXPERTISE-DESIGN.md

**File:** `planning/shared/RAG-EXPERTISE-DESIGN.md`

**Purpose:** Document how "expertise accumulation" actually works (RAG, not team learning)

**Structure:**
```markdown
# Experience-as-Parameters: RAG-Based Expertise

## The Anthropomorphism Fallacy
- Agents do NOT learn like humans
- "Expertise" = Better retrieval, not team structure
- Externalized state, not internal learning

## Architecture

### Semantic Memory (Vector Store)
- API contracts
- Data schemas
- Past successful patterns

### Episodic Memory (Conversation Logs)
- Past debugging sessions
- Terminal outputs
- Decision traces

### Procedural Memory (Skill Files)
- SKILL.md files
- Executable workflows
- Trigger-based loading

## Retrieval Strategy
- Late-interaction (ColBERT-style)
- Hybrid search (BM25 + vectors)
- Top-k with reranking

## Token Efficiency
- Retrieve only relevant patterns
- <10% overhead vs 40%+ for DDD
```

---

## 📁 File Boundaries

**Create:**
- `planning/shared/GRAPH-WORKFLOW-DESIGN.md`
- `planning/shared/RAG-EXPERTISE-DESIGN.md`

**Update:**
- `planning/shared/DDD-TDD-AGENT-TEAMS.md` (add REJECTED status)
- `AGENTS.md` (remove DDD, add Graph)
- `planning/shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md` (already created, reference it)

**DO NOT Edit:**
- `crates/` (implementation, Agent B/C will handle)
- `research/` (research files are historical)

---

## ✅ Success Criteria

- [ ] DDD-TDD-AGENT-TEAMS.md has REJECTED status at top
- [ ] GRAPH-WORKFLOW-DESIGN.md created (2000-3000 words)
- [ ] RAG-EXPERTISE-DESIGN.md created (1500-2000 words)
- [ ] AGENTS.md updated (no DDD references, Graph added)
- [ ] All docs link to PHASE-2-PIVOT-GRAPH-WORKFLOW.md
- [ ] Zero contradictions between docs

---

## 🔗 References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Main pivot doc
- [`research/prompts/10-ddd-agents-validation.md`](../research/prompts/10-ddd-agents-validation.md) — R-10 research
- [`research/prompts/14-memory-architecture.md`](../research/prompts/14-memory-architecture.md) — Memory/RAG design

---

**Start NOW. Focus on clarity and consistency. All other agents will read your docs.**
