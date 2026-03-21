# 03. Actors, Roles, and Permissions

## Purpose

Document the actual actors and role semantics currently implemented in the backend.

## Executive Summary

OPENAKTA currently implements system and agent roles, not a mature end-user permission model. The primary actors in code are coordinator, workers, planner/executor/reviewer/resolver-style task roles, daemon operators, and gRPC clients. Permission logic is mostly capability- and task-type-oriented rather than tenant- or customer-role-oriented. There is no implemented RLS, org membership, or account-based authorization system in the backend.

## Confirmed Current State

- `Agent` entities have `id`, `name`, `role`, and `status` in the protobuf contract.
- Agent implementations expose a `role()` method and are categorized by runtime responsibilities.
- The decomposition and workflow layers use role semantics such as planner, executor, reviewer, and resolver.
- Task assignments carry target files, target symbols, and capabilities rather than customer-facing permissions.
- The strongest current access control is structural:
  - typed message validation
  - diff-only publication guard for code-edit tasks
  - task-type constraints
  - workflow and queue coordination rules

## Detailed Breakdown

### Implemented actors

| Actor | Current meaning |
| --- | --- |
| Daemon operator | Starts and configures the backend process |
| gRPC client | Registers agents, submits tasks, streams messages |
| Coordinator | Decomposes missions, dispatches tasks, validates outputs, publishes results |
| Worker agent | Executes assigned tasks under coordinator control |
| Planner / Executor / Reviewer / Resolver | Workflow roles used in decomposition and workflow graph semantics |
| Retrieval/indexing subsystems | Supporting backend actors, not human users |

### Role model in code

The role model is primarily execution-oriented. Roles affect:

- suggested task ownership
- task typing
- workflow transitions
- capability matching

The code does not implement:

- customer roles
- organization admin/member relationships
- route-level end-user authorization
- row-level security or tenant policy enforcement

### Permissions that do exist

The real enforced permissions today are closer to protocol guards than product entitlements:

- typed orchestration messages must use the correct typed fields
- code modification results must pass diff validation
- patch application can fail on invalid, conflicting, or stale-base changes
- task queue checkout ensures single-assignee execution

## Implementation Evidence

- `proto/collective/v1/core.proto`
- `crates/openakta-agents/src/agent.rs`
- `crates/openakta-agents/src/decomposer.rs`
- `crates/openakta-agents/src/graph.rs`
- `crates/openakta-agents/src/communication.rs`
- `crates/openakta-agents/src/result_contract.rs`
- `crates/openakta-indexing/src/task_queue.rs`

## Business Meaning

This is a system built around coordinating machine actors. Any future human-facing permission system will need to coexist with, and not overwrite, this lower-level execution role model. The current business truth is that correctness and safety are enforced through execution contracts, not user auth.

## Open Ambiguities

- The long-term mapping between human operators and agent roles is not encoded yet.
- There may eventually be internal admin/operator distinctions in the desktop app, but that is not a backend-enforced truth today.

## Deprecated / Contradicted / Legacy Patterns

- Auth- and permission-themed examples appear in tests, docs, and traceability fixtures, but they do not reflect a live backend auth system.
- Historical docs mention policy-style enforcement that is not implemented in server/storage/runtime code.

## Confidence Assessment

Medium. Agent roles and system actors are clear; customer-facing permissions are largely absent rather than explicitly modeled.
