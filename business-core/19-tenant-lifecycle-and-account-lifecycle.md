# 19. Tenant Lifecycle and Account Lifecycle

## Purpose

Record what the backend currently does and does not implement around tenants, accounts, and lifecycle management.

## Executive Summary

There is no real tenant or account lifecycle in the backend today. The repository does not implement organizations, workspaces as tenant objects, user membership models, account activation, suspension, deletion, or account-scoped entitlements. The closest analogous lifecycle is the lifecycle of agents, tasks, sessions, and runtime state.

## Confirmed Current State

- `Agent`, `Task`, `Message`, and `Session` tables exist in SQLite schema.
- Agents can be registered and unregistered.
- Tasks move through statuses.
- “Workspace” in live backend code usually refers to a repository/workspace root path for code execution, not to a tenant object.
- No user-account or tenant-account tables are present in the live schema.

## Detailed Breakdown

### What lifecycle does exist

| Lifecycle | Current states or operations |
| --- | --- |
| Agent | register, active/listed, unregister |
| Task | pending, assigned/in progress, completed/failed/cancelled |
| Queue task | pending, in progress, completed, failed |
| Session | schema exists, but broader account meaning is unclear |

### What lifecycle does not exist

- sign up
- login
- membership
- organization creation
- invite acceptance
- account suspension
- plan downgrade or upgrade
- tenant deletion

## Implementation Evidence

- `crates/openakta-storage/migrations/0001_init.sql`
- `proto/collective/v1/core.proto`
- `crates/openakta-core/src/server.rs`
- `crates/openakta-agents/src/task.rs`
- `crates/openakta-indexing/src/task_queue.rs`
- `crates/openakta-agents/src/coordinator/v2.rs`

## Business Meaning

This backend is not yet an account platform. Treating it as one would create incorrect assumptions about readiness for customer self-serve adoption. The current lifecycle truth is operational and agent-centric.

## Open Ambiguities

- The `sessions` table may eventually support richer account semantics, but it does not currently establish a user lifecycle.

## Deprecated / Contradicted / Legacy Patterns

- Historical docs implying tenancy, policy, or account lifecycle should be read as future-facing or stale, not current backend truth.

## Confidence Assessment

Low.
