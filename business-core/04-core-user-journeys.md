# 04. Core User Journeys

## Purpose

Describe the journeys that are actually implemented in the current backend.

## Executive Summary

The real journeys are now mission-first developer journeys around `openakta do ...`. OPENAKTA supports a live end-to-end flow: automatic runtime bootstrap, native MCP startup, base squad initialization, mission decomposition, compressed-context assembly, live provider execution, dual-thread ReAct task execution, deterministic patch application, memory logging, and doc synchronization.

## Confirmed Current Journeys

- Run `openakta do "..."` and let the runtime bootstrap itself.
- Submit or execute a mission through `CoordinatorV2`.
- Run coding tasks through a planner/actor ReAct loop with MCP-routed tools.
- Execute code modifications through validated diff output and deterministic patch application.
- Persist thoughts, actions, and outcomes into episodic memory while updating semantic and procedural stores in the background.

## Journey: Start the Runtime

1. Developer runs `openakta do "<mission>"`.
2. Workspace root is inferred from the current directory.
3. `.openakta/` runtime paths are created automatically.
4. SQLite, procedural skills, and local semantic stores are initialized.
5. The native MCP service is started as an embedded runtime dependency.
6. Memory pruning, consolidation, and doc-sync services begin running.
7. `CoordinatorV2` boots the built-in Base Squad.

## Journey: Execute a Mission

1. Mission text enters `CoordinatorV2`.
2. The decomposer produces task units and queue dependencies.
3. Retrieval and prompt assembly build a compact context pack.
4. TOON payload, MetaGlyph commands, and prompt instructions are sent to the provider boundary.
5. Live Anthropic/OpenAI transport is used by default when credentials are present.
6. Results are validated, merged, and published to shared state.

## Journey: Dual-Thread Worker Execution

1. Planner thread observes task state and blackboard snapshots.
2. Planner emits an action proposal.
3. Actor thread executes the selected action.
4. Filesystem and command operations go through MCP.
5. Interrupts can stop the actor during tool execution instead of waiting for the next loop boundary.
6. Thought/action/observation records are logged to episodic memory.

## Journey: Code Modification

1. Task is marked as `CodeModification`.
2. Context is assembled under token budget.
3. Live model output is validated as diff or search/replace patch.
4. Patch is wrapped in a `PatchEnvelope`.
5. Patch is applied deterministically to the workspace.
6. Result submission includes token usage and patch receipt.

## Implementation Evidence

- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-cli/src/main.rs`
- `crates/openakta-core/src/bootstrap.rs`
- `crates/openakta-core/src/runtime_services.rs`
- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/provider_transport.rs`
- `crates/openakta-agents/src/prompt_assembly.rs`
- `crates/openakta-agents/src/react.rs`
- `crates/openakta-agents/src/mcp_client.rs`
- `crates/openakta-mcp-server/src/lib.rs`

## Business Meaning

The core journey is no longer “simulate execution contracts.” It is now “run real software engineering work with controlled local action and live remote reasoning.”

## Open Ambiguities

- If provider credentials are absent, the runtime now fails fast with a single actionable bootstrap error.
- Customer onboarding and billing flows are still outside the enforced backend truth.

## Confidence Assessment

High.
