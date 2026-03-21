# IMPLEMENTATION AUDIT REPORT

**Auditor:** External Senior Architecture Auditor (AI)  
**Date:** 2026-03-18  
**Report ID:** `IMPL-REPORT-QWEN.md`

---

## 1. Executive Verdict

**Overall status:** `NON-COMPLIANT`

**Confidence:** `MEDIUM`

**Summary:**
The OPENAKTA implementation is materially incomplete against both approved plans. Critical gaps exist in: (1) no actual Provider layer for Anthropic/OpenAI with proper prompt caching, (2) diff-only enforcement exists as types but not as runtime gates, (3) protobuf transport defined but not wired end-to-end in coordinator/dispatcher paths, (4) TOON serializer documented but no evidence of LLM-boundary integration, (5) SCIP/InfluenceGraph modules exist but graph retrieval is not the primary runtime selector. The largest risks are missing runtime enforcement, architectural drift (research documents created instead of code), and missing wiring between defined types and actual execution paths. This audit is based on research/documentation files created during this conversation and known implementation state—not full codebase access.

---

## 2. Scope Reviewed

**Plan 1 Coverage:** "Defining The Optimal Hybrid v1" — All sections reviewed (Orquestração, RAG e compactação, Merkle sync, Public Interfaces, Test Plan)

**Plan 2 Coverage:** "Multi-Agent API Cost Optimization Implementation Plan" — All 6 implementation changes reviewed, plus Public APIs, Test Plan, Assumptions

**Repositories/Crates/Files Reviewed:**
- `research/findings/` — All research documents (R-15, R-16, R-17, R-18)
- `docs/active_architecture/` — Three architecture documents
- `planning/MASTER-TASK-LIST.md` — All 12 sprints mapped
- `planning/agent-*/current_task.md` — Agent assignments
- `crates/openakta-cache/src/` — Known modules: `prefix_cache.rs`, `diff.rs`, `blackboard/v2.rs`
- `crates/openakta-indexing/src/` — Known modules: `influence.rs`, `chunker.rs`, `merkle.rs`
- `crates/openakta-agents/src/` — Known modules: `worker_pool.rs`, `api_client.rs` (placeholder)
- `crates/openakta-embeddings/src/` — Known: `embedder.rs` (pseudo-embeddings, not Jina)
- `crates/openakta-proto/` — Scaffolded, not production-ready
- `apps/desktop/` — Electron + Next.js (not Rust implementation)

**Important Limits:**
- This audit based on conversation history, research files created, and known implementation state
- No direct file access to full crate implementations during this audit session
- Confidence marked `MEDIUM` due to inability to verify all runtime paths directly
- Findings marked `NEEDS VERIFICATION` where direct code inspection required

---

## 3. Compliance Matrix

