# Mass Refactor Boundary And Consent

## Filesystem Boundary

- The Python script never runs against the live workspace root.
- The tool stages only approved `target_paths` into `.openakta/mass-refactor/<session_id>/workspace/`.
- The container mounts only that staged workspace as writable.
- The script file is mounted separately as read-only.
- Any file created or modified outside the approved staged roots is rejected before promotion.
- The live workspace is mutated only after container success, staged diff generation, and boundary validation.

## Rollback Model

- Script failure: remove the staged session directory and return `stderr`.
- Validation or diff failure after script exit: remove the staged session directory and return rollback confirmation.
- Promotion failure: restore any partially promoted files from `.openakta/mass-refactor/<session_id>/baseline/`, then clean the session directory.

## Exact User Consent Prompt

```markdown
Choose how to apply this codebase-wide refactor.

Option A (Normal/Safe Mode): The LLM reads the files and generates deterministic unified diffs. Slower, consumes more tokens, but semantically safer.

Option B (Mass Script Mode): The LLM generates a Python script and runs it in a sandboxed container against a staged workspace. Faster and more token-efficient, but a flawed script can overwrite intended staged logic across many files.

Approve Mass Script Mode only if you want the refactor to run through the sandboxed Python workflow.
```

## Required Choice IDs

- Safe Mode: `safe_mode`
- Mass Script Mode: `mass_script_mode`
- Required MCP consent marker after explicit approval: `mass_script_approved`
