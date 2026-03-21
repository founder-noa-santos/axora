# 01. Company Current Business Core

## Purpose

Define what OPENAKTA is now, based on the validated Rust runtime.

## Executive Summary

OPENAKTA is now a batteries-included autonomous software engineering runtime with live cloud reasoning, secure tool execution over MCP/gRPC, deterministic patch application, aggressive context compression, persistent cognitive memory, and a mission-first CLI entrypoint. The business core is no longer “synthetic orchestration with future plans”; it is a real execution platform for high-concurrency code work that can be started from a single command.

## Confirmed Current State

- `CoordinatorV2` builds real provider requests using the **Dynamic Model Registry** for metadata-driven token budgeting.
- Provider instances support **heterogeneous execution lanes** (cloud and local) with automatic or explicit fallback policies.
- API keys are **file-based secrets** stored in `.openakta/secrets/`, never inline in configuration.
- Routing consults `ModelRegistryEntry::preferred_instance` for model-specific instance selection.
- Token budgets are **dynamically derived** from `max_context_window` and `max_output_tokens` metadata, not hardcoded.
- Live HTTP transport is used by default when provider credentials are present; synthetic transport is a development fallback.
- Tool execution is routed through a native MCP boundary for filesystem, diff, AST, graph, and bounded command operations instead of raw in-process agent shelling.
- ReAct execution is split into planner and actor tasks so planning and acting are no longer one blocking loop.
- Model-bound context is compacted through TOON, with MetaGlyph commands and latent-context preparation available in the context envelope.
- Memory is separated into semantic, episodic, and procedural domains, with default skills seeded on first run and pruning/consolidation services started by the runtime bootstrap.
- LivingDocs/doc-sync runs as a background daemon service and feeds documentation changes into semantic memory.
- The primary operator-facing entrypoint is now the `openakta` CLI mission flow rather than manual daemon bring-up.
- Billing, customer tenancy, and account lifecycle are still not the product core in code.

## What OPENAKTA Sells Technically

OPENAKTA’s differentiated value is now the combination of:

- controlled multi-agent orchestration
- live cloud LLM execution with token accounting
- secure local tool access behind MCP
- patch-first code modification with deterministic application
- compressed context transport and retrieval-aware prompt building
- persistent memory and governance services running beside the coordinator

## What OPENAKTA Is Not Yet

OPENAKTA is still not a full commercial SaaS business system with:

- customer billing
- tenant administration
- strong user identity and entitlement models
- external CRM or contract workflows

## Implementation Evidence

- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-cli/src/main.rs`
- `crates/openakta-core/src/bootstrap.rs`
- `crates/openakta-core/src/runtime_services.rs`
- `crates/openakta-core/src/config.rs`
- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/provider_transport.rs`
- `crates/openakta-agents/src/react.rs`
- `crates/openakta-agents/src/mcp_client.rs`
- `crates/openakta-mcp-server/src/lib.rs`
- `crates/openakta-memory/src/lifecycle.rs`
- `crates/openakta-docs/src/reconciler.rs`

## Business Meaning

OPENAKTA’s current business core is a production-oriented execution substrate for autonomous coding work: reason in the cloud, act locally through a hardened tool boundary, preserve state across runs, and keep token costs low enough for repeated use.

## Multi-Provider Architecture

OPENAKTA now supports dynamic, multi-provider execution:

### Provider Instances
- **Cloud lanes**: HTTP-backed providers (Anthropic, OpenAI, OpenAI-compatible)
- **Local lanes**: Ollama or other local runtimes
- **Instance selection**: Deterministic priority lists or registry-driven routing

### Model Registry
- **Builtin catalog**: Known models with verified metadata
- **Remote registry**: Optional JSON endpoint for dynamic updates
- **TOML extensions**: Local overrides for custom models

### Routing Modes
- **Routing enabled**: Difficulty-aware routing (fast paths → local, architecture-heavy → cloud)
- **Routing disabled**: Single-lane fallback to configured default
- **Fallback policies**: `never`, `explicit`, or `automatic` downgrade to local

## Open Ambiguities

- Commercial packaging is still thinner than the runtime.
- Some legacy subsystems still exist in the repository outside the new batteries-included entry path.
- MCP tool coverage is broader now, but it is still a core subset rather than a complete external tool ecosystem.
- `ProviderKind` conflation: currently used for both telemetry and transport selection (technical debt).

## Confidence Assessment

High.
