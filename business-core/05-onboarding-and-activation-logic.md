# 05. Onboarding and Activation Logic

## Purpose

Explain the current activation and setup logic that exists in code, even where classic product onboarding is absent.

## Executive Summary

AXORA still does not implement customer onboarding in the conventional SaaS sense, but the activation surface is now mission-first instead of operator-first. The enforced path is: export provider credentials, run `axora do "<mission>"`, and let the runtime bootstrap storage, MCP, skills, and the default squad automatically.

## Confirmed Current State

- The batteries-included CLI can create runtime defaults silently from the current workspace.
- SQLite, semantic memory, and default procedural skills are initialized automatically on first run.
- `CoordinatorV2` now boots a built-in Base Squad instead of requiring manual agent assembly in the happy path.
- MCP is started as a managed runtime dependency instead of requiring external installation.
- There is no user-facing state machine for signup completion, workspace provisioning, or subscription activation in the backend.

## Detailed Breakdown

### Current activation sequence

1. Export provider credentials.
2. Run `axora do "<mission>"`.
3. Infer workspace and create `.axora/` local runtime paths.
4. Initialize SQLite, memory services, and skill library.
5. Start native MCP and runtime-managed background services.
6. Bootstrap the Base Squad inside `CoordinatorV2`.
7. Execute work against the current workspace.

### Operational blockers

- Missing provider credentials
- Database initialization failure
- MCP bootstrap failure
- No available workers
- Invalid typed message structure
- Invalid diff output for code-edit tasks
- Patch conflict or stale base during application

### What is not currently implemented

- account creation
- email verification
- tenant/workspace activation
- progressive setup checklists
- subscription-gated activation
- role-specific onboarding paths

## Implementation Evidence

- `crates/axora-daemon/src/main.rs`
- `crates/axora-core/src/config.rs`
- `crates/axora-core/src/bootstrap.rs`
- `crates/axora-core/src/runtime_services.rs`
- `crates/axora-core/src/server.rs`
- `crates/axora-cli/src/main.rs`
- `crates/axora-agents/src/coordinator/v2.rs`
- `proto/collective/v1/core.proto`

## Business Meaning

Current activation reflects a CLI-first product stage. The platform becomes usable from a single mission command rather than from manual daemon bring-up, even though classic account-based onboarding is still absent.

## Open Ambiguities

- The desktop shell may eventually become another batteries-included entrypoint, but the CLI is the enforced truth today.
- Advanced daemon and config paths still exist for operations and debugging, but they are no longer the intended default journey.

## Deprecated / Contradicted / Legacy Patterns

- Any older docs implying a subscription-driven or account-driven activation flow are not current backend truth.

## Confidence Assessment

Medium.
