# ADR-054: Mission Operating Layer — Story Intake and Preparation State Machine

**Date:** 2026-03-26  
**Status:** Accepted  
**Relates to:** [story-preparation-flow.md](../aios/story-preparation-flow.md), [ADR-055-local-workflow-authority-and-cloud-boundary.md](./ADR-055-local-workflow-authority-and-cloud-boundary.md)

## Context

Mission Operating Layer (MOL) work is modeled with two persisted aggregates in the authoritative local workflow read model:

1. **Story intake** — raw request and early lifecycle metadata.
2. **Prepared story** — mission/execution cards and downstream MOL data keyed by `prepared_story_id`.

Statuses are represented as string-valued workflow states in local contracts and read models. Clients and the daemon must agree on **legal states**, **which aggregate owns which phase**, and **how transitions are performed** so that future hard gates (e.g. AB2) can reject illegal jumps and direct writes to terminal states without going through authoritative closure paths (see ADR for closure engine when added).

The local workflow runtime validates status strings, but transition hardening is still being completed across every path.

## Decision

### 1. Two phases, two status fields

- **Intake phase** — status on the story-intake aggregate. Covers discovery, classification, clarification, and triage *before or alongside* binding a prepared story.
- **Preparation / execution phase** — status on the prepared-story aggregate. Starts when a prepared story exists and tracks readiness, execution, closure handoff, and terminal outcomes for that artifact.

A single end-to-end “story” may therefore expose **two** status values at once (intake aggregate + optional preparation aggregate). Product and APIs should label them distinctly (e.g. “intake status” vs “preparation status”) to avoid “false done” confusion.

### 2. Legal states (normative)

**Intake** (`validate_story_intake_status` — superset used for capture):

| State | Meaning (normative) |
| --- | --- |
| `captured` | Intake recorded; raw request captured. |
| `classified` | Kind / routing metadata assigned. |
| `clarification_pending` | Waiting on answers before triage/preparation can proceed. |
| `triaged` | Prioritized and ready to enter or continue preparation. |
| `preparing` | Work is actively moving toward a prepared story (may overlap prepared-story creation). |
| `prepared` | Intake considered satisfied relative to preparation (see alignment with preparation below). |
| `ready` | Intake aligned with a preparation that is ready to execute (optional mirror; may duplicate preparation). |
| `executing` | Story work is in flight. |
| `closure_pending` | Mission closure / verification in progress. |
| `closed` | Terminal — no further intake-side movement. |
| `blocked` | Paused by dependency or policy. |
| `abandoned` | Terminal — intentionally dropped. |

**Preparation** (`validate_story_preparation_status` — subset):

| State | Meaning (normative) |
| --- | --- |
| `preparing` | Prepared story exists; cards/graph may still be incomplete. |
| `prepared` | Artifacts consistent enough for review; not necessarily cleared for execution. |
| `ready` | Readiness satisfied (`ready_at` may be set); compiler may schedule work. |
| `executing` | Execution plan is active for this prepared story. |
| `closure_pending` | Awaiting closure gates / verification outcomes. |
| `closed` | Terminal for the prepared story (must align with authoritative closure when enforced). |
| `blocked` | Paused. |

Preparation **does not** use `captured`, `classified`, `clarification_pending`, `triaged`, or `abandoned` — those belong to intake only.

### 3. Commands and write paths (as implemented)

| Command | Aggregate | Effect |
| --- | --- | --- |
| `capture_story_intake` | Intake | Appends a new story-intake aggregate to the local workflow read model with validated `status`. |
| `prepare_story` | Preparation | Appends a new prepared-story aggregate linked to `story_id`, with validated `status`. |
| `transition_story_preparation` | Preparation | Updates the prepared-story aggregate in the local workflow read model (`status`, optional `readiness_blockers_json`, `ready_at` when moving to `ready`). |

There is still no complete intake-transition surface for every path. That gap must be closed in the local workflow command layer, not reintroduced through hosted APIs.

### 4. Intended transitions (for enforcement — AB2)

**Normative rules** (to be enforced in the work-management command layer, not only documented):

1. **Preparation** — transitions should follow a **directed** lifecycle, allowing reasonable backward steps only where explicitly permitted (e.g. `blocked` → `ready` after unblock). Illegal jumps (e.g. `preparing` → `closed` without passing through execution and closure rules) must be rejected with a stable error code/message.
2. **Intake** — when an intake transition API exists, the same principle applies: only defined edges from the current intake state.
3. **Terminal states** — `closed` and `abandoned` are terminal for their respective aggregates; re-opening requires an explicit product decision and a dedicated command or admin path (not silent `UPDATE`).
4. **Alignment** — moving preparation to `ready` or `executing` should be consistent with intake not remaining in an earlier phase indefinitely where the product requires parity (exact rules TBD with readiness gates and AB9).

### 5. Error model (target)

Clients and the daemon should receive **machine-readable** rejection reasons for illegal transitions, for example:

- `INVALID_STORY_INTAKE_TRANSITION` — edge not allowed from current intake status.
- `INVALID_STORY_PREPARATION_TRANSITION` — edge not allowed from current preparation status.
- `STORY_PREPARATION_NOT_FOUND` — `prepared_story_id` not in workspace.
- `STORY_INTAKE_TRANSITION_NOT_SUPPORTED` — valid status string but no write path yet (until intake transitions are implemented).

Exact error transport may differ by surface, but the rejection reasons must stay machine-readable and consistent.

## Why

- Splits **early funnel** (intake) from **prepared artifact** (preparation) so that MOL graphs, coverage, and verification stay keyed to `prepared_story_id` without overloading one aggregate state field.
- Gives AB2 a single reference for **allowed edges** and **forbidden shortcuts** (e.g. skipping to `closed`).
- Aligns documentation with the local workflow implementation while calling out remaining gaps (intake updates, transition matrix).

## Consequences

- **Implementers** must not add new status literals without updating validators, proto/read models if exposed, and this ADR.
- **AB2** should implement the transition matrix against **current local workflow state**, not only validate the target string.
- **Closure** — authoritative transition to `closed` on preparation should eventually go through the closure engine (MOL roadmap: ABC1) once gates/claims/findings are wired; until then, document any temporary permissive behavior under feature flags (see A6).

## References

- `aktacode/crates/openakta-daemon/src/background/local_workflow.rs`
- `aktacode/crates/openakta-daemon/src/background/work_management_service.rs`
- `aktacode/crates/openakta-workflow/src/transition.rs`
- `aktacode/docs/aios/story-preparation-flow.md` — high-level state list
- `aktacode/crates/openakta-daemon/src/background/work_mirror.rs`
