# OpenAKTA Daemon

Local-first AI coding assistant runtime.

## Free Tier (Default)

- **Vector Backend:** sqlite-vec HNSW (384-dim semantic memory)
- **Embeddings:** Candle local (JinaCode 768-dim, BGE-Skill 384-dim)
- **RAM Usage:** <50MB target
- **Distribution:** Single static binary (musl)

## Paid Tier (Cloud)

Authenticate with cloud backend:

```bash
openakta auth login
```

Cloud tier provides:

- **Embeddings:** Cohere embed-v3-multilingual
- **Vector DB:** Qdrant Cloud (Azure Marketplace)
- **API:** `api.openakta.dev`
- **Auth:** Clerk.dev (GitHub, Google providers)

## Self-Hosted / External Backend

Configure external vector backend in `openakta.toml`:

```toml
[core]
semantic_vector_backend = "external"
endpoint = "https://your-qdrant-instance.com"
api_key = "your-api-key"  # optional
```

## Configuration

Example `openakta.toml`:

```toml
[core]
bind_address = "127.0.0.1"
port = 50051
database_path = ".openakta/openakta.db"
semantic_store_path = ".openakta/semantic-memory.db"

# Vector backend selection:
# - "sqlite_vec" (default) - local HNSW ANN
# - "sqlite_linear" - fallback/migration path
# - "external" - cloud or self-hosted
semantic_vector_backend = "sqlite_vec"

[core.retrieval]
backend = "qdrant"
qdrant_url = "http://127.0.0.1:6334"
sqlite_path = ".openakta/vectors.db"
```

## Architecture

| Component | Free Tier | Paid Tier |
|-----------|-----------|-----------|
| Vector Backend | sqlite-vec | Qdrant Cloud |
| Embeddings | Candle (local) | Cohere embed-v3 |
| Auth | None | Clerk.dev |
| Rate Limit | N/A | Pro: 100/min, Free: 10/min |

## Related

- [Business Core](../../business-core/) — Business rules and domain model
- [Active Architecture](../../docs/active_architecture/) — Technical architecture docs