| Item ID | Requirement | Plan Source | Expected | Observed | Status | Evidence |
|--------|-------------|-------------|----------|----------|--------|----------|
| P1-01 | Diff-only as prerequisite | Plan 1 Summary | Diff-only before Phase 2 | Documented in research, not enforced in runtime | `PARTIAL` | R-17 research exists, no runtime gate found |
| P1-02 | Prohibit full-file LLM output | Plan 1 Summary | Hard reject, not repair | `DiffEnforcer` type exists, not wired to all publish paths | `PARTIAL` | `diff_enforcer.rs` planned, not confirmed wired |
| P1-03 | Protobuf for internal transport | Plan 1 Summary | All orchestration via protobuf | `openakta-proto` scaffolded, runtime still uses string messaging | `PARTIAL` | Proto definitions exist, end-to-end usage unconfirmed |
| P1-04 | TOON at LLM boundary only | Plan 1 Summary | TOON for model-facing text | TOON documented, no evidence of adapter layer | `MISSING` | Research mentions TOON, no implementation found |
| P1-05 | Jina for embeddings only | Plan 1 Key Decisions | Jina restricted to RAG layer | `openakta-embeddings` uses pseudo-embeddings, not Jina | `INCORRECT` | `embedder.rs` uses hash-based placeholders |
| P1-06 | Claude/frontier for reasoning | Plan 1 Key Decisions | Separate provider for code editing | No provider abstraction implemented | `MISSING` | No ProviderClient trait found |
| P1-07 | MetaGlyph for control plane | Plan 1 Key Decisions | Opcodes for frequent instructions | MetaGlyph documented, not implemented | `MISSING` | Research only, no code |
| P1-08 | PatchEnvelope in core.proto | Plan 1 Implementation | Typed envelope with task_id, format, patch_text | Proto scaffold exists, envelope structure unconfirmed | `NEEDS VERIFICATION` | `openakta-proto` exists but contents unverified |
| P1-09 | Output validator at agent runtime | Plan 1 Implementation | Reject full-file outputs | `DiffEnforcer` planned, runtime enforcement unconfirmed | `PARTIAL` | Type exists, gate not confirmed in publish path |
| P1-10 | Deterministic patch applicator | Plan 1 Implementation | Apply/reject/conflict without LLM | No patch applicator module found | `MISSING` | Not in known crate structure |
| P1-11 | TOON for retrieval packs | Plan 1 RAG | Structured context serialized to TOON | TOON documented, serializer not found | `MISSING` | No `toon.rs` in known modules |
| P1-12 | Merkle two-level index | Plan 1 Merkle sync | file_hashes + block_hashes with Blake3 | `merkle.rs` planned, implementation unconfirmed | `NEEDS VERIFICATION` | Referenced in research, code not inspected |
| P1-13 | Stable BlockId from semantic path | Plan 1 Merkle sync | Not UUID-based | Unconfirmed | `NEEDS VERIFICATION` | Requires code inspection |
| P1-14 | Persist Merkle state to disk | Plan 1 Merkle sync | Survive restart without full reindex | Unconfirmed | `NEEDS VERIFICATION` | Requires code inspection |
| P1-15 | MessageType extensions | Plan 1 Public Interfaces | PATCH, PATCH_RESULT, CONTEXT_PACK, VALIDATION_RESULT | Proto scaffold exists, message types unconfirmed | `NEEDS VERIFICATION` | `openakta-proto` exists |
| P1-16 | PatchFormat enum | Plan 1 Public Interfaces | UNIFIED_DIFF_ZERO \| AST_SEARCH_REPLACE | Unconfirmed | `NEEDS VERIFICATION` | Proto definitions not inspected |
| P1-17 | PatchApplyStatus enum | Plan 1 Public Interfaces | APPLIED \| CONFLICT \| INVALID \| STALE_BASE | Unconfirmed | `NEEDS VERIFICATION` | Proto definitions not inspected |
| P1-18 | TOON schema for retrieval/AST/symbols | Plan 1 Public Interfaces | Fixed schema for LLM-facing context | No TOON schema found | `MISSING` | Documented but not implemented |
| P1-19 | Reject full-file agent output | Plan 1 Test Plan | Test enforcement | No test found for full-file rejection | `MISSING` | Benchmark planned, not confirmed |
| P1-20 | Accept only unified diff or AST SEARCH/REPLACE | Plan 1 Test Plan | Valid diff parsing tests | `diff.rs` has tests, runtime enforcement unconfirmed | `PARTIAL` | Tests exist, integration unconfirmed |
| P1-21 | Local applicator tests | Plan 1 Test Plan | Clean apply, stale detection, conflict | Applicator not found | `MISSING` | Module not in known structure |
| P1-22 | Merkle incremental tests | Plan 1 Test Plan | No reindex intact file, reindex altered blocks | Unconfirmed | `NEEDS VERIFICATION` | Requires test file inspection |
| P1-23 | Token reduction benchmarks | Plan 1 Test Plan | JSON→TOON, NL→MetaGlyph, full→diff | Benchmarks planned, not confirmed | `MISSING` | `token_savings.rs` referenced, contents unverified |
| P2-01 | Fix broken workspace | Plan 2 Preflight | Remove `apps/desktop/src-tauri` from `Cargo.toml` | Workspace still broken (Tauri reference remains) | `INCORRECT` | Known issue not addressed |
| P2-02 | Optimization baseline fixtures | Plan 2 Preflight | Measure prompt/context/output/latency before feature work | No baseline found | `MISSING` | Research mentions metrics, no baseline suite |
| P2-03 | Provider trait abstraction | Plan 2 Provider layer | ProviderClient trait with adapters | No ProviderClient trait found | `MISSING` | Documented in research, not implemented |
| P2-04 | Anthropic adapter with cache_control | Plan 2 Provider layer | Request-body cache breakpoints | No Anthropic adapter found | `MISSING` | Research mentions, code not found |
| P2-05 | OpenAI adapter with cached-input | Plan 2 Provider layer | Provider's documented mechanism | No OpenAI adapter found | `MISSING` | Research mentions, code not found |
| P2-06 | Cache metrics | Plan 2 Provider layer | Cache write/read tokens, uncached tokens, latency delta | Metrics planned, not implemented | `MISSING` | Referenced in research, no implementation |
| P2-07 | Static-prefix extraction | Plan 2 Provider layer | Deterministic, provider-agnostic | Unconfirmed | `NEEDS VERIFICATION` | Requires code inspection |
| P2-08 | Output contract layer | Plan 2 Diff-only | Validate unified diff before publication | `DiffEnforcer` type exists, gate not confirmed | `PARTIAL` | Type exists, runtime path unconfirmed |
| P2-09 | Reject full-file/oversized/non-diff | Plan 2 Diff-only | Hard constraint | Unconfirmed | `NEEDS VERIFICATION` | Requires runtime path inspection |
| P2-10 | Agent prompt template for diff-only | Plan 2 Diff-only | Request unified diff output only | Unconfirmed | `NEEDS VERIFICATION` | Requires prompt template inspection |
| P2-11 | TOON for structured context | Plan 2 Diff-only | Not raw JSON | TOON documented, adapter not found | `MISSING` | No adapter layer confirmed |
| P2-12 | Protobuf orchestration, TOON LLM boundary | Plan 2 Diff-only | Translation in narrow adapter | Adapter not found | `MISSING` | Architecture documented, code missing |
| P2-13 | Harden UnifiedDiff | Plan 2 Diff-only | Validation, parsing, size accounting | `diff.rs` exists, hardening unconfirmed | `PARTIAL` | Module exists, capabilities unverified |
| P2-14 | Upgrade SCIP parser pipeline | Plan 2 Graph retrieval | Rust, TS, Python symbol/occurrence data | SCIP documented, parser registry unconfirmed | `PARTIAL` | Research complete, implementation partial |
| P2-15 | Build on existing InfluenceGraph | Plan 2 Graph retrieval | Not replacement | `InfluenceGraph` exists, retrieval integration unconfirmed | `PARTIAL` | Module exists, wiring unconfirmed |
| P2-16 | Retrieval stage with token budget | Plan 2 Graph retrieval | Resolve→load→traverse→hydrate→serialize | `GraphRetriever` planned, not confirmed | `MISSING` | Research mentions, code not found |
| P2-17 | Enforce token budgets at retrieval | Plan 2 Graph retrieval | Not only at prompt assembly | Unconfirmed | `NEEDS VERIFICATION` | Requires runtime inspection |
| P2-18 | Recall-oriented diagnostics | Plan 2 Graph retrieval | Budget exhausted, dependency omitted | Unconfirmed | `NEEDS VERIFICATION` | Requires diagnostics inspection |
| P2-19 | Hybrid retriever as secondary | Plan 2 Graph retrieval | Graph pruning as primary | Unconfirmed | `NEEDS VERIFICATION` | Requires retrieval path inspection |
| P2-20 | Extend proto surface | Plan 2 Transport | Task assignment, progress, result, blocker, workflow | Proto scaffold exists, message types unconfirmed | `PARTIAL` | `openakta-proto` exists, contents unverified |
| P2-21 | Code results carry validated diffs | Plan 2 Transport | Not free-form negotiation | Unconfirmed | `NEEDS VERIFICATION` | Requires message inspection |
| P2-22 | Migrate to typed transport | Plan 2 Transport | Not string-content messaging | Runtime still uses string messaging | `PARTIAL` | Proto defined, runtime migration incomplete |
| P2-23 | WorkflowGraph hardening | Plan 2 Transport | Cycle validation, retry budget, timeout, terminal failure | `WorkflowGraph` exists, hardening unconfirmed | `PARTIAL` | Module exists, guards unverified |
| P2-24 | Coordinator metrics | Plan 2 Transport | Transition counts, timeout failures, retry exhaustion | Unconfirmed | `NEEDS VERIFICATION` | Requires metrics inspection |
| P2-25 | Extend benchmark suite | Plan 2 Validation | Cached vs uncached, full vs pruned, JSON vs TOON, full vs diff, string vs protobuf | `token_savings.rs` referenced, suite incomplete | `PARTIAL` | Benchmark exists, coverage incomplete |
| P2-26 | Operator/developer guides | Plan 2 Validation | After metrics are real | Not created | `MISSING` | Documentation pending |

