# 18. Testing Coverage of Business-Critical Flows

## Purpose

Explain which business-critical backend flows are actually covered by tests today.

## Executive Summary

The repository has meaningful tests around orchestration, patch validation, blackboard behavior, queue semantics, config loading, and parts of retrieval/indexing. The tests are stronger for backend mechanics than for customer business flows, which is consistent with the current product stage. There is little to no test coverage for billing, tenancy, onboarding, or user account logic because those domains are not materially implemented.

## Confirmed Current State

- `CoordinatorV2` mission execution and result publication are tested.
- Diff validation and publication guard behavior are tested.
- gRPC server startup, task submission, and config loading are tested.
- Blackboard V2 diff and subscription behavior are tested.
- Atomic checkout queue behavior is heavily tested.
- Indexing, SCIP, and retrieval modules include targeted tests.
- Benchmarks exist for token savings and optimization themes.

## Detailed Breakdown

### Well-covered areas

| Area | Evidence |
| --- | --- |
| Mission execution | `crates/axora-agents/tests/coordinator_v2.rs` |
| Diff-only enforcement | `crates/axora-agents/src/result_contract.rs` tests |
| Server startup and basic RPCs | `crates/axora-core/tests/integration.rs` |
| Blackboard V2 | `crates/axora-cache/src/blackboard/v2.rs` tests, `crates/axora-cache/tests/blackboard_v2.rs` |
| Atomic checkout queue | `crates/axora-indexing/src/task_queue.rs` tests |

### Weak or absent areas

| Area | Status |
| --- | --- |
| Billing and subscription | No meaningful live tests because no live backend implementation |
| Customer auth/tenancy | No meaningful live tests because no live backend implementation |
| Live provider HTTP integration | Present but still thinner than the rest of the runtime because it depends on credentialed environments |
| Fully integrated retrieval freshness loop | Partial |
| LivingDocs PR automation | Partial |

## Implementation Evidence

- `crates/axora-agents/tests/coordinator_v2.rs`
- `crates/axora-core/tests/integration.rs`
- `crates/axora-cache/tests/blackboard_v2.rs`
- `crates/axora-indexing/src/task_queue.rs`
- `crates/axora-cache/benches/token_savings.rs`

## Business Meaning

The test suite aligns with AXORA’s real business core: execution safety, coordination, transport, and context optimization. It does not support the illusion that customer/business SaaS flows are production-ready.

## Open Ambiguities

- Some benchmark claims still need stronger end-to-end measurement.
- Credentialed end-to-end tests for real provider calls are still a meaningful testing boundary.

## Deprecated / Contradicted / Legacy Patterns

- Example-domain tests using auth/payment terminology should not be mistaken for product-domain coverage.

## Confidence Assessment

Medium.
