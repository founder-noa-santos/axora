# 02. Product Surfaces and App Boundaries

## Purpose

Describe the actual runtime surfaces, interfaces, and package boundaries that make up AXORA today.

## Executive Summary

The backend reality is centered on the daemon and Rust workspace crates. The desktop shell exists as a client-facing surface, but the strongest current product boundaries are backend boundaries: gRPC transport, orchestration runtime, storage, indexing, cache/context, memory, and docs subsystems. The codebase shows a modular platform rather than a monolithic application.

## Confirmed Current State

- The daemon binary (`axora-daemon`) is the main operational entry point for the backend.
- The gRPC server in `axora-core` exposes the current network/API surface.
- `axora-agents` contains the orchestration, task, workflow, provider, retrieval, and patch runtime.
- `axora-cache` contains blackboard v2, TOON, prefix caching, diff parsing, and pruning utilities.
- `axora-indexing` contains repository map, SCIP, influence graph, Merkle state, task queue, and indexing utilities.
- `axora-storage` provides SQLite schema and partial persistence helpers.
- The Electron/Next desktop app is a shell around the backend, not the primary business logic location.

## Detailed Breakdown

### Surface: Daemon CLI and process

The daemon starts tracing, loads config, initializes SQLite, and launches the gRPC server. This is the clearest backend operational surface and the closest thing to an operator-facing entry point.

### Surface: gRPC Collective service

The protobuf service defines the external transport contract for registering agents, submitting tasks, listing agents, streaming messages, and sending orchestration messages. This is the most formal interface boundary in the repo.

### Surface: Agent orchestration runtime

The orchestration runtime is not just an internal library. It is a product surface because it defines how missions become tasks, how workers are assigned, how results are validated, and how code changes become deterministic patch receipts.

### Surface: Context and retrieval subsystems

Repository map, SCIP, influence graph, Merkle change detection, TOON, prefix caching, and blackboard v2 are backend surfaces because other layers depend on them for cost control and context assembly.

### Shared boundaries

Shared boundaries enforced in code include:

- protobuf contracts between transport participants
- typed task/result/patch structures between coordinator and workers
- blackboard publication and subscription boundaries
- provider boundary conversion from typed context to TOON

## Implementation Evidence

- `crates/axora-daemon/src/main.rs`
- `crates/axora-core/src/server.rs`
- `proto/collective/v1/core.proto`
- `crates/axora-agents/src/lib.rs`
- `crates/axora-cache/src/lib.rs`
- `crates/axora-indexing/src/lib.rs`
- `apps/desktop/`

## Business Meaning

The product is currently shaped by backend surfaces, not by end-user portals. That means the company’s operational leverage comes from runtime correctness, cost discipline, and context infrastructure. The desktop shell matters, but it is downstream of the backend core.

## Open Ambiguities

- The current gRPC service is only partially backed by persistence; some RPCs remain shallow.
- The exact contract between the desktop shell and the backend is less central to current business truth than the internal Rust and protobuf boundaries.

## Deprecated / Contradicted / Legacy Patterns

- References to removed Tauri surfaces remain in some historical material, but the current repo surface is Electron + Next.js on top of the Rust backend.
- Older architecture docs describe messaging and workflow patterns that are only partially reflected in the live runtime.

## Confidence Assessment

High.
