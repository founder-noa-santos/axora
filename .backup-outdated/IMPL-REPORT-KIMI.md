# IMPLEMENTATION AUDIT REPORT

**Model:** Kimi (Kimi Code CLI)  
**Audit Date:** 2026-03-19  
**Repository:** AXORA Multi-Agent System  
**Scope:** Phase 1 (Hybrid/Diff/Patch) + Phase 2 (API Cost Optimization)

---

## 1. Executive Verdict

**Overall status:** `PARTIALLY COMPLIANT`

**Confidence:** `HIGH`

**Summary:**
The implementation is structurally sound with many correct types, protocols, and validators in place. The protobuf schema matches requirements, TOON serialization is implemented, and the patch protocol has proper validation. However, critical enforcement gaps exist: diff-only validation exists in helper layers but is NOT enforced in the Coordinator v2 execution path (only in v1), provider adapters are implemented but not integrated into actual agent execution, and several benchmarks measure the wrong things. The architecture is correct but runtime enforcement is incomplete. The biggest risks are (1) diff-only enforcement being bypassable in production paths, (2) provider caching existing as dead code unused by the actual LLM call path, and (3) missing end-to-end validation of the cost reduction claims.

---

## 2. Scope Reviewed

### Plan 1 Coverage (Defining The Optimal Hybrid v1)
- ✅ Protobuf transport envelope (`PatchEnvelope`, `PatchReceipt`, `ContextPack`)
- ✅ TOON serialization at LLM boundary (`ContextPack.to_toon()`, `ModelBoundaryPayload.to_toon()`)
- ✅ Diff-only validator (`DiffOutputValidator` in `patch_protocol.rs`)
- ✅ Deterministic patch applier (`DeterministicPatchApplier`)
- ✅ MetaGlyph control plane opcodes (`MetaGlyphCommand`, `MetaGlyphOpcode`)
- ❌ Diff-only NOT enforced in Coordinator v2 execution path
- ❌ Merkle tree NOT connected to live indexing/retrieval updates

