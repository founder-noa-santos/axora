# ADR-054: Mission Operating Layer — Story Intake and Preparation State Machine

**Date:** 2026-03-26  
**Status:** Accepted  
**Relates to:** [story-preparation-flow.md](../aios/story-preparation-flow.md), `openakta-api/src/work_management.rs`, migration `0005_mission_operating_layer.sql`

## Context

Mission Operating Layer (MOL) work is modeled with two persisted aggregates:

1. **Story intake** (`wm_story_intakes`) — raw request and early lifecycle metadata.
2. **Prepared story** (`wm_story_preparations`) — mission/execution cards and downstream MOL data keyed by `prepared_story_id`.

Statuses are stored as `TEXT` columns. Clients and the daemon must agree on **legal states**, **which aggregate owns which phase**, and **how transitions are performed** so that future hard gates (e.g. AB2) can reject illegal jumps and direct writes to terminal states without going through authoritative closure paths (see ADR for closure engine when added).

Today, the API validates status strings against allow-lists but does **not** enforce a transition matrix or require intake updates to go through a dedicated command.

## Decision

### 1. Two phases, two status columns

- **Intake phase** — status on `wm_story_intakes.status`. Covers discovery, classification, clarification, and triage *before or alongside* binding a prepared story.
- **Preparation / execution phase** — status on `wm_story_preparations.status`. Starts when a prepared story row exists and tracks readiness, execution, closure handoff, and terminal outcomes for that artifact.

A single end-to-end “story” may therefore expose **two** status values at once (intake row + optional preparation row). Product and APIs should label them distinctly (e.g. “intake status” vs “preparation status”) to avoid “false done” confusion.

### 2. Legal states (normative)

**Intake** (`validate_story_intake_status` — superset used for capture):

| State | Meaning (normative) |
| --- | --- |
| `captured` | Row created; raw request recorded. |
| `classified` | Kind / routing metadata assigned. |
| `clarification_pending` | Waiting on answers before triage/preparation can proceed. |
| `triaged` | Prioritized and ready to enter or continue preparation. |
| `preparing` | Work is actively moving toward a prepared story (may overlap preparation row creation). |
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
| `preparing` | Prepared story row exists; cards/graph may still be incomplete. |
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
| `capture_story_intake` | Intake | `INSERT` into `wm_story_intakes` with validated `status`. |
| `prepare_story` | Preparation | `INSERT` into `wm_story_preparations` linked to `story_id`, with validated `status`. |
| `transition_story_preparation` | Preparation | `UPDATE` `wm_story_preparations` (`status`, optional `readiness_blockers_json`, `ready_at` when moving to `ready`). |

There is **no** `UPDATE` on `wm_story_intakes` in the current API surface. Intake status after insert is therefore only whatever was written at `capture_story_intake`, until a future command (or deliberate schema evolution) adds an intake transition path.

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

Exact JSON shape stays aligned with existing `ApiJsonError` patterns in `openakta-api`.

## Why

- Splits **early funnel** (intake) from **prepared artifact** (preparation) so that MOL graphs, coverage, and verification stay keyed to `prepared_story_id` without overloading one status column.
- Gives AB2 a single reference for **allowed edges** and **forbidden shortcuts** (e.g. skipping to `closed`).
- Aligns documentation with the **actual** validators in `work_management.rs`, while calling out gaps (intake updates, transition matrix).

## Consequences

- **Implementers** must not add new status literals without updating validators, proto/read models if exposed, and this ADR.
- **AB2** should implement the transition matrix against **current DB state** inside `transition_story_preparation` (and any new intake transition command), not only validate the target string.
- **Closure** — authoritative transition to `closed` on preparation should eventually go through the closure engine (MOL roadmap: ABC1) once gates/claims/findings are wired; until then, document any temporary permissive behavior under feature flags (see A6).

## References

- `openakta-api/src/work_management.rs` — `capture_story_intake`, `prepare_story`, `transition_story_preparation`, `validate_story_intake_status`, `validate_story_preparation_status`
- `aktacode/docs/aios/story-preparation-flow.md` — high-level state list
- `openakta-api/migrations/0005_mission_operating_layer.sql` — `wm_story_intakes`, `wm_story_preparations`
