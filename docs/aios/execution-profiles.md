# Execution Profiles

Execution profiles are policy bundles that tune process rigor, not model power.

## Profiles

- `Fast Iterate`
  - Minimal preparation
  - Lightweight requirement graph
  - High parallelism
  - Smoke verification

- `Balanced`
  - Full prepared story
  - Scoped requirement graph
  - Review Steward approval
  - Unit plus integration or contract checks when interfaces move

- `High Assurance`
  - Explicit requirements for closure
  - High clarification strictness
  - Mandatory independent verification
  - Reduced parallelism unless handoffs are explicit

- `Critical Change`
  - Risk memo, rollback expectations, and strongest gates
  - Minimal parallelism
  - Reliability Steward protection and human approval for destructive changes

## Inference and escalation

- Initial profile inference may use intake signals, sensitive surfaces, ambiguity, reversibility, incidents, and runtime health.
- Profiles may escalate during execution when verification fails, a sensitive surface appears, or reliability brakes activate.
