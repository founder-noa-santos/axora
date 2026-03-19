# 01. Company Current Business Core

## Purpose

Define what AXORA is now, based on the validated Rust runtime.

## Executive Summary

AXORA is now a batteries-included autonomous software engineering runtime with live cloud reasoning, secure tool execution over MCP/gRPC, deterministic patch application, aggressive context compression, persistent cognitive memory, and a mission-first CLI entrypoint. The business core is no longer “synthetic orchestration with future plans”; it is a real execution platform for high-concurrency code work that can be started from a single command.

## Confirmed Current State

- `CoordinatorV2` builds real provider requests and uses live HTTP transport by default when provider credentials are present.
- Tool execution is routed through a native MCP boundary for filesystem, diff, AST, graph, and bounded command operations instead of raw in-process agent shelling.
- ReAct execution is split into planner and actor tasks so planning and acting are no longer one blocking loop.
- Model-bound context is compacted through TOON, with MetaGlyph commands and latent-context preparation available in the context envelope.
- Memory is separated into semantic, episodic, and procedural domains, with default skills seeded on first run and pruning/consolidation services started by the runtime bootstrap.
- LivingDocs/doc-sync runs as a background daemon service and feeds documentation changes into semantic memory.
- The primary operator-facing entrypoint is now the `axora` CLI mission flow rather than manual daemon bring-up.
- Billing, customer tenancy, and account lifecycle are still not the product core in code.

## What AXORA Sells Technically

AXORA’s differentiated value is now the combination of:

- controlled multi-agent orchestration
- live cloud LLM execution with token accounting
- secure local tool access behind MCP
- patch-first code modification with deterministic application
- compressed context transport and retrieval-aware prompt building
- persistent memory and governance services running beside the coordinator

## What AXORA Is Not Yet

AXORA is still not a full commercial SaaS business system with:

- customer billing
- tenant administration
- strong user identity and entitlement models
- external CRM or contract workflows

## Implementation Evidence

- `crates/axora-daemon/src/main.rs`
- `crates/axora-cli/src/main.rs`
- `crates/axora-core/src/bootstrap.rs`
- `crates/axora-core/src/runtime_services.rs`
- `crates/axora-core/src/config.rs`
- `crates/axora-agents/src/coordinator/v2.rs`
- `crates/axora-agents/src/provider_transport.rs`
- `crates/axora-agents/src/react.rs`
- `crates/axora-agents/src/mcp_client.rs`
- `crates/axora-mcp-server/src/lib.rs`
- `crates/axora-memory/src/lifecycle.rs`
- `crates/axora-docs/src/reconciler.rs`

## Business Meaning

AXORA’s current business core is a production-oriented execution substrate for autonomous coding work: reason in the cloud, act locally through a hardened tool boundary, preserve state across runs, and keep token costs low enough for repeated use.

## Open Ambiguities

- Commercial packaging is still thinner than the runtime.
- Some legacy subsystems still exist in the repository outside the new batteries-included entry path.
- MCP tool coverage is broader now, but it is still a core subset rather than a complete external tool ecosystem.

## Confidence Assessment

High.
