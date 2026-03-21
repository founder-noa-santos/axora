# 15. Notifications and Communication Model

## Purpose

Describe how OPENAKTA currently communicates state, progress, and results across the backend.

## Executive Summary

OPENAKTA’s communication model is based on typed protobuf messages, in-process communication helpers, gRPC message streaming, and blackboard-style publication/subscription. Notifications are primarily machine-to-machine signals about task assignments, progress, blockers, workflow transitions, and results. There is no meaningful end-user notification system such as email or SMS in the backend.

## Confirmed Current State

- gRPC supports streaming messages through the collective service.
- Internal communication supports typed message helpers for assignments, results, blockers, patch artifacts, and workflow transitions.
- Blackboard V2 supports key-scoped pub/sub with diff-based updates.
- Coordinator V2 publishes result state to a shared blackboard path.
- No customer-facing notification delivery channel is currently implemented.

## Detailed Breakdown

### Transport communication

The collective service exposes:

- message submission
- message streaming
- typed orchestration content carried through proto-enforced structures

### Runtime communication

The communication layer supports:

- sending typed task assignments
- progress updates
- result submissions
- blocker alerts
- workflow transition events
- patch and validation artifacts

### Shared-state notification

Blackboard V2 turns state publication into diff-aware subscriber updates. This is useful for:

- status propagation
- result observation
- minimizing update payload size

## Implementation Evidence

- `proto/collective/v1/core.proto`
- `crates/openakta-core/src/server.rs`
- `crates/openakta-agents/src/communication.rs`
- `crates/openakta-cache/src/blackboard/v2.rs`
- `crates/openakta-cache/src/blackboard/v2_pubsub.rs`

## Business Meaning

Communication in OPENAKTA is about coordination fidelity, not user messaging. That reinforces the view that today’s backend is an execution platform whose “notifications” are internal orchestration signals.

## Open Ambiguities

- The exact future mapping from internal blackboard publications to user-visible UI events is outside current backend truth.

## Deprecated / Contradicted / Legacy Patterns

- Older string-content messaging patterns are weaker than the current typed transport direction.

## Confidence Assessment

High.
