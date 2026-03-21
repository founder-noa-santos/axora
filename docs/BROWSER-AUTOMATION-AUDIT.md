# Section 1: Current State

OPENAKTA does not currently implement real browser automation in the runtime.

- The native MCP surface is statically registered inside `EmbeddedToolRegistry::builtin_with_hitl`, and the built-in tools are `read_file`, `generate_diff`, `apply_patch`, `ast_chunk`, `symbol_lookup`, `run_command`, `graph_retrieve_skills`, `graph_retrieve_code`, plus optional `request_user_input` when HITL is enabled (`crates/openakta-mcp-server/src/lib.rs:241-259`).
- The gRPC contract exposes generic `ListTools`, `CallTool`, and `StreamAudit`; there is no browser-specific service, session contract, navigation API, click API, DOM read API, screenshot API, or page snapshot API in `proto/mcp/v1/mcp.proto` (`proto/mcp/v1/mcp.proto:9-87`).
- The actual tool handlers implemented in `openakta-mcp-server` are file IO, diff/patch, AST chunking, symbol lookup, command execution, retrieval, and HITL. There are no handlers for browser navigation, click, fill, evaluate DOM, screenshot, or page snapshot (`crates/openakta-mcp-server/src/lib.rs:688-1100`).
- The execution backends are only `Direct`, `Containerized`, and `WASI`, and they only support bounded command execution and patch application. There is no browser executor or browser session manager (`crates/openakta-mcp-server/src/execution.rs:18-171`).
- The Rust workspace crates do not declare Playwright or any browser automation crate in the workspace manifest or in `openakta-mcp-server` / `openakta-agents` manifests (`Cargo.toml:1-128`, `crates/openakta-mcp-server/Cargo.toml:1-38`, `crates/openakta-agents/Cargo.toml:1-54`).
- The JS manifests also do not declare Playwright. Repo-wide source search found no Playwright runtime wiring in `apps`, `crates`, `tests`, `integrations`, or `sdks`. The root package and desktop app both omit Playwright dependencies (`package.json:1-35`, `apps/desktop/package.json:1-100`).

Conclusion: in the repo as implemented today, Playwright is not wired into OPENAKTA's runtime. There is no built-in browser exploration capability.

# Section 2: Architectural Fit

Tool exposure is static and compile-time, not dynamic.

- Tool registration is hardcoded in Rust in `EmbeddedToolRegistry::builtin_with_hitl`; there is no config-driven external tool loader and no dynamic plugin catalog (`crates/openakta-mcp-server/src/lib.rs:241-259`).
- `ListTools` returns a role-filtered view over that static registry via `definitions_for_role` and `role_allows_tool` (`crates/openakta-mcp-server/src/lib.rs:269-275`, `crates/openakta-mcp-server/src/lib.rs:550-558`, `crates/openakta-mcp-server/src/lib.rs:1634-1691`).
- The runtime starts `McpService` directly inside OPENAKTA bootstrap and daemon startup; the MCP server is an embedded OPENAKTA service, not a broker over external MCP sidecars (`crates/openakta-core/src/bootstrap.rs:402-445`, `crates/openakta-daemon/src/main.rs:104-143`).

Current execution flow from agent code to MCP and back is:

1. A ReAct `ToolSet` either executes a locally registered tool or forwards the action to `McpClient::call_tool` (`crates/openakta-agents/src/react.rs:252-315`).
2. `McpClient::call_tool` sends a `ToolCallRequest` with `agent_id`, `role`, `tool_name`, `arguments`, `policy`, `workspace_root`, and optional `mission_id` (`crates/openakta-agents/src/mcp_client.rs:48-80`).
3. `McpService::call_tool` resolves the tool from the static registry, applies role filtering and capability policy checks, resolves scope, executes the tool, and emits an `AuditEvent` (`crates/openakta-mcp-server/src/lib.rs:561-636`).
4. The tool result returns over gRPC and is converted back into a ReAct `Observation` (`crates/openakta-agents/src/mcp_client.rs:82-130`).

Agents do not currently auto-discover tools in the active execution path.

