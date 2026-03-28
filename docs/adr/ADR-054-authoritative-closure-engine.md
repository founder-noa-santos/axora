# ADR-054: Authoritative Closure Engine

**Date:** 2026-03-26  
**Status:** Accepted

## Context

Mission Operating Layer (MOL) closure must be **authoritative**: a prepared story (and its upstream story) should reach a terminal **closed** state only when defined invariants hold across gates, claims, verification, and handoffs. Ad-hoc writes or “success” signals that skip this evaluation produce **false closure**—rich data without hard guarantees.

Today, canonical objects and reads exist (`ClosureReport`, gates, claims, findings, handoffs in `aktacode/proto/work/v1/work.proto`), but the product needs a single **decision surface** that decides whether closure is allowed and performs the **one** state transition to closed when it is.

This ADR complements the intake/preparation state-machine ADR (A7): that document governs **how** work enters preparation; this document governs **how** preparation (and the story) may exit to **closed**.

## Decision

Introduce an **authoritative closure engine** implemented in the local workflow runtime with the following contract.

### Inputs (must be evaluated together)

The engine loads and evaluates **only** persisted, scoped facts for a given `workspace_id` + `prepared_story_id` (and associated `story_id`):

| Input | Role | Representative types |
| ----- | ---- | -------------------- |
| **Closure gates** | Policy outcomes per gate type (coverage, verification, handoff, review, reliability, documentation alignment, etc.) | `ClosureGate` |
| **Closure claims** | Evidence-backed completion claims per requirement / work item | `ClosureClaim` |
| **Verification findings** | Independent proof results linked to verification runs and requirements | `VerificationFinding` |
| **Handoff contracts** | Cross–work-item commitments that must be satisfied before closure | `HandoffContract` |

**Supporting context** (not substitutes for the four inputs): requirement graph, acceptance checks, and requirement coverage constrain *what must be true* and *which gates apply*; the engine uses them to interpret gate types and to reject closure when mandatory checks or coverage are missing. They do **not** replace evaluation of gates, claims, findings, and handoffs.

### Output

- **Single transition**: when all configured predicates pass, the engine performs **one** authoritative transition of the prepared story’s lifecycle to **closed** (and updates dependent records such as gate timestamps or closure metadata as defined by the implementation).
- **Explicit failure**: when predicates fail, the engine returns structured errors (or a non-closed report) and **does not** write a closed state. Partial satisfaction without waivers/decision records remains **not closed**.

No other code path may set the prepared story (or story) to **closed** for MOL-scoped work without going through this engine or an explicitly deprecated, flag-guarded legacy path (see feature-flag ADR / A6).

### Invariants

1. **Idempotent evaluation**: re-running closure evaluation on an already **closed** prepared story is a no-op or returns success without duplicate side effects.
2. **Findings**: open or blocking verification findings (per policy and severity) prevent closure until resolved, waived through recorded decisions, or downgraded per explicit rules—exact rules belong in engine policy, not scattered in callers.
3. **Independence**: where an execution profile requires verification independent of the implementer, the engine must enforce that separation using existing persona and run metadata—not ad-hoc trust in narrative text.
4. **Read path alignment**: `GetClosureReport` and related reads remain **projections** of the same facts the engine uses; they must not show “closed” if the engine would reject closure (once enforcement lands).

## Why

- Makes **closed** a first-class, auditable outcome instead of an optimistic label.
- Centralizes closure logic so new RPCs or legacy patches cannot bypass invariants without review.
- Aligns runtime behavior with [Requirement Closure Engine](../aios/requirement-closure-engine.md) and [Mission Operating Layer](../aios/mission-operating-layer.md).

## Consequences

- Implementation work (**ABC1**): a local workflow module plus a single command or RPC invoked by daemon workflows; hosted services must not duplicate closure rules.
- **ABC2**: mission-success and daemon paths that today might advance state must consult closure readiness (e.g. block “success” from implying closure when findings are open).
- Tests: regression tests for illegal transitions and bypass attempts (**ABC6**); E2E closure path (**ABC5**).
- Documentation: this ADR is the stable reference; detailed gate-type matrices may live in `docs/aios/` and stay in sync with proto/SQL.

## References

- `aktacode/proto/work/v1/work.proto` — `ClosureGate`, `ClosureClaim`, `VerificationFinding`, `HandoffContract`, `ClosureReport`, `PreparedStory`
- `docs/aios/requirement-closure-engine.md`
- `docs/aios/mission-operating-layer.md`
