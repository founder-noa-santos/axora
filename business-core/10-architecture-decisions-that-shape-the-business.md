# 10. Architecture Decisions That Shape the Business

## Purpose

Record the architecture decisions that define OPENAKTA's product behavior and business model.

## Executive Summary

OPENAKTA's business is shaped by its **local-first, cloud-upgrade** architecture:

1. **sqlite-vec as default local backend** — Free tier is fully local, no external dependencies
2. **Qdrant Cloud (Azure Marketplace) for paid tier** — Managed cloud, not self-hosted
3. **Candle embeddings local, Cohere embeddings cloud** — Dual embedding strategy
4. **Clerk.dev auth with GitHub/Google only** — Simplified identity model
5. **Rate limiting via Redis + tower-governor** — Pro: 100/min, Free: 10/min
6. **VectorStore trait abstraction** — Future migration path (Turbopuffer) via trait compatibility

## Active Decisions

### Decision: sqlite-vec is the default local vector backend

**Rationale:**
- Pure SQLite extension (no separate process)
- HNSW ANN for production performance
- <100MB RAM for 100K vectors
- Matches Rust/SQLite stack

**Alternatives rejected:**
- LanceDB (not current architecture)
- pgvector (requires Postgres)
- Chroma (Python dependency)

### Decision: Qdrant Cloud (Azure Marketplace) for paid tier

**Rationale:**
- Managed infrastructure (no self-hosting burden)
- Azure Marketplace provisioning channel
- Namespace isolation: `openakta_{user_id}`
- Production-proven at scale

**Alternatives rejected:**
- Self-hosted Qdrant as official path (operational burden)
- Turbopuffer as current choice (future migration only)

### Decision: Candle local, Cohere cloud for embeddings

**Rationale:**
- **Local:** JinaCode 768-dim, BGE-Skill 384-dim via Candle (no external API)
- **Cloud:** Cohere embed-v3-multilingual via Azure Foundry

**Alternatives rejected:**
- Voyage code-3 (not implemented)
- Local-only embeddings (limits cloud tier value)

### Decision: Clerk.dev with GitHub/Google only

**Rationale:**
- Simplified identity model
- `<SignIn />` component for Next.js
- Token-based auth for CLI (`openakta auth login`)

**Alternatives rejected:**
- Custom auth system (operational burden)
- Additional identity providers (scope creep)

### Decision: Rate limiting via Redis + tower-governor

**Rationale:**
- Pro: 100/min, Free: 10/min
- Redis-backed for distributed enforcement
- `tower-governor` middleware for Axum

### Decision: VectorStore trait for backend abstraction

**Rationale:**
- Trait surface: `upsert`, `search`, `delete`, `count`, `scan_for_pruning`, `backend_id`
- Enables future migration (Turbopuffer) without breaking changes
- Supports local (sqlite-vec), fallback (SqliteLinear), and external (Qdrant) backends

### Decision: Local-first with cloud upgrade path

**Rationale:**
- Free tier is fully functional offline
- Cloud tier is opt-in via `openakta auth login`
- Single binary distribution (musl static linking)

## Implementation Evidence

- `crates/openakta-memory/src/vector_backend.rs` — `VectorStore` trait, backends
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend` enum
- `openakta-api/` — Cloud API (private: Rust + Axum)
- `openakta-web/` — Next.js + Clerk frontend
- `crates/openakta-daemon/README.md` — Local/cloud tier documentation

## Business Meaning

These decisions optimize OPENAKTA for:

- **Low barrier to entry:** Free tier works offline, no signup required
- **Clear upgrade path:** Cloud tier via Azure Marketplace with managed infrastructure
- **Operational simplicity:** No self-hosting burden for cloud tier
- **Future flexibility:** `VectorStore` trait enables backend migration

## Open Ambiguities

- **Turbopuffer:** Future migration path only, not current architecture
- **Enterprise self-host:** Available via `External` config, but not the official paid path

## Confidence Assessment

High. These decisions reflect implemented architecture and current business model.