- `McpClient::list_tools` exists, but there are no call sites using it in the current mainline agent runtime (`crates/openakta-agents/src/mcp_client.rs:31-46`).
- ReAct action selection does not use `ListTools`; it prefers locally registered tool names and otherwise hardcodes `run_command` when MCP exists (`crates/openakta-agents/src/react.rs:995-1024`).
- Model-facing tool exposure is also fixed. `PromptAssembly` hardcodes `graph_retrieve_skills`, `graph_retrieve_code`, and optional `patch_contract`; it does not query MCP at runtime (`crates/openakta-agents/src/prompt_assembly.rs:57-105`).
- Worker profiles also hardcode per-role tool permissions (`crates/openakta-agents/src/coordinator/v2_core.rs:163-235`).

Conclusion: OPENAKTA's current MCP architecture is a native, static, audited tool boundary. Tool use depends on fixed local registration patterns, not runtime discovery.

# Section 3: Concurrency and Isolation

The current MCP service supports concurrent tool calls for stateless tools.

- `McpService` is an async tonic service, started once and shared by clone (`crates/openakta-core/src/bootstrap.rs:409-441`, `crates/openakta-daemon/src/main.rs:109-140`).
- `call_tool` has no global serialization lock around request execution; each request resolves its tool and awaits execution independently (`crates/openakta-mcp-server/src/lib.rs:561-636`).
- `ExecutorRouter` and its backends are `Clone`/`Arc` based and execute commands per request (`crates/openakta-mcp-server/src/execution.rs:124-171`).
- Direct and container command backends launch isolated child processes with `kill_on_drop(true)` (`crates/openakta-mcp-server/src/execution/direct.rs:16-50`, `crates/openakta-mcp-server/src/execution/container.rs:43-85`).

However, OPENAKTA has no browser session isolation model today.

- There is no browser session registry, no browser context ID, no page handle, and no session token in the MCP proto (`proto/mcp/v1/mcp.proto:20-87`).
- The only visible `session_id` in `openakta-mcp-server` belongs to HITL question envelopes, not tool execution state (`crates/openakta-mcp-server/src/lib.rs:1168-1248`).
- No MCP tool currently owns long-lived per-agent state beyond the optional HITL gate.

Audit identity is only partially preserved today.

- The wire protocol and audit event both carry `agent_id` and `role` (`proto/mcp/v1/mcp.proto:50-87`).
- `McpClient::call_tool` forwards those fields end to end (`crates/openakta-agents/src/mcp_client.rs:48-80`).
- `McpService::build_audit` records the incoming `agent_id` and `role` into each audit event (`crates/openakta-mcp-server/src/lib.rs:505-518`).
- But the default ReAct `ToolSet` flattens identity to `react-agent` / `worker` unless the caller explicitly overrides it with `with_mcp_runtime_context` (`crates/openakta-agents/src/react.rs:195-233`).
- There is a test proving explicit override works for trusted mission context, which confirms the preservation path exists but is not automatic (`crates/openakta-agents/tests/react_mcp_mission_id.rs:57-76`).

If two agents invoked the same future browser tool at the same time, contention would most likely happen in the missing session layer.

- A naive shared browser/page singleton inside `openakta-mcp-server` would race immediately because the current runtime has no per-agent browser ownership model.
- A `run_command`-based browser approach would shift contention to coarse process-level orchestration, shared browser profiles, and weak action-level audit granularity.
- The existing audit log itself is not the main risk; the missing browser session registry is.

Conclusion: OPENAKTA can run concurrent stateless MCP calls today, but it does not yet have the state model required for concurrent multi-agent browser automation.

# Section 4: Option Analysis

## A. Playwright CLI via existing bounded command execution

Fit with current architecture:
- Partial fit. It can reuse `run_command`, which already travels through MCP and existing execution backends (`crates/openakta-mcp-server/src/lib.rs:891-964`).

Built in by default with no manual configuration:
- No. OPENAKTA does not currently ship Playwright in the Rust or JS manifests, and there is no browser packaging path in the runtime. This fails the non-negotiable built-in constraint (`package.json:1-35`, `apps/desktop/package.json:1-100`, `Cargo.toml:1-128`).

Multiple isolated browser instances in parallel:
- Weak. Separate CLI processes could be launched, but OPENAKTA has no first-class session ownership, no browser profile isolation policy, and no browser lifecycle manager.

Security alignment with `CapabilityPolicy` and `AuditEvent`:
- Weak. OPENAKTA would audit `run_command`, not first-class browser verbs like navigate, click, DOM read, or screenshot.

