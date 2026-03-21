# RETRIEVAL_PHASE2_IMPLEMENTATION

## Scope
- Finalize the local reranking stage with a real Candle-backed cross-encoder execution path.
- Remove per-request corpus rescans by introducing incremental sync against persisted file state.
- Add typed gRPC integration coverage for the `RetrieveSkills` RPC.

## Fixes

### Fix: Replace bi-encoder approximation with a local Candle cross-encoder

**Files**
- `/Users/noasantos/Fluri/openakta/crates/openakta-rag/src/reranker.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-rag/Cargo.toml`

**Design**
- `CandleCrossEncoder` is now a true local transformer reranker backed by:
  - `candle-core`
  - `candle-nn`
  - `candle-transformers`
  - `tokenizers`
- The reranker loads a BERT-style sequence-classification checkpoint from:
  - `OPENAKTA_CROSS_ENCODER_MODEL_ROOT`
  - default fallback: `.openakta/models/cross-encoder`
- Expected local artifacts:
  - `config.json`
  - `tokenizer.json`
  - `model.safetensors`
- The implementation tokenizes `(query, document)` as a pair, truncates to `max_length`, pads batched inputs, runs `BertModel::forward`, extracts the `CLS` embedding, and applies a classification head to produce relevance logits.

**Caching**
- Model runtime is cached globally via:
  - `static MODEL_CACHE: Lazy<Mutex<HashMap<String, Arc<CachedCrossEncoder>>>>`
- Cache key is `model_root`.
- The MCP path does not reload weights on every request.

**Runtime contract**
- `CrossEncoderScorer` remains the abstraction boundary.
- The trait is now `Send`-safe through `#[async_trait::async_trait]`, which is required by the typed gRPC service path.

**Rust changes**
- Added:
  - `CrossEncoderConfig`
  - `CachedCrossEncoder`
  - `BertSequenceClassifier`
  - `load_device`
  - `compact_document_text`
- Replaced embedding-similarity reranking with actual transformer inference.

### Fix: Make async retrieval contracts `Send` across the gRPC boundary

**Files**
- `/Users/noasantos/Fluri/openakta/crates/openakta-rag/src/reranker.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-memory/src/procedural_store.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-rag/Cargo.toml`
- `/Users/noasantos/Fluri/openakta/crates/openakta-memory/Cargo.toml`

**Problem**
- The typed gRPC integration test exposed that `async fn` in traits did not guarantee `Send` futures for:
  - `CrossEncoderScorer`
  - `SkillIndexBackend`
- That made generic `SkillRetrievalPipeline<I, R>` unusable behind the tonic service wrapper.

**Resolution**
- Added `async-trait = "0.1"` to `openakta-rag` and `openakta-memory`.
- Annotated:
  - `CrossEncoderScorer`
  - `SkillIndexBackend`
  - their concrete implementations and test doubles
- Result: `SkillRetrievalPipeline::retrieve()` is now safely callable from tonic service tasks.

### Fix: Replace per-request corpus rescans with incremental sync

**Files**
- `/Users/noasantos/Fluri/openakta/crates/openakta-memory/src/procedural_store.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/runtime_services.rs`

**Design**
- Added a persisted file-state table:
  - `skill_source_state(source_path, skill_id, checksum, modified_at_ns, file_size_bytes)`
- Added new state carriers:
  - `SkillSourceState`
  - `FastFileSnapshot`
  - `SkillSyncSummary`
- Added `SkillCorpusSynchronizer<I>` to own delta sync.

**Incremental algorithm**
1. Recursively discover only `SKILL.md` files.
2. Read filesystem metadata first:
   - `modified_at_ns`
   - `file_size_bytes`
3. Compare metadata against `skill_source_state`.
4. If metadata is unchanged, skip file hashing and index writes.
5. If metadata changed, compute full-file BLAKE3.
6. If checksum changed:
   - strip frontmatter
   - rebuild `SkillDocument`
   - upsert catalog row
   - upsert dense index
   - upsert sparse index
   - persist new source state
7. Delete catalog/index entries for files no longer present on disk.

**Latency effect**
- Retrieval no longer performs a full ingest/index pass on every request.
- The hot path is now:
  - metadata check
  - optional checksum only for changed files
  - delta-only index updates

**Rust changes**
- `SkillCatalog::ensure_schema()` now provisions `skill_source_state`.
- Added:
  - `SkillCatalog::upsert_source_state`
  - `SkillCatalog::list_source_states`
