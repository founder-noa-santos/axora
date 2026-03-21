---
skill_id: "MERGE_CONFLICT_RESOLUTION"
name: "Resolve Git Merge Conflicts Safely"
triggers:
  - "merge conflict"
  - "rebase conflict"
  - "conflict markers"
domain: "git"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.76
---

# Resolve Git Merge Conflicts Safely

### Step 1: Understand both sides before editing
**Validation:** Identify what each branch intended to change.

### Step 2: Keep semantic intent, not marker order
**Validation:** Produce the smallest merged result that preserves both valid changes.

### Step 3: Re-run affected tests or builds
**Validation:** Verify the merged state compiles and behaves correctly.