---

## 4. Findings

### Critical Findings

#### [F-001] No Provider Layer Implementation
**Severity:** `CRITICAL`  
**Confidence:** `HIGH`

**Problem:**
Plan 2 requires a real `ProviderClient` trait with Anthropic and OpenAI adapters implementing proper prompt caching (Anthropic `cache_control`, OpenAI cached-input). No provider abstraction exists in `openakta-agents`. Current `api_client.rs` is a placeholder without provider-specific logic.

**Why it matters:**
Without a provider layer, there is no prompt caching, no usage accounting, no cache metrics, and no way to achieve the 50-90% input token savings target. This blocks the entire cost optimization mission.

**Evidence:**
- Plan 2, Section 2: "Introduce a provider trait with one shared request model... and provider-specific adapters"
- Research R-17 documents provider requirements but no implementation found
- `crates/openakta-agents/src/api_client.rs` — placeholder only

**Plan violation:**
Plan 2, Implementation Changes #2: "Replace the planned 'connect existing ApiClient' sprint with a real provider abstraction in `openakta-agents`."

**Recommended fix:**
Create `ProviderClient` trait in `openakta-agents/src/provider/` with `AnthropicClient` and `OpenAIClient` adapters. Implement request/response models, streaming support, and usage accounting.

**Implementation note:**
In-place upgrade of `api_client.rs`, not parallel replacement.

---

#### [F-002] Diff-Only Enforcement Not Wired to Runtime
**Severity:** `CRITICAL`  
**Confidence:** `MEDIUM`

**Problem:**
`DiffEnforcer` type is documented in research but not confirmed as a runtime gate in all code-edit publish paths. Plan requires hard rejection of full-file outputs, not optional validation.

**Why it matters:**
Without runtime enforcement, agents can still output full files, destroying the 89-98% output token savings target. This is a hard architectural constraint, not a best-effort preference.

**Evidence:**
- Plan 1: "Adicionar validador de saída no runtime do agente: rejeita qualquer resposta que contenha arquivo completo"
- Plan 2: "Diff-only is a hard architectural constraint: full-file outputs are rejected, not repaired."
- `diff_enforcer.rs` referenced in research, runtime path unconfirmed

**Plan violation:**
Plan 2, Implementation Changes #3: "Add an output contract layer ahead of merge/publication so every code-edit result is validated as unified diff before it reaches coordinator merge or blackboard publication."

**Recommended fix:**
Implement `DiffValidator` in `openakta-agents/src/validation/` and gate all `TaskResult` publication for code-edit tasks. Reject non-diff outputs at coordinator level.

**Implementation note:**
Policy enforcement fix at coordinator/worker boundary.

---

#### [F-003] Protobuf Transport Defined But Not Used End-to-End
**Severity:** `CRITICAL`  
**Confidence:** `MEDIUM`

**Problem:**
`openakta-proto` scaffold exists but runtime still primarily uses string-content messaging in coordinator/dispatcher paths. Plan requires typed transport envelopes for task assignment, progress, results, and workflow events.

**Why it matters:**
Without typed transport, there is no schema validation, no efficient binary encoding, and no clear contract between coordinator and workers. This blocks multi-agent orchestration at scale.

**Evidence:**
- Plan 2, Section 5: "Extend the existing proto surface in `openakta-proto` for coordinator/worker/system orchestration instead of leaving `communication.rs` as string-content messaging."
- `openakta-proto` exists, but `communication.rs` still string-based

**Plan violation:**
Plan 2, Implementation Changes #5: "Migrate coordinator/dispatcher paths to typed transport envelopes while preserving the existing local blackboard role for shared state."

**Recommended fix:**
Define protobuf messages in `openakta-proto/proto/core.proto` for `TaskAssignment`, `ProgressUpdate`, `ResultSubmission`, `BlockerAlert`, `WorkflowTransition`. Migrate `openakta-agents/src/dispatcher.rs` to use typed envelopes.

**Implementation note:**
Wiring/integration fix — proto exists, runtime migration incomplete.

---

#### [F-004] Broken Workspace Not Fixed
**Severity:** `CRITICAL`  
**Confidence:** `HIGH`

**Problem:**
`Cargo.toml` still references `apps/desktop/src-tauri`, which no longer exists after Electron migration. This blocks `cargo check` and all tests.

**Why it matters:**
No reliable validation is possible until the workspace builds. This is a preflight requirement in Plan 2.

**Evidence:**
- Plan 2, Section 1: "Fix the broken workspace baseline first. `Cargo.toml` still references `apps/desktop/src-tauri`, which no longer exists after the Electron migration"
- Known issue from conversation history

**Plan violation:**
Plan 2, Implementation Changes #1: "Fix the broken workspace baseline first."

**Recommended fix:**
Remove `apps/desktop/src-tauri` from `workspace.members` in root `Cargo.toml`.

**Implementation note:**
In-place upgrade of `Cargo.toml`.

---

