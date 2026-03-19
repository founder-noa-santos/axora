# 12. Deprecated, Conflicting, or Stale Material

## Purpose

Capture contradictions and stale materials without polluting the canonical business-core documents.

## Executive Summary

The repository contains meaningful stale and aspirational material. The most important pattern is that older docs and example content describe a broader SaaS/business domain than the backend actually implements. There are also legacy runtime paths that overlap with newer V2-oriented implementations. This file isolates those divergences.

## Confirmed Current State

| Item | What it claims | What code does now | Confidence |
| --- | --- | --- | --- |
| `docs/business_rules/PAY-001.md`, `PAY-002.md` | Payment processing and refunds are business rules | No live payment integration or billing enforcement exists in backend crates | High |
| `docs/active_architecture/*` | Broad future architecture including auth/payment examples | Mixed historical and design material; only portions match live code | High |
| older coordinator path in `crates/axora-agents/src/coordinator.rs` | Parallel coordination implementation | V2 path is the stronger current direction; legacy path remains for compatibility/history | Medium |
| `crates/axora-agents/src/memory.rs` vs cache blackboard v2 | Multiple shared-state abstractions | State truth is split between simpler agent memory/blackboard and stronger cache blackboard v2 | High |
| example config and older docs | Broader product/runtime support | Some entries are aspirational or only partially backed | Medium |

## Detailed Breakdown

### Payment and business-rule docs

These docs read like operational business policy, but they are not backed by live payment code. They should be preserved as historical artifacts or intended policy, not current implementation truth.

### Auth and permissions examples

Auth-heavy examples appear across docs, tests, benchmarks, and parser fixtures. Most are synthetic content for indexing, documentation, or performance tests rather than live application logic.

### Legacy coordinator overlap

There are now at least two coordination paths in the repo. V2 reflects the current hard-break runtime direction, but the older path still exists and can confuse audits if treated equally.

### Overlapping memory/state models

The repository contains both:

- a simpler agent memory/blackboard layer in `crates/axora-agents/src/memory.rs`
- a stronger versioned/diff-publishing blackboard in `crates/axora-cache/src/blackboard/v2.rs`

That overlap is not fatal, but it is not clean.

## Implementation Evidence

- `docs/business_rules/PAY-001.md`
- `docs/business_rules/PAY-002.md`
- `docs/active_architecture/`
- `crates/axora-agents/src/coordinator.rs`
- `crates/axora-agents/src/coordinator/v2.rs`
- `crates/axora-agents/src/memory.rs`
- `crates/axora-cache/src/blackboard/v2.rs`

## Business Meaning

Without separating stale material from live truth, teams can overestimate what AXORA already supports commercially and underappreciate what is actually strong in the backend. This is particularly risky around billing, auth, and coordination architecture.

## Open Ambiguities

- Some historical docs may still reflect future direction, but they are not current backend truth.
- Parts of the old coordinator may still be referenced in tests or non-primary paths.

## Deprecated / Contradicted / Legacy Patterns

- Treat all items in the table above as legacy, contradictory, or non-authoritative unless revalidated in code.

## Confidence Assessment

High.
