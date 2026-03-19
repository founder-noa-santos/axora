# 09. Integrations and External Dependencies

## Purpose

Describe the external systems and technical boundaries that now matter to AXORA.

## Executive Summary

AXORA’s external dependency model is now anchored around two hard boundaries: cloud LLM APIs for reasoning and native MCP/gRPC for local tool execution. Around those boundaries sit local persistence, compressed transport, structural indexing, and runtime-managed governance services.

## Core Integrations

| Dependency | Current role |
| --- | --- |
| `axora-cli` | Mission-first entrypoint for runtime bootstrap |
| `tonic` / gRPC | Collective service and MCP service transport |
| `prost` | Typed protobuf contracts for orchestration and tooling |
| `reqwest` | Live HTTP transport to Anthropic and OpenAI |
| Anthropic API | Live reasoning backend |
| OpenAI API | Live reasoning backend |
| `tokio` | Async runtime for coordinator, MCP, and ReAct execution |
| `rusqlite` / SQLite | Local persistence for operational and memory data |

## Security and Tool Boundary

| Dependency | Current role |
| --- | --- |
| MCP ToolService | Secure native boundary for file, diff, AST, graph, and execution tools |
| CapabilityPolicy | Scope, action, and timeout constraints |
| AuditEvent stream | Tool-execution audit trail |
| local filesystem | Workspace substrate, but accessed through MCP for sensitive operations |

## Compression and Context Stack

| Dependency | Current role |
| --- | --- |
| TOON serializer | Canonical compact context payload |
| MetaGlyph commands | Symbolic prompt/control compression layer |
| Prefix cache | Local prefix reuse and token saving |
| repository map | Low-token codebase navigation |
| influence graph | Dependency-aware retrieval and pruning |

## Memory and Governance Dependencies

| Dependency | Current role |
| --- | --- |
| semantic store | Local embedded knowledge store |
| procedural skill store | Filesystem-backed reusable skill artifacts seeded on first run |
| episodic store | SQLite chronological reasoning/action log |
| Merkle tree | Change detection for LivingDocs |
| doc reconciler | Governance/diff engine for documentation freshness |

## Implementation Evidence

- `crates/axora-daemon/src/main.rs`
- `crates/axora-cli/src/main.rs`
- `crates/axora-core/src/bootstrap.rs`
- `crates/axora-core/src/config.rs`
- `crates/axora-agents/src/provider_transport.rs`
- `crates/axora-agents/src/mcp_client.rs`
- `crates/axora-mcp-server/src/lib.rs`
- `crates/axora-cache/src/toon.rs`
- `crates/axora-cache/src/prefix_cache.rs`
- `crates/axora-indexing/src/merkle.rs`
- `crates/axora-daemon/src/services.rs`

## Business Meaning

The business now depends on reliable cloud reasoning and a hardened local execution boundary, not on synthetic provider simulation or manual operator setup. The key operational risks live at the provider, MCP, storage, retrieval, and governance layers.

## Open Ambiguities

- The MCP surface is now broader and native, but still intentionally opinionated rather than fully open-ended.
- Some semantic-memory ingestion still uses lightweight local embeddings rather than a richer production embedding model.

## Confidence Assessment

High.
