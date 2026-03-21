# 11. Current Source-of-Truth Map

## Purpose

Show where the most reliable business and operational truth lives in the codebase today.

## Executive Summary

The fastest way to understand OPENAKTA is to start with the daemon entry point, the protobuf contract, the V2 coordinator, patch protocol, provider layer, cache/context infrastructure, and indexing subsystem. Older docs and business-rule files are useful mainly as historical context and should not be treated as authoritative without code support.

## Confirmed Current State

### Highest-value source-of-truth paths

| Area | Primary paths |
| --- | --- |
| Daemon and bootstrap | `crates/openakta-daemon/src/main.rs`, `crates/openakta-core/src/config.rs` |
| gRPC/API contract | `proto/collective/v1/core.proto`, `crates/openakta-core/src/server.rs` |
| Runtime orchestration | `crates/openakta-agents/src/coordinator/v2.rs`, `crates/openakta-agents/src/decomposer.rs`, `crates/openakta-agents/src/communication.rs` |
| Diff/patch safety | `crates/openakta-agents/src/patch_protocol.rs`, `crates/openakta-agents/src/result_contract.rs` |
| Provider configuration | `crates/openakta-core/src/config_resolve.rs`, `crates/openakta-agents/src/provider_transport.rs` |
| Model registry | `crates/openakta-agents/src/model_registry/mod.rs`, `crates/openakta-agents/src/provider_registry.rs` |
| Routing logic | `crates/openakta-agents/src/routing/mod.rs`, `crates/openakta-agents/src/token_budget.rs` |
| Secret resolution | `crates/openakta-core/src/config_resolve.rs::resolve_secret_ref` |
| Context and blackboard | `crates/openakta-cache/src/blackboard/v2.rs`, `crates/openakta-cache/src/toon.rs`, `crates/openakta-cache/src/prefix_cache.rs` |
| Indexing and retrieval | `crates/openakta-indexing/src/scip.rs`, `crates/openakta-indexing/src/influence.rs`, `crates/openakta-indexing/src/merkle.rs`, `crates/openakta-indexing/src/task_queue.rs`, `crates/openakta-agents/src/retrieval.rs` |
| Persistence schema | `crates/openakta-storage/migrations/0001_init.sql`, `crates/openakta-indexing/migrations/0002_task_queue.sql` |
| Runtime tests | `crates/openakta-agents/tests/coordinator_v2.rs`, `crates/openakta-core/tests/integration.rs` |

## Detailed Breakdown

### Where to learn the live runtime

Start with:

1. `proto/collective/v1/core.proto`
2. `crates/openakta-agents/src/coordinator/v2.rs`
3. `crates/openakta-agents/src/patch_protocol.rs`
4. `crates/openakta-agents/src/provider.rs`

### Where to learn the state model

Look at:

- `crates/openakta-storage/migrations/0001_init.sql`
- `crates/openakta-indexing/src/task_queue.rs`
- `crates/openakta-cache/src/blackboard/v2.rs`

### Where not to start

Avoid starting with:

- `docs/business_rules/`
- broad architecture notes that are not cross-checked with runtime code
- synthetic benchmark fixtures that use auth/payment examples

## Implementation Evidence

- paths listed above are themselves the evidence

## Business Meaning

This map reduces onboarding time for senior contributors and protects the team from building on stale assumptions. OPENAKTA’s current truth is distributed across runtime crates, not concentrated in business docs.

## Open Ambiguities

- Some truth is split between legacy and V2 paths, especially in coordination and memory/state sharing.

## Deprecated / Contradicted / Legacy Patterns

- `docs/business_rules/` and portions of `docs/active_architecture/` are not current business truth unless corroborated elsewhere.

## Confidence Assessment

High.
