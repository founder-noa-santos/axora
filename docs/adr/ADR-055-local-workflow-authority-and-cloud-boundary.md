# ADR-055: Local Workflow Authority and Cloud Boundary

**Date:** 2026-03-27  
**Status:** Accepted  
**Supersedes assumptions in:** `docs/aios/mission-operating-layer.md`, `docs/active_architecture/01_CORE_ARCHITECTURE.md`

## Context

OpenAkta drifted into an invalid authority model.

- `openakta-api` accumulated work-management persistence and business logic.
- The daemon treated hosted workflow data as canonical and local state as a mirror.
- Server builds depended on `aktacode` crates for application behavior, which blurred the product boundary.
- Paid/cloud paths risked becoming a hidden excuse to host user workflow state.

That shape violates the product requirement: OpenAkta is local-first in structure, not only in marketing.

## Decision

Workflow authority is local-only.

### Ownership

- The local runtime owns conversations.
- The local runtime owns tasks, plans, storyboard state, and workflow transitions.
- The local runtime owns orchestration and tool-use decisions.
- The local runtime persists authoritative workflow state in local storage under the workspace.
- `openakta-api` is not allowed to own canonical workflow state.

### Cloud boundary

The cloud is limited to infrastructure-facing services:

- provider proxy and routing
- managed embeddings
- quotas, billing, and entitlements
- OAuth callbacks and webhooks where local execution is impossible
- optional provider-facing integrations

Paid plans do not move conversations, tasks, plans, storyboard state, or orchestration into the cloud.

### Code-sharing rule

Shared crates may contain:

- protocol contracts
- transport DTOs
- neutral low-level utilities

Shared crates may not contain:

- application authority
- workflow orchestration shortcuts
- server ownership of local workflow semantics

`openakta-api` must not depend on desktop or daemon orchestration crates to implement server behavior.

## Rationale

- Local workflow authority preserves offline operation, privacy, and deployment independence.
- One runtime must own workflow semantics; hybrid authority guarantees drift.
- Narrow cloud scope keeps paid features additive without turning the API into a second product backend.
- Hard crate boundaries are required because runtime authority follows the build graph.

## Consequences

### Positive

- The daemon becomes the single source of truth for work management.
- Compatibility behavior is explicit instead of pretending cloud-hosted workflow is still supported.
- Architecture reviews can reject boundary violations mechanically.

### Required follow-through

- Docs must describe the daemon as authoritative for workflow.
- New workflow features must land in local crates first.
- Any proposal to reintroduce server workflow semantics requires an ADR proving why local execution is impossible.

### Migration note

Brownfield compatibility code may exist temporarily, but it is not architectural precedent. Legacy server workflow surfaces are deprecation scaffolding only.

## References

- `crates/openakta-daemon/src/background/work_management_service.rs`
- `crates/openakta-daemon/src/background/work_mirror.rs`
- `crates/openakta-daemon/src/background/local_workflow.rs`
- `crates/openakta-workflow/src/lib.rs`
- `docs/aios/mission-operating-layer.md`
