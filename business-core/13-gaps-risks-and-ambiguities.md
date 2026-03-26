# 13. Gaps, Risks, and Ambiguities

## Purpose

Capture remaining risks and ambiguities in OPENAKTA's architecture and business model.

## Executive Summary

OPENAKTA's architecture is now coherent around **local-first sqlite-vec + cloud-tier Qdrant Cloud**. The remaining gaps are narrow and well-bounded.

## Current State

The prior risks around synthetic provider execution and missing memory architecture are no longer the main story. The runtime now has:

- **Live providers** (Anthropic, OpenAI via HTTP)
- **MCP tool boundary** (secure local execution)
- **Dual-thread ReAct** (planner/actor split)
- **Tripartite memory** (semantic, episodic, procedural)
- **Daemonized doc sync** (background governance)
- **sqlite-vec ANN** (default local backend)
- **Cloud tier** (Qdrant Cloud via Azure Marketplace)

## Remaining Risks

### 1. Legacy Path Overlap

**Risk:** Some legacy runtime paths still coexist with V2 stack.

**Examples:**
- Older coordinator path in `crates/openakta-agents/src/coordinator.rs`
- Legacy memory abstraction in `crates/openakta-agents/src/memory.rs`

**Mitigation:** Prefer `CoordinatorV2` and `blackboard/v2` in new code.

### 2. MCP Tool Breadth

**Risk:** MCP surface is opinionated but not fully comprehensive.

**Current coverage:**
- File, diff, AST, graph, bounded command operations

**Gap:** External tool ecosystem is still core subset, not fully open-ended.

### 3. Cloud Tier Rate Limit Enforcement

**Risk:** Rate limiting (Pro: 100/min, Free: 10/min) is implemented but pricing tiers beyond rate limits are not codified.

**Current truth:**
- Redis + `tower-governor` enforces rate limits
- No subscription/entitlement tables in Postgres

### 4. Self-Hosted Configuration

**Risk:** `External` backend config is available but not fully documented/tested.

**Current truth:**
- `SemanticVectorBackend::External { endpoint, api_key }` exists
- Enterprise self-host pricing not implemented

### 5. ProviderKind Technical Debt

**Risk:** `ProviderKind` conflation noted in historical docs.

**Current truth:**
- `WireProfile` drives transport (R4 completed 2026-03-20)
- `ProviderKind` is telemetry-only

## What Is NOT a Risk (Resolved)

The following are **no longer risks**:

- ✅ **Vector backend choice:** sqlite-vec is default,  is fallback
- ✅ **Cloud tier architecture:** Qdrant Cloud (Azure Marketplace) is official path
- ✅ **Embedding models:** Candle local, Cohere cloud
- ✅ **Auth model:** Clerk.dev with GitHub/Google
- ✅ **Memory architecture:** Tripartite with Ebbinghaus pruning
- ✅ **Provider transport:** Live HTTP, not synthetic

## Open Ambiguities

| Ambiguity | Status |
|-----------|--------|
| **Enterprise pricing beyond rate limits** | Not codified in backend |
| **Self-hosted backend testing** | Available but not fully validated |
| **Turbopuffer migration timeline** | Future path, no timeline |

## Implementation Evidence

- `crates/openakta-memory/src/vector_backend.rs` — sqlite-vec, , External stub
- `crates/openakta-core/src/config.rs` — `SemanticVectorBackend` enum
- `business-core/06-billing-monetization-and-plan-enforcement.md` — Cloud tier architecture
- `business-core/09-integrations-and-external-dependencies.md` — Local/cloud dependencies

## Confidence Assessment

High. Remaining risks are narrow and well-understood. Core architecture is stable.
