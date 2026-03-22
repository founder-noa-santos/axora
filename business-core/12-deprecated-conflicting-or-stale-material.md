# 12. Deprecated, Conflicting, or Stale Material

## Purpose

Isolate stale and contradictory material so canonical architecture docs reflect current truth.

## Executive Summary

This document catalogs architecture references that are **not current truth**. These items have been purged or normalized in canonical docs and code.

## Purged Architecture References

### Vector Backends (Not Current)

| Reference | Status | Replacement |
|-----------|--------|-------------|
| **LanceDB** | Purged from current architecture | sqlite-vec (local), Qdrant Cloud (paid) |
| **pgvector** | Never implemented | sqlite-vec |
| **Milvus** | Never implemented | — |
| **Chroma** | Never implemented | — |
| **Turbopuffer** | Future migration path only | Not current architecture |
| **Qdrant Docker / self-hosted as official** | Purged from cloud tier narrative | Qdrant Cloud (Azure Marketplace) |

### Embedding Models (Not Current)

| Reference | Status | Replacement |
|-----------|--------|-------------|
| **Voyage code-3** | Never implemented | Candle (JinaCode, BGE-Skill) local, Cohere embed-v3-multilingual cloud |
| **Jina 1.5B** | Outdated reference | JinaCode 768-dim via Candle |

### Auth/Billing (Not Current Backend Truth)

| Reference | Status | Current Truth |
|-----------|--------|---------------|
| **Stripe/Paddle integration** | Not implemented | Azure Marketplace provisioning |
| **Subscription/seat/entitlement tables** | Not in schema | Cloud tier is infra-based, not billing-enforced |
| **Synthetic provider execution** | Historical | Live HTTP transport (Anthropic, OpenAI) |

### Deprecated Code Patterns

| Reference | Status | Current Pattern |
|-----------|--------|-----------------|
| **ProviderKind drives transport** | Deprecated (R4 completed 2026-03-20) | WireProfile drives transport |
| **Hardcoded token limits** | Deprecated | Dynamic model metadata |
| **Env var API keys** | Deprecated (pre-2026-03-20) | File-based secrets in `.openakta/secrets/` |
| **Inline API keys in TOML** | Deprecated | `api_key_file` pattern |

## Detailed Breakdown

### LanceDB References

LanceDB appeared in historical architecture docs as a potential embedded columnar store. It is **not current architecture**.

**Purged from:**
- `docs/active_architecture/02_LOCAL_RAG_AND_MEMORY.md` — Now documents sqlite-vec
- `docs/architecture-communication.md` — Now uses `VectorStore` trait
- `docs/architecture-context-rag.md` — Now documents sqlite-vec + Candle

### Turbopuffer References

Turbopuffer is a **future migration path** via `VectorStore` trait compatibility. It is not current architecture.

**Current framing:**
- Mentioned only as future compatibility in `vector_backend.rs` stubs
- Not in active architecture docs

### Self-Hosted Qdrant Framing

Self-hosted Qdrant is available via `External` backend config, but it is **not the official paid cloud path**.

**Current framing:**
- Paid cloud = Qdrant Cloud (Azure Marketplace)
- Self-hosted = `External { endpoint, api_key }` config option

### Payment/Billing Docs

`docs/business_rules/PAY-001.md`, `PAY-002.md` describe payment processing not implemented in backend.

**Current truth:**
- Cloud tier monetization is infra-based (Azure Marketplace, rate limiting)
- No Stripe, invoices, or entitlement enforcement in code

## Implementation Evidence

- `crates/openakta-memory/src/vector_backend.rs` — No LanceDB, only sqlite-vec/SqliteLinear/External
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend` enum: `SqliteVec`, `SqliteLinear`, `External`
- `business-core/14-glossary-and-canonical-terms.md` — Terms to avoid section

## Confidence Assessment

High. Stale references have been purged or normalized in canonical docs.
