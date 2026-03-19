# 11. Current Source-of-Truth Map

## Purpose

Show where the most reliable business and operational truth lives in the codebase today.

## Executive Summary

The fastest way to understand AXORA is to start with the daemon entry point, the protobuf contract, the V2 coordinator, patch protocol, provider layer, cache/context infrastructure, and indexing subsystem. Older docs and business-rule files are useful mainly as historical context and should not be treated as authoritative without code support.

## Confirmed Current State

### Highest-value source-of-truth paths

| Area | Primary paths |
| --- | --- |
| Daemon and bootstrap | `crates/axora-daemon/src/main.rs`, `crates/axora-core/src/config.rs` |
| gRPC/API contract | `proto/collective/v1/core.proto`, `crates/axora-core/src/server.rs` |
| Runtime orchestration | `crates/axora-agents/src/coordinator/v2.rs`, `crates/axora-agents/src/decomposer.rs`, `crates/axora-agents/src/communication.rs` |
| Diff/patch safety | `crates/axora-agents/src/patch_protocol.rs`, `crates/axora-agents/src/result_contract.rs` |
| Provider/model boundary | `crates/axora-agents/src/provider.rs` |
| Context and blackboard | `crates/axora-cache/src/blackboard/v2.rs`, `crates/axora-cache/src/toon.rs`, `crates/axora-cache/src/prefix_cache.rs` |
| Indexing and retrieval | `crates/axora-indexing/src/scip.rs`, `crates/axora-indexing/src/influence.rs`, `crates/axora-indexing/src/merkle.rs`, `crates/axora-indexing/src/task_queue.rs`, `crates/axora-agents/src/retrieval.rs` |
| Persistence schema | `crates/axora-storage/migrations/0001_init.sql`, `crates/axora-indexing/migrations/0002_task_queue.sql` |
| Runtime tests | `crates/axora-agents/tests/coordinator_v2.rs`, `crates/axora-core/tests/integration.rs` |

## Detailed Breakdown

### Where to learn the live runtime

Start with:

1. `proto/collective/v1/core.proto`
2. `crates/axora-agents/src/coordinator/v2.rs`
3. `crates/axora-agents/src/patch_protocol.rs`
4. `crates/axora-agents/src/provider.rs`

### Where to learn the state model

Look at:

- `crates/axora-storage/migrations/0001_init.sql`
- `crates/axora-indexing/src/task_queue.rs`
- `crates/axora-cache/src/blackboard/v2.rs`

### Where not to start

Avoid starting with:

- `docs/business_rules/`
- broad architecture notes that are not cross-checked with runtime code
- synthetic benchmark fixtures that use auth/payment examples

## Implementation Evidence

- paths listed above are themselves the evidence

## Business Meaning

This map reduces onboarding time for senior contributors and protects the team from building on stale assumptions. AXORA’s current truth is distributed across runtime crates, not concentrated in business docs.

## Open Ambiguities

- Some truth is split between legacy and V2 paths, especially in coordination and memory/state sharing.

## Deprecated / Contradicted / Legacy Patterns

- `docs/business_rules/` and portions of `docs/active_architecture/` are not current business truth unless corroborated elsewhere.

## Confidence Assessment

High.
