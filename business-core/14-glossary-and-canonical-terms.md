# 14. Glossary and Canonical Terms

## Purpose

Standardize key vocabulary so future docs and implementation discussions do not drift.

## Executive Summary

OPENAKTA’s current vocabulary should center on mission execution, typed orchestration, patch safety, and local context infrastructure. Many words that appear elsewhere in the repository, such as auth, payment, or subscription, are often example-domain terms rather than live OPENAKTA business terms. This glossary prefers the implemented backend language.

## Confirmed Current State

| Term | Canonical meaning | Notes |
| --- | --- | --- |
| OPENAKTA | Multi-agent coding backend/runtime platform | Current business-core term |
| Mission | High-level work request decomposed into tasks | Core orchestration term |
| Task | Executable unit of work | Has type and status |
| CoordinatorV2 | Primary current coordination runtime | Prefer over generic “coordinator” when precision matters |
| Worker | Execution participant assigned tasks | Agent-level runtime actor |
| TaskAssignment | Typed work instruction payload | Proto and runtime contract |
| ContextPack | Typed structured context passed across orchestration | Converted to TOON only at model boundary |
| TOON | Token-Optimized Object Notation | Model-bound compact serialization |
| Prefix Cache | Local prompt prefix cache metadata and lookup mechanism | Cost-optimization term |
| PatchEnvelope | Canonical patch submission artifact | Includes target files and base revision |
| PatchReceipt | Deterministic apply result | Use instead of vague “patch result” |
| Unified diff zero-context | Preferred diff protocol for code edits | Canonical code-edit format |
| Search/Replace block | AST-style patch fallback format | Alternative code-edit format |
| Blackboard V2 | Versioned diff-publishing shared state | Prefer over generic “shared memory” when referring to cache crate implementation |
| Merkle state | File/block hash index for incremental change detection | Indexing term |
| SCIP index | Symbol/occurrence structure for code navigation | Retrieval/indexing term |
| Influence graph | Dependency/influence graph for retrieval | Retrieval term |
| Repository map | Compact codebase navigation structure | Token-optimization term |

## Detailed Breakdown

### Terms to avoid using as canonical business truth

| Term | Why to avoid |
| --- | --- |
| User account | Not a first-class backend entity today |
| Organization / tenant / workspace owner | Not an implemented backend truth |
| Subscription / plan / entitlement | Not a live backend domain |
| Auth module / payment module | Often appears only in examples, tests, or historical docs |

### Preferred naming guidance

- Use `CoordinatorV2` when referring to the current intended runtime.
- Use `code-edit task` instead of vague “LLM coding response.”
- Use `typed transport` instead of “message JSON.”
- Use `patch receipt` instead of “merge result” when the deterministic apply outcome is intended.

## Implementation Evidence

- `proto/collective/v1/core.proto`
- `crates/openakta-agents/src/coordinator/v2.rs`
- `crates/openakta-agents/src/patch_protocol.rs`
- `crates/openakta-agents/src/provider.rs`
- `crates/openakta-cache/src/toon.rs`
- `crates/openakta-cache/src/blackboard/v2.rs`
- `crates/openakta-indexing/src/merkle.rs`
- `crates/openakta-indexing/src/scip.rs`

## Business Meaning

Shared vocabulary is essential because the repository mixes implemented runtime language with example-domain language. This glossary reduces the risk of designing around synthetic examples instead of real OPENAKTA concepts.

## Open Ambiguities

- “Workspace” currently refers more often to repository root / execution root than to a SaaS tenant object.

## Deprecated / Contradicted / Legacy Patterns

- Historical naming from business-rule docs should not dominate current backend terminology.

## Confidence Assessment

High.