Operational stability:
- Weak to moderate. Shelling out is coarse, brittle, and makes browser state hard to manage or resume.

Implementation complexity:
- Lowest initial complexity.

Failure isolation:
- Moderate at process level, weak at product level because session semantics remain implicit.

Architecture regression:
- It would not replace MCP/gRPC, but it would reduce browser access to a workaround instead of a first-class OPENAKTA capability.

Verdict:
- Not the best choice. It is an expedient workaround, not the right built-in user access pattern.

## B. External Playwright MCP sidecar/server

Fit with current architecture:
- Mixed. It matches the idea of MCP boundaries, but OPENAKTA currently owns a single native embedded MCP service and has no implemented sidecar registration, brokering, or forwarding layer (`crates/openakta-core/src/bootstrap.rs:402-445`, `crates/openakta-daemon/src/main.rs:104-143`).

Built in by default with no manual configuration:
- Not proven from this repo. External assumption required.

Multiple isolated browser instances in parallel:
- Potentially yes, but only if the external sidecar supports explicit browser sessions and isolation. External assumption required.

Security alignment with `CapabilityPolicy` and `AuditEvent`:
- Weak to mixed unless OPENAKTA proxies every sidecar action through its own audited policy path. That proxy layer is not implemented today.

Operational stability:
- Mixed. A dedicated sidecar can isolate crashes, but it adds another process surface OPENAKTA does not currently manage.

Implementation complexity:
- Medium to high because OPENAKTA would need process management, startup health checks, discovery/routing, policy translation, and audit correlation.

Failure isolation:
- Better than CLI if well-managed, but only after OPENAKTA builds that management plane.

Architecture regression:
- Likely. It risks widening the current native MCP/gRPC architecture into a second MCP layer that bypasses or duplicates OPENAKTA's built-in audit and policy path.

Verdict:
- Not the best primary path as the repo exists today. If used at all, it would need to be hidden behind OPENAKTA-owned tooling rather than exposed as a user-managed external dependency.

## C. First-class embedded browser tools inside `openakta-mcp-server`

Fit with current architecture:
- Strongest fit. OPENAKTA already has a native MCP/gRPC tool boundary, static registration, capability policy checks, and audit events (`proto/mcp/v1/mcp.proto:9-87`, `crates/openakta-mcp-server/src/lib.rs:230-275`, `crates/openakta-mcp-server/src/lib.rs:561-636`).

Built in by default with no manual configuration:
- Yes, if OPENAKTA bundles the browser runtime as part of its own runtime packaging. That packaging work does not exist yet, but this option is the only one that matches the product constraint cleanly.

Multiple isolated browser instances in parallel:
- Yes, if `openakta-mcp-server` adds a per-session registry keyed by stable OPENAKTA-owned session IDs and owner identity.

Security alignment with `CapabilityPolicy` and `AuditEvent`:
- Strongest. Browser actions can become first-class allowed actions and first-class audit records instead of opaque shell commands.

Operational stability:
- Strongest if OPENAKTA adds session cleanup, bounded concurrency, and explicit cancellation.

Implementation complexity:
- Highest.

Failure isolation:
- Good if the embedded tool layer internally uses a managed child process or worker per session. External assumption required about the specific browser engine used internally.

Architecture regression:
- No. This extends OPENAKTA's existing MCP/gRPC architecture instead of bypassing it.

Verdict:
- Best architectural fit.

# Section 5: Required Changes

The recommended production path is first-class embedded browser tooling exposed by OPENAKTA's own MCP server. The table below lists the minimum changes for that direction.