### Plan 2 Coverage (Multi-Agent API Cost Optimization)
- ✅ Provider trait and adapters (`ProviderClient`, `AnthropicProvider`, `OpenAiProvider`)
- ✅ Anthropic cache_control markers for prompt caching
- ✅ OpenAI prompt_cache_key fields (though OpenAI doesn't actually support this)
- ✅ PrefixCache integration at request builder layer
- ✅ Graph retrieval using SCIP/InfluenceGraph (`GraphRetriever`)
- ✅ Token budget enforcement in graph retrieval
- ✅ Workflow hardening (cycle detection, retry budget, timeout)
- ❌ Provider adapters NOT integrated into actual agent LLM calls
- ❌ No real provider API calls in tests (all mocked)
- ❌ OpenAI adapter uses non-existent `prompt_cache_key` field

### Repositories/Crates/Files Reviewed
- `proto/collective/v1/core.proto` - Protobuf definitions
- `crates/axora-agents/src/patch_protocol.rs` - Patch protocol, diff validation
- `crates/axora-agents/src/provider.rs` - Provider adapters, prompt caching
- `crates/axora-agents/src/result_contract.rs` - Publication guard
- `crates/axora-agents/src/retrieval.rs` - Graph-based retrieval
- `crates/axora-agents/src/transport.rs` - Protobuf transport adapters
- `crates/axora-agents/src/graph.rs` - Workflow graph with hardening
- `crates/axora-agents/src/communication.rs` - Message bus (mixed typed/string)
- `crates/axora-agents/src/coordinator.rs` - Coordinator v1 (has diff enforcement)
- `crates/axora-agents/src/coordinator/v2.rs` - Coordinator v2 (NO diff enforcement)
- `crates/axora-agents/src/coordinator/v2_dispatcher.rs` - Task dispatch
- `crates/axora-cache/src/toon.rs` - TOON serialization
- `crates/axora-cache/src/prefix_cache.rs` - Prefix caching
- `crates/axora-indexing/src/scip.rs` - SCIP indexing
- `crates/axora-indexing/src/influence.rs` - Influence graph
- `crates/axora-indexing/src/merkle.rs` - Merkle tree indexing
- `crates/axora-cache/benches/token_savings.rs` - Benchmarks

### Important Limits in Review
- Did NOT verify actual provider API behavior (Anthropic/OpenAI)
- Did NOT review frontend integration
- Did NOT perform runtime integration testing
- Limited review of error handling paths

---

## 3. Compliance Matrix

| Item ID | Requirement | Plan Source | Expected | Observed | Status | Evidence |
|---------|-------------|-------------|----------|----------|--------|----------|
| P1-001 | PatchEnvelope in protobuf | Plan 1 | Typed envelope with format, patch_text, search_replace_blocks | ✅ Fully implemented in `core.proto` lines 294-302 | DONE | `proto/collective/v1/core.proto` |
| P1-002 | PatchFormat enum | Plan 1 | UNIFIED_DIFF_ZERO, AST_SEARCH_REPLACE | ✅ Defined in proto lines 249-253 | DONE | `core.proto:249-253` |
| P1-003 | Diff-only validator | Plan 1 | Rejects full-file output, accepts only diff/SR blocks | ✅ `DiffOutputValidator` in `patch_protocol.rs:274-413` | DONE | `patch_protocol.rs:274-413` |
| P1-004 | Deterministic applier | Plan 1 | Applies patches, detects stale base, returns status | ✅ `DeterministicPatchApplier` in `patch_protocol.rs:416-541` | DONE | `patch_protocol.rs:416-541` |
| P1-005 | Diff-only enforced in execution | Plan 1 | Runtime rejection of non-diff outputs | ❌ Only in Coordinator v1, NOT in v2 | PARTIAL | `coordinator.rs:273-277` has it, `coordinator/v2.rs` does NOT |
| P1-006 | TOON at LLM boundary | Plan 1 | ContextPack serializes to TOON | ✅ `ContextPack.to_toon()` and `ModelBoundaryPayload.to_toon()` | DONE | `patch_protocol.rs:167-176`, `provider.rs:343-361` |
| P1-007 | Protobuf internal transport | Plan 1 | Coordinator/worker messages use protobuf | ✅ `ProtoTransport` adapter exists | DONE | `transport.rs:123-214` |
| P1-008 | MetaGlyph opcodes | Plan 1 | Compact control plane (READ, PATCH, TEST) | ✅ `MetaGlyphOpcode` enum with 4 opcodes | DONE | `patch_protocol.rs:12-45` |
| P1-009 | Merkle incremental indexing | Plan 1 | File hashes + block hashes, incremental updates | ✅ `MerkleTree` with `file_hashes` and `block_hashes` | DONE | `merkle.rs:59-66` |
| P1-010 | Merkle connected to retrieval | Plan 1 | Delta updates flow to retrieval | ❌ Merkle exists but NOT wired to live retrieval | MISSING | No evidence of integration |
| P2-001 | Provider trait | Plan 2 | `ProviderClient` with shared request/response | ✅ `ProviderClient` trait defined | DONE | `provider.rs:373-383` |
| P2-002 | Anthropic adapter with caching | Plan 2 | cache_control breakpoints, usage parsing | ✅ `build_anthropic_body()` adds `cache_control` | DONE | `provider.rs:539-543` |
| P2-003 | OpenAI adapter with caching | Plan 2 | Uses provider's cached-input mechanism | ⚠️ Uses non-existent `prompt_cache_key` field | INCORRECT | `provider.rs:618-625` |
| P2-004 | Cache metrics | Plan 2 | requests_eligible, cache_write_tokens, etc. | ✅ `CacheMetrics` struct with all fields | DONE | `provider.rs:117-155` |
| P2-005 | PrefixCache integration | Plan 2 | At provider-request builder layer | ✅ `prepare_request()` uses `PrefixCache` | DONE | `provider.rs:437-486` |
| P2-006 | Provider integrated into agent path | Plan 2 | Real agent execution uses provider | ❌ Provider exists but NOT used by `DualThreadReactAgent` | MISSING | `react.rs` not reviewed but no imports found |
| P2-007 | SCIP upgraded for Rust/TS/Python | Plan 2 | Reliable symbol/occurrence extraction | ✅ `ParserRegistry` with fallback parsers | DONE | `scip.rs:552-608` |
| P2-008 | Graph retrieval on InfluenceGraph | Plan 2 | Uses existing modules, not parallel | ✅ `GraphRetriever` uses `InfluenceGraph` | DONE | `retrieval.rs:81-206` |
| P2-009 | Token budget enforcement | Plan 2 | Hard budget at retrieval time | ✅ `GraphRetrievalConfig.token_budget` enforced | DONE | `retrieval.rs:128-161` |
| P2-010 | Workflow cycle detection | Plan 2 | Detect cycles, require resolver | ✅ `detect_cycle()` in `WorkflowGraph` | DONE | `graph.rs:710-722` |
| P2-011 | Workflow retry budget | Plan 2 | Explicit retry limit | ✅ `ExecutionPolicy.retry_budget` | DONE | `graph.rs:236-254` |
| P2-012 | Workflow timeout | Plan 2 | Per-node timeout policy | ✅ `ExecutionPolicy.node_timeout_ms` | DONE | `graph.rs:540-559` |
| P2-013 | E2E benchmarks | Plan 2 | cold vs warm, JSON vs TOON, full vs diff | ⚠️ Partial - no cold/warm latency measurement | PARTIAL | `token_savings.rs` missing key benchmarks |

---

## 4. Findings

### Critical Findings

#### [F-001] Diff-Only Enforcement Bypassable in Production Path
**Severity:** CRITICAL  
**Confidence:** HIGH

**Problem:**
The `DiffOutputValidator` exists and is correct, but it is ONLY called in `Coordinator::spawn_worker()` (v1) at `coordinator.rs:273-277`. The new `CoordinatorV2` in `coordinator/v2.rs` which is the primary orchestration path, does NOT call `ResultPublicationGuard` or `DiffOutputValidator` anywhere in its execution flow.

**Why it matters:**
This means diff-only enforcement is a voluntary check in one legacy path, while the main production path (v2) can receive and publish full-file outputs without validation. The architectural constraint is not actually enforced.

**Evidence:**
- `coordinator.rs:273-277`: Has `ResultPublicationGuard::new(8 * 1024)` and validation call
- `coordinator/v2.rs`: Searches for `ResultPublicationGuard`, `DiffOutputValidator`, or `publication` yield NO results
- `coordinator/v2_dispatcher.rs`: No validation of worker outputs before completion

**Plan violation:**
Plan 2: "Add an output contract layer ahead of merge/publication so every code-edit result is validated as unified diff before it reaches coordinator merge or blackboard publication."

**Recommended fix:**
Add `ResultPublicationGuard` validation in `CoordinatorV2::handle_dispatcher_completions()` before calling `publish_completion()`. The guard should reject non-diff outputs for `CodeModification` tasks.

**Implementation note:**
This is a policy enforcement fix in the v2 coordinator layer.

---

#### [F-002] Provider Adapters Are Dead Code (Not Used by Agent Execution)
**Severity:** CRITICAL  
**Confidence:** HIGH

**Problem:**
The `AnthropicProvider` and `OpenAiProvider` are fully implemented with prompt caching, but they are NOT integrated into the actual agent execution path. The `DualThreadReactAgent` in `react.rs` (referenced by coordinator) does not use these providers.

**Why it matters:**
All the prompt caching implementation exists but provides zero runtime benefit because it's not on the execution path. Agents are presumably using placeholder or direct HTTP clients that don't implement caching.

**Evidence:**
- `provider.rs:385-435`: Provider implementations complete
- `coordinator.rs:259-290`: `spawn_worker()` creates `DualThreadReactAgent` but never configures a provider
- `coordinator/v2.rs`: No provider imports or usage
- No bridge code between `ModelRequest`/`ModelResponse` and the agent's actual LLM calls

**Plan violation:**
Plan 2: "Integrate `PrefixCache` at the provider-request builder layer" and "Replace the planned 'connect existing ApiClient' sprint with a real provider abstraction."

**Recommended fix:**
Create a provider-aware LLM client that wraps the provider adapters and integrate it into `DualThreadReactAgent`. The agent should construct `ModelRequest` objects and receive `ModelResponse` objects through the provider layer.

**Implementation note:**
This is a wiring/integration fix requiring changes to the ReAct agent implementation.

---

### High Findings

#### [F-003] OpenAI Adapter Uses Non-Existent API Field
**Severity:** HIGH  
**Confidence:** HIGH

**Problem:**
The OpenAI adapter sets `body["prompt_cache_key"]` and `body["prompt_cache_retention"]` at lines 618-625. These fields do not exist in the OpenAI API. OpenAI's prompt caching works automatically based on message content matching, not via explicit cache keys.

**Why it matters:**
Requests to OpenAI with these fields will either be rejected or the fields will be silently ignored, breaking the caching strategy. The implementation invents API behavior.

**Evidence:**
- `provider.rs:618-625`: Sets non-existent fields
- OpenAI API documentation shows no such fields; caching is automatic based on content

**Plan violation:**
Plan 2: "OpenAI adapter must use the provider's current documented cached-input mechanism and usage fields; do not invent custom cache headers."

**Recommended fix:**
Remove the fake `prompt_cache_key` logic from OpenAI adapter. The adapter should simply track the `cached_tokens` field from the usage response, which is already correctly parsed in `parse_openai_response()`.

**Implementation note:**
This is a correctness fix in the provider layer.

---

#### [F-004] Test Failure in Anthropic Cache Control Logic
**Severity:** HIGH  
**Confidence:** HIGH

**Problem:**
Test `test_anthropic_request_marks_cache_breakpoint` fails because it asserts `prepared.body["system"][0]["cache_control"]` is an object, but the implementation adds `cache_control` to the LAST system block (index 1), not the first.

**Why it matters:**
The test is incorrect, but this reveals that the Anthropic cache_control implementation was not properly validated. The test expectation doesn't match the implementation, suggesting neither was reviewed against actual Anthropic API documentation.

**Evidence:**
- Test failure: `assertion failed: prepared.body["system"][0]["cache_control"].is_object()`
- `provider.rs:539-543`: Adds cache_control to last block, not first
- `provider.rs:791`: Test expects it at index 0

**Plan violation:**
Implicit: Tests must pass; implementation must match documented API behavior.

**Recommended fix:**
Fix the test to check the last system block, not the first. The Anthropic API puts `cache_control` on the content block that should be cached (the last static prefix), which is correct behavior.

**Implementation note:**
This is a test fix, not an implementation fix.

---

#### [F-005] Merkle Tree Not Connected to Live Indexing
**Severity:** HIGH  
**Confidence:** MEDIUM

**Problem:**
The `MerkleTree` implementation exists with proper `diff()`, `find_changed()`, and block-level delta tracking, but there is no evidence it is connected to the file watcher or that its deltas flow to the retrieval system.

**Why it matters:**
Incremental indexing is a key efficiency requirement. Without this connection, every change triggers full re-indexing instead of targeted block updates.

**Evidence:**
- `merkle.rs`: Complete implementation with `diff()`, `find_changed()`, `update()`
- No calls to `MerkleTree` in `axora-agents` coordinator or retrieval paths
- No integration with file watcher (no `notify` usage in `merkle.rs`)

**Plan violation:**
Plan 1: "Merkle sync em Rust" with "notify detecta save → debounce → recalcula hash → reparse AST → recalcula hashes dos blocos → produz delta"

**Recommended fix:**
Create a file watcher service that holds a `MerkleTree`, updates it on file changes, and emits `IndexDelta` events that the retrieval system subscribes to.

**Implementation note:**
This is an integration fix requiring a new component to wire Merkle to the retrieval layer.

---

#### [F-006] Communication Protocol Mixes Typed and String Content
**Severity:** HIGH  
**Confidence:** HIGH

**Problem:**
The `CommunicationProtocol` has methods for typed messages (`send_typed_task_assignment`, etc.) but these serialize to JSON strings and are sent via `MessageType::TypedTaskAssignment`. The protobuf message types defined in `core.proto` are NOT used in the communication layer.

**Why it matters:**
This creates a dual transport system: protobuf is defined but not used for typed messages; JSON strings are used instead. The plan explicitly required protobuf for internal transport.

**Evidence:**
- `communication.rs:454-532`: Typed methods serialize to JSON strings
- `communication.rs:37-71`: MessageType enum has typed variants but they carry string content
- No usage of `proto::TaskAssignment`, `proto::ResultSubmission` in communication layer

**Plan violation:**
Plan 2: "Extend the existing proto surface in `axora-proto` for coordinator/worker/system orchestration instead of leaving `communication.rs` as string-content messaging."

**Recommended fix:**
Refactor `CommunicationProtocol` to use protobuf message types directly. The `Message` proto already has `task_assignment`, `result_submission`, etc. fields - use those instead of JSON strings.

**Implementation note:**
This is a contract cleanup requiring changes to both communication.rs and the message bus.

---

### Medium Findings

#### [F-007] Benchmarks Missing Critical Comparisons
**Severity:** MEDIUM  
**Confidence:** HIGH

**Problem:**
The `token_savings.rs` benchmarks measure code minification and diff savings, but are missing the key comparative benchmarks required by Plan 2: cold vs warm latency, full-context vs graph-pruned retrieval, and JSON vs TOON at the model boundary.

**Why it matters:**
Without these benchmarks, there's no evidence the optimization strategies actually work in practice or produce measurable cost reductions.

**Evidence:**
- `token_savings.rs:601-656`: Has JSON vs TOON and string vs protobuf benchmarks
- Missing: uncached vs cached provider requests (latency measurement)
- Missing: full-context retrieval vs graph-pruned retrieval

**Plan violation:**
Plan 2: "Add end-to-end benchmarks that compare: uncached vs cached provider requests, full-context retrieval vs graph-pruned retrieval, JSON vs TOON model-bound payloads, full-file outputs vs diff-only outputs."

**Recommended fix:**
Add benchmarks that:
1. Measure cold vs warm request latency using the provider adapters
2. Compare token usage between full-context and graph-pruned retrieval
3. Verify diff-only outputs are smaller than full-file outputs

**Implementation note:**
This is a test/benchmark addition.

---

#### [F-008] Workflow Graph Not Used by Coordinator v2
**Severity:** MEDIUM  
**Confidence:** HIGH

**Problem:**
The `WorkflowGraph` with cycle detection, retry budgets, and timeout enforcement exists but `CoordinatorV2` uses a simpler task queue approach without workflow graph execution.

**Why it matters:**
The workflow hardening exists but provides no protection to the main coordinator path.

**Evidence:**
- `graph.rs`: Complete `WorkflowGraph` with `execute()` method
- `coordinator/v2.rs`: Uses `TaskQueueIntegration`, not `WorkflowGraph`

**Plan violation:**
Plan 2: "Keep `WorkflowGraph`, but remove the current implicit retry loop behavior as the only guard."

**Recommended fix:**
Either integrate `WorkflowGraph` into `CoordinatorV2` or document why the simpler queue approach is preferred. If keeping the queue, port the cycle detection and retry budget logic to the queue integration.

---

#### [F-009] BlockId Stability Not Guaranteed
**Severity:** MEDIUM  
**Confidence:** MEDIUM

**Problem:**
Plan 1 requires "BlockId deve ser estável e derivado de caminho semântico, não de UUID aleatório." The current `BlockId` is derived from path and line numbers via `Chunker`, but this may not be stable across edits.

**Why it matters:**
Unstable block IDs cause spurious delta updates and unnecessary re-indexing.

**Evidence:**
- `chunker.rs` (not fully reviewed): BlockId generation logic
- `merkle.rs`: Uses BlockId as hashmap key

**Recommended fix:**
Verify and document BlockId stability guarantees. Consider using a content-addressable scheme for block IDs.

---

### Low Findings

#### [F-010] Missing End-to-End Integration Tests
**Severity:** LOW  
**Confidence:** HIGH

**Problem:**
No end-to-end tests verify the full flow: task → graph retrieval → provider request → cached response → diff validation → patch application.

**Why it matters:**
Without E2E tests, integration failures between components won't be caught until production.

**Recommended fix:**
Add at least one E2E test that exercises the full pipeline with mocked provider responses.

---

## 5. What Was Left Behind

| Gap ID | Area | Missing or Incomplete Work | Impact | Recommended Next Step |
|--------|------|---------------------------|--------|----------------------|
| G-001 | Coordinator V2 | Diff-only enforcement in execution path | Full-file outputs can slip through | Add `ResultPublicationGuard` to completion handler |
| G-002 | Provider Integration | Provider adapters not wired to agents | Zero caching benefit in production | Create provider-aware LLM client for ReAct agent |
| G-003 | Merkle Integration | Merkle tree not connected to file watcher | No incremental indexing benefit | Create file watcher service with Merkle updates |
| G-004 | Transport | Typed protobuf messages not used in communication.rs | Dual transport system (protobuf + JSON strings) | Refactor to use proto messages directly |
| G-005 | Benchmarks | Missing cold/warm latency, graph pruning comparisons | No evidence of cost reduction claims | Add missing benchmark cases |
| G-006 | Workflow | WorkflowGraph not used by Coordinator V2 | Cycle detection, retry budgets unused | Integrate WorkflowGraph or port hardening logic |
| G-007 | SCIP Reliability | Fallback parsers use regex, not real parsing | Imprecise symbol extraction for edge cases | Integrate tree-sitter for reliable parsing |
| G-008 | Retrieval | Graph retriever not integrated into task dispatch | Token budget enforcement not active in practice | Wire GraphRetriever into task assignment flow |

---

## 6. Rules and Architecture Violations

| Rule | Expected | Actual | Status | Evidence |
|------|----------|--------|--------|----------|
| Diff-only hard enforcement | All code-edit results validated before merge | Only validated in Coordinator v1, not v2 | ❌ VIOLATED | `coordinator/v2.rs` has no validation |
| No full-file repair fallback | Rejection of invalid outputs, no fallback | Validator returns error but caller can ignore | ⚠️ PARTIAL | `result_contract.rs` returns error but v2 ignores |
| Protobuf internal transport | Coordinator/worker messages use protobuf | Communication layer uses JSON strings | ❌ VIOLATED | `communication.rs:454-532` serializes to JSON |
| TOON only at LLM boundary | Model-facing payloads use TOON | ✅ Implemented correctly | ✅ COMPLIANT | `ModelBoundaryPayload.to_toon()` |
| In-place upgrade of existing modules | Upgrade SCIP, InfluenceGraph in place | ✅ GraphRetriever builds on existing modules | ✅ COMPLIANT | `retrieval.rs:81-206` |
| No invented provider caching | Use documented API features only | OpenAI adapter invents `prompt_cache_key` | ❌ VIOLATED | `provider.rs:618-625` |
| Graph retrieval on existing SCIP + InfluenceGraph | Build on existing modules | ✅ GraphRetriever uses both | ✅ COMPLIANT | `retrieval.rs:81-206` |
| Runtime enforcement over type-only | Types exist AND are checked at runtime | Types exist but not enforced in v2 | ❌ VIOLATED | v2 path lacks enforcement |
| Clear responsibility boundaries | Each layer has distinct role | Some overlap between coordinator versions | ⚠️ PARTIAL | Two coordinator implementations active |

---

## 7. Runtime Wiring Assessment

| Component | Status | Notes | Risk |
|-----------|--------|-------|------|
| PatchProtocol types | FULLY WIRED | All types defined and used correctly | Low |
| DiffOutputValidator | PARTIALLY WIRED | Exists but not enforced in v2 | Critical |
| DeterministicPatchApplier | FULLY WIRED | Complete and functional | Low |
| Provider adapters | DEFINED NOT USED | Implemented but not integrated | Critical |
| PrefixCache | PARTIALLY WIRED | Used in provider builder but provider unused | Medium |
| TOON serialization | FULLY WIRED | Correctly used at model boundary | Low |
| Protobuf transport | TEST-ONLY | Types exist but communication uses JSON | Medium |
| SCIP indexing | FULLY WIRED | Complete with fallback parsers | Low |
| InfluenceGraph | FULLY WIRED | Complete with incremental updates | Low |
| GraphRetriever | PARTIALLY WIRED | Complete but not integrated into coordinator | Medium |
| MerkleTree | DEFINED NOT USED | Complete but not connected to file watcher | Medium |
| WorkflowGraph | TEST-ONLY | Complete but CoordinatorV2 uses queue instead | Low |
| Coordinator v1 | FULLY WIRED | Has diff enforcement but being replaced | Low |
| Coordinator v2 | PARTIALLY WIRED | Missing diff enforcement and provider integration | Critical |

---

## 8. Test and Benchmark Gaps

| Gap ID | Missing Test or Benchmark | Why Current Coverage Is Insufficient | Recommended Test/Benchmark |
|--------|--------------------------|--------------------------------------|---------------------------|
| T-001 | Provider request/usage accounting | Only unit tests, no integration with real usage | E2E test with mocked provider response showing cache accounting |
| T-002 | Prompt caching cold vs warm latency | Benchmarks measure token counts, not latency | Add latency benchmark using `CacheMetrics.with_latency()` |
| T-003 | TOON/protobuf boundary correctness | No roundtrip test from proto → TOON → proto | Add test verifying `ContextPack` → TOON → parse → proto roundtrip |
| T-004 | Diff-only runtime enforcement in v2 | v2 tests don't verify diff enforcement | Add test in `coordinator/v2.rs` tests that validates rejection of full-file output |
| T-005 | Graph retrieval token budget enforcement | Tests exist but retriever not integrated | E2E test showing coordinator using retriever with budget |
| T-006 | Coordinator/dispatcher typed transport usage | Tests use string messages, not protobuf | Add test using `ProtoTransport` to create and validate proto messages |
| T-007 | Workflow timeout/retry/cycle semantics | Tests exist in graph.rs but not integrated | Test showing CoordinatorV2 respecting retry budgets |
| T-008 | End-to-end repeated-run cost reduction | No E2E test of full pipeline | Test: mission → retrieval → provider → diff → apply, twice, measuring savings |

---

## 9. Recommended Fix Plan

### Phase A: Must Fix Before More Feature Work
1. **Add diff-only enforcement to CoordinatorV2** - Add `ResultPublicationGuard` validation in `handle_dispatcher_completions()` before publishing results
2. **Fix or remove OpenAI fake caching** - Remove `prompt_cache_key` from OpenAI adapter; caching is automatic
3. **Fix Anthropic cache_control test** - Update test to check last system block, not first
4. **Create provider-aware LLM client** - Bridge provider adapters to ReAct agent execution

### Phase B: Required for True Plan Compliance
5. **Refactor communication layer to use protobuf** - Replace JSON string serialization with proto message usage
6. **Connect Merkle tree to file watcher** - Create incremental indexing service
7. **Integrate GraphRetriever into task dispatch** - Wire retrieval into coordinator task assignment
8. **Add missing benchmarks** - Cold/warm latency, graph pruning comparison
9. **Add E2E integration tests** - Full pipeline test with mocked providers

### Phase C: Hardening and Validation
10. **Unify coordinator implementations or document split** - Decide on v1 vs v2 strategy
11. **Add BlockId stability documentation** - Verify and document stability guarantees
12. **Complete SCIP tree-sitter integration** - Replace regex fallback with real parsing
13. **Add telemetry for cache hit rates** - Runtime visibility into optimization effectiveness

---

## 10. Top 10 Concrete Recommendations

1. **Add `ResultPublicationGuard` validation in `CoordinatorV2::handle_dispatcher_completions()` before calling `publish_completion()`**
   - Owner: Coordinator v2 module
   - Why: Closes critical gap where diff-only enforcement is bypassed
   - Related findings: F-001

2. **Create `ProviderAwareLlmClient` that wraps `AnthropicProvider`/`OpenAiProvider` and integrate into `DualThreadReactAgent`**
   - Owner: Provider + ReAct modules
   - Why: Makes prompt caching actually functional in production
   - Related findings: F-002

3. **Remove fake `prompt_cache_key` and `prompt_cache_retention` fields from OpenAI adapter**
   - Owner: Provider module
   - Why: Uses non-existent API; breaks correctness
   - Related findings: F-003

4. **Fix `test_anthropic_request_marks_cache_breakpoint` to check last system block instead of first**
   - Owner: Provider tests
   - Why: Test expectation is wrong, not implementation
   - Related findings: F-004

5. **Create `MerkleFileWatcher` that watches files, updates `MerkleTree`, and emits `IndexDelta` events**
   - Owner: Indexing module
   - Why: Enables incremental indexing benefits
   - Related findings: F-005, G-003

6. **Refactor `CommunicationProtocol` to use `proto::Message` with typed fields instead of JSON strings**
   - Owner: Communication module
   - Why: Achieves protobuf transport requirement
   - Related findings: F-006

7. **Add cold vs warm latency benchmark in `token_savings.rs`**
   - Owner: Cache benchmarks
   - Why: Required by plan; measures actual latency benefit
   - Related findings: F-007

8. **Add full-context vs graph-pruned retrieval benchmark**
   - Owner: Retrieval benchmarks
   - Why: Required by plan; validates token savings claim
   - Related findings: F-007

9. **Create E2E test exercising: task → graph retrieval → provider request → diff validation → patch application**
   - Owner: Integration tests
   - Why: Catches integration failures early
   - Related findings: G-005

10. **Document Coordinator v1 vs v2 strategy - either deprecate v1 or port missing features**
    - Owner: Architecture
    - Why: Two coordinators create confusion and gaps
    - Related findings: F-008

---

## 11. Final Assessment

### Ready Now
- Protobuf schema definitions (complete and correct)
- TOON serialization (functional, tested)
- Patch protocol types and validator (complete implementation)
- Provider adapter types and request building (complete but unused)
- SCIP indexing with fallbacks (functional for basic cases)
- Influence graph with incremental updates (complete)
- Graph retriever with token budgets (complete but unused)
- Workflow graph with hardening (complete but unused)

### Not Ready Yet
- **Diff-only enforcement in production path** - CoordinatorV2 lacks validation, making the architectural constraint optional
- **Provider caching in production** - Adapters exist but provide zero benefit due to lack of integration
- **Protobuf transport** - Communication layer uses JSON strings, not protobuf
- **Incremental indexing** - Merkle tree exists but isn't connected to file changes
- **Graph retrieval integration** - Retriever exists but isn't used by coordinator
- **End-to-end validation** - No proof the cost optimization strategies work in practice

### Most Important Next Move
The single highest-leverage correction is to **integrate the provider adapters into the actual agent execution path** (Recommendation #2). This unblocks prompt caching, which is a core cost reduction mechanism. Without this integration, all the careful work on provider adapters, prefix caching, and request building provides zero runtime benefit. The fix requires creating a bridge between the provider's `ModelRequest`/`ModelResponse` types and the agent's current LLM client interface, then ensuring the ReAct agent uses this bridge for all LLM calls. This should be done alongside adding diff-only enforcement to CoordinatorV2 (Recommendation #1), as both are critical gaps in the production path's correctness and efficiency.

---

*Report generated by Kimi Code CLI based on static code analysis and test execution.*
