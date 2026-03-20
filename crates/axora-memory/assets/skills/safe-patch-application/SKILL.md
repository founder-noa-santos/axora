---
skill_id: "SAFE_PATCH_APPLICATION"
name: "Apply Patches Deterministically"
triggers:
  - "apply patch"
  - "unified diff"
  - "deterministic edit"
domain: "editing"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.79
---

# Apply Patches Deterministically

### Step 1: Read the current file before writing
**Validation:** Ensure the base text still matches the patch intent.

### Step 2: Apply the smallest diff possible
**Validation:** Avoid rewriting unrelated sections.

### Step 3: Validate the resulting file immediately
**Validation:** Re-read or compile the file after patch application.
