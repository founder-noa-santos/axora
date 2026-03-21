# 10. Architecture Decisions That Shape the Business

## Purpose

Record the architecture decisions that now materially define OPENAKTA’s product behavior.

## Executive Summary

OPENAKTA’s business is shaped by six active decisions: live cloud reasoning, MCP as the local tool boundary, patch-first code editing, dual-thread ReAct execution, compressed context transport, and tripartite memory with background governance services.

## Active Decisions

### Decision: Use Dynamic Model Registry for metadata-driven execution

Model metadata (context windows, output tokens, preferred instances) is the authoritative source for routing and budgeting. Hardcoded limits are rejected.

### Decision: Use file-based secrets for provider authentication

API keys are stored in `.openakta/secrets/<instance>.key`, never inline in TOML. This prevents accidental credential leakage in version control.

### Decision: Support heterogeneous execution lanes (cloud + local)

Provider instances define independent cloud and local lanes. Routing can be difficulty-aware or fixed to a single lane.

### Decision: Fail fast on missing provider configuration

A system without provider configuration cannot function. The bootstrap panics rather than allowing silent degradation.

### Decision: Use live cloud providers as the default reasoning path

`CoordinatorV2` uses transport injection so live HTTP is the default when credentials exist. Synthetic transport remains a development and test fallback, not the primary runtime story.

### Decision: Use hybrid CLI-first, MCP-backed local execution

Local work stays local, but filesystem and command actions cross an MCP/gRPC boundary with scope checks, allowlists, timeouts, and audit events. This is a security and product decision, not just an implementation detail.

### Decision: Keep TOON as the canonical compressed payload

TOON is the canonical text representation at the model boundary. MetaGlyph is used to compress prompt intent, and latent context is optional preparation work rather than the source of truth.

### Decision: Split cognition and action

ReAct is organized as planner and actor tasks. That reduces blocking behavior, enables interrupts, and better matches the product goal of responsive autonomous execution.

### Decision: Keep patch-first code editing

Code modifications remain constrained to validated patch formats and deterministic application. This keeps edits auditable and lowers token overhead.

### Decision: Treat memory and docs as runtime services

Semantic, episodic, and procedural memory are persisted separately, and pruning, consolidation, and LivingDocs sync run continuously in the daemon.

## Implementation Evidence

- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/provider_transport.rs`
- `crates/openakta-agents/src/prompt_assembly.rs`
- `crates/openakta-agents/src/react.rs`
- `crates/openakta-agents/src/model_registry/mod.rs`
- `crates/openakta-agents/src/routing/mod.rs`
- `crates/openakta-agents/src/token_budget.rs`
- `crates/openakta-core/src/config_resolve.rs`
- `crates/openakta-core/src/bootstrap.rs`
- `crates/openakta-mcp-server/src/lib.rs`
- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-daemon/src/services.rs`
- `proto/collective/v1/core.proto`
- `proto/mcp/v1/mcp.proto`

## Business Meaning

These decisions optimize OPENAKTA for controlled autonomy: live reasoning power, low token cost, bounded local action, persistent memory, and auditable governance.

## Open Ambiguities

- Legacy runtime paths still coexist with the clearer V2 stack.
- The latent-context path is prepared but still experimental by design.
- `ProviderKind` conflation: currently drives both telemetry and transport selection (R4 technical debt).

## Confidence Assessment

High.
