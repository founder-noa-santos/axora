# Phase 2 Update: Documentation Management Improvements

**Date:** 2026-03-16  
**Source:** R-09 Research Findings (Documentation Management for AI Agent Systems)  
**Impact:** Updates to Sprint 6 (already complete) + Future sprints

---

## ✅ What's Already Validated (Sprint 6 - Agent A)

**Agent A implemented (2510 lines, 54 tests passing):**
- ✅ DocSchema with versioning
- ✅ DocIndex with retrieval
- ✅ LivingDocs with auto-update detection
- ✅ ADR system with linking
- ✅ Staleness detection
- ✅ Full workflow tests

**Verdict:** **FOUNDATIONS ARE SOLID** — no major rework needed.

---

## 🔄 Updates Required (Evolution, Not Rework)

### 1. Documentation Format Standardization

**Current:** Markdown with custom structure  
**Update:** Add **YAML frontmatter** standard per R-09 findings

**Why:** Research shows Markdown + YAML frontmatter is optimal for agent parsing (34-38% token savings vs JSON, 80% vs XML)

**Change:**
```markdown
---
id: AUTH-001
version: 1.0
last_updated: 2026-03-16
maintainer: auth-team
status: active
related: [AUTH-002, AUTH-003]
---

# User Authentication

Users must authenticate with valid credentials...
```

**Action:** Update `docs/DOCUMENTATION-FORMAT.md` with YAML frontmatter spec

**Effort:** ~2 hours (documentation update only, no code changes)

---

### 2. AGENTS.md Standard Enhancement

**Current:** We have `AGENTS.md` (created during business alignment)  
**Update:** Follow **R-09 AGENTS.md standard** more strictly

**R-09 Spec:**
- Root-level AGENTS.md (<300 lines)
- Hierarchical nesting (directory-specific overrides)
- Strict invariants, not suggestions
- Machine-parseable rules

**Change:**
```markdown
# Current (too verbose)
This project uses Rust for performance. We value safety and...

# Updated (per R-09 spec)
## Tech Stack
- Language: Rust 1.94+
- Async: Tokio
- Database: SQLite

## Invariants
❌ NEVER use unwrap() in production code
❌ NEVER commit without tests
✅ ALWAYS validate user input
✅ ALWAYS use parameterized queries
```

**Action:** Update `AGENTS.md` to follow R-09 spec

**Effort:** ~1 hour (content update)

---

### 3. ADRs as Active Governance

**Current:** ADRs are passive documentation  
**Update:** Make ADRs **executable invariants**

**R-09 Insight:**
> "Instead of merely recording a past decision, ADRs serve as the active control plane for the agent."

**Change:**
```markdown
# Current (Passive)
## Decision
We chose SQLite for local-first storage.

# Updated (Active Governance)
## Invariant Rules (Agent Directives)
❌ NEVER use remote databases without explicit user configuration
✅ ALWAYS default to SQLite for local storage
✅ ALWAYS use migrations for schema changes
## Validation
- CI check: `cargo sqlx prepare` validates migrations
- Agent check: Verify DbConnectionPool usage
```

**Action:** Update `docs/ADR-TEMPLATE.md` with active governance format

**Effort:** ~2 hours (template update + migrate existing ADRs)

---

### 4. llms.txt Protocol Adoption

**Current:** Not implemented  
**Update:** Implement **llms.txt** standard

**R-09 Spec:**
```
# llms.txt (at root path)
# Curated sitemap for reasoning engines

# Project Summary
OPENAKTA - Multi-Agent AI Coding System

## Core Documentation
- [Architecture](docs/architecture.md)
- [Agents](docs/agents.md)
- [API Reference](docs/api.md)

## Optional (can omit under context constraints)
- [Tutorials](docs/tutorials/)
- [Historical ADRs](docs/adr/)
```

**Action:** Create `llms.txt` and `llms-full.txt` at root

**Effort:** ~1 hour (create files, no code changes)

---

### 5. SKILL.md Modular Pattern

**Current:** Not implemented  
**Update:** Adopt **SKILL.md** pattern for modular agent capabilities

**R-09 Spec:**
```markdown
---
skill_id: code-review
name: Code Review
version: 1.0
tokens: ~100
triggers: ["review", "audit", "check code"]
---

# Code Review Skill

When triggered, agent will:
1. Parse code AST
2. Check against invariants
3. Generate review comments
```

**Why:** Lazy-load skills only when needed (zero idle token cost)

**Action:** Create `skills/` directory with modular SKILL.md files

**Effort:** ~4 hours (create 5-10 initial skills)

---

### 6. Hybrid Retrieval Specification

**Current:** Planned in Sprint 8 (Context Distribution)  
**Update:** **Specify hybrid search** more precisely per R-09

**R-09 Spec:**
```rust
pub struct DocRetriever {
    vector_index: VectorStore,  // Dense embeddings (semantic)
    keyword_index: TantivyIndex, // BM25 (lexical)
}

impl DocRetriever {
    pub async fn retrieve(&self, query: &str) -> Vec<DocChunk> {
        // Parallel hybrid retrieval
        let vector_results = self.vector_search(query, 10).await?;
        let keyword_results = self.keyword_search(query, 10).await?;
        
        // Merge + rerank
        self.rerank_and_merge(vector_results, keyword_results)
    }
}
```