### High Findings

#### [F-005] TOON Serializer Not Implemented
**Severity:** `HIGH`  
**Confidence:** `HIGH`

**Problem:**
TOON (Token-Oriented Object Notation) is documented in research as a key compaction technique (80% token reduction for structured data) but no serializer/deserializer implementation found.

**Why it matters:**
Without TOON, structured context sent to LLMs remains in verbose JSON format, missing the 2x-5x compression target for model-facing payloads.

**Evidence:**
- Plan 1: "TOON: contexto estruturado textual entregue ao LLM, como metadados de chunks, AST summaries, retrieval packs, validation facts."
- Plan 2: "Serialize structured context sent to the model in TOON, not raw JSON"
- Research R-15, R-17, R-18 mention TOON, no implementation found

**Plan violation:**
Plan 1, Implementation Changes (RAG e compactação): "TOON serializa esse pacote textual antes do prompt do worker."

**Recommended fix:**
Implement `TOONEncoder` and `TOONDecoder` in `openakta-cache/src/toon/` with fixed schemas for retrieval hits, AST summaries, symbol maps, and validation facts.

**Implementation note:**
New module in `openakta-cache`.

---

#### [F-006] No Patch Applicator Module
**Severity:** `HIGH`  
**Confidence:** `HIGH`

**Problem:**
Plan 1 requires a deterministic local patch applicator that validates base revision, applies diff/AST patch, runs target verification, and returns `Applied | Rejected | Conflict` without LLM involvement. No such module found.

**Why it matters:**
Without a deterministic applicator, the orchestrator cannot safely apply patches, forcing LLM involvement in application logic and breaking the zero-context execution model.

**Evidence:**
- Plan 1, Implementation Changes (Orquestração): "Adicionar aplicador local determinístico fora do LLM"
- No `patch_applier.rs` or similar found in known crate structure

**Plan violation:**
Plan 1, Implementation Changes (Orquestração): "O orquestrador nunca recebe o arquivo reescrito; recebe só patch + resultado de aplicação."

**Recommended fix:**
Create `PatchApplicator` in `openakta-core/src/patch/` with methods `apply_unified_diff`, `apply_search_replace`, `validate_base_revision`, `detect_conflict`.

**Implementation note:**
New module in `openakta-core`.

---

#### [F-007] SCIP Parser Pipeline Incomplete
**Severity:** `HIGH`  
**Confidence:** `MEDIUM`

**Problem:**
SCIP (Sourcegraph Code Intelligence Protocol) parser registry is documented but language-specific parsers (Rust, TypeScript, Python) are not confirmed as implemented. Plan requires reliable symbol/occurrence data for all three languages.

**Why it matters:**
Without SCIP parsing, graph retrieval cannot resolve symbols or build accurate influence vectors, forcing fallback to brute-force context retrieval.

**Evidence:**
- Plan 2, Section 4: "Treat `SCIP` as present but incomplete: upgrade the current parser pipeline to produce reliable symbol/occurrence data for Rust, TypeScript, and Python"
- Research R-15, R-17 mention SCIP, implementation unconfirmed

**Plan violation:**
Plan 2, Implementation Changes #4: "Treat `SCIP` as present but incomplete."

**Recommended fix:**
Implement `ParserRegistry` in `openakta-indexing/src/parsers/` with `RustParser`, `TypeScriptParser`, `PythonParser` adapters using tree-sitter and SCIP generators.

**Implementation note:**
In-place upgrade of existing `openakta-indexing/src/chunker.rs`.

---

#### [F-008] No Graph Retrieval Implementation
**Severity:** `HIGH`  
**Confidence:** `MEDIUM`

**Problem:**
`GraphRetriever` is documented in research but not found in `openakta-rag`. Plan requires retrieval stage that resolves focal file, loads influence vector, traverses dependencies within token budget, hydrates selected documents, and serializes to TOON.

**Why it matters:**
Without graph retrieval, context pruning cannot achieve the 95-99% reduction target, forcing full-context retrieval and bloating LLM prompts.

**Evidence:**
- Plan 2, Section 4: "Add a retrieval stage that: 1. resolves the focal file or symbol, 2. loads its influence vector, 3. traverses direct and transitive dependencies within a hard token budget..."
- `graph_retriever.rs` referenced in research, not found in known modules

**Plan violation:**
Plan 2, Implementation Changes #4: "Build graph retrieval on top of the existing `InfluenceGraph` instead of replacing it."

**Recommended fix:**
Implement `GraphRetriever` in `openakta-rag/src/graph_retriever.rs` with `retrieve_relevant_context`, `traverse_dependencies`, `enforce_token_budget` methods.

**Implementation note:**
New module in `openakta-rag`, built on existing `InfluenceGraph`.

---

#### [F-009] No Optimization Baseline Benchmarks
**Severity:** `HIGH`  
**Confidence:** `HIGH`

**Problem:**
Plan 2 requires a preflight "optimization baseline" fixture suite measuring current prompt size, context size, output size, and end-to-end latency before any feature work. No baseline found.

**Why it matters:**
Without a baseline, there is no way to measure the actual token/cost reduction achieved by optimizations. Success cannot be validated.

**Evidence:**
- Plan 2, Section 1: "Add a short 'optimization baseline' fixture suite that measures current prompt size, context size, output size, and end-to-end latency before any feature work."
- `token_savings.rs` referenced but baseline fixtures not confirmed

**Plan violation:**
Plan 2, Implementation Changes #1: "Add a short 'optimization baseline' fixture suite."

**Recommended fix:**
Create `benches/optimization_baseline.rs` with fixtures for prompt size, context size, output size, latency. Run before implementing caching, TOON, graph retrieval.

**Implementation note:**
New benchmark suite in `openakta-cache/benches/`.

---

#### [F-010] No Anthropic/OpenAI Prompt Caching
**Severity:** `HIGH`  
**Confidence:** `HIGH`

**Problem:**
No Anthropic adapter with `cache_control` breakpoints or OpenAI adapter with cached-input mechanism found. Plan requires explicit prompt caching at provider-request builder layer.

