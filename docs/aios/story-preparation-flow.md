# Story Preparation Flow

Stories are **designed** to move through a preparation state machine before execution. **Today**, these states are represented in the data model and APIs; **illegal transitions and writes to terminal states** are **target** behaviors to enforce uniformly in `openakta-api` (handlers under `openakta-api/src/work_management.rs`)—do not assume every invalid jump is rejected yet.

## States (intended model)

Values align with the preparation / story lifecycle stored for MOL (see `wm_story_preparations`, `wm_story_intakes` in `openakta-api/migrations/0005_mission_operating_layer.sql`):

- `captured`
- `classified`
- `clarification_pending`
- `triaged`
- `preparing`
- `prepared`
- `ready`
- `executing`
- `closure_pending`
- `closed`
- `blocked`
- `abandoned`

## Preparation outputs (artifacts)

- `Story Intake`
- `Prepared Story`
- `Mission Card`
- `Execution Card`
- `Requirement Graph`
- `Execution Profile Decision`

## Today vs target

- **Today:** RPCs and persistence can record these states and artifacts; daemon compilation (`aktacode/crates/openakta-daemon/src/background/work_plan_compiler.rs`) consumes available prepared-story data when present.
- **Target:** A single **authoritative FSM** for intake/preparation (documented in ADR roadmap) rejects invalid transitions and blocks direct jumps to `closed` without passing closure rules.
- **Legacy:** The daemon still supports **raw work-item execution** without a full preparation record; it may **synthesize or partially fill** metadata. That path exists for compatibility, not as the long-term definition of “done.”