**Action:** Update `planning/agent-b/AGENT-B-SPRINT-8.md` with hybrid search spec

**Effort:** Already planned, just refine spec (~1 hour)

---

### 7. Memory Architecture Tripartite

**Current:** Not implemented  
**Update:** Adopt **Semantic/Episodic/Procedural** memory architecture

**R-09 Spec:**
```
Semantic Memory: API contracts, data schemas (vector DB)
Episodic Memory: Past interactions, conversation logs (chronological)
Procedural Memory: Learned workflows, skills (SKILL.md files)
```

**Action:** Create research prompt for memory architecture (R-14)

**Effort:** ~3-4 hours research + implementation later

---

### 8. Documentation Health Metrics

**Current:** Partially implemented in Sprint 6  
**Update:** Expand to full **DocHealth** spec per R-09

**R-09 Spec:**
```rust
pub struct DocHealth {
    pub coverage_ratio: f32,        // % modules with docs
    pub freshness_index: f32,       // Avg days since sync
    pub stale_percentage: f32,      // % flagged as stale
    pub retrieval_success_rate: f32, // Agent success rate
    pub update_latency_hours: f32,  // Commit → doc update time
    pub human_override_rate: f32,   // % code requiring human fix
}
```

**Action:** Update `crates/openakta-docs/src/living.rs` with full metrics

**Effort:** ~4 hours (expand existing implementation)

---

### 9. Tombstone Protocol (Auto-Purge)

**Current:** Not implemented  
**Update:** Implement **tombstone protocol** for stale data purge

**R-09 Insight:**
> "When an API endpoint is deprecated, the corresponding documentation must be actively purged from the vector database."

**Action:** Create new sprint for Tombstone Protocol

**Effort:** ~4 hours implementation

---

### 10. Minimum Viable Documentation (MVD)

**Current:** Not specified  
**Update:** Adopt **MVD principle** (<300 lines for AGENTS.md)

**R-09 Spec:**
> "The optimal root-level AGENTS.md should be heavily constrained—ideally kept under 300 lines."

**Action:** Update `AGENTS.md` to follow MVD principle

**Effort:** ~1 hour (content pruning)

---

## 📊 Summary of Changes

| Update | Effort | Priority | When |
|--------|--------|----------|------|
| YAML Frontmatter | 2h | 🔴 HIGH | Now (doc update) |
| AGENTS.md Standard | 1h | 🔴 HIGH | Now (content update) |
| Active ADRs | 2h | 🟠 MEDIUM | This week |
| llms.txt Protocol | 1h | 🔴 HIGH | Now (create files) |
| SKILL.md Pattern | 4h | 🟠 MEDIUM | This week |
| Hybrid Retrieval Spec | 1h | 🔴 HIGH | Update Sprint 8 |
| Memory Architecture | 4h | 🟡 LOW | Research (R-14) |
| DocHealth Metrics | 4h | 🟠 MEDIUM | Expand Sprint 6 |
| Tombstone Protocol | 4h | 🟠 MEDIUM | New sprint |
| MVD Principle | 1h | 🔴 HIGH | Now (prune content) |

**Total Effort:** ~24 hours (3 days for single developer)

---

## 🚀 Immediate Actions (This Week)

### Week 1 Sprint Updates

**Agent A (already on Sprint 9 - Integration):**
- [ ] Add YAML frontmatter to existing docs (~2h)
- [ ] Update AGENTS.md to R-09 spec (~1h)
- [ ] Create llms.txt + llms-full.txt (~1h)
- [ ] Create SKILL.md files (5-10 skills, ~4h)

**Agent B (on Sprint 10 - Documentation):**
- [ ] Update Sprint 8 spec with hybrid search (~1h)
- [ ] Expand DocHealth metrics in Sprint 6 code (~4h)

**Agent C (on Sprint 7 - Decomposition):**
- [ ] No changes needed (not affected by R-09)

**New Sprint (Tombstone Protocol):**
- [ ] Assign to Agent A or B after current sprints complete

---

## 📋 New Research Prompt

**R-14: Memory Architecture for AI Agents**

Create research prompt for tripartite memory (Semantic, Episodic, Procedural)

**File:** `research/prompts/14-memory-architecture.md`

**Estimated Time:** 3-4 hours

---

## ✅ Validation Criteria

Updates are successful when:
- [ ] YAML frontmatter added to all docs
- [ ] AGENTS.md follows R-09 spec (<300 lines)
- [ ] ADRs updated to active governance format
- [ ] llms.txt + llms-full.txt created
- [ ] 5-10 SKILL.md files created
- [ ] Hybrid search spec added to Sprint 8
- [ ] DocHealth metrics expanded
- [ ] Tombstone protocol implemented
- [ ] MVD principle applied

---

**These updates evolve our existing work without invalidating any completed sprints.**

**All changes are backward-compatible and additive.**
