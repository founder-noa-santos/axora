# 20. Feature Flags, Config, and Environment-Sensitive Behavior

## Purpose

Explain how configuration and environment currently shape backend behavior.

## Executive Summary

OPENAKTA’s behavior today is materially shaped by runtime config rather than by a large formal feature-flag system. CoreConfig controls the daemon/server process, while coordinator config controls workspace root, provider kind, model, retry budget, timeout, retrieval token budget, and graph retrieval toggles. Environment sensitivity exists through bind settings, debug logging, filesystem paths, and provider/runtime selection.

## Confirmed Current State

- `CoreConfig` controls server bind address, port, DB path, max concurrent agents, frame duration, and debug mode.
- The daemon can load config from TOML or CLI.
- `CoordinatorV2` has runtime flags/settings for:
  - provider
  - model
  - workspace root
  - task timeout
  - retry budget
  - retrieval token budget
  - retrieval max docs
  - graph retrieval enabled/disabled
  - task token budget
- `RUST_LOG` is environment-sensitive at startup.
- The active runtime depends heavily on local filesystem paths and current workspace content.

## Detailed Breakdown

### Operationally important config

| Config area | Business consequence |
| --- | --- |
| bind address / port | Determines daemon reachability |
| database path | Determines local persistence location |
| workspace root | Determines what codebase the system reasons over and edits |
| provider/model | Determines model-bound execution shape |
| retrieval budgets | Determines context size and pruning behavior |
| retry/timeout | Determines operational resilience and failure semantics |
| debug logging | Determines observability depth |

### What is not present

- a mature remote feature-flag control plane
- account-tier-based feature gating
- environment-based commercial entitlement logic

## Implementation Evidence

- `crates/openakta-core/src/config.rs`
- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-agents/src/coordinator/v2.rs`
- `openakta.example.toml`

## Business Meaning

In the current stage, config is the main operational control plane. Business behavior changes through deployment/runtime settings more than through product-level feature gates or entitlement systems.

## Open Ambiguities

- Example config files expose more surface area than the live backend always enforces.
- Future hosted or multi-environment behavior may require a stronger feature-flag model than the current local/runtime-centric approach.

## Deprecated / Contradicted / Legacy Patterns

- None severe, but some example config entries should be treated as secondary evidence until verified against runtime code.

## Confidence Assessment

Medium.