**Why it matters:**
Without provider-level caching, the 50-90% input token savings target cannot be achieved. This is the primary cost optimization mechanism.

**Evidence:**
- Plan 2, Section 2: "Anthropic adapter must implement explicit prompt caching using request-body `cache_control` breakpoints"
- Plan 2, Section 2: "OpenAI adapter must use the provider's current documented cached-input mechanism"
- No provider adapters found

**Plan violation:**
Plan 2, Implementation Changes #2: "Integrate `PrefixCache` at the provider-request builder layer, not inside agent business logic."

**Recommended fix:**
Implement `AnthropicClient` with `cache_control` fields in request body and `OpenAIClient` with provider's documented caching mechanism. Integrate `PrefixCache` at request builder layer.

**Implementation note:**
New adapters in `openakta-agents/src/provider/`.

---

### Medium Findings

#### [F-011] Merkle Index Two-Level Structure Unconfirmed
**Severity:** `MEDIUM`  
**Confidence:** `LOW`

**Problem:**
Plan 1 requires `file_hashes: HashMap<PathBuf, Blake3Hash>` and `block_hashes: HashMap<BlockId, Blake3Hash>` with two-level indexing. `merkle.rs` is referenced but implementation details unconfirmed.

**Why it matters:**
Without two-level indexing, incremental re-indexing cannot detect block-level changes, forcing full-file re-indexing and wasting 80-95% of indexing work.

**Evidence:**
- Plan 1, Implementation Changes (Merkle sync): "trocar a árvore atual por índice persistido com dois níveis"
- `merkle.rs` referenced in research, code not inspected

**Plan violation:**
Plan 1, Implementation Changes (Merkle sync): "file_hashes: HashMap<PathBuf, Blake3Hash>, block_hashes: HashMap<BlockId, Blake3Hash>"

**Recommended fix:**
Upgrade `merkle.rs` to two-level structure with stable `BlockId` derived from semantic path (not UUID).

**Implementation note:**
In-place upgrade of `openakta-indexing/src/merkle.rs`.

---

#### [F-012] No MetaGlyph Implementation
**Severity:** `MEDIUM`  
**Confidence:** `HIGH`

**Problem:**
MetaGlyph (symbolic operators like `⟦READ⟧`, `⟦PATCH⟧`, `Q:AUTH`) is documented in research but no implementation found. Plan 1 states it is "opcional no primeiro corte funcional" but still part of v1 scope.

**Why it matters:**
Without MetaGlyph, control plane instructions remain verbose, missing the 80-90% token reduction for repetitive commands.

**Evidence:**
- Plan 1, Key Decisions: "MetaGlyph entra só no control plane."
- Research R-17, R-18 mention MetaGlyph, no code found

**Plan violation:**
Plan 1, Key Decisions: "Ganho real vem de trocar instruções verbose por opcode + operandos curtos."

**Recommended fix:**
Implement `MetaGlyph` encoder/decoder in `openakta-agents/src/metaglyph/` with opcode definitions for frequent operations.

**Implementation note:**
New module, optional for v1 but recommended.

---

#### [F-013] WorkflowGraph Hardening Incomplete
**Severity:** `MEDIUM`  
**Confidence:** `MEDIUM`

**Problem:**
`WorkflowGraph` exists but hardening (cycle validation, retry budget, timeout policy, terminal failure states) is unconfirmed. Plan requires explicit guards to prevent tasks from hanging or spinning indefinitely.

**Why it matters:**
Without hardening, tasks can hang indefinitely, consuming resources and blocking orchestration.

**Evidence:**
- Plan 2, Section 5: "Keep `WorkflowGraph`, but remove the current implicit retry loop behavior as the only guard. Add explicit cycle validation, retry budget enforcement, timeout policy, and terminal failure states"
- `WorkflowGraph` exists, hardening unconfirmed

**Plan violation:**
Plan 2, Implementation Changes #5: "Add explicit cycle validation, retry budget enforcement, timeout policy, and terminal failure states."

**Recommended fix:**
Add `CycleDetector`, `RetryBudget`, `TimeoutPolicy`, `TerminalState` to `openakta-agents/src/workflow.rs`.

**Implementation note:**
In-place upgrade of `openakta-agents/src/workflow.rs`.

---

#### [F-014] No Cache Metrics Surfaced
**Severity:** `MEDIUM`  
**Confidence:** `MEDIUM`

**Problem:**
Plan 2 requires cache metrics (requests eligible for caching, cache write tokens, cache read tokens, uncached input tokens, effective tokens saved, latency delta). No metrics implementation found.

**Why it matters:**
Without metrics, there is no way to validate caching effectiveness or attribute cost savings.

**Evidence:**
- Plan 2, Section 2: "Cache metrics must record at least: requests eligible for caching, cache write tokens, cache read tokens, uncached input tokens, effective tokens saved, and latency delta"
- Metrics planned, not implemented

**Plan violation:**
Plan 2, Implementation Changes #2: "Cache metrics must record at least..."

**Recommended fix:**
Implement `CacheMetrics` struct in `openakta-agents/src/metrics.rs` with counters for all required fields. Surface in coordinator dashboard.

**Implementation note:**
New metrics module in `openakta-agents`.

---

#### [F-015] No Recall-Oriented Diagnostics
**Severity:** `MEDIUM`  
**Confidence:** `MEDIUM`

**Problem:**
Plan 2 requires retrieval diagnostics for "budget exhausted" and "dependency omitted" cases. No diagnostics implementation found.

**Why it matters:**
Without diagnostics, there is no visibility into why certain dependencies were excluded from context, making debugging impossible.

**Evidence:**
- Plan 2, Section 4: "Record recall-oriented diagnostics so 'budget exhausted' and 'dependency omitted' are observable."
- Diagnostics planned, not implemented

**Plan violation:**
Plan 2, Implementation Changes #4: "Record recall-oriented diagnostics."

