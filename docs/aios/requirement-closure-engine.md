# Requirement Closure Engine

Requirement closure is the **intended canonical definition of done** for MOL-shaped work: a story reaches **`closed`** only when requirement-level evidence and gates say so—not merely when implementation tasks finish.

## Model (persisted today)

- Requirements live in a **rooted graph** with typed edges (`wm_requirements`, `wm_requirement_edges` in `openakta-api/migrations/0005_mission_operating_layer.sql`).
- **Acceptance checks** describe proof expectations (e.g. `unit`, `contract`, `integration`, `review`, `docs`)—schema and commands evolve with `work.proto` / `openakta-api/src/work_management.rs`.
- **Requirement coverage** links work items to requirements they implement.
- **Completion claims** and **verification findings** attach evidence-backed state; empty or partial evidence is a known risk area under audit (see agents `result_contract` and MOL roadmap).

## Closure behavior — target vs today

| Aspect | Target | Today (honest) |
|--------|--------|----------------|
| Done vs implementation | Execution finishes → move toward **`closure_pending`**; **`closed`** only after authoritative closure evaluation | Data can represent `closure_pending` / claims / findings; **automated blocking of `closed`** with open findings or failed gates is **roadmap** (closure engine + daemon integration) |
| Verification independence | Where the execution profile requires it, verification is independent of the implementer | **Policy** and automation vary; treat as **design intent** to validate in code paths |
| Gates | Coverage, verification, handoff, review, reliability, documentation alignment | Stored and reported; **single authoritative evaluator** for the last hop to **`closed`** is **target** |
| Waivers / partial | Explicit decision records, visible in closure reporting | **Target** for full auditability |

## Legacy

- Work routed through **raw execution** or **legacy work-item** APIs may not populate the full claim/verification graph; do not infer parity with a fully prepared MOL story.

**Code touchpoints:** `openakta-api/src/work_management.rs`, daemon `aktacode/crates/openakta-daemon/src/background/work_management_service.rs`, plan compilation `work_plan_compiler.rs`.
