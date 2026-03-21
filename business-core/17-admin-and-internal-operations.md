# 17. Admin and Internal Operations

## Purpose

Document the internal/operator workflows that are currently supported by the backend.

## Executive Summary

OPENAKTA currently supports a meaningful internal operations model even though it lacks customer administration flows. Operators can configure and start the daemon, manage runtime config, register agents, observe task/message behavior through gRPC, and run mission execution through the coordinated runtime. Internal operations matter more than customer admin in the current stage of the product.

## Confirmed Current State

- Daemon startup is parameterized by CLI/config.
- The gRPC server exposes agent and task operations.
- Worker pools and health/heartbeat infrastructure exist in the agent runtime.
- Queue and blackboard infrastructure support internal coordination.
- Tests cover key backend operations such as server startup and mission execution.

## Detailed Breakdown

### Operator responsibilities visible in code

- choose bind address, port, DB path, and debug level
- initialize the daemon
- register or connect agents
- submit tasks or missions
- observe runtime message streams

### Internal runtime operations

- worker slot management
- task dispatch and completion handling
- workflow transitions
- queue checkout and release
- blackboard publication
- retrieval/indexing support

### Missing internal ops layers

- formal admin dashboard logic in the backend
- RBAC for operators
- audit-log administration workflows

## Implementation Evidence

- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-core/src/config.rs`
- `crates/openakta-core/src/server.rs`
- `crates/openakta-agents/src/worker_pool.rs`
- `crates/openakta-agents/src/heartbeat.rs`
- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-core/tests/integration.rs`

## Business Meaning

OPENAKTA’s current operational center of gravity is internal administration of an execution system, not customer self-service operations. This is typical of an infrastructure-heavy product in an earlier commercialization stage.

## Open Ambiguities

- The eventual boundary between operator workflows and customer-visible workflows is still open.

## Deprecated / Contradicted / Legacy Patterns

- Some historical docs imply richer internal governance than the live backend exposes.

## Confidence Assessment

Medium.
