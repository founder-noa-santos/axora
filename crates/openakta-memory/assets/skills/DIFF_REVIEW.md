---
skill_id: "DIFF_REVIEW"
name: "Review Unified Diffs"
triggers:
  - "review diff"
  - "check patch"
  - "code review"
domain: "review"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.78
---

# Review Unified Diffs

### Step 1: Check changed files against task intent
**Validation:** Every file in the patch must be justified by the task.

### Step 2: Look for missing tests and regressions
**Validation:** Flag behavior changes that are not verified.

### Step 3: Confirm patch is minimal
**Validation:** Remove unrelated edits and compatibility shims.
