# IMPLEMENTATION AUDIT REPORT

## 1. Executive Verdict
Overall status: `NON-COMPLIANT`

Confidence: `HIGH`

Summary:
The repository restored workspace compileability and added several plan-shaped contracts: protobuf message types, provider adapters, diff validators, graph retrieval types, and persisted Merkle state. That is not enough for compliance. The runtime still relies on string-content messaging, placeholder LLM and agent execution, prose completions in `CoordinatorV2`, and unwired patch-application, retrieval, and provider layers. The dominant risks are correctness and architecture drift, not polish, because the code repeatedly defines the right abstractions without routing real execution through them. Targeted tests for provider caching and coordinator creation already fail, which confirms the current implementation is not operationally hardened.

## 2. Scope Reviewed
- Plan 1 coverage: diff-only protocol, protobuf vs TOON boundary, patch envelope/apply flow, retrieval/context compaction, Merkle/block hashing, and required validation/benchmarking.
- Plan 2 coverage: workspace preflight, provider layer and prompt caching, diff-only enforcement, model-bound TOON compaction, graph retrieval on existing `SCIP` and `InfluenceGraph`, protobuf transport, workflow hardening, validation, and benchmarks.
- Repositories/crates/files reviewed: `/Users/noasantos/Fluri/openakta/Cargo.toml`, `/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto`, `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/{communication.rs,transport.rs,provider.rs,patch_protocol.rs,result_contract.rs,coordinator.rs,coordinator/v2.rs,react.rs,retrieval.rs,decomposer.rs,decomposer/llm_decomposer.rs,agent.rs,graph.rs}`, `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs`, `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/{merkle.rs,indexer.rs,chunker.rs}`, `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs`, `/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs`.
- Commands executed: `cargo check`; targeted tests for provider caching and coordinator creation; codebase-wide `rg` searches for runtime wiring.
- External references checked: official Anthropic prompt-caching docs and OpenAI model/API docs on cached input, only to validate time-sensitive provider assumptions.
- Important limits: no live provider credentials were used, so provider behavior was audited from code, tests, and official documentation rather than live API calls.

