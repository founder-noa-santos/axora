# Sandboxed Mass Refactoring

## Summary

`mass_refactor` is now a standalone MCP capability in `crates/openakta-mcp-server` that stages approved workspace targets under `.openakta/mass-refactor/<session_id>/`, runs a Python script only inside a dedicated container image, synthesizes a unified diff from the staged before/after trees, promotes the staged changes into the live workspace only after success, and cleans up or rolls back automatically on error.

## Implementation

- `crates/openakta-mcp-server/src/mass_refactor.rs`
  - `WorkspaceCheckpointer`
    - Creates `baseline/` and `workspace/` copies for the approved `target_paths`.
    - Treats current workspace contents as the pre-script staged baseline, so existing dirty files are preserved rather than reset.
    - Rejects post-script writes outside the approved path roots.
    - Promotes only changed files to the live workspace and restores from `baseline/` if promotion partially fails.
  - `WorkspaceDiffGenerator`
    - Builds `MerkleTree` for `baseline/` and staged `workspace/`.
    - Calls `MerkleTree::find_changed`.
    - Renders per-file unified diffs with `UnifiedDiff::generate` and concatenates them into one response string.
  - `MassRefactorTool`
    - Requires `script`, `target_paths`, and `consent_mode=mass_script_approved`.
    - Uses a dedicated `MassRefactorExecutorConfig`.
    - Executes the Python script through `ContainerExecutor::run_command_with_mounts`.
    - Returns unified diff output on success or `stderr` with `rollback_performed=true` on failure.

- `crates/openakta-mcp-server/src/execution.rs`
  - Added `MassRefactorExecutorConfig` for the dedicated Python sandbox image, interpreter, mount path, and timeout.
  - Added `ContainerMount` so containerized commands can mount the staged workspace plus a read-only script file.

- `crates/openakta-mcp-server/src/execution/container.rs`
  - Added `run_command_with_mounts` to support the mass-refactor container invocation without using `DirectExecutor` or `ExecutorRouter::run_command`.

- `crates/openakta-mcp-server/src/lib.rs`
  - Registered `mass_refactor` in the embedded tool registry.
  - Added `mass_refactor_executor` to `McpServiceConfig`.
  - Added per-target path validation and RBAC enforcement before staging.
  - Restricted `mass_refactor` to the `refactorer` role in `role_allows_tool`.

- `crates/openakta-core/src/config.rs`
  - Added `mass_refactor_executor` to the runtime config surface with defaults.

- `crates/openakta-core/src/bootstrap.rs`
- `crates/openakta-daemon/src/main.rs`
  - Thread the dedicated mass-refactor executor config into the MCP service.

## Public Tool Contract

### Input

```json
{
  "script": "string",
  "target_paths": ["string"],
  "consent_mode": "mass_script_approved",
  "timeout_secs": 120
}
```

### Success Output

```json
{
  "diff": "unified diff text",
  "changed_files": ["relative/path.rs"],
  "rollback_performed": false
}
```

### Failure Output

```json
{
  "changed_files": ["attempted/path.rs"],
  "rollback_performed": true
}
```

`stderr` and container exit status are returned through the normal MCP `ToolCallResult` fields.

## Verification

- `cargo check`
- `cargo test -p openakta-mcp-server -p openakta-agents`
