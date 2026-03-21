# 13. Gaps, Risks, and Ambiguities

## Purpose

Identify the real remaining risks after validating the current implementation.

## Executive Summary

The prior risks around synthetic provider execution and missing memory architecture are no longer the main story. The runtime now has live providers, MCP, dual-thread ReAct, tripartite memory, and daemonized doc sync. The remaining risks are narrower: legacy-path overlap, limited MCP tool breadth, lightweight semantic ingestion in some paths, and incomplete automation around governance outputs.

## Current Risks

| Risk | Why it matters |
| --- | --- |
| Legacy runtime overlap | Old coordinator and state abstractions still coexist with the stronger V2 path |
| Narrow MCP tool catalog | The secure boundary is real, but the default tool surface is still small |
| Lightweight semantic embeddings in doc sync | Some semantic ingestion uses a simple local embedding strategy, which may limit retrieval quality |
| Partial governance automation | LivingDocs can detect and stage doc updates, but does not yet auto-open PRs by default |
| Retrieval/index freshness | Context quality still depends on reliable change detection and indexing freshness |
| `WireProfile` adoption | Ensure all new providers use `WireProfile` for transport and `ProviderKind` for telemetry (R4 implemented 2026-03-20) |

## Resolved Risks

- Synthetic provider execution is no longer the primary runtime claim.
- Tripartite memory and memory pruning are no longer architectural gaps.
- MCP-backed tool sandboxing is now a real system boundary, not a research note.
- Hardcoded token limits are replaced by dynamic model metadata (as of 2026-03-20 refactor).
- Environment variable fallbacks are fully purged; file-based secrets are enforced.
- Model registry provides authoritative metadata for routing and budgeting.
- `ProviderKind` conflation resolved: `WireProfile` now drives transport, `ProviderKind` only for telemetry (R4 completed 2026-03-20).

## Business-Layer Gaps

| Gap | Current status |
| --- | --- |
| User accounts | Not part of the current backend truth |
| Tenant/workspace ownership | Still thin as a business model |
| Billing and plan enforcement | Not implemented |
| Customer onboarding workflows | Not implemented as a backend state machine |

## Ambiguities

- How much of the legacy stack will be retired versus maintained for compatibility is still open.
- The long-term semantic-memory quality plan is not fully inferable from the current embedding strategy alone.

## Implementation Evidence

- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/provider_transport.rs`
- `crates/openakta-agents/src/react.rs`
- `crates/openakta-agents/src/model_registry/mod.rs`
- `crates/openakta-agents/src/routing/mod.rs`
- `crates/openakta-agents/src/token_budget.rs`
- `crates/openakta-core/src/config_resolve.rs`
- `crates/openakta-core/src/bootstrap.rs`
- `crates/openakta-mcp-server/src/lib.rs`
- `crates/openakta-daemon/src/services.rs`
- `crates/openakta-docs/src/reconciler.rs`

## Business Meaning

OPENAKTA’s remaining uncertainty is no longer “can the platform run end-to-end?” It is “how far and how fast to harden and commercialize the now-real execution core.”

## Confidence Assessment

High.
