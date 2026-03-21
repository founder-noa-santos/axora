# ALIGNMENT_AND_REMEDIATION_REPORT

## 1. State of the Codebase

The multi-provider implementation was partially correct but not fully operationalized in the active V2 path.

- `api_key_file` already overrides inline `api_key` in [`crates/openakta-core/src/config_resolve.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs).
- The active runtime path remains [`crates/openakta-agents/src/coordinator/v2.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs).
- The active blackboard path remains [`crates/openakta-cache/src/blackboard/v2.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/v2.rs).
- The durable queue root in [`crates/openakta-indexing/src/task_queue.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs) is intact.
- The blackboard module root in [`crates/openakta-cache/src/blackboard.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs) is intact.

The primary quality gap was that dynamic registry metadata existed, but the runtime was not consistently using it for routing and budgeting.

## 2. Architectural Violations Found

### Registry metadata was not authoritative at runtime

- [`crates/openakta-agents/src/routing/mod.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs) previously ignored `preferred_instance` from the runtime model registry.
- Result: routing still depended on lane heuristics and default instances even when explicit model metadata existed.

### Budgeting still contained hardcoded behavior

- [`crates/openakta-agents/src/coordinator/v2.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs) previously hardcoded `512` as the model request output cap.
- Task assignment and retrieval budgeting were emitted from static config caps instead of the routed model’s registry metadata.

### Remote registry config was declared but not merged

- [`crates/openakta-core/src/config.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config.rs) exposed `remote_registry`.
- [`crates/openakta-core/src/config_resolve.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs) previously ignored it and only merged builtin + TOML metadata.

### Safe pruning and sandbox boundary status

- No destructive deletion of the persistence/concurrency roots named in the handoff was found.
- MCP zero-trust checks still resolve scope and enforce `RbacEngine` validation in [`crates/openakta-mcp-server/src/lib.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-mcp-server/src/lib.rs).

## 3. The Remediation Patch

### Applied file modifications

- [`crates/openakta-agents/src/provider_registry.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs)
  Added `model_metadata()` so routing can consult the active registry snapshot rather than duplicating lookup logic.

- [`crates/openakta-agents/src/routing/mod.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs)
  Changed the router to honor `preferred_instance` before default-lane fallback and lane ordering. Added a regression test proving registry metadata overrides the default cloud binding.

- [`crates/openakta-agents/src/coordinator/v2.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs)
  Refactored V2 budgeting so:
  - the coordinator predicts the active route before finalizing assignment budgets,
  - `InternalTaskAssignment.token_budget` is derived from the selected model’s context metadata,
  - graph retrieval uses the selected model’s effective retrieval cap,
  - `ModelRequest.max_output_tokens` comes from registry metadata instead of a hardcoded fallback.
  Added a regression test covering metadata-driven assignment/output budgeting.

- [`crates/openakta-core/src/config_resolve.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs)
  Activated `remote_registry` by merging builtin catalog, optional remote JSON, and TOML extensions into the runtime snapshot.

- [`crates/openakta-core/src/bootstrap.rs`](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/bootstrap.rs)
  Updated bootstrap to await the async registry assembly so the live runtime consumes the merged snapshot.

### Security and runtime invariants verified

- `api_key_file > api_key` remains enforced.
- The V2 runtime path and V2 blackboard path remain the active source of truth.
- The SQLite queue root and blackboard root were preserved.
- MCP RBAC validation was not bypassed or relaxed by this remediation.