## 3. Compliance Matrix
| Item ID | Requirement | Plan Source | Expected | Observed | Status | Evidence |
|--------|-------------|-------------|----------|----------|--------|----------|
| P1-01 | Diff-only becomes a prerequisite, not a later optimization | PLAN 1 | Code-edit execution should be patch-only from the main runtime path | Diff validator exists, but only one coordinator path uses it and `CoordinatorV2` still emits prose completions | `INCORRECT` | [result_contract.rs:37](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs#L37), [coordinator.rs:268](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L268), [coordinator/v2.rs:238](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L238) |
| P1-02 | Only `git diff --unified=0` or AST SEARCH/REPLACE are accepted for code edits | PLAN 1 | Full-file bodies and prose must be rejected | `DiffOutputValidator` enforces this locally, but not all runtime publication paths use it | `PARTIAL` | [patch_protocol.rs:273](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L273), [coordinator/v2.rs:349](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L349) |
| P1-03 | Orchestrator should operate in zero-context patch execution and receive only patch plus apply result | PLAN 1 | Coordinator dispatches targets and receives patch/apply status, not rewritten files or prose | `CoordinatorV2` manufactures completion strings; `coordinator.rs` publishes raw `result.output` to blackboard | `INCORRECT` | [coordinator/v2.rs:239](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L239), [coordinator.rs:332](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L332) |
| P1-04 | Protobuf is the internal transport contract | PLAN 1 | Runtime messages should travel as typed proto fields | `Message.content` remains present and is still the primary payload carrier | `INCORRECT` | [core.proto:66](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L66), [core.proto:155](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L155), [communication.rs:396](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L396) |
| P1-05 | TOON should be used only for LLM-facing structured context | PLAN 1 | TOON appears in the model adapter layer, not as a generic transport payload | TOON is produced for provider requests, but `ContextPack` also stores `toon_payload` and `communication.rs` sends TOON through generic `content` | `PARTIAL` | [provider.rs:455](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L455), [core.proto:284](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L284), [communication.rs:428](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L428) |
| P1-06 | Patch envelope contract in protobuf | PLAN 1 | Typed `PatchEnvelope` with task, targets, format, base revision, validation facts | Implemented in proto and Rust | `DONE` | [core.proto:294](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L294), [patch_protocol.rs:23](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L23) |
| P1-07 | Runtime output validator rejects full-file responses | PLAN 1 | Validation must run before coordinator merge/publication | Validator exists and `coordinator.rs` uses it before returning a task result | `PARTIAL` | [patch_protocol.rs:287](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L287), [coordinator.rs:273](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L273) |
| P1-08 | Deterministic local patch applier validates base revision and returns `Applied|Rejected|Conflict`-style statuses | PLAN 1 | Runtime should apply patches outside the LLM and report statuses deterministically | `DeterministicPatchApplier` exists, but no runtime caller was found outside tests | `PARTIAL` | [patch_protocol.rs:415](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L415), [rg wiring result](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L668) |
| P1-09 | Orchestrator never receives rewritten files | PLAN 1 | Patch/result protocol only | `coordinator.rs` still merges `result.output` blindly and `CoordinatorV2` publishes prose | `INCORRECT` | [coordinator.rs:332](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L332), [coordinator/v2.rs:454](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L454) |
| P1-10 | Retrieval pack should be slim, structured, and TOON-serialized for the coder | PLAN 1 | Retrieval returns symbols/ranges/snippets/hashes under budget | `GraphRetriever` returns TOON payload plus diagnostics, but runtime integration is absent and retrieved content is raw document text from an in-memory map | `PARTIAL` | [retrieval.rs:104](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L104), [retrieval.rs:180](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L180) |
| P1-11 | Jina limited to embeddings; coder LLM limited to edit/review tasks | PLAN 1 | Deterministic routing by operation type | Real provider/runtime routing is not wired; execution is still placeholder-driven | `NEEDS VERIFICATION` | [provider.rs:374](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L374), [react.rs:232](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L232), [decomposer/llm_decomposer.rs:43](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer/llm_decomposer.rs#L43) |
| P1-12 | Merkle index should be two-level and persisted | PLAN 1 | `file_hashes` and `block_hashes` persisted for restart | Data structure exists and can save/load to disk | `DONE` | [merkle.rs:58](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L58), [merkle.rs:122](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L122) |
| P1-13 | `BlockId` should be stable and semantically derived | PLAN 1 | Stable ID from semantic path, not random UUID | Block IDs are stable values, but chunk extraction is still line-based placeholder logic rather than robust semantic parsing | `PARTIAL` | [chunker.rs:170](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/chunker.rs#L170), [merkle.rs:25](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L25) |
| P1-14 | Incremental Merkle flow should only reindex changed blocks and update BM25/vector stores | PLAN 1 | Live delta application into index stores | `IncrementalIndexer::index` still rebuilds comparison trees and leaves file processing as `TODO` | `MISSING` | [indexer.rs:53](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs#L53), [indexer.rs:65](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs#L65) |
| P1-15 | Merkle state should survive restart without full reindex | PLAN 1 | Runtime loads persisted state, not test-only save/load | Persistence helpers exist, but only test usage was found | `PARTIAL` | [merkle.rs:133](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L133), [rg wiring result](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L361) |
| P1-16 | Explicit contracts for `BlockId`, `IndexDelta`, `ContextPack`, `PatchEnvelope`, `PatchReceipt` | PLAN 1 | Shared contracts across crates | Contracts exist | `DONE` | [merkle.rs:39](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L39), [patch_protocol.rs:114](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L114), [core.proto:294](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L294) |
| P1-17 | Tests: reject full-file outputs and validate deterministic patch apply statuses | PLAN 1 | Runtime-adjacent tests should cover rejection and apply receipts | Unit tests exist for validator and applier | `DONE` | [result_contract.rs:97](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs#L97), [patch_protocol.rs:645](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L645) |
| P1-18 | Measure savings for JSON->TOON, NL->MetaGlyph, full-file->diff | PLAN 1 | Benchmarks should cover all required stages | Benchmarks cover JSON vs TOON and diff savings, but no MetaGlyph benchmark or operational baseline | `PARTIAL` | [token_savings.rs:303](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L303), [token_savings.rs:601](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L601) |
| P2-01 | Remove stale Tauri workspace member before feature work | PLAN 2 | Workspace should no longer reference `apps/desktop/src-tauri` | Current workspace members exclude the deleted Tauri path; `cargo check` succeeds | `DONE` | [Cargo.toml:1](/Users/noasantos/Fluri/openakta/Cargo.toml#L1) |
| P2-02 | Add preflight optimization baseline fixture suite | PLAN 2 | Baseline prompt/context/output/latency measurements before feature work | No preflight baseline suite was found | `MISSING` | [token_savings.rs](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs), `rg` search found no baseline suite |
| P2-03 | Introduce a real provider trait with shared request/response models | PLAN 2 | Provider abstraction in `openakta-agents` | Implemented | `DONE` | [provider.rs:374](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L374), [provider.rs:157](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L157) |
| P2-04 | Ship Anthropic and OpenAI adapters in the MVP | PLAN 2 | Both providers implemented and usable | Adapters exist, but actual agent runtime does not call them | `PARTIAL` | [provider.rs:386](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L386), [provider.rs:398](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L398), [rg wiring result](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/lib.rs#L83) |
| P2-05 | Integrate `PrefixCache` in the provider request builder layer | PLAN 2 | Cache segmentation happens at request preparation, not business logic | Implemented in provider builder | `DONE` | [provider.rs:455](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L455), [provider.rs:461](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L461) |
| P2-06 | Anthropic adapter should use documented `cache_control` breakpoints and usage accounting | PLAN 2 | Correct breakpoint placement plus usage parsing | Usage parsing exists, but the adapter fails its own breakpoint-placement test | `INCORRECT` | [provider.rs:523](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L523), [provider.rs:785](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L785) |
| P2-07 | OpenAI adapter must use the provider’s documented cached-input mechanism, not invented fields | PLAN 2 | Request should follow official API surface | Code writes `prompt_cache_key` and `prompt_cache_retention`; this needs verification against official docs and is not exercised in runtime | `NEEDS VERIFICATION` | [provider.rs:618](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L618), [provider.rs:795](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L795) |
| P2-08 | Cache metrics should include eligibility, write/read, uncached, saved, latency delta | PLAN 2 | Metrics should be recorded and surfaced | Metrics struct exists, but no runtime aggregation/emission path was found | `PARTIAL` | [provider.rs:117](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L117), `rg` usage shows only provider/transport-local usage |
| P2-09 | Static-prefix extraction must be deterministic and provider-agnostic | PLAN 2 | System, tools, invariant context cached; dynamic state uncached | Implemented in shared prompt segmentation | `DONE` | [provider.rs:487](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L487), [provider.rs:513](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L513) |
| P2-10 | Add an output contract layer ahead of merge/publication | PLAN 2 | Coordinator merge/publish should be gated by diff validation | One coordinator path uses `ResultPublicationGuard`, but merge and `CoordinatorV2` publication still bypass typed diff results | `PARTIAL` | [coordinator.rs:273](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L273), [coordinator.rs:319](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L319), [coordinator/v2.rs:445](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L445) |
| P2-11 | Reject full-file outputs, mixed prose/code payloads, and malformed diffs | PLAN 2 | Hard reject, no repair path | Validator does reject them, but the guard API can still downgrade accepted publication to `StatusText` when used generically | `INCORRECT` | [patch_protocol.rs:326](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L326), [result_contract.rs:75](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs#L75) |
| P2-12 | Prompt template for code-edit tasks should request unified diff only | PLAN 2 | Production prompts enforce diff-only output | Only provider test fixtures contain `"Return unified diff only."`; no production prompt builder does | `MISSING` | [provider.rs:761](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L761), [react.rs:389](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L389) |
| P2-13 | Serialize model-bound structured context in TOON rather than raw JSON | PLAN 2 | TOON used on model side | Implemented in provider request construction and graph retrieval output | `DONE` | [provider.rs:455](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L455), [retrieval.rs:199](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L199) |
| P2-14 | Keep protobuf on orchestration side and TOON only at model boundary via narrow adapter | PLAN 2 | Coordinator/worker transport remains typed; TOON only in boundary adapter | Transport adapter exists, but communication/runtime still serializes JSON/TOON into generic `content` | `INCORRECT` | [transport.rs:123](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/transport.rs#L123), [communication.rs:460](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L460), [server.rs:180](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs#L180) |
| P2-15 | Preserve existing `UnifiedDiff` and harden it for validation/parsing/size accounting | PLAN 2 | Canonical diff type should be usable in enforcement paths | `UnifiedDiff` has parsing and size helpers, but runtime enforcement is still elsewhere and end-to-end patch flow is unwired | `PARTIAL` | [diff.rs:54](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs#L54), [diff.rs:153](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs#L153) |
| P2-16 | Upgrade `SCIP` pipeline for Rust, TypeScript, and Python using existing registry shape | PLAN 2 | Reliable symbol and occurrence extraction across languages | `GraphRetriever` assumes `SCIPIndex`, but chunking/indexing remain placeholder and no evidence of production-ready multi-language hardening was found in runtime paths | `PARTIAL` | [chunker.rs:170](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/chunker.rs#L170), [indexer.rs:65](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs#L65) |
| P2-17 | Build graph retrieval on top of existing `InfluenceGraph` | PLAN 2 | No parallel graph stack | Implemented on `SCIPIndex` plus `InfluenceGraph` | `DONE` | [retrieval.rs:80](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L80) |
| P2-18 | Retrieval must enforce token budgets and emit recall diagnostics | PLAN 2 | Budget during traversal, observable omissions | Implemented in `GraphRetriever`, but not wired into coordinator/runtime | `PARTIAL` | [retrieval.rs:131](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L131), [retrieval.rs:153](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L153) |
| P2-19 | Graph pruning becomes the primary selector; hybrid retriever secondary | PLAN 2 | Runtime retrieval should prefer graph-pruned selection | No runtime caller for `GraphRetriever` was found | `MISSING` | [retrieval.rs:81](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L81), `rg` wiring showed test-only references |
| P2-20 | Extend proto surface for task/progress/result/blocker/workflow messages | PLAN 2 | Proto contract includes typed orchestration messages | Implemented | `DONE` | [core.proto:216](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L216), [core.proto:223](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L223), [core.proto:233](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L233) |
| P2-21 | Code-result transport payloads carry validated diffs, token usage, and context references | PLAN 2 | Result submission schema is typed | Proto and internal transport types exist | `DONE` | [core.proto:223](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L223), [transport.rs:74](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/transport.rs#L74) |
| P2-22 | Coordinator/dispatcher runtime must migrate to typed transport envelopes | PLAN 2 | Runtime paths use typed envelopes end-to-end | Actual send path still serializes structs to JSON string payloads, and server forwards `content` | `INCORRECT` | [communication.rs:455](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L455), [server.rs:185](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs#L185) |
| P2-23 | `WorkflowGraph` should add explicit cycle validation, retry budgets, timeouts, and terminal failure | PLAN 2 | Workflow control should be explicit rather than implicit retries | Module contains these concepts, but execution is still placeholder-backed and not tied to real provider/edit runtime | `PARTIAL` | [graph.rs:525](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/graph.rs#L525), [graph.rs:621](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/graph.rs#L621) |
| P2-24 | Coordinator metrics should include transitions, timeout failures, retry exhaustion, schema validation failures | PLAN 2 | Operational metrics emitted from runtime | Some workflow metrics exist in-module, but no end-to-end coordinator emission path was confirmed | `PARTIAL` | [graph.rs:665](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/graph.rs#L665), [provider.rs:117](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L117) |
| P2-25 | Reuse `token_savings.rs` direction and add plan-required E2E comparisons | PLAN 2 | Benchmarks for cached vs uncached, retrieval pruning, JSON vs TOON, diff vs full-file, string vs protobuf | Existing benchmark file covers synthetic diff and TOON/protobuf size only | `PARTIAL` | [token_savings.rs:303](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L303), [token_savings.rs:649](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L649) |
| P2-26 | Produce operator-facing rollout guide and developer-facing integration guide after metrics are real | PLAN 2 | Two concrete docs | No such docs were found | `MISSING` | `rg` over `/Users/noasantos/Fluri/openakta/docs` found no rollout/integration guide |
| P2-27 | End-to-end provider missions proving cached-input savings and diff-only merge | PLAN 2 | One mission per provider through real runtime | No E2E path uses the providers; targeted provider and coordinator tests fail | `MISSING` | [provider.rs:785](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L785), [decomposer.rs:345](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer.rs#L345) |

## 4. Findings
### Critical Findings
#### [F-001] Typed protobuf transport exists on paper, but runtime still sends string payloads
Severity: `CRITICAL`
Confidence: `HIGH`

Problem:
- The repository defines typed proto messages for patches, task assignments, progress, results, blockers, and workflow transitions.
- The actual send path in `communication.rs` serializes those Rust structs to JSON strings or TOON strings and places them in generic `Message.content`.
- The gRPC server then forwards `req.content` unchanged into `Message.content`, leaving the free-form string channel alive in the critical path.

Why it matters:
- This violates the transport plan directly. Schema validation, compatibility guarantees, and typed result handling are optional if the runtime still treats `content` as the real payload.
- Any policy relying on typed transport can be bypassed by code paths that continue to read and write `content`.

Evidence:
- [core.proto:66-82](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L66)
- [core.proto:155-168](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L155)
- [communication.rs:391-530](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L391)
- [server.rs:174-196](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs#L174)
- [transport.rs:123-171](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/transport.rs#L123)

Plan violation:
- PLAN 1 required protobuf as the internal transport contract.
- PLAN 2 required coordinator/dispatcher paths to migrate to typed transport envelopes instead of leaving critical paths on string messaging.

Recommended fix:
- Remove `content` as the primary carrier for typed orchestration messages.
- Route coordinator, dispatcher, worker, and server message handling through `ProtoTransport` and the typed proto fields only.
- Add schema-level rejection when a typed message type arrives with only `content` populated.

Implementation note:
- This is a wiring/integration fix and contract cleanup.

#### [F-002] Diff-only is not a hard runtime constraint across coordinator paths
Severity: `CRITICAL`
Confidence: `HIGH`

Problem:
- `DiffOutputValidator` is strict enough to reject full-file outputs and prose for code-edit tasks.
- That enforcement is not universal. `CoordinatorV2` fabricates success completions from task descriptions and publishes prose strings to the blackboard instead of validated diffs.
- `ResultPublicationGuard::publication_payload` can also degrade to `StatusText` instead of refusing publication outright.

Why it matters:
- The plans required diff-only as a hard architectural constraint. The current implementation treats it as a local validator in one path and a soft preference elsewhere.
- This permits silent escape from the patch protocol, which undermines merge safety, patch application determinism, and output accounting.

Evidence:
- [patch_protocol.rs:273-338](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L273)
- [result_contract.rs:75-88](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs#L75)
- [coordinator.rs:268-289](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L268)
- [coordinator/v2.rs:238-245](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L238)
- [coordinator/v2.rs:357-365](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L357)
- [coordinator/v2.rs:445-468](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L445)

Plan violation:
- PLAN 1 required rejection of any full-file output and acceptance only of zero-context diff or SEARCH/REPLACE.
- PLAN 2 required an output contract layer ahead of merge/publication and explicitly said full-file outputs are rejected, not repaired.

Recommended fix:
- Make validated diff output mandatory for every code-edit completion path, including `CoordinatorV2`.
- Delete the `StatusText` fallback for code-modification tasks.
- Gate blackboard publication and coordinator merge on a typed validated patch/result object, not free-form `String`.

Implementation note:
- This is a policy enforcement fix plus deletion of a redundant bypass path.

#### [F-003] The deterministic patch applier is not part of the real execution path
Severity: `CRITICAL`
Confidence: `HIGH`

Problem:
- The codebase includes `DeterministicPatchApplier`, which validates base revisions and returns `Applied`, `Conflict`, `Invalid`, and `StaleBase`.
- No runtime caller was found outside tests. The orchestrator paths do not submit a validated `PatchEnvelope` to the applier and do not consume `PatchReceipt` as the real completion contract.
- Instead, runtime paths return raw output strings or synthetic completion text.

Why it matters:
- This leaves the most important safety property unfulfilled: patch application is not actually deterministic in production flow because it is not happening in production flow.
- Base-revision staleness and merge conflict behavior are therefore implemented but unenforced.

Evidence:
- [patch_protocol.rs:415-540](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L415)
- [patch_protocol.rs:668-720](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L668)
- `rg -n "apply_to_workspace\\(|DeterministicPatchApplier" /Users/noasantos/Fluri/openakta/crates -g '*.rs'` only found definition and tests
- [coordinator.rs:332-345](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L332)

Plan violation:
- PLAN 1 required a deterministic local applier outside the LLM and required the orchestrator to receive patch plus result of application instead of rewritten file content.

Recommended fix:
- Make worker completion produce a `PatchEnvelope`.
- Apply it through `DeterministicPatchApplier` before blackboard publication or task completion.
- Replace string task outputs with `PatchReceipt` plus validated diff metadata in coordinator/runtime structs.

Implementation note:
- This is a wiring/integration fix.

#### [F-004] Provider abstractions exist, but the execution runtime is still placeholder-backed
Severity: `CRITICAL`
Confidence: `HIGH`

Problem:
- `ProviderClient`, `AnthropicProvider`, and `OpenAiProvider` are implemented.
- The agent runtime, decomposer, and ReAct execution still use placeholder logic: synthetic task outputs, deterministic fake decomposition, and simulated planning.
- No real runtime path was found that constructs a `ModelRequest`, calls a provider adapter, parses a `ModelResponse`, and returns a validated diff into coordinator merge.

Why it matters:
- The plan’s core economics depended on provider caching, model-bound TOON payloads, and measured repeated-run savings.
- None of that can be considered implemented if the runtime that should use it still returns deterministic placeholder text.

Evidence:
- [provider.rs:374-484](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L374)
- [react.rs:227-238](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L227)
- [react.rs:320-331](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L320)
- [react.rs:389-401](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L389)
- [decomposer/llm_decomposer.rs:43-129](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer/llm_decomposer.rs#L43)
- [agent.rs:82-89](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/agent.rs#L82)
- `rg -n "prepare_request\\(|parse_response\\(|ProviderClient" /Users/noasantos/Fluri/openakta/crates/openakta-agents -g '*.rs'` found provider-local code and tests, not coordinator/runtime callers

Plan violation:
- PLAN 2 required a real provider layer integrated into `openakta-agents`, not a type-only abstraction living beside placeholder execution.

Recommended fix:
- Replace placeholder agent/decomposer execution with a provider-backed model call path.
- Build `ModelRequest` in the agent runtime, use provider adapters there, validate diff output there, and surface typed token usage back to the coordinator.

Implementation note:
- This is an in-place upgrade and wiring/integration fix.

### High Findings
#### [F-005] Graph retrieval is implemented as a side module, not as the primary runtime selector
Severity: `HIGH`
Confidence: `HIGH`

Problem:
- `GraphRetriever` is correctly built on `SCIPIndex` and `InfluenceGraph`, enforces a hard token budget during traversal, and emits diagnostics.
- No runtime caller was found. The coordinator, dispatcher, and worker execution paths do not use it to assemble context.
- The indexing pipeline feeding it is also incomplete, leaving retrieval dependent on in-memory fixture-like document maps rather than live incremental codebase state.

Why it matters:
- The plan explicitly moved graph pruning into the primary retrieval role for file/symbol-anchored work.
- Without runtime integration, graph retrieval remains a demonstration, not an operating system behavior.

Evidence:
- [retrieval.rs:80-205](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L80)
- `rg -n "GraphRetriever|GraphRetrievalRequest|GraphRetrievalConfig" /Users/noasantos/Fluri/openakta/crates -g '*.rs'` found definitions and tests only
- [indexer.rs:53-72](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs#L53)
- [chunker.rs:170-225](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/chunker.rs#L170)

Plan violation:
- PLAN 2 required graph pruning to become the primary selector, with the hybrid retriever kept as a secondary source.

Recommended fix:
- Integrate `GraphRetriever` into the task-context assembly path for file- or symbol-anchored tasks.
- Feed it from a real incremental index state instead of ad hoc document maps.

Implementation note:
- This is a wiring/integration fix and in-place upgrade of existing retrieval paths.

#### [F-006] Incremental indexing and semantic block stability are still incomplete
Severity: `HIGH`
Confidence: `HIGH`

Problem:
- The persisted Merkle structure is a meaningful step forward, but the actual incremental indexer does not yet process changed files into updated vector/BM25 stores.
- `Chunker::extract_chunks` still uses simple line-based heuristics as a placeholder, which weakens semantic block identity and any retrieval logic depending on stable semantic blocks.
- Persistence helpers are only exercised in tests.

Why it matters:
- The plan required block-level delta handling and restart-safe incremental updates.
- Without that, retrieval freshness, change detection, and token-budget-aware context selection remain unreliable.

Evidence:
- [merkle.rs:58-187](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L58)
- [merkle.rs:122-139](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/merkle.rs#L122)
- [indexer.rs:53-72](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs#L53)
- [chunker.rs:155-225](/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/chunker.rs#L155)
- `rg -n "save_to_path\\(|load_from_path\\(" /Users/noasantos/Fluri/openakta/crates -g '*.rs'` found test-only usage outside the implementation itself

Plan violation:
- PLAN 1 required only changed blocks to be reindexed and persisted for restart.
- PLAN 2 required SCIP and influence-driven retrieval to be production-ready, not placeholder-fed.

Recommended fix:
- Finish `IncrementalIndexer::index` so Merkle deltas drive chunk refresh, embedding refresh, and BM25/vector updates.
- Replace placeholder line-based chunking with robust language-aware chunk extraction where semantic block stability matters.

Implementation note:
- This is an in-place upgrade plus wiring/integration fix.

#### [F-007] Core validation paths are red: coordinator creation and Anthropic caching tests fail
Severity: `HIGH`
Confidence: `HIGH`

Problem:
- `provider::tests::test_anthropic_request_marks_cache_breakpoint` fails because the expected `cache_control` placement is not present where the test expects it.
- `coordinator::tests::test_coordinator_creation` fails because `MissionDecomposer::decompose()` calls `block_in_place` when not on a multi-threaded runtime.
- These are not edge cases; they directly hit the provider layer and coordinator setup path.

Why it matters:
- The plan required a validated runtime baseline before more optimization claims.
- Failing targeted tests in core modules invalidate claims of completion and create immediate risk around coordinator orchestration and provider behavior.

Evidence:
- [provider.rs:785-792](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L785)
- [decomposer.rs:345-354](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer.rs#L345)
- Targeted test result: `cargo test -q -p openakta-agents provider::tests::test_anthropic_request_marks_cache_breakpoint -- --exact` failed at `provider.rs:791`
- Targeted test result: `cargo test -q -p openakta-agents coordinator::tests::test_coordinator_creation -- --exact` failed at `decomposer.rs:347`

Plan violation:
- PLAN 2 made workspace stabilization and reliable validation a prerequisite for further feature work.

Recommended fix:
- Fix the Anthropic request construction to match the intended cache breakpoint contract.
- Remove the invalid `block_in_place` path from synchronous decomposition or constrain it to a confirmed multi-threaded runtime.
- Re-run the full targeted module test suite before claiming provider/coordinator readiness.

Implementation note:
- This is a correctness fix and test hardening item.

#### [F-008] Cache metrics and provider usage accounting stop at builder-local structs
Severity: `HIGH`
Confidence: `HIGH`

Problem:
- `CacheMetrics` and `ProviderUsage` track the right fields.
- The only confirmed usage is request preparation and proto conversion helpers. No runtime coordinator aggregation or operator-facing metric emission path was found.
- That means repeated-run savings and latency improvements are not observable in the actual execution system.

Why it matters:
- The optimization program was supposed to prove savings against a repo-local baseline.
- Metrics that never leave request-construction code cannot support operational validation or benchmarking claims.

Evidence:
- [provider.rs:117-171](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L117)
- [provider.rs:461-484](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L461)
- [transport.rs:23-40](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/transport.rs#L23)
- `rg -n "cache_metrics|effective_tokens_saved|latency_delta_ms" /Users/noasantos/Fluri/openakta/crates/openakta-agents -g '*.rs'` found provider-local definitions and transport mapping, not coordinator/runtime aggregation

Plan violation:
- PLAN 2 required cache metrics to be recorded and surfaced consistently, including token savings and latency deltas.

Recommended fix:
- Plumb `ProviderUsage` and `CacheMetrics` into task results, coordinator metrics, and benchmark capture.
- Emit them from the real provider-backed runtime path rather than only from request builders.

Implementation note:
- This is a wiring/integration fix.

### Medium Findings
#### [F-009] The TOON boundary is narrower than string JSON, but still broader than the plan allowed
Severity: `MEDIUM`
Confidence: `HIGH`

Problem:
- The model-bound payload adapter correctly turns typed payloads into TOON for provider requests.
- However, `ContextPack` stores `toon_payload` directly in the proto schema, and `communication.rs` sends context packs as generic `content` strings after TOON serialization.
- This spreads TOON into transport and storage concerns that the plans wanted to keep typed.

Why it matters:
- Layer leakage makes it harder to reason about where structured context is authoritative and where typed transport ends.
- It also weakens the “protobuf internally, TOON only at the model boundary” rule.

Evidence:
- [provider.rs:455-481](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L455)
- [core.proto:284-292](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L284)
- [communication.rs:423-435](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L423)

Plan violation:
- PLAN 1 and PLAN 2 both required TOON to stay on the model-facing side, not become a general transport payload.

Recommended fix:
- Keep typed context packs on the orchestration side and generate TOON only inside the provider/model adapter.
- Remove `toon_payload` from transport contracts if the system can derive it from typed fields.

Implementation note:
- This is a contract cleanup and responsibility-boundary fix.

#### [F-010] `UnifiedDiff` is functional, but its hardening is not yet sufficient for the claimed end-to-end role
Severity: `MEDIUM`
Confidence: `MEDIUM`

Problem:
- `UnifiedDiff` can generate, parse, and estimate token size for simple zero-context diffs.
- The actual merge/apply/runtime path does not consistently depend on it, so its correctness is not being validated under real coordinator flow.
- The implementation is also intentionally simple and not obviously hardened against all edge cases that a real patch application pipeline will hit.

Why it matters:
- PLAN 2 required the existing `UnifiedDiff` to become the canonical diff type rather than remain a helper beside bypass paths.
- If the system later routes real patches through it, latent edge cases become production failures.

Evidence:
- [diff.rs:54-139](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs#L54)
- [diff.rs:153-240](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs#L153)
- [coordinator.rs:332-345](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L332)

Plan violation:
- PLAN 2 required `UnifiedDiff` to be hardened enough for validation, parsing, and size accounting in the actual diff-only flow.

Recommended fix:
- Route the real code-edit flow through `UnifiedDiff` plus `DeterministicPatchApplier`, then extend tests around multi-hunk, delete-only, create-only, and path-normalization cases.

Implementation note:
- This is an in-place upgrade plus test addition.

#### [F-011] Required operational docs and benchmarks were deferred without the runtime evidence they depend on
Severity: `MEDIUM`
Confidence: `HIGH`

Problem:
- The benchmark file reuses the existing `token_savings.rs` direction, which is correct as a starting point.
- It does not provide the preflight baseline or the plan-required end-to-end comparisons for cached vs uncached requests, graph-pruned vs full retrieval, and diff-only vs full-file runtime behavior.
- The operator rollout guide and developer integration guide are also missing.

Why it matters:
- The plans were explicit: success must be measured against the repo’s own baseline and documented after the metrics are real.
- Without those artifacts, optimization claims remain unverified.

Evidence:
- [token_savings.rs:303-356](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L303)
- [token_savings.rs:601-656](/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs#L601)
- `rg -n "rollout guide|integration guide|operator|developer-facing" /Users/noasantos/Fluri/openakta/docs /Users/noasantos/Fluri/openakta/crates -g '*.md' -g '*.rs'` found no matching implementation docs

Plan violation:
- PLAN 2 required real preflight baselines, end-to-end optimization benchmarks, and two operator/developer guides after metrics were real.

Recommended fix:
- Add the missing baseline and E2E benchmark suite only after runtime integration is real enough to measure.
- Write the guides from the actual runtime path, not the intended architecture.

Implementation note:
- This is a test/benchmark addition and documentation follow-through item.

### Low Findings
#### [F-012] Production diff-only prompt enforcement is not visible outside provider tests
Severity: `LOW`
Confidence: `HIGH`

Problem:
- A test fixture uses the system instruction `"Return unified diff only."`.
- No production prompt builder was found that applies this requirement consistently to code-edit tasks.
- The current ReAct planning path is simulated and does not build a provider-backed code-edit prompt at all.

Why it matters:
- Even after wiring providers in, prompt-level instructions are still part of the contract and should not be left implicit.
- This becomes more important once the placeholder runtime is replaced.

Evidence:
- [provider.rs:760-773](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L760)
- [react.rs:389-401](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs#L389)

Plan violation:
- PLAN 2 required the code-edit prompt template to request unified diff output only.

Recommended fix:
- Centralize code-edit prompt construction in the provider-backed runtime and stamp diff-only instructions there for every code-modification task.

Implementation note:
- This is a policy enforcement fix.

## 5. What Was Left Behind
| Gap ID | Area | Missing or Incomplete Work | Impact | Recommended Next Step |
|-------|------|-----------------------------|--------|-----------------------|
| G-001 | Transport | End-to-end migration off `Message.content` onto typed proto fields | Typed transport remains bypassable and schema validation is weak | Remove string-first send paths and make typed fields mandatory for typed message types |
| G-002 | Diff enforcement | Universal diff-only gating across `CoordinatorV1`, `CoordinatorV2`, merge, and blackboard publication | Code-edit outputs can still escape as prose or raw strings | Introduce a single validated patch completion contract and delete prose completion bypasses |
| G-003 | Patch application | Runtime use of `DeterministicPatchApplier` and `PatchReceipt` | Base-revision and conflict handling are not actually enforced | Apply patches before publication and store receipts as the completion result |
| G-004 | Provider integration | Real provider-backed execution path | Prompt caching and usage accounting are not operational | Build provider calls into worker execution and decomposer/runtime paths |
| G-005 | Retrieval | Coordinator/runtime use of `GraphRetriever` as primary selector | Graph pruning is not affecting actual context assembly | Integrate graph retrieval into task-context preparation for anchored tasks |
| G-006 | Indexing | Finish incremental indexing and semantic chunking | Retrieval freshness and block stability remain unreliable | Implement changed-file processing and replace placeholder chunking where needed |
| G-007 | Metrics | Coordinator-level aggregation of provider/cache/retry/schema metrics | Savings and failures are not observable end-to-end | Plumb metrics into task results, coordinator summaries, and benchmarks |
| G-008 | Validation | Preflight baseline suite and required E2E benchmarks | No evidence for claimed cost reductions | Add baseline and E2E benchmarks after runtime wiring lands |
| G-009 | Documentation | Operator rollout guide and developer integration guide | Operators and contributors lack an accurate integration contract | Write the guides from the actual runtime architecture after metric collection |
| G-010 | Test stability | `decompose()` runtime misuse and provider test failures | Core coordinator/provider tests are red | Fix the blocking runtime bug and cache breakpoint behavior before more feature work |

## 6. Rules and Architecture Violations
| Rule | Expected | Actual | Status | Evidence |
|------|----------|--------|--------|----------|
| Diff-only hard enforcement | All code-edit runtime paths reject non-diff outputs before merge/publication | Only one coordinator path validates; `CoordinatorV2` emits and publishes prose | `VIOLATED` | [coordinator.rs:273](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L273), [coordinator/v2.rs:445](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs#L445) |
| No full-file repair fallback | Invalid code-edit outputs must fail, not degrade to status text | `publication_payload()` can return `StatusText` instead of refusing publication | `VIOLATED` | [result_contract.rs:75-88](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs#L75) |
| Protobuf internal transport | Typed proto messages should be the real internal contract | Runtime still serializes JSON/TOON into `content` | `VIOLATED` | [communication.rs:396-530](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L396), [server.rs:180-186](/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs#L180) |
| TOON only at LLM boundary | TOON generated in a narrow model adapter layer | TOON also appears in `ContextPack` transport and `content` messaging | `PARTIAL` | [core.proto:284-290](/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto#L284), [communication.rs:423-435](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L423) |
| In-place upgrade of existing core modules | Upgrade `UnifiedDiff`, `SCIPIndex`, `InfluenceGraph`, `WorkflowGraph` in place | Some upgrades happened in place, but critical runtime still bypasses them through side abstractions and placeholder flows | `PARTIAL` | [diff.rs:54](/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs#L54), [retrieval.rs:80](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L80), [graph.rs:525](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/graph.rs#L525) |
| No invented provider caching behavior | Provider adapters must follow documented provider APIs | Anthropic breakpoint behavior is failing its own test; OpenAI uses request fields that need verification against official docs | `NEEDS VERIFICATION` | [provider.rs:523-572](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L523), [provider.rs:575-625](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs#L575) |
| Graph retrieval on existing `SCIP` + `InfluenceGraph` | Retrieval built on existing graph/index modules and used as primary runtime selector | Built on existing modules, but not used by runtime | `PARTIAL` | [retrieval.rs:80-205](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L80) |
| Runtime enforcement over type-only enforcement | Contracts should be active in execution, not just defined in types/tests | Providers, transport, patch apply, retrieval, and metrics are largely type-only or test-only | `VIOLATED` | [transport.rs:123-171](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/transport.rs#L123), [patch_protocol.rs:415-540](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs#L415), [retrieval.rs:212-260](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs#L212) |
| Clear responsibility boundaries across layers | Orchestration, model boundary, retrieval, transport, and storage should be separated cleanly | `communication.rs` mixes payload serialization concerns, TOON leaks into transport, and coordinator publishes raw strings | `VIOLATED` | [communication.rs:423-530](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs#L423), [coordinator.rs:332-345](/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs#L332) |

## 7. Runtime Wiring Assessment
| Component | Status | Notes | Risk |
|-----------|--------|-------|------|
| `ProviderClient` adapters | `DEFINED NOT USED` | Implemented for Anthropic and OpenAI, but no coordinator/worker runtime caller was found | High |
| `PrefixCache` request segmentation | `PARTIALLY WIRED` | Implemented in provider builder, but not exercised by actual runtime execution | High |
| `DiffOutputValidator` | `PARTIALLY WIRED` | Real validator, but only one coordinator path uses it | High |
| `ResultPublicationGuard` | `PARTIALLY WIRED` | Exists, but permits `StatusText` fallback and is bypassed by `CoordinatorV2` | High |
| `DeterministicPatchApplier` | `DEFINED NOT USED` | Strong local contract, only test usage confirmed | High |
| `ProtoTransport` | `DEFINED NOT USED` | Converts internal Rust types to proto, but send path does not rely on it | High |
| `communication.rs` typed send helpers | `PARTIALLY WIRED` | Message types exist, but payload bodies are still JSON/TOON strings in `content` | High |
| `GraphRetriever` | `TEST-ONLY` | Good retrieval logic, no runtime callers found | High |
| `MerkleTree` persistence | `TEST-ONLY` | Save/load exists, but runtime persistence/load path not found | Medium |
| `IncrementalIndexer` | `PLACEHOLDER` | `index()` still leaves changed-file processing as `TODO` | High |
| `Chunker` semantic parsing | `PLACEHOLDER` | Uses line-based chunking as a stopgap | Medium |
| `WorkflowGraph` hardening | `PARTIALLY WIRED` | Cycle/retry/timeout semantics exist, but node execution is still placeholder-backed | Medium |
| `DualThreadReactAgent` | `PLACEHOLDER` | Simulated planning, no real dual-thread model runtime | High |
| `LLMDecomposer` | `PLACEHOLDER` | Deterministic backend is default; synchronous wrapper currently panics in targeted test | High |
| `token_savings.rs` benchmarks | `TEST-ONLY` | Useful synthetic benchmarks, but not the plan-required operational suite | Medium |

## 8. Test and Benchmark Gaps
| Gap ID | Missing Test or Benchmark | Why Current Coverage Is Insufficient | Recommended Test/Benchmark |
|-------|----------------------------|--------------------------------------|----------------------------|
| T-001 | End-to-end typed transport test through server/coordinator/worker | Proto messages exist, but runtime still uses `content`; no test proves typed envelopes are the live contract | Add an integration test that sends `TaskAssignment`, `ResultSubmission`, and `WorkflowTransitionEvent` through the real server path with `content` empty |
| T-002 | Runtime diff-only enforcement across all coordinator paths | Current tests cover validator units, not universal coordinator publication behavior | Add integration tests for both coordinator versions proving prose/full-file outputs are rejected before blackboard publish |
| T-003 | Patch application integration test | Applier tests are isolated and do not prove runtime use | Add worker/coordinator test that produces a validated `PatchEnvelope`, applies it, and asserts `PatchReceipt` semantics |
| T-004 | Provider request and usage accounting in real runtime | Provider tests stop at builder/parsing units | Add runtime tests that execute a provider-backed task and assert token usage and cache metrics propagate into task/coordinator results |
| T-005 | Prompt caching behavior validation | Anthropic cache breakpoint test is already failing; OpenAI request fields are not validated against live or contract-level expectations | Fix and expand provider tests, then add contract tests around cached vs uncached request construction and parsed usage attribution |
| T-006 | TOON/protobuf boundary correctness | There is a model-bound TOON roundtrip test, but transport still carries TOON strings directly | Add tests showing typed transport structs convert to TOON only inside the provider adapter and never require `content`/`toon_payload` in transport |
| T-007 | Graph retrieval integration under token budget | `GraphRetriever` unit tests do not prove runtime selection | Add coordinator/worker context assembly tests showing anchored tasks invoke graph retrieval and emit omission diagnostics when budget is exhausted |
| T-008 | Incremental indexing restart and delta propagation | Merkle persistence is only test-local and indexer processing is unfinished | Add restart tests where saved Merkle state is loaded, one file changes, and only affected blocks are re-embedded/reindexed |
| T-009 | Workflow timeout/retry/cycle semantics under real execution | Module tests do not prove coordinator-level behavior once tasks are provider-backed | Add workflow integration tests for cycle rejection, timeout transition, retry exhaustion, and terminal failure propagation |
| T-010 | Preflight optimization baseline | No baseline exists for prompt/context/output/latency before optimization | Add a fixture suite that records baseline prompt size, context size, output size, and latency for representative missions |
| T-011 | End-to-end repeated-run cost reduction per provider | Current benchmarks are synthetic and provider runtime is unwired | Add one repeatable mission per provider, measuring cold vs warm latency and cached-input token savings |
| T-012 | Graph-pruned vs full-context retrieval benchmark | No runtime benchmark demonstrates pruning benefits | Benchmark the same anchored task with graph-pruned retrieval versus unpruned retrieval, capturing token usage and result coverage diagnostics |

## 9. Recommended Fix Plan
### Phase A: Must Fix Before More Feature Work
- Eliminate string-first transport in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs` and `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/server.rs`; expected outcome is typed proto envelopes as the actual runtime contract; this belongs first because every higher-level guarantee depends on transport integrity.
- Remove diff/prose bypasses from `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs` and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/result_contract.rs`; expected outcome is hard rejection of non-diff code-edit outputs on every path; this belongs first because current runtime behavior violates the central architecture rule.
- Wire `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs` into worker completion and coordinator merge; expected outcome is patch application through `DeterministicPatchApplier` with `PatchReceipt`-based completion; this belongs first because diff validation without deterministic application is incomplete.
- Replace placeholder execution in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/react.rs`, `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/agent.rs`, and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer/llm_decomposer.rs` with provider-backed calls; expected outcome is real model execution through the declared provider layer; this belongs first because provider/caching work is otherwise non-operational.
- Fix test blockers in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/decomposer.rs` and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs`; expected outcome is a stable coordinator/provider validation baseline; this belongs first because the current targeted failures undermine all progress claims.

### Phase B: Required for True Plan Compliance
- Integrate `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/retrieval.rs` into anchored task context assembly; expected outcome is graph-pruned retrieval as the primary selector under hard token budgets; this belongs here because plan compliance requires runtime use, not just module existence.
- Finish `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/indexer.rs` and tighten `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/chunker.rs`; expected outcome is live incremental indexing with stable-enough semantic blocks and real store updates; this belongs here because retrieval quality depends on it.
- Constrain TOON generation to the provider/model adapter and remove transport-level TOON leakage from `/Users/noasantos/Fluri/openakta/proto/collective/v1/core.proto` and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/communication.rs`; expected outcome is cleaner layer separation; this belongs here because it resolves a direct architecture rule violation.
- Plumb provider usage, cache metrics, retry metrics, and protocol validation failures into coordinator outputs and metrics aggregation; expected outcome is measurable runtime savings and failure visibility; this belongs here because the plan required observable optimization results.
- Verify provider-specific caching behavior against official APIs and adjust `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs` accordingly; expected outcome is documented, non-invented provider request construction; this belongs here because provider integration must be correct before cost claims are credible.

### Phase C: Hardening and Validation
- Add the missing preflight baseline and end-to-end benchmark suite in `/Users/noasantos/Fluri/openakta/crates/openakta-cache/benches/token_savings.rs` or adjacent benchmark files; expected outcome is measurable before/after optimization evidence; this belongs here because meaningful benchmarking only follows real runtime integration.
- Expand `UnifiedDiff` and patch-application tests in `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/diff.rs` and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/patch_protocol.rs`; expected outcome is confidence in multi-hunk, create/delete, and path-normalization behavior; this belongs here because it is hardening after the main path is wired.
- Add server-to-worker typed transport integration tests and workflow runtime tests; expected outcome is proof that cycles, retries, timeouts, and terminal failures behave correctly under real execution; this belongs here because module-level semantics need end-to-end confirmation.
- Add repeated-run provider missions and graph-pruned retrieval comparisons; expected outcome is actual cached-input savings and context reduction evidence per provider; this belongs here because the cost-optimization plan is not complete without measured repeated-run results.
- Write one operator-facing rollout guide and one developer-facing integration guide in `/Users/noasantos/Fluri/openakta/docs`; expected outcome is an accurate runtime contract for operators and maintainers; this belongs here because documentation should reflect the corrected implementation, not the current hybrid state.

## 10. Top 10 Concrete Recommendations
1. Replace `Message.content` as the payload carrier for typed orchestration messages and require the typed proto fields instead.
Owner:
`openakta-core` and `openakta-agents` transport/runtime
Why:
This closes the largest architecture gap and makes schema enforcement real.
Related findings:
`[F-001]`, `[F-009]`

2. Delete the `StatusText` publication path for code-modification tasks in `ResultPublicationGuard`.
Owner:
`openakta-agents` result contract layer
Why:
Diff-only is currently bypassable by design.
Related findings:
`[F-002]`

3. Rework `CoordinatorV2` so successful code-edit completion requires a validated `PatchEnvelope` and `PatchReceipt`, not synthesized prose.
Owner:
`openakta-agents` coordinator runtime
Why:
`CoordinatorV2` is the clearest live violation of the plans.
Related findings:
`[F-002]`, `[F-003]`

4. Wire `DeterministicPatchApplier` into the worker completion path before blackboard publication.
Owner:
`openakta-agents` worker/coordinator integration
Why:
Without deterministic apply, diff validation is not enough.
Related findings:
`[F-003]`, `[F-010]`

5. Replace placeholder ReAct and decomposition execution with provider-backed model calls that produce validated diff outputs.
Owner:
`openakta-agents` execution/runtime
Why:
Provider work is currently non-operational.
Related findings:
`[F-004]`, `[F-012]`

6. Fix the synchronous decomposition wrapper so it does not call `block_in_place` on unsupported runtimes.
Owner:
`openakta-agents` decomposition
Why:
A core coordinator test already fails on this path.
Related findings:
`[F-007]`

7. Correct Anthropic cache breakpoint placement and then verify OpenAI cached-input request fields against official API docs before shipping them.
Owner:
`openakta-agents` provider layer
Why:
Current provider caching behavior is partly broken and partly unverified.
Related findings:
`[F-007]`, `[F-008]`

8. Integrate `GraphRetriever` into task-context assembly and make it the primary selector for anchored tasks.
Owner:
`openakta-agents` retrieval/runtime
Why:
Graph retrieval currently exists only as a side module.
Related findings:
`[F-005]`

9. Finish `IncrementalIndexer::index` so Merkle deltas drive real chunk, embedding, and store updates.
Owner:
`openakta-indexing`
Why:
Retrieval correctness depends on live, restart-safe incremental indexing.
Related findings:
`[F-006]`

10. Add a real baseline and repeated-run benchmark suite after runtime wiring lands, then write the operator and developer guides from that measured system.
Owner:
`openakta-cache`, `openakta-agents`, `docs`
Why:
The optimization program is not defensible without repo-local measurements and accurate documentation.
Related findings:
`[F-008]`, `[F-011]`

## 11. Final Assessment
### Ready Now
- The workspace baseline is no longer blocked by the removed Tauri member, and `cargo check` succeeds.
- The codebase contains useful raw materials: typed proto definitions, a plausible provider abstraction, a strict diff validator, a deterministic patch applier, graph retrieval logic on top of existing graph/index modules, and persisted Merkle state structures.
- Some unit-level contracts are directionally correct, especially around patch format validation and model-bound TOON serialization.

### Not Ready Yet
- The implementation is not complete enough to claim compliance with either plan because the runtime still bypasses the very contracts those plans required.
- Typed protobuf transport is not the live transport.
- Diff-only is not a universal hard constraint.
- Patch application is not part of the real completion path.
- Provider adapters, graph retrieval, and cache metrics are mostly type-only or test-only from a runtime perspective.
- Core targeted tests in provider and coordinator paths are already failing.

### Most Important Next Move
The single highest-leverage correction is to force the real execution path through one typed completion contract: provider-backed model call -> validated diff output -> `PatchEnvelope` -> deterministic local apply -> `PatchReceipt` -> typed protobuf result submission. That one change collapses several current failures at once: it removes prose completions, makes diff-only enforceable, gives transport a concrete payload to carry, creates a place to attach provider usage and cache metrics, and provides the natural insertion point for graph-pruned context and TOON boundary discipline. Until that path is real, the implementation will continue to look architecturally complete in type definitions while remaining non-compliant in runtime behavior.
