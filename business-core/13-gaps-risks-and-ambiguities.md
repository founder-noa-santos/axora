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

## Resolved Risks

- Synthetic provider execution is no longer the primary runtime claim.
- Tripartite memory and memory pruning are no longer architectural gaps.
- MCP-backed tool sandboxing is now a real system boundary, not a research note.

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

- `crates/axora-agents/src/coordinator/v2.rs`
- `crates/axora-agents/src/provider_transport.rs`
- `crates/axora-agents/src/react.rs`
- `crates/axora-mcp-server/src/lib.rs`
- `crates/axora-daemon/src/services.rs`
- `crates/axora-docs/src/reconciler.rs`

## Business Meaning

AXORA’s remaining uncertainty is no longer “can the platform run end-to-end?” It is “how far and how fast to harden and commercialize the now-real execution core.”

## Confidence Assessment

High.
