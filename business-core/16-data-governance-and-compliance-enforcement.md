# 16. Data Governance and Compliance Enforcement

## Purpose

Describe the data-handling and compliance posture that is actually visible in the backend today.

## Executive Summary

The repository currently shows a pragmatic local-runtime data posture rather than a formal compliance framework. Data is stored in SQLite, in-memory runtime structures, local files, and derived indexing artifacts. There is no strong evidence of implemented GDPR/CCPA/HIPAA-style controls, retention policies, audit-grade data governance, or customer-facing privacy controls. The main current governance signals are technical: local-first processing, deterministic patch application, and explicit typed artifacts.

## Confirmed Current State

- Runtime data is stored locally in SQLite and in memory.
- Patch application, blackboard state, and indexing all operate over local repository/workspace data.
- Live provider execution is now part of the active V2 path when credentials are present.
- MCP emits audit events for tool execution decisions, adding a real governance signal at the local action boundary.
- No explicit compliance enforcement layer, privacy policy engine, or data deletion workflow is implemented in backend crates.

## Detailed Breakdown

### What is currently true

- The system is biased toward local control over code and context artifacts.
- Shared runtime state is explicit in blackboard/message structures.
- SQLite schema is simple and locally scoped.
- Memory lifecycle management and doc-sync now create a more explicit retention and governance posture than before, even if they are not formal compliance controls.

### What is not currently true

- No data retention scheduler
- No policy-driven personal data classification
- No tenant-scoped deletion or export workflows
- No explicit audit trail for compliance reporting
- No formal secrets governance framework in the backend itself

### Risk profile visible in code

- Local-first execution can reduce some external exposure.
- Lack of formal governance means operational controls are mostly implicit.

## Implementation Evidence

- `crates/axora-storage/migrations/0001_init.sql`
- `crates/axora-daemon/src/main.rs`
- `crates/axora-core/src/config.rs`
- `crates/axora-agents/src/coordinator/v2.rs`
- `crates/axora-cache/src/blackboard/v2.rs`
- `crates/axora-indexing/src/merkle.rs`

## Business Meaning

Compliance is not yet a productized or enforced backend capability. The current system is better described as locally controlled engineering infrastructure than as a regulated enterprise platform.

## Open Ambiguities

- Live cloud reasoning is now real, so provider data-handling posture matters more than before.
- The desktop shell may store or expose additional data-handling behavior outside this backend-focused review.

## Deprecated / Contradicted / Legacy Patterns

- No strong stale compliance regime was found; the main issue is absence rather than contradiction.

## Confidence Assessment

Low.
