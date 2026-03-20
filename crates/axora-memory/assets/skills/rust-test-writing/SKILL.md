---
skill_id: "RUST_TEST_WRITING"
name: "Write Standard Rust Tests"
triggers:
  - "write rust test"
  - "add unit tests"
  - "cover edge cases"
domain: "rust"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.8
---

# Write Standard Rust Tests

### Step 1: Target the public behavior
**Validation:** Assert behavior, not implementation details.

### Step 2: Cover happy path and one failure mode
**Validation:** Add at least one success assertion and one regression assertion.

### Step 3: Prefer focused fixtures
**Validation:** Use the smallest setup that still proves the behavior.
