# 09. Integrations and External Dependencies

## Purpose

Describe the external systems and technical boundaries that OPENAKTA depends on.

## Executive Summary

OPENAKTA's external dependency model is anchored around two tiers:

1. **Local Tier (Free, Default):** sqlite-vec + Candle embeddings, fully offline-capable
2. **Cloud Tier (Paid):** Qdrant Cloud (Azure Marketplace) + Cohere embeddings, accessed via `api.openakta.dev`

The local tier has no mandatory external dependencies. The cloud tier depends on Azure Marketplace provisioning, Clerk.dev auth, and Cohere embeddings via Azure Foundry.

## Core Integrations

### Local Tier (Default)

| Dependency | Current Role | Tier |
|------------|--------------|------|
| **sqlite-vec** | Vector storage with HNSW ANN | Local |
| **Candle** | Embedding inference (JinaCode, BGE-Skill) | Local |
| **rusqlite** | SQLite bindings | Local |
| **tokio** | Async runtime | Local |
| **tonic / gRPC** | Internal service transport | Local |
| **MCP** | Tool execution boundary | Local |

### Cloud Tier (Paid)

| Dependency | Current Role | Tier |
|------------|--------------|------|
| **Qdrant Cloud (Azure Marketplace)** | Managed vector database | Cloud |
| **Cohere embed-v3-multilingual** | Cloud embeddings | Cloud |
| **Clerk.dev** | Authentication (GitHub, Google) | Cloud |
| **Azure Foundry Serverless** | Inference access for Cohere | Cloud |
| **Redis** | Rate limiting backend | Cloud |
| **Postgres** | User/quota tracking | Cloud |
| **api.openakta.dev** | Cloud API (Rust + Axum) | Cloud |

## Security and Tool Boundary

| Dependency | Current Role |
|------------|--------------|
| **MCP ToolService** | Secure boundary for file, diff, AST, graph, execution tools |
| **CapabilityPolicy** | Scope, action, and timeout constraints |
| **AuditEvent stream** | Tool-execution audit trail |
| **local filesystem** | Workspace substrate, accessed through MCP for sensitive operations |

## Memory Architecture

| Dependency | Current Role | Tier |
|------------|--------------|------|
| **Semantic Store** | sqlite-vec vectors (384-dim) | Local |
| **Episodic Store** | SQLite chronological log | Local |
| **Procedural Store** | `SKILL.md` files on filesystem | Local |
| **Ebbinghaus lifecycle** | Background pruning model | Local |

## Cloud API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/v1/validate` | POST | Validate token and quota |
| `/v1/:user_id/upsert` | POST | Upsert vectors to Qdrant |
| `/v1/:user_id/search` | POST | Search vectors in Qdrant |

## CLI Auth Flow

```bash
openakta auth login
# → Token stored in keyring
# → Daemon downloads cloud backend config
```

## Rate Limiting

| Tier | Limit | Backend |
|------|-------|---------|
| **Pro** | 100/min | Redis + `tower-governor` |
| **Free** | 10/min | Redis + `tower-governor` |

## Implementation Evidence

- `crates/openakta-memory/src/vector_backend.rs` — `VectorStore` trait, sqlite-vec backend
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend` enum
- `openakta-api/` — Cloud API (private: Rust + Axum)
- `openakta-web/` — Next.js + Clerk frontend
- `crates/openakta-mcp-server/src/lib.rs` — MCP tool boundary

## Business Meaning

OPENAKTA's architecture is **local-first by default**, with cloud tier as an opt-in upgrade:

- **Local tier:** No external dependencies, fully functional offline
- **Cloud tier:** Managed infrastructure via Azure Marketplace, not self-hosted Qdrant

The business depends on:
1. **Local execution capability** (sqlite-vec + Candle)
2. **Cloud infrastructure** (Qdrant Cloud via Azure, Cohere via Azure Foundry)
3. **Auth bridge** (Clerk.dev for GitHub/Google sign-in)

## Open Ambiguities

- **Turbopuffer:** Future migration path via `VectorStore` trait compatibility, not current architecture
- **Self-hosted Qdrant:** Available via `External` backend config, but not the official paid path

## Confidence Assessment

High. This document reflects current local-first architecture with cloud tier via Azure Marketplace.
