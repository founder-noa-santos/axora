# Story Preparation Flow

Stories are **designed** to move through a preparation state machine before execution. The authoritative state machine is now local to the daemon workflow runtime. Illegal transitions and writes to terminal states must be enforced there, not delegated to hosted APIs.

## States (intended model)

Values align with the local workflow lifecycle defined in `aktacode/crates/openakta-workflow/src/transition.rs` and applied in `aktacode/crates/openakta-daemon/src/background/local_workflow.rs`:

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

- **Today:** The daemon persists and serves these states locally; compilation (`aktacode/crates/openakta-daemon/src/background/work_plan_compiler.rs`) consumes authoritative prepared-story data from the local workflow store.
- **Target:** A single **authoritative FSM** for intake/preparation (documented in ADR roadmap) rejects invalid transitions and blocks direct jumps to `closed` without passing closure rules.
- **Legacy:** The daemon still supports **raw work-item execution** without a full preparation record; it may **synthesize or partially fill** metadata. That path exists for compatibility, not as the long-term definition of “done.”
