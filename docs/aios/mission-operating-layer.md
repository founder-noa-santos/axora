# Mission Operating Layer

The Mission Operating Layer (MOL) is the local workflow system for AIOS-shaped software delivery.

## Today (what exists in code)

- **Canonical store:** The daemon persists authoritative workflow state in local SQLite at `.openakta/work-management.db` (`aktacode/crates/openakta-daemon/src/background/work_mirror.rs`).
- **Command authority:** Workflow commands are applied locally in `aktacode/crates/openakta-daemon/src/background/work_management_service.rs` and `aktacode/crates/openakta-daemon/src/background/local_workflow.rs`.
- **Execution path:** `aktacode/crates/openakta-daemon/src/background/work_plan_compiler.rs` compiles plans; `aktacode/crates/openakta-agents/src/coordinator/v2.rs` runs tasks under local orchestration.
- **Cloud boundary:** Hosted services may support provider calls, embeddings, billing, quotas, OAuth callbacks, and webhooks. They do not own MOL state.
- **Honesty:** “False-done” remains a risk area until every closure rule is enforced locally. The fix is stronger local gating, not moving authority back to the server.

## Target (intended product behavior)

- **Flow:** `story intake → preparation → readiness → execution → verification → closure → learning`
- **Truth:** The local runtime is the **authoritative** store for stories, preparations, requirement graphs, profile decisions, persona assignments, verification findings, and closure gates.
- **Closure:** Mission success should **not** imply story closure; execution advances **toward** closure evidence; only the closure pipeline should mark **`closed`** when gates pass.
- **Boundary:** `openakta-api` is not allowed to become the canonical backend for MOL again, including on paid plans.

## Canonical objects (model)

- **Story Intake** — raw ask plus source metadata.
- **Prepared Story** — normalized execution packet, mission card, readiness blockers, primary execution profile.
- **Requirement Graph** — what must be true before a story can close.
- **Verification Run** — independent proof attempts.
- **Closure Gate** — outcomes for coverage, verification, handoff, review, reliability, documentation alignment.

## Legacy paths

- **Raw / legacy work items:** Clients can still create or patch work items without a full prepared-story lifecycle; fields like `story_id`, `prepared_story_id`, `owner_persona_id`, requirement slice JSON, handoff state, and claim state still exist in local workflow payloads and read models, but **legacy usage** may bypass the intended FSM until gates are enforced.
- **Prepared-story-first** is the **preferred** path for new work; compatibility with older flows is intentional until deprecated behind flags (see MOL config roadmap).
- **Hosted compatibility:** Legacy hosted workflow routes may exist only as explicit compatibility shims. They are not a source of truth.

## Rules going forward

- Workflow state remains local on free and paid plans.
- New task, plan, storyboard, and closure semantics must land in local crates first.
- Shared crates may carry contracts, never workflow authority.
- Any proposal to reintroduce hosted workflow ownership requires an ADR and must prove local execution is impossible.