- `SkillCorpusIngestor::sync()` now persists source-state metadata.
- `SkillRetrievalPipeline::retrieve()` now calls `sync_if_needed()` instead of a full rebuild.
- `runtime_services.rs` now primes the pipeline through `sync_if_needed()`.

### Fix: Make the MCP skill retriever lazy and reusable

**File**
- `/Users/noasantos/Fluri/openakta/crates/openakta-mcp-server/src/lib.rs`

**Design**
- Added `SkillRetrieverService` trait to decouple the tonic service from concrete pipeline construction.
- Added `LazyPipelineSkillRetriever` backed by:
  - `tokio::sync::OnceCell<Arc<SkillRetrievalPipeline>>`
- Result:
  - one pipeline initialization
  - one model load
  - one catalog/index runtime per process

**Testability**
- Added `McpService::with_skill_retriever(...)` under `#[cfg(test)]`.
- This allows a fully typed gRPC test to inject a deterministic pipeline implementation without patching production code paths.

## Features

### Feat: Generic pipeline injection for deterministic retrieval tests

**Files**
- `/Users/noasantos/Fluri/openakta/crates/openakta-memory/src/procedural_store.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-mcp-server/src/lib.rs`

**Design**
- `SkillRetrievalPipeline` is now generic over:
  - `I = SkillIndexBackend`
  - `R = CrossEncoderScorer`
- Added:
  - `SkillRetrievalPipeline::with_components(...)`
- This made it possible to:
  - inject a mock index backend with exact fused candidates
  - inject deterministic cross-encoder scores
  - assert GMM reject-set behavior and knapsack selection exactly

### Feat: End-to-end typed gRPC integration coverage

**File**
- `/Users/noasantos/Fluri/openakta/crates/openakta-mcp-server/src/lib.rs`

**Test added**
- `retrieve_skills_grpc_enforces_budget_and_filters_noise`

**What it validates**
- A real tonic client calls `GraphRetrievalService::RetrieveSkills`.
- Request flows through:
  - typed RPC service
  - `SkillRetrievalPipeline`
  - MemGAS accept/reject split
  - cross-encoder reranking
  - knapsack selection
- Response assertions cover:
  - one selected skill
  - correct `skill_id`
  - `token_cost <= requested budget`
  - `selected_count == 1`
  - `reject_count == 1`
  - `used_tokens <= requested budget`

**Bootstrap hardening**
- Added bounded client connect retry in the test to eliminate server-start race failures.

### Feat: Incremental-sync unit coverage

**File**
- `/Users/noasantos/Fluri/openakta/crates/openakta-memory/src/procedural_store.rs`

**Tests added**
- `incremental_sync_only_indexes_changed_files`
- `pipeline_respects_token_budget_and_rejects_noise`

**Assertions**
- Unchanged `SKILL.md` files are skipped on the second sync.
- Index writes happen only when file state changes.
- Reject-set candidates do not appear in the final selected payload.
- Knapsack output respects the exact token ceiling.

## Architectural Notes

### Cross-encoder model shape
- The implementation assumes a BERT-compatible sequence-classification checkpoint.
- Current head loading supports either:
  - `classifier`
  - `score`
- This covers common `sentence-transformers` / MS MARCO classifier naming patterns.

### Why the gRPC test uses deterministic fused candidates
- Pure RRF over a tiny three-document set compresses scores into nearly identical values.
- That is not a stable way to prove a reject-set assertion in an integration test.
- The typed RPC test therefore injects a deterministic post-RRF backend and leaves:
  - GMM
  - cross-encoder
  - knapsack
  - gRPC serialization
  under full end-to-end execution.

### Remaining production gap
- The local reranker is now a real Candle transformer path, but production deployment still requires shipping an actual quantized checkpoint into `.openakta/models/cross-encoder` or `OPENAKTA_CROSS_ENCODER_MODEL_ROOT`.
- The runtime is ready; model packaging is the remaining operational step.

## Verification
- `cargo check -q -p openakta-rag -p openakta-memory -p openakta-mcp-server`
- `cargo test -q -p openakta-memory incremental_sync_only_indexes_changed_files`
- `cargo test -q -p openakta-memory pipeline_respects_token_budget_and_rejects_noise`
- `cargo test -q -p openakta-mcp-server retrieve_skills_grpc_enforces_budget_and_filters_noise`