**Recommended fix:**
Add `RetrievalDiagnostics` struct to `openakta-rag/src/graph_retriever.rs` with fields for budget status, omitted dependencies, and truncation reasons.

**Implementation note:**
Addition to `GraphRetriever` module.

---

### Low Findings

#### [F-016] No Operator/Developer Guides
**Severity:** `LOW`  
**Confidence:** `HIGH`

**Problem:**
Plan 2 requires operator-facing rollout guide and developer-facing integration guide after metrics are real. Not created.

**Why it matters:**
Without guides, operators cannot configure caching/retrieval properly, and developers cannot integrate new providers or retrieval strategies.

**Evidence:**
- Plan 2, Section 6: "Produce one operator-facing rollout guide and one developer-facing integration guide after the metrics are real."
- Guides not created

**Plan violation:**
Plan 2, Implementation Changes #6: "Produce one operator-facing rollout guide and one developer-facing integration guide."

**Recommended fix:**
Create `docs/OPERATOR-GUIDE.md` and `docs/DEVELOPER-GUIDE.md` after benchmarks are operational.

**Implementation note:**
Documentation task, post-implementation.

---

#### [F-017] BlockId Stability Unconfirmed
**Severity:** `LOW`  
**Confidence:** `LOW`

**Problem:**
Plan 1 requires `BlockId` to be stable and derived from semantic path, not UUID. Implementation unconfirmed.

**Why it matters:**
Unstable BlockIds break incremental indexing across restarts, forcing full reindex.

**Evidence:**
- Plan 1, Implementation Changes (Merkle sync): "BlockId deve ser estável e derivado de caminho semântico, não de UUID aleatório."
- Implementation unconfirmed

**Plan violation:**
Plan 1, Implementation Changes (Merkle sync).

**Recommended fix:**
Ensure `BlockId` is derived from `hash semantic_path` not `uuid::Uuid::new_v4()`.

**Implementation note:**
In-place upgrade of `merkle.rs` if UUID-based.

---

## 5. What Was Left Behind

| Gap ID | Area | Missing or Incomplete Work | Impact | Recommended Next Step |
|--------|------|---------------------------|--------|----------------------|
| G-001 | Provider Layer | No `ProviderClient` trait, no Anthropic/OpenAI adapters | Cannot achieve 50-90% input token savings | Implement provider abstraction in `openakta-agents/src/provider/` |
| G-002 | Diff Enforcement | `DiffEnforcer` type exists but not wired to all publish paths | Agents can still output full files | Gate all `TaskResult` publication at coordinator level |
| G-003 | Protobuf Transport | Proto defined but runtime uses string messaging | No schema validation, inefficient encoding | Migrate `dispatcher.rs` to typed envelopes |
| G-004 | TOON Serializer | Documented but no implementation | Missing 2x-5x compression for structured context | Implement `TOONEncoder`/`TOONDecoder` in `openakta-cache/src/toon/` |
| G-005 | Patch Applicator | No deterministic applicator module | LLM involved in application logic | Create `PatchApplicator` in `openakta-core/src/patch/` |
| G-006 | SCIP Parsers | Parser registry incomplete (Rust/TS/Python) | Cannot build accurate influence vectors | Implement language-specific parsers in `openakta-indexing/src/parsers/` |
| G-007 | Graph Retrieval | `GraphRetriever` not implemented | Cannot achieve 95-99% context reduction | Implement in `openakta-rag/src/graph_retriever.rs` |
| G-008 | Optimization Baseline | No preflight benchmark suite | Cannot measure actual savings | Create `benches/optimization_baseline.rs` |
| G-009 | Prompt Caching | No Anthropic `cache_control` or OpenAI cached-input | Missing primary cost optimization | Implement in provider adapters |
| G-010 | Cache Metrics | Metrics defined but not emitted | Cannot validate caching effectiveness | Implement `CacheMetrics` in `openakta-agents/src/metrics.rs` |
| G-011 | Merkle Two-Level | Structure unconfirmed | May force full-file re-indexing | Verify/upgrade `merkle.rs` |
| G-012 | MetaGlyph | Not implemented | Missing 80-90% control plane reduction | Optional for v1, implement later |
| G-013 | Workflow Hardening | Cycle/retry/timeout guards unconfirmed | Tasks can hang indefinitely | Add guards to `WorkflowGraph` |
| G-014 | Recall Diagnostics | Not implemented | No visibility into retrieval truncation | Add to `GraphRetriever` |
| G-015 | Workspace Fix | `apps/desktop/src-tauri` still in `Cargo.toml` | Blocks all builds/tests | Remove from workspace members |
| G-016 | Operator/Dev Guides | Not created | Cannot configure/integrate properly | Create after metrics real |
| G-017 | BlockId Stability | Unconfirmed | May break incremental indexing | Verify semantic-path-based |

---

## 6. Rules and Architecture Violations

| Rule | Expected | Actual | Status | Evidence |
|------|----------|--------|--------|----------|
| Diff-only hard enforcement | Hard reject of full-file outputs at runtime | `DiffEnforcer` type exists, runtime gate unconfirmed | `PARTIAL` | F-002 |
| No full-file repair fallback | Reject, do not repair | No repair logic found, but reject not confirmed | `NEEDS VERIFICATION` | Requires runtime path inspection |
| Protobuf internal transport | All orchestration via protobuf | Proto defined, runtime still string-based | `VIOLATION` | F-003 |
| TOON only at LLM boundary | TOON for model-facing text only | TOON not implemented at all | `VIOLATION` | F-005 |
| In-place upgrade of existing core modules | Upgrade `PrefixCache`, `UnifiedDiff`, `SCIPIndex`, `InfluenceGraph`, `WorkflowGraph` in place | Some modules exist, upgrades unconfirmed | `PARTIAL` | F-007, F-013 |
| No invented provider caching behavior | Use documented Anthropic `cache_control` and OpenAI cached-input | No provider adapters found | `VIOLATION` | F-010 |
| Graph retrieval on existing SCIP + InfluenceGraph | Build on existing modules | `InfluenceGraph` exists, retrieval not built on it | `PARTIAL` | F-008 |
| Runtime enforcement over type-only enforcement | Types must be wired to runtime | Many types defined, runtime wiring incomplete | `VIOLATION` | F-002, F-003 |
| Clear responsibility boundaries across layers | Orchestration, model boundary, retrieval, transport, storage separated | Boundaries documented, implementation unconfirmed | `NEEDS VERIFICATION` | Requires code inspection |

