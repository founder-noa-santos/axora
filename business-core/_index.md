# Business Core Index

## Purpose

This index maps the business-core documentation set, its audience, and its current confidence level.

## Executive Summary

The recommended reading order starts with the business core, domain model, operational rules, and source-of-truth map. Readers focused on execution should then move into integrations, architecture constraints, testing coverage, and gaps. Documents covering billing, onboarding, tenancy, and compliance are included because they matter strategically, but several of them document current absence or weak enforcement rather than a mature implementation.

## Confirmed Current State

| File | Purpose | Intended Audience | Confidence |
| --- | --- | --- | --- |
| `README.md` | Explains the audit method and maintenance model | All readers | High |
| `01-company-current-business-core.md` | Defines what OPENAKTA currently is in code | Founders, product, engineering | High |
| `02-product-surfaces-and-app-boundaries.md` | Maps runtime surfaces and system boundaries | Engineering, operations | High |
| `03-actors-roles-and-permissions.md` | Describes implemented actors and role semantics | Engineering, operations | Medium |
| `04-core-user-journeys.md` | Explains real end-to-end backend journeys | Product, engineering | Medium |
| `05-onboarding-and-activation-logic.md` | Documents startup and activation gating | Operations, engineering | Medium |
| `06-billing-monetization-and-plan-enforcement.md` | Records current absence of billing enforcement and available signals | Founders, product | Low |
| `07-domain-model-and-business-entities.md` | Canonical entity map | Engineering, product | High |
| `08-operational-rules-and-business-logic.md` | Consolidates enforced rules | Engineering, operations | High |
| `09-integrations-and-external-dependencies.md` | Maps external dependencies and their business impact | Engineering, operations | High |
| `10-architecture-decisions-that-shape-the-business.md` | Focuses on business-shaping technical constraints | Founders, engineering | High |
| `11-current-source-of-truth-map.md` | Shows where truth lives in the repo | Engineering | High |
| `12-deprecated-conflicting-or-stale-material.md` | Isolates stale or conflicting material | Engineering, product | High |
| `13-gaps-risks-and-ambiguities.md` | Captures weak enforcement and unresolved truth | Founders, engineering | High |
| `14-glossary-and-canonical-terms.md` | Standardizes language | All readers | High |
| `15-notifications-and-communication-model.md` | Documents message flow, pub/sub, and coordination signaling | Engineering, operations | High |
| `16-data-governance-and-compliance-enforcement.md` | Describes current data handling and lack of formal compliance logic | Operations, founders | Low |
| `17-admin-and-internal-operations.md` | Covers daemon operation and internal operator workflows | Operations, engineering | Medium |
| `18-testing-coverage-of-business-critical-flows.md` | Maps tests to business-critical backend flows | Engineering | Medium |
| `19-tenant-lifecycle-and-account-lifecycle.md` | Documents current absence of real tenant/account lifecycle | Founders, product | Low |
| `20-feature-flags-config-and-env-sensitive-behavior.md` | Explains configuration-sensitive runtime behavior | Engineering, operations | Medium |

## Suggested Reading Order

### Founders

1. `01-company-current-business-core.md`
2. `06-billing-monetization-and-plan-enforcement.md`
3. `10-architecture-decisions-that-shape-the-business.md`
4. `13-gaps-risks-and-ambiguities.md`
5. `19-tenant-lifecycle-and-account-lifecycle.md`

### Product

1. `01-company-current-business-core.md`
2. `04-core-user-journeys.md`
3. `07-domain-model-and-business-entities.md`
4. `06-billing-monetization-and-plan-enforcement.md`
5. `12-deprecated-conflicting-or-stale-material.md`

### Engineering

1. `11-current-source-of-truth-map.md`
2. `07-domain-model-and-business-entities.md`
3. `08-operational-rules-and-business-logic.md`
4. `09-integrations-and-external-dependencies.md`
5. `10-architecture-decisions-that-shape-the-business.md`
6. `13-gaps-risks-and-ambiguities.md`

### Operations

1. `02-product-surfaces-and-app-boundaries.md`
2. `05-onboarding-and-activation-logic.md`
3. `15-notifications-and-communication-model.md`
4. `17-admin-and-internal-operations.md`
5. `20-feature-flags-config-and-env-sensitive-behavior.md`

## Implementation Evidence

- `README.md`
- `crates/`
- `proto/collective/v1/core.proto`
- `crates/openakta-daemon/src/main.rs`

## Business Meaning

The index is intentionally audience-oriented because the current repository mixes strong backend truth with stale documentation about broader product areas. Readers need a fast path to the most trustworthy documents.

## Open Ambiguities

- Confidence varies substantially by domain area.
- Billing, tenancy, onboarding, and compliance documents are intentionally conservative because live enforcement is weak or absent.

## Deprecated / Contradicted / Legacy Patterns

- None in this index itself. See `12-deprecated-conflicting-or-stale-material.md`.

## Confidence Assessment

High.
