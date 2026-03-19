# Files Proposed for Archive/Deletion

**Date:** 2026-03-18  
**Reason:** Strategic pivot — These files contain deprecated concepts (black-listed)

---

## 🚨 Files to Move to `research/OUTDATED/`

### High Priority (Contains Black-Listed Concepts)

| File | Reason | Black-Listed Concepts |
|------|--------|----------------------|
| `research/DECISIONS.md` | Contains Ollama, llama.cpp, Turbopuffer decisions | Ollama, llama.cpp, Turbopuffer |
| `research/NEXT-STEPS.md` | References Ollama, local model inference | Ollama, local LLM |
| `research/R-01-IMPLEMENTATION-PLAN.md` | Likely contains deprecated RAG decisions | Outdated RAG approach |
| `research/R-02-IMPLEMENTATION-PLAN.md` | Likely contains deprecated agent architecture | Outdated agent design |
| `research/R-03-IMPLEMENTATION-PLAN.md` | Likely contains deprecated token optimization | Outdated approach |
| `research/prompts/09--documentation-management.md` | References DDD agent teams | DDD, Domain-Driven Design |

### Medium Priority (Partially Outdated)

| File | Reason | Action |
|------|--------|--------|
| `research/README.md` | Index file — needs update | Update, don't move |
| `research/BUSINESS-ALIGNMENT.md` | Business decisions still valid | Keep, but review |
| `research/findings/multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md` | References AutoGen (but as anti-pattern) | Keep (research valid) |
| `research/findings/local-first-rag/R-16-LOCAL-FIRST-RAG.md` | Research still valid | Keep (aligned with pivot) |

### Low Priority (Review Needed)

| File | Reason | Action |
|------|--------|--------|
| `planning/archive/shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md` | Contains SCIP research (valid) but may have outdated concepts | Review before moving |
| `planning/archive/shared/RAG-EXPERTISE-DESIGN.md` | Tripartite memory (valid) but may have outdated RAG approach | Review before moving |

---

## ✅ Files to KEEP (White-Listed Concepts)

### Active Research

| File | Location | White-Listed Concepts |
|------|----------|----------------------|
| Local-First RAG Research | `research/findings/local-first-rag/` | Jina, Qdrant Embedded, Tree-sitter, Merkle Trees |
| Multi-Agent Optimization | `research/findings/multi-agent-optimization/` | Prefix Caching, Diff Communication, SCIP |
| Active Architecture Docs | `docs/active_architecture/` | Blackboard, Dual-Thread ReAct, Influence Graph |

### Implementation Code

| Component | Location | Status |
|-----------|----------|--------|
| PrefixCache | `crates/axora-cache/src/prefix_cache.rs` | ✅ Keep (implemented) |
| Diff | `crates/axora-cache/src/diff.rs` | ✅ Keep (implemented) |
| InfluenceGraph | `crates/axora-indexing/src/influence.rs` | ✅ Keep (implemented) |
| Blackboard v2 | `crates/axora-cache/src/blackboard/v2.rs` | ✅ Keep (implemented) |

---

## 📋 Action Plan

### Step 1: Move High-Priority Files to OUTDATED

```bash
# Move deprecated research files
mv research/DECISIONS.md research/OUTDATED/
mv research/NEXT-STEPS.md research/OUTDATED/
mv research/R-01-IMPLEMENTATION-PLAN.md research/OUTDATED/
mv research/R-02-IMPLEMENTATION-PLAN.md research/OUTDATED/
mv research/R-03-IMPLEMENTATION-PLAN.md research/OUTDATED/
mv research/prompts/09--documentation-management.md research/OUTDATED/
```

### Step 2: Update Medium-Priority Files

```bash
# Update index file
# Edit research/README.md to remove references to deprecated files
# Add references to new active_architecture folder
```

### Step 3: Review Low-Priority Files

```bash
# Review archive files
# Move if mostly outdated
# Keep if still valuable
```

---

## ⚠️ DO NOT DELETE YET

**Wait for confirmation before executing any moves.**

This is a **proposal** only. Review the list above and confirm which files should be moved.

---

## 📊 Impact Analysis

### What We Lose (If We Delete)

| Concept | Impact | Mitigation |
|---------|--------|------------|
| Ollama/llama.cpp research | None (not using local LLM) | ✅ Safe to delete |
| Turbopuffer research | None (using local Qdrant) | ✅ Safe to delete |
| DDD agent teams | None (using Blackboard) | ✅ Safe to delete |
| AutoGen research | Historical context only | ⚠️ Keep as reference |

### What We Keep

| Concept | Reason |
|---------|--------|
| Local-First RAG | ✅ Aligned with pivot (local indexing) |
| Multi-Agent Optimization | ✅ Aligned with pivot (token efficiency) |
| Active Architecture Docs | ✅ Single Source of Truth |
| Implemented Code | ✅ Production-ready |

---

## ✅ Confirmation Checklist

Before executing moves, confirm:

- [ ] I have reviewed all files listed above
- [ ] I understand what will be moved to OUTDATED
- [ ] I understand what will be kept
- [ ] I have backed up important research (optional)
- [ ] I am ready to proceed with the cleanup

**Reply with "CONFIRMED" to execute the cleanup, or provide feedback on what should be kept/moved.**

---

**Last Updated:** 2026-03-18  
**Prepared By:** Architect Agent
