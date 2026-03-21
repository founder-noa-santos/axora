# 06. Billing, Monetization, and Plan Enforcement

## Purpose

Capture what the repository currently implements, and does not implement, around monetization and commercial enforcement.

## Executive Summary

There is no meaningful billing or plan-enforcement backend implemented today. The repository contains historical business-rule documents about payments and synthetic example content referencing billing concepts, but the Rust backend does not contain a live subscription model, payment provider integration, checkout flow, invoice lifecycle, seat management, or entitlement enforcement. The monetization truth in code is therefore weak and largely absent.

## Confirmed Current State

- No active Stripe, Paddle, or equivalent billing integration exists in the backend crates.
- No subscription, plan, entitlement, or seat tables are defined in the actual SQLite schema used by `openakta-storage`.
- No billing-aware middleware, service layer, or route protection exists in the backend runtime.
- Payment and billing references found in the repository are primarily:
  - business-rule Markdown documents
  - benchmark/sample code
  - synthetic examples unrelated to live backend behavior

## Detailed Breakdown

### What is actually enforced

Nothing in the current backend enforces:

- plan tiers
- usage caps tied to payment state
- paid feature access
- trials
- checkout completion
- invoice status
- payment failure handling
- seat or organization billing

### Business model signals that do exist

There are indirect signals that future monetization may matter:

- strong investment in token-cost reduction and provider-aware caching
- architectural attention to cost and resource budgets
- historical docs about business rules

These are strategic signals, not implemented commercial enforcement.

### How to interpret billing docs currently in the repo

`docs/business_rules/PAY-001.md` and `docs/business_rules/PAY-002.md` read like product/business policy artifacts, but they are not backed by live payment code in the current backend. They should not be mistaken for current production capability.

## Implementation Evidence

- `crates/openakta-storage/migrations/0001_init.sql`
- `crates/openakta-core/src/server.rs`
- `crates/openakta-daemon/src/main.rs`
- `crates/openakta-agents/`
- `docs/business_rules/PAY-001.md`
- `docs/business_rules/PAY-002.md`
- repository-wide search showing billing/payment terms concentrated in docs and sample content rather than runtime crates

## Business Meaning

The company’s commercial layer is not yet codified in the backend. Current value is being created through infrastructure and execution capability, not through enforced monetization logic. This is important for internal alignment: there is technical product value, but not yet a code-enforced revenue system.

## Open Ambiguities

- The intended pricing model is not inferable from the backend.
- The degree to which token optimization is meant to feed pricing or margin is strategic rather than implemented.

## Deprecated / Contradicted / Legacy Patterns

- Payment business-rule docs imply a more mature commercial platform than the code actually implements.
- Synthetic benchmark content about revenue, roles, or permissions should not be treated as domain evidence.

## Confidence Assessment

Low. The strongest truth is the absence of implementation.
