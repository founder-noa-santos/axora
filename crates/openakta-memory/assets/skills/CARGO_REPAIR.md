---
skill_id: "CARGO_REPAIR"
name: "Repair cargo fmt clippy and test failures"
triggers:
  - "cargo fmt"
  - "clippy failure"
  - "cargo test failed"
domain: "rust"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.8
---

# Repair cargo fmt clippy and test failures

### Step 1: Run the narrowest failing cargo command
**Validation:** Reproduce the failure before changing code.

### Step 2: Fix compiler and lint errors before style
**Validation:** Resolve build blockers before cosmetic issues.

### Step 3: Re-run the exact failing target
**Validation:** Confirm the original command now passes.