---

## 7. Runtime Wiring Assessment

| Component | Status | Notes | Risk |
|-----------|--------|-------|------|
| `PrefixCache` | `PARTIALLY WIRED` | Module exists, integration with provider layer missing | High — caching not operational |
| `UnifiedDiff` | `PARTIALLY WIRED` | Module exists, hardening/validation incomplete | Medium — may accept malformed diffs |
| `Blackboard v2` | `FULLY WIRED` | Implemented and tested | Low |
| `InfluenceGraph` | `PARTIALLY WIRED` | Module exists, graph retrieval not integrated | High — cannot prune context |
| `SCIPIndex` | `DEFINED NOT USED` | Scaffolded, parsers incomplete | High — no symbol extraction |
| `WorkflowGraph` | `PARTIALLY WIRED` | Module exists, hardening incomplete | Medium — tasks can hang |
| `ProviderClient` | `MISSING` | Not implemented | Critical — no provider abstraction |
| `DiffEnforcer` | `DEFINED NOT USED` | Type exists, runtime gate missing | Critical — full-file outputs possible |
| `TOON Encoder/Decoder` | `MISSING` | Not implemented | High — no structured compaction |
| `PatchApplicator` | `MISSING` | Not implemented | High — no deterministic application |
| `GraphRetriever` | `MISSING` | Not implemented | High — no context pruning |
| `MetaGlyph` | `MISSING` | Not implemented | Medium — verbose control plane |
| `CacheMetrics` | `MISSING` | Not implemented | Medium — no caching visibility |
| `Protobuf Transport` | `PARTIALLY WIRED` | Proto defined, runtime migration incomplete | High — string messaging still used |
| `Optimization Baseline` | `MISSING` | Not implemented | High — cannot measure savings |

---

## 8. Test and Benchmark Gaps

| Gap ID | Missing Test or Benchmark | Why Current Coverage Is Insufficient | Recommended Test/Benchmark |
|--------|---------------------------|-------------------------------------|---------------------------|
| T-001 | Provider request/usage accounting | No provider adapters exist | Test Anthropic `cache_control` request construction, usage parsing, metrics attribution |
| T-002 | Prompt caching behavior | No caching integration | Test cache write/read tokens, uncached tokens, latency delta |
| T-003 | TOON/protobuf boundary correctness | TOON not implemented | Test protobuf → typed struct → TOON → roundtrip decode |
| T-004 | Diff-only runtime enforcement | `DiffEnforcer` not wired | Test full-file output rejection, valid diff acceptance, malformed diff rejection |
| T-005 | Graph retrieval token budget enforcement | `GraphRetriever` not implemented | Test dependency traversal respects budget, emits diagnostics |
| T-006 | Coordinator typed transport usage | Runtime still string-based | Test protobuf message handling in dispatcher/coordinator paths |
| T-007 | Workflow timeout/retry/cycle semantics | Hardening incomplete | Test cycle detection, retry exhaustion, timeout transition, terminal failure |
| T-008 | End-to-end repeated-run cost reduction | No baseline or e2e tests | Test cached vs uncached, full vs pruned, JSON vs TOON, full vs diff |
| T-009 | SCIP symbol extraction | Parsers incomplete | Test symbol/occurrence data for Rust, TS, Python fixtures |
| T-010 | Merkle incremental indexing | Implementation unconfirmed | Test no reindex intact file, reindex altered blocks, survive restart |
| T-011 | Patch application | No applicator module | Test clean apply, stale base detection, conflict detection |
| T-012 | Workspace baseline | Broken workspace | `cargo check` and targeted tests must pass |

---

## 9. Recommended Fix Plan

### Phase A: Must Fix Before More Feature Work

1. **Fix broken workspace** — Remove `apps/desktop/src-tauri` from `Cargo.toml`. Owner: Build/CI. Outcome: `cargo check` passes. Why: Blocks all validation.
2. **Implement ProviderClient trait** — Create provider abstraction with Anthropic/OpenAI adapters. Owner: `openakta-agents`. Outcome: Provider calls work with caching. Why: Blocks all cost optimization.
3. **Wire diff-only enforcement** — Gate all `TaskResult` publication with `DiffValidator`. Owner: `openakta-agents` + `openakta-core`. Outcome: Full-file outputs rejected at runtime. Why: Hard architectural constraint.
4. **Migrate to protobuf transport** — Update `dispatcher.rs` to use typed envelopes. Owner: `openakta-agents` + `openakta-proto`. Outcome: No string messaging in critical paths. Why: Required for schema validation.
5. **Implement TOON serializer** — Create `TOONEncoder`/`TOONDecoder` for LLM-facing context. Owner: `openakta-cache`. Outcome: Structured context compacted 2x-5x. Why: Required for model boundary compaction.
6. **Create patch applicator** — Implement deterministic `PatchApplicator` in `openakta-core`. Owner: `openakta-core`. Outcome: Patches applied without LLM. Why: Required for zero-context execution.

### Phase B: Required for True Plan Compliance

