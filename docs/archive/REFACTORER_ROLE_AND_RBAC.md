# Refactorer Role And RBAC

## Summary

The base squad now includes a dedicated `refactorer` role for systematic staged refactors. This role is the only role allowed to invoke the `mass_refactor` MCP tool.

## Runtime Changes

- `crates/openakta-agents/src/coordinator/v2_core.rs`
  - Added `SquadRole::Refactorer`.
  - Added the `refactorer` worker to the default base squad.
  - Granted `read_file`, `graph_retrieve_skills`, `graph_retrieve_code`, `request_user_input`, and `mass_refactor`.

- `crates/openakta-agents/src/decomposer/llm_decomposer.rs`
  - Added `refactorer` as an explicit capability for broad scripted refactors.

- `crates/openakta-agents/src/decomposer.rs`
  - Prefers worker assignment `refactorer` for tasks tagged with the `refactorer` capability or global/systematic refactor intent.

- `crates/openakta-agents/src/coordinator/v2_dispatcher.rs`
  - Prefers the task’s pre-assigned worker when one is present, which preserves refactorer delegation without changing the coordinator’s mission loop shape.

- `crates/openakta-agents/src/prompt_assembly.rs`
  - Adds a refactorer-specific system instruction block.
  - Exposes `request_user_input` and `mass_refactor` tool schemas only to the refactorer worker path.

- `crates/openakta-mcp-server/src/lib.rs`
  - Adds `mass_refactor` to the tool registry.
  - Restricts the tool to role `refactorer` inside `role_allows_tool`.

## Exact Refactorer System Prompt

```text
You are the OPENAKTA RefactorerAgent. Your job is to design deterministic Python scripts for staged, sandboxed codebase transformations. Do not hand-write large Rust or TypeScript patches when a systematic script is more appropriate. Before using the sandboxed script path, request explicit human consent for Mass Script Mode. When approved, operate only on the declared target paths, preserve existing staged logic, and return concise execution outcomes.
```

## Enforcement

- `coder` is denied `mass_refactor` at the MCP role gate.
- `refactorer` can use `mass_refactor` only after explicit consent is carried through `consent_mode=mass_script_approved`.
- The prompt path instructs the refactorer to request user approval first, and the tool path re-validates that approval marker.
