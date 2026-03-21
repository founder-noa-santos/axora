# Business Core Documentation

## Purpose

This folder documents the current business core of OPENAKTA as it exists in the repository today. It is intended to give founders, operators, product leads, and senior engineers a durable view of what the company is actually building and enforcing in code.

## Executive Summary

OPENAKTA's implemented business core is a backend platform for orchestrating multi-agent coding work with strict transport contracts, diff-only code modification, local-first context management, and token-cost optimization. The repository contains meaningful infrastructure for agents, orchestration, typed messaging, retrieval, blackboard state, indexing, and provider-bound model execution. It does not yet contain a real customer account system, billing system, or production-grade SaaS tenancy model. Those themes appear in older docs, benchmarks, and synthetic fixtures, but not as current backend truth.

The documents in this folder were derived primarily from Rust crates, protobuf contracts, daemon/server entry points, storage code, indexing code, validation logic, and tests. Existing Markdown docs, architecture notes, and prompts were used only as secondary evidence and are called out explicitly when they diverge from the implementation.

## Audit Methodology

The audit used the following evidence hierarchy:

1. Current implementation in `crates/`, `proto/`, and daemon entry points
2. Tests and benchmarks that exercise implemented behavior
3. Config and example config files when backed by code
4. Existing docs, ADRs, architecture notes, prompts, and business rules as secondary evidence only

The repository review prioritized repeated signals over isolated examples. Example snippets, synthetic test fixtures, commented code, and obsolete docs were not treated as current truth unless corroborated by live implementation.

## Source-of-Truth Hierarchy

Confirmed source-of-truth order:

1. Runtime code paths in `crates/openakta-agents`, `crates/openakta-core`, `crates/openakta-cache`, `crates/openakta-indexing`, `crates/openakta-storage`
2. Protobuf contracts in `proto/collective/v1/core.proto`
3. Daemon/bootstrap code in `crates/openakta-daemon`
4. Tests covering runtime behavior
5. Config structures such as `crates/openakta-core/src/config.rs`
6. Existing docs only where consistent with implementation

## Confidence Model

- High: directly implemented and reinforced in multiple runtime paths or tests
- Medium: strongly inferred from multiple modules but not fully live end-to-end
- Low: weakly inferred, partially implemented, or contradicted by other repository evidence

## Maintenance Instructions

When code changes:

1. Update these documents only after verifying the new runtime behavior in code and tests.
2. If a flow becomes obsolete, move it into `12-deprecated-conflicting-or-stale-material.md` instead of silently deleting history.
3. Keep terminology aligned with `14-glossary-and-canonical-terms.md`.
4. Prefer updating the most specific affected document first, then refresh cross-references in `_index.md`.
5. Do not document planned behavior as current state.

## Contributor Warning

Do not trust older architecture docs, prompts, or business-rule Markdown over implementation. The codebase currently contains historical and aspirational material that references auth, payments, and broader product flows that are not enforced by the backend. If a document here conflicts with live code, the code wins and the document must be corrected.

## Confidence Assessment

High. This folder is intentionally grounded in the current repository rather than planning material.
