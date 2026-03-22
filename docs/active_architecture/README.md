# Active architecture documentation

**Status:** Active  
**Last updated:** 2026-03-21  

---

## Purpose

This folder is the **primary narrative** for OPENAKTA architecture: how the system is meant to work and which crates implement which ideas. When code and these docs disagree, **verify in code and tests**, then update the docs.

---

## Documents (read in order)

| File | Topics |
|------|--------|
| [01_CORE_ARCHITECTURE.md](./01_CORE_ARCHITECTURE.md) | Blackboard, ReAct, influence graph, coordination |
| [02_LOCAL_RAG_AND_MEMORY.md](./02_LOCAL_RAG_AND_MEMORY.md) | Local RAG, embeddings, memory, chunking |
| [03_CONTEXT_AND_TOKEN_OPTIMIZATION.md](./03_CONTEXT_AND_TOKEN_OPTIMIZATION.md) | Prefix cache, diff communication, context pruning |
| [plan-06-ssot-conflict-resolution-ui-spec.md](./plan-06-ssot-conflict-resolution-ui-spec.md) | Plan 6: SSOT conflict resolver, review queue UI, gRPC contracts (LivingDocsReviewService) |
| [plan-06-llm-continuation-handoff.md](./plan-06-llm-continuation-handoff.md) | Plan 6: LLM/engineer handoff — gaps, risks, file map, master prompt for production completion |

---

## Strategic pivot (summary)

| Deprecated direction | Current direction |
|---------------------|-------------------|
| Local-only LLM hosting as default | Cloud APIs (Anthropic, OpenAI, etc.) where configured |
| Cloud-managed vector SaaS as default | Local-friendly stores where implemented |
| Purely conversational “swarms” | Deterministic orchestration + graph workflows |

Older experiments (Ollama-only, certain DDD agent-team models, etc.) are **not** documented here as current product truth. Use git history if you need the old research tree.

---

## Related

| Resource | Role |
|----------|------|
| [../ARCHITECTURE-LEDGER.md](../ARCHITECTURE-LEDGER.md) | Ledger, desktop ADR summary |
| [../adr/](../adr/) | Desktop shell ADRs |
| [../../business-core/](../../business-core/) | Business rules grounded in implementation |
| [../../DOCS-INDEX.md](../../DOCS-INDEX.md) | Full documentation map |

---

## For contributors

1. Read the three pillar docs above.  
2. Cross-check against `crates/` and tests.  
3. Update this folder when you change architecture meaningfully.  
