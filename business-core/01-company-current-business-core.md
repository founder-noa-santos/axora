# 01. Company Current Business Core

## Purpose

Define what OPENAKTA is now, based on the validated Rust runtime and cloud tier architecture.

## Executive Summary

OPENAKTA is a **local-first AI coding assistant** with a **cloud upgrade tier**:

### Free Tier (Local, Default)
- **Vector Backend:** sqlite-vec HNSW ANN (384-dim semantic memory)
- **Embeddings:** Candle (JinaCode 768-dim, BGE-Skill 384-dim)
- **RAM:** <50MB target
- **Distribution:** Single static binary (musl)

### Paid Tier (Cloud)
- **Auth:** `openakta auth login` (Clerk.dev, GitHub/Google)
- **Vector DB:** Qdrant Cloud (Azure Marketplace)
- **Embeddings:** Cohere embed-v3-multilingual
- **API:** `api.openakta.dev` (Rust + Axum)
- **Rate Limiting:** Pro 100/min, Free 10/min (Redis + tower-governor)

### Self-Hosted Option
- **Config:** `semantic_vector_backend = "external"`
- **Endpoint:** User-supplied Qdrant or compatible

## Confirmed Current State

### Local Tier Architecture

| Component | Implementation |
|-----------|----------------|
| Vector Backend | sqlite-vec HNSW ANN |
| Fallback | SqliteJson linear scan (migration/legacy) |
| Embeddings | Candle (JinaCode 768-dim, BGE-Skill 384-dim) |
| Memory Model | Tripartite: Semantic (vectors), Episodic (text/time), Procedural (skills) |
| Pruning Model | Ebbinghaus lifecycle |
| Config Default | `CoreConfig.semantic_vector_backend = "sqlite_vec"` |

### Cloud Tier Architecture

| Component | Implementation |
|-----------|----------------|
| API Server | Rust + Axum (`openakta-api` private) |
| Vector DB | Qdrant Cloud (Azure Marketplace) |
| Namespace | `openakta_{user_id}` |
| Embeddings | Cohere embed-v3-multilingual |
| Inference | Azure Foundry Serverless |
| Auth | Clerk.dev |
| Identity Providers | GitHub, Google only |
| Rate Limiting | tower-governor + Redis |
| Relational DB | Postgres (`users`, `quota_remaining`) |
| API Paths | `/v1/validate`, `/v1/:user_id/upsert`, `/v1/:user_id/search` |

### CLI Auth Flow

```bash
openakta auth login
# → Token stored in keyring
# → Daemon downloads cloud backend config
```

### Frontend (openakta-web)

| Component | Implementation |
|-----------|----------------|
| Framework | Next.js |
| Auth | Clerk `<SignIn />` |
| Identity Providers | GitHub, Google |
| UX Scope | Single page (auth bridge + instruct to run `openakta auth login`) |
| Deploy Target | Vercel |

## What OPENAKTA Sells Technically

OPENAKTA's differentiated value:

1. **Local-first by default** — Free tier works offline, no external dependencies
2. **Cloud upgrade via Azure** — Managed Qdrant Cloud, not self-hosting burden
3. **Dual embedding strategy** — Candle local, Cohere cloud
4. **Simplified auth** — Clerk.dev with GitHub/Google only
5. **Single binary distribution** — musl static linking, <50MB RAM target

## What OPENAKTA Is Not Yet

OPENAKTA is still not a full SaaS with:

- Traditional billing infrastructure (Stripe, invoices, seats)
- Complex entitlement enforcement
- Multi-provider identity beyond GitHub/Google

## Implementation Evidence

- `crates/openakta-memory/src/vector_backend.rs` — `VectorStore` trait, sqlite-vec backend
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend` enum
- `crates/openakta-daemon/README.md` — Local/cloud tier documentation
- `openakta-api/` — Cloud API (private repo)
- `openakta-web/` — Next.js + Clerk frontend

## Business Meaning

OPENAKTA's business core is **infrastructure-based monetization**:

- **Free tier:** Fully functional local-first assistant
- **Paid tier:** Managed cloud infrastructure via Azure Marketplace
- **Self-hosted:** Enterprise option via `External` config

Value is delivered through infrastructure quality, not billing enforcement.

## Open Ambiguities

- Enterprise pricing beyond rate limits not codified
- Self-hosted backend testing not fully validated
- Turbopuffer migration is future path only

## Confidence Assessment

High. This document reflects implemented local-first architecture with cloud tier via Azure Marketplace.
