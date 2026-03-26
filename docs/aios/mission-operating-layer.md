# Mission Operating Layer

The Mission Operating Layer (MOL) extends the existing cloud-first work-management backend toward an AIOS-shaped workflow for software delivery.

## Today (what exists in code)

- **Canonical store:** `openakta-api` persists MOL tables in Postgres (see `openakta-api/migrations/0005_mission_operating_layer.sql`) and serves work-management gRPC (`openakta-api/src/work_management.rs`, `aktacode/proto/work/v1/work.proto`).
- **Local mirror:** The daemon keeps a **SQLite** mirror for read models and command replay at `.openakta/work-management.db` (`aktacode/crates/openakta-daemon/src/background/work_mirror.rs`); it is for **local-first execution and sync**, not a second source of truth for authoritative writes.
- **Execution path:** `aktacode/crates/openakta-daemon/src/background/work_plan_compiler.rs` compiles plans; `aktacode/crates/openakta-agents/src/coordinator/v2.rs` runs tasks. Behavior may **prefer** prepared-story metadata when present but **does not yet** enforce every MOL invariant on every path.
- **Honesty:** “False-done” (rich payloads without **hard gates**) is a known gap; closing it is **target** work (API validation, compiler, coordinator, closure engine), not something readers should assume from schema alone.

## Target (intended product behavior)

- **Flow:** `story intake → preparation → readiness → execution → verification → closure → learning`
- **Truth:** `openakta-api` remains the **authoritative** store for stories, preparations, requirement graphs, profile decisions, persona assignments, verification findings, and closure gates once sync and commands are applied.
- **Closure:** Mission success should **not** imply story closure; execution advances **toward** closure evidence; only the closure pipeline should mark **`closed`** when gates pass.

## Canonical objects (model)

- **Story Intake** — raw ask plus source metadata.
- **Prepared Story** — normalized execution packet, mission card, readiness blockers, primary execution profile.
- **Requirement Graph** — what must be true before a story can close.
- **Verification Run** — independent proof attempts.
- **Closure Gate** — outcomes for coverage, verification, handoff, review, reliability, documentation alignment.

## Legacy paths

- **Raw / legacy work items:** Clients can still create or patch work items without a full prepared-story lifecycle; fields like `story_id`, `prepared_story_id`, `owner_persona_id`, requirement slice JSON, handoff and claim state exist on `wm_work_items` but **legacy usage** may bypass the intended FSM until gates are enforced.
- **Prepared-story-first** is the **preferred** path for new work; compatibility with older flows is intentional until deprecated behind flags (see MOL config roadmap).
