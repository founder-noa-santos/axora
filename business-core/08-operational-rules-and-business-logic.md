# 08. Operational Rules and Business Logic

## Purpose

Capture the rules that the validated runtime now enforces.

## Executive Summary

OPENAKTA’s business logic is operational and safety-oriented. The system enforces typed transport, patch-only code edits, MCP-scoped tool access, retry and timeout budgets, compressed context assembly, memory lifecycle rules, and background governance sync.

## Core Rules

- Typed orchestration messages must match their protobuf payloads.
- Code modification outputs must validate as accepted patch formats before workspace application.
- Sensitive local actions such as file reads and command execution must go through MCP.
- MCP command execution is allowlisted, workspace-scoped, audited, and time-bounded.
- `CoordinatorV2` tracks retries, task timeouts, protocol failures, and token usage.
- Provider transport retries transient errors with bounded backoff.
- ReAct planning and acting run as separate tasks, and actor interruption can stop an in-flight tool execution.

## Context Rules

- TOON is the canonical compressed text payload at the model boundary.
- MetaGlyph commands are part of the prompt/control layer, not a replacement for canonical text context.
- Latent context is optional and experimental; it does not replace TOON as source of truth.
- Retrieval and context hydration run under explicit token budgets.

## Memory Rules

- Episodic memory stores chronological execution traces.
- Semantic memory stores embedded knowledge artifacts such as synced docs.
- Procedural memory stores reusable skill artifacts.
- FadeMem pruning uses exponential Ebbinghaus-style decay plus retrieval and importance reinforcement.
- Procedural pruning also considers utility scores from prior success/failure outcomes.
- Consolidation runs in the background to promote repeated episodic patterns into procedural memory.

## Governance Rules

- Repository changes are fingerprinted through Merkle diffing.
- Documentation reconciliation can mark a change as `Noop`, `UpdateRequired`, or `ReviewRequired`.
- LivingDocs updates are fed into semantic memory even when a human review is still required for final prose changes.

## Implementation Evidence

- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/provider_transport.rs`
- `crates/openakta-agents/src/react.rs`
- `crates/openakta-mcp-server/src/lib.rs`
- `crates/openakta-memory/src/lifecycle.rs`
- `crates/openakta-daemon/src/services.rs`
- `crates/openakta-docs/src/reconciler.rs`

## Business Meaning

Trust in OPENAKTA comes from bounded, inspectable execution. The product’s operational discipline is now enforced through runtime guards rather than only through design intent.

## Open Ambiguities

- Some legacy paths still enforce weaker rules than the V2 path.
- LivingDocs sync is real, but automated PR creation is not yet the default governance action.

## Confidence Assessment

High.
