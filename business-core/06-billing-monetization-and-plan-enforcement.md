# 06. Billing, Monetization, and Plan Enforcement

## Purpose

Capture what the repository currently implements around monetization and commercial enforcement.

## Executive Summary

OPENAKTA's monetization is implemented through a cloud tier architecture, not through traditional SaaS billing infrastructure. The paid tier is delivered via:

- **Provisioning:** Azure Marketplace
- **Auth:** Clerk.dev (GitHub, Google providers)
- **Rate Limiting:** `tower-governor` + Redis (Pro: 100/min, Free: 10/min)
- **Vector DB:** Qdrant Cloud (Azure Marketplace)
- **Embeddings:** Cohere embed-v3-multilingual
- **API:** `api.openakta.dev` (Rust + Axum)

The relational layer (Postgres) tracks `users` and `quota_remaining` only—no Stripe, invoices, or seat management exists in the backend.

## Confirmed Current State

### Cloud Tier Architecture

| Component | Implementation |
|-----------|----------------|
| API Server | Rust + Axum (`openakta-api` private repo) |
| Vector DB | Qdrant Cloud (Azure Marketplace) |
| Embeddings | Cohere embed-v3-multilingual |
| Auth | Clerk.dev |
| Identity Providers | GitHub, Google only |
| Rate Limiting | `tower-governor` + Redis |
| Relational DB | Postgres (`users`, `quota_remaining`) |
| Namespace Pattern | `openakta_{user_id}` |
| Inference | Azure Foundry Serverless |

### CLI Auth Flow

```bash
openakta auth login
# Token stored in keyring
# Daemon downloads cloud backend config
```

### API Endpoints

- `POST /v1/validate` — Validate token and quota
- `POST /v1/:user_id/upsert` — Upsert vectors to Qdrant
- `POST /v1/:user_id/search` — Search vectors in Qdrant

### What is NOT Implemented

- No Stripe, Paddle, or payment provider integration
- No subscription tables, invoice lifecycle, or seat management
- No entitlement middleware or route protection based on payment state
- No checkout flow or trial management

## Detailed Breakdown

### Free Tier (Local-First)

The default tier is fully local:

- **Vector Backend:** sqlite-vec HNSW ANN
- **Embeddings:** Candle (JinaCode 768-dim, BGE-Skill 384-dim)
- **RAM:** <50MB target
- **Distribution:** Single static binary (musl)

### Paid Tier (Cloud)

Cloud tier is accessed via `openakta auth login`:

- Vectors stored in Qdrant Cloud (Azure Marketplace)
- Embeddings generated via Cohere embed-v3-multilingual
- Rate limited via Redis-backed `tower-governor`
- Namespace isolated per user: `openakta_{user_id}`

### Self-Hosted Option

Enterprise can self-host via `External` backend configuration:

```toml
[core]
semantic_vector_backend = "external"
endpoint = "https://your-qdrant-instance.com"
api_key = "your-api-key"  # optional
```

## Implementation Evidence

- `openakta-api/` (private repo) — Cloud API implementation
- `openakta-web/` — Next.js frontend with Clerk `<SignIn />`
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend::External`
- `crates/openakta-memory/src/vector_backend.rs` — `ExternalVectorStore` stub

## Business Meaning

OPENAKTA's monetization is infrastructure-based, not billing-enforced:

- **Value delivery:** Cloud vector storage + managed embeddings
- **Access control:** Token auth via Clerk, rate limiting via Redis
- **Provisioning:** Azure Marketplace (not direct Stripe integration)

This is a lean monetization model: the product *is* the infrastructure, not a subscription wrapper around it.

## Open Ambiguities

- Pricing tiers beyond rate limits (Pro vs Free) are not codified in backend
- Enterprise self-host pricing is not implemented

## Confidence Assessment

High for cloud tier architecture. Low for traditional billing infrastructure (because it does not exist).