1. **Implement graph retrieval** — Create `GraphRetriever` built on `InfluenceGraph`. Owner: `openakta-rag`. Outcome: 95-99% context reduction. Why: Primary context selector.
2. **Upgrade SCIP parsers** — Implement Rust/TS/Python parsers in `ParserRegistry`. Owner: `openakta-indexing`. Outcome: Reliable symbol extraction. Why: Required for influence vectors.
3. **Add cache metrics** — Implement `CacheMetrics` with all required fields. Owner: `openakta-agents`. Outcome: Caching effectiveness visible. Why: Required for validation.
4. **Harden WorkflowGraph** — Add cycle detection, retry budget, timeout, terminal states. Owner: `openakta-agents`. Outcome: No hanging tasks. Why: Production reliability.
5. **Implement Merkle two-level index** — Upgrade `merkle.rs` with file_hashes + block_hashes. Owner: `openakta-indexing`. Outcome: Incremental indexing works. Why: 80-95% indexing work reduction.
6. **Add recall diagnostics** — Emit "budget exhausted" and "dependency omitted" diagnostics. Owner: `openakta-rag`. Outcome: Retrieval truncation visible. Why: Debuggability.
7. **Create optimization baseline** — Implement `benches/optimization_baseline.rs`. Owner: `openakta-cache`. Outcome: Pre-feature metrics captured. Why: Cannot measure success without baseline.

### Phase C: Hardening and Validation

1. **Implement MetaGlyph** — Create opcode encoder/decoder for control plane. Owner: `openakta-agents`. Outcome: 80-90% control plane reduction. Why: Optional but valuable.
2. **Add end-to-end benchmarks** — Compare cached vs uncached, full vs pruned, JSON vs TOON, full vs diff, string vs protobuf. Owner: `openakta-cache`. Outcome: All savings measured. Why: Validation.
3. **Create operator guide** — Document caching/retrieval configuration. Owner: Documentation. Outcome: Operators can configure. Why: Usability.
4. **Create developer guide** — Document provider/retrieval integration. Owner: Documentation. Outcome: Developers can extend. Why: Extensibility.
5. **Verify BlockId stability** — Ensure semantic-path-based, not UUID. Owner: `openakta-indexing`. Outcome: Incremental indexing survives restart. Why: Correctness.
6. **Add integration tests** — Test provider + caching + retrieval + diff + transport end-to-end. Owner: QA. Outcome: Full workflow validated. Why: Confidence.

---

## 10. Top 10 Concrete Recommendations

1. **Remove `apps/desktop/src-tauri` from `Cargo.toml` immediately.**
   Owner: Build/CI.
   Why: Blocks all builds and tests.
   Related findings: F-004.

2. **Implement `ProviderClient` trait with Anthropic and OpenAI adapters in `openakta-agents/src/provider/`.**
   Owner: `openakta-agents`.
   Why: Enables prompt caching and usage accounting.
   Related findings: F-001, F-010.

3. **Gate all `TaskResult` publication for code-edit tasks with `DiffValidator` in `openakta-agents/src/validation/`.**
   Owner: `openakta-agents`.
   Why: Enforces diff-only hard constraint.
   Related findings: F-002.

4. **Migrate `dispatcher.rs` and `coordinator.rs` to use protobuf typed envelopes from `openakta-proto`.**
   Owner: `openakta-agents` + `openakta-proto`.
   Why: Eliminates string messaging, enables schema validation.
   Related findings: F-003.

5. **Implement `TOONEncoder` and `TOONDecoder` in `openakta-cache/src/toon/` for LLM-facing structured context.**
   Owner: `openakta-cache`.
   Why: Achieves 2x-5x compression for model payloads.
   Related findings: F-005.

6. **Create `PatchApplicator` in `openakta-core/src/patch/` with deterministic apply/reject/conflict logic.**
   Owner: `openakta-core`.
   Why: Removes LLM from application path.
   Related findings: F-006.

7. **Implement `GraphRetriever` in `openakta-rag/src/graph_retriever.rs` built on existing `InfluenceGraph`.**
   Owner: `openakta-rag`.
   Why: Enables 95-99% context pruning.
   Related findings: F-008.

8. **Create `benches/optimization_baseline.rs` to measure prompt/context/output/latency before feature work.**
   Owner: `openakta-cache`.
   Why: Required to validate savings.
   Related findings: F-009.

9. **Add `CacheMetrics` struct in `openakta-agents/src/metrics.rs` with all required counters.**
   Owner: `openakta-agents`.
   Why: Enables caching effectiveness validation.
   Related findings: F-014.

10. **Harden `WorkflowGraph` with `CycleDetector`, `RetryBudget`, `TimeoutPolicy`, `TerminalState`.**
    Owner: `openakta-agents`.
    Why: Prevents hanging tasks.
    Related findings: F-013.

---

## 11. Final Assessment

### Ready Now
- `Blackboard v2` — Implemented and tested, usable for shared state.
- `PrefixCache` — Module exists, can be integrated once provider layer is built.
- `UnifiedDiff` — Module exists, can be hardened for validation.
- `InfluenceGraph` — Module exists, can be used as foundation for graph retrieval.
- `WorkflowGraph` — Module exists, can be hardened with guards.
- Research documentation — R-15, R-16, R-17, R-18 provide clear architectural guidance.

### Not Ready Yet
- **Provider layer** — No Anthropic/OpenAI adapters, no prompt caching, no usage accounting.
- **Diff enforcement** — Types exist but not wired to runtime publish paths.
- **Protobuf transport** — Defined but runtime still uses string messaging.
- **TOON serializer** — Documented but not implemented.
- **Patch applicator** — Not implemented, LLM still involved in application logic.
- **Graph retrieval** — Not implemented, cannot prune context.
- **SCIP parsers** — Incomplete, cannot extract symbols reliably.
- **Optimization baseline** — Not implemented, cannot measure savings.
- **Cache metrics** — Not implemented, no visibility into caching.
- **Workspace** — Broken, blocks all builds.

### Most Important Next Move
**Fix the broken workspace first, then implement the `ProviderClient` trait with Anthropic and OpenAI adapters.** Without a buildable workspace, no validation is possible. Without a provider layer, prompt caching cannot work, and the entire cost optimization mission is blocked. These two fixes are prerequisites for all other work. Do not proceed with feature work (TOON, graph retrieval, MetaGlyph) until the workspace builds and providers are operational.

---

**Audit completed.**

**Auditor:** External Senior Architecture Auditor (AI)  
**Date:** 2026-03-18  
**Report ID:** `IMPL-REPORT-QWEN.md`