| Layer | Minimum change | Classification |
| --- | --- | --- |
| `crates/openakta-mcp-server` | Add first-class browser tools or a browser service owned by OPENAKTA, including at minimum session start, navigate, click, type/fill, DOM read/evaluate, page snapshot, screenshot, and close session. | Required for correctness |
| `crates/openakta-mcp-server` | Add a `BrowserSessionRegistry` keyed by OPENAKTA session ID and storing owner `agent_id`, `role`, `mission_id`, last activity, and runtime handle. Refuse cross-owner access. | Required for correctness, security, concurrency |
| `crates/openakta-mcp-server` | Add browser session quotas, TTL cleanup, idle reaping, and crash recovery. | Required for production stability, concurrency |
| `crates/openakta-mcp-server` | Add browser-specific policy checks, including action allowlists and network/origin restrictions. Current scope-path RBAC is not enough for URL navigation. | Required for security |
| `crates/openakta-mcp-server` | Keep browser tooling behind OPENAKTA's existing MCP surface even if the internal engine is a managed child process. Do not expose a separate user-facing MCP sidecar. | Required for production stability |
| `crates/openakta-agents` | Surface browser tools to the model/task layer. Today `PromptAssembly` only exposes retrieval tools and optional `patch_contract`; browser schemas must be added intentionally. | Required for correctness |
| `crates/openakta-agents` | Extend worker profiles so the right roles can use browser tools. Today tool permissions are hardcoded and browser actions do not exist. | Required for correctness |
| `crates/openakta-agents` | Stop relying on default flattened MCP identity. Ensure every worker/task execution path injects stable unique `agent_id` and role into MCP requests. | Required for security, concurrency |
| `crates/openakta-agents` | Optional but recommended: actually consume `ListTools` for runtime reflection instead of relying entirely on hardcoded tool assumptions. | Optional |
| `proto/mcp/v1/mcp.proto` | Recommended minimum for production: add typed browser session metadata or a dedicated `BrowserService` with explicit session lifecycle and action RPCs. The existing generic `CallTool` can prototype this, but it is not the best production wire contract for long-lived browser sessions. | Required for production stability, concurrency |
| `proto/mcp/v1/mcp.proto` | Add session identifiers and browser action metadata to the audit path in a typed way. | Required for security, production stability |
| `crates/openakta-core` | Extend `CoreConfig` and bootstrap wiring to carry browser runtime configuration: bundled browser path, max concurrent browser sessions, cleanup interval, allowed origins/domains, and default browser backend mode. | Required for correctness, security, production stability |
| `crates/openakta-core` | Add startup health checks so the runtime verifies browser backend readiness before advertising browser tools. | Required for production stability |
| audit / policy path | Expand `CapabilityPolicy` semantics from filesystem/action scope to browser action scope and URL/origin scope. Expand audit records to capture session ID, URL, and browser action. | Required for security |
| execution backend / container / direct mode | Add a dedicated browser backend instead of routing browser actions through `run_command`. If container mode is supported, the image must include the browser runtime. If direct mode is supported, OPENAKTA must launch isolated browser profiles per session. | Required for correctness, security, production stability |
| session lifecycle and cleanup | Add explicit create/lease/release/close semantics, mission-end cleanup, idle timeout cleanup, and crash cleanup. | Required for correctness, concurrency, production stability |
| interruption / cancellation handling | Extend current interrupt behavior so an in-flight browser action receives a real cancellation signal and the runtime decides whether to keep or dispose the session. Current actor interruption can stop waiting, but browser-state cleanup would need explicit implementation. | Required for correctness, production stability |

Implementation note:

- A zero-proto-change prototype is possible by adding browser tool names to the existing generic `CallTool` path.
- That is not the recommended minimum for the production direction because OPENAKTA's requirements are session-heavy, concurrent, and audit-sensitive.

# Section 6: Recommended Direction

OPENAKTA should choose first-class embedded browser tooling inside `openakta-mcp-server`.

Playwright CLI is not the best choice. It fits only as a workaround through `run_command`, fails the built-in-by-default constraint in the current repo, and collapses browser actions into coarse shell execution.

External Playwright MCP is also not the best choice as OPENAKTA exists today. It requires external assumptions about packaging, session isolation, routing, and audit preservation, and it would widen OPENAKTA's current native MCP/gRPC architecture instead of extending it cleanly.

The right path is:

- Keep OPENAKTA's public tool boundary as its own native MCP/gRPC service.
- Add first-class browser session and browser action support inside `openakta-mcp-server`.
- If OPENAKTA uses Playwright internally, use it only as an implementation detail behind OPENAKTA-owned tools and OPENAKTA-owned session management. External assumption required about the exact bundled engine.

In other words: choose option C. If a hybrid is used, the hybrid should be "OPENAKTA-native embedded browser tools backed by an internal bundled browser worker," not "user-managed Playwright CLI" and not "external Playwright MCP exposed directly to agents."
