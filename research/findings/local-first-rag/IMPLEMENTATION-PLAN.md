# Local-First RAG Implementation Plan

**Date:** 2026-03-18  
**Status:** Ready for Implementation  
**Priority:** 🔴 CRITICAL  
**Owner:** Agent B (Storage/Context Specialist)  
**Estimated Duration:** 4 weeks (8 sprints)  

---

## 🎯 Goal

Implement a **zero-cloud-cost RAG system** that:
- ✅ Runs 100% locally (no embedding API calls)
- ✅ Uses <1GB RAM (works on any dev machine)
- ✅ Achieves <100ms retrieval latency
- ✅ Supports incremental indexing (no full re-index on every change)
- ✅ Pure Rust implementation (no Python dependencies)

---

## 📊 Phase Breakdown

### Phase 1: Core Infrastructure (Week 1-2)

**Goal:** Build the four pillars of local-first RAG

#### Sprint 1: Jina Code Embeddings Integration

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL (blocks all other sprints)

**Tasks:**
1. [ ] Download Jina Code v2 weights from HuggingFace
2. [ ] Convert model to Candle Safetensors format
3. [ ] Implement `JinaEmbedder` struct in `openakta-embeddings`
4. [ ] Add CPU-only inference (AVX2 acceleration)
5. [ ] Implement embedding normalization
6. [ ] Add embedding cache (disk-based, avoid re-computation)
7. [ ] Write benchmarks (target: <25ms per block)

**Deliverables:**
- `crates/openakta-embeddings/src/jina.rs` — Jina model wrapper
- `crates/openakta-embeddings/src/cache.rs` — Embedding cache
- `benches/embed_bench.rs` — Performance benchmarks

**Success Criteria:**
- [ ] Can embed 100 code blocks in <3 seconds
- [ ] Single embedding takes <25ms on CPU
- [ ] Cache hit reduces latency to <5ms
- [ ] Memory usage <600MB during inference

**Dependencies:** None (can start immediately)

---

#### Sprint 2: Qdrant Embedded Setup

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL (blocks RAG integration)

**Tasks:**
1. [ ] Add `qdrant-client` crate to workspace (embedded mode)
2. [ ] Implement `VectorStore` initialization
3. [ ] Define payload schema (file path, language, block type, line numbers)
4. [ ] Implement CRUD operations:
   - `insert(embedding, payload)`
   - `delete(block_id)`
   - `update(block_id, embedding, payload)`
   - `search(query_embedding, k, filter)`
5. [ ] Add hybrid search (BM25 + vectors)
6. [ ] Implement payload filtering (filter by file path, language)
7. [ ] Add persistence (survive app restarts)

**Deliverables:**
- `crates/openakta-rag/src/vector_store.rs` — Qdrant wrapper
- `crates/openakta-rag/src/hybrid_search.rs` — BM25 + vectors
- `crates/openakta-rag/src/schema.rs` — Payload schema

**Success Criteria:**
- [ ] Can store 100K vectors in <200MB RAM
- [ ] Search latency <5ms P95
- [ ] Hybrid search improves precision by 15-20%
- [ ] Payload filtering works correctly

**Dependencies:** Sprint 1 (need embedder for testing)

---

#### Sprint 3: AST-Based Code Chunking

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 CRITICAL (blocks indexing)

**Tasks:**
1. [ ] Add `tree-sitter` + language grammars:
   - `tree-sitter-rust`
   - `tree-sitter-typescript`
   - `tree-sitter-python`
   - `tree-sitter-javascript`
2. [ ] Implement `CodeChunker` struct
3. [ ] Extract semantic units:
   - Functions (with full body)
   - Classes (with methods)
   - Modules/imports
   - Type definitions
4. [ ] Implement chunk size normalization:
   - Split large classes into method-level chunks
   - Merge tiny functions (<10 lines) with parent
5. [ ] Add chunk metadata:
   - File path
   - Language
   - Block type (function/class/module)
   - Line numbers (start-end)
   - Parent scope (for nested structures)
6. [ ] Handle edge cases:
   - Malformed code (parsing errors)
   - Mixed-language files (e.g., TSX)
   - Very large files (>10K lines)

**Deliverables:**
- `crates/openakta-indexing/src/chunker.rs` — AST chunker
- `crates/openakta-indexing/src/languages/` — Language-specific logic
- `crates/openakta-indexing/src/metadata.rs` — Chunk metadata

**Success Criteria:**
- [ ] Chunks preserve semantic meaning (functions intact)
- [ ] Chunk size distribution: 50-500 tokens (80% of chunks)
- [ ] Parsing speed: >100 files/sec
- [ ] Error handling: graceful degradation for parse failures

**Dependencies:** None (independent of embedder/vector store)

---

#### Sprint 4: Merkle Tree + Change Detection

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 HIGH (enables incremental indexing)

**Tasks:**
1. [ ] Add `blake3` crate for fast hashing
2. [ ] Implement `MerkleIndex` struct:
   - File-level hashes (detect file changes)
   - Block-level hashes (detect specific changed blocks)
3. [ ] Integrate with file watcher (`notify` crate):
   - Detect file saves in real-time
   - Debounce rapid changes (100ms window)
4. [ ] Implement change detection algorithm:
   - Check file hash first (skip if unchanged)
   - Parse AST only if file hash changed
   - Compare block hashes (find exact changed blocks)
   - Re-index only changed blocks
5. [ ] Add hash persistence:
   - Save hashes to disk (survive restarts)
   - Load hashes on startup
   - Incremental updates (don't rebuild entire index)
6. [ ] Implement garbage collection:
   - Detect deleted blocks (remove from vector store)
   - Periodic cleanup (remove orphaned hashes)

**Deliverables:**
- `crates/openakta-indexing/src/merkle.rs` — Merkle tree index
- `crates/openakta-indexing/src/watcher.rs` — File watcher integration
- `crates/openakta-indexing/src/gc.rs` — Garbage collection

**Success Criteria:**
- [ ] Single function edit triggers re-index of 1 block (not entire file)
- [ ] File save → re-index latency <50ms
- [ ] Re-indexing reduction: 80-95% vs full re-index
- [ ] Hash persistence survives app restart

**Dependencies:** Sprint 3 (need chunker for block hashes)

---

### Phase 2: Integration & Optimization (Week 3-4)

#### Sprint 5: RAG Pipeline Integration

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 CRITICAL (unifies all components)

**Tasks:**
1. [ ] Create `RagPipeline` struct that connects:
   - File watcher → Chunker → Embedder → Vector Store
2. [ ] Implement `retrieve_relevant_context(query, k=10)`:
   - Parse query (extract keywords, file references)
   - Generate query embedding
   - Hybrid search (BM25 + vectors)
   - Apply filters (file paths, languages)
   - Return top-k results with metadata
3. [ ] Add reranking (optional, for precision):
   - Use cross-encoder for fine-grained scoring
   - Re-rank top-20 results to top-10
4. [ ] Integrate with existing `openakta-rag` crate:
   - Merge with BM25-only implementation
   - Add feature flag for vector search
5. [ ] Add retrieval metrics:
   - Latency (P50, P95, P99)
   - Precision@k (user feedback loop)
   - Recall@k (coverage metric)

**Deliverables:**
- `crates/openakta-rag/src/pipeline.rs` — Unified RAG pipeline
- `crates/openakta-rag/src/retrieval.rs` — Retrieval logic
- `crates/openakta-rag/src/reranker.rs` — Cross-encoder reranker

**Success Criteria:**
- [ ] End-to-end retrieval latency <100ms P95
- [ ] Hybrid search improves precision by 15-20% vs BM25-only
- [ ] Reranking improves precision by additional 5-10%
- [ ] Can retrieve from 100K+ blocks in <100ms

**Dependencies:** Sprints 1, 2, 3, 4 (all pillars must be ready)

---

#### Sprint 6: Performance Optimization

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 HIGH (ensures production readiness)

**Tasks:**
1. [ ] Batch embedding for initial scan:
   - Process 100 blocks in parallel
   - Target: >100 blocks/sec
2. [ ] Parallel chunking:
   - Multi-threaded AST parsing
   - Process multiple files concurrently
3. [ ] Disk cache optimization:
   - LRU cache for embeddings
   - Pre-fetch likely-needed embeddings
4. [ ] Memory optimization:
   - Lazy model loading (load only when needed)
   - Unload model after idle timeout (5 minutes)
   - Memory budget enforcement (<1GB hard limit)
5. [ ] Benchmark suite:
   - Initial scan speed (blocks/sec)
   - Retrieval latency (P50, P95, P99)
   - Memory usage (idle vs peak)
   - CPU usage (during indexing)

**Deliverables:**
- `crates/openakta-embeddings/src/batch.rs` — Batch embedding
- `crates/openakta-rag/src/cache.rs` — Disk cache
- `benches/rag_bench.rs` — End-to-end benchmarks

**Success Criteria:**
- [ ] Initial scan: >100 blocks/sec
- [ ] Memory usage: <1GB peak, <300MB idle
- [ ] Retrieval latency: <100ms P95
- [ ] CPU usage: <50% during indexing

**Dependencies:** Sprint 5 (need working pipeline)

---

#### Sprint 7: Developer Experience

**Owner:** Agent B  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM (polish, not critical)

**Tasks:**
1. [ ] Progress bar for initial scan:
   - Show files processed / total files
   - Show ETA for completion
   - Show blocks indexed
2. [ ] Status indicator:
   - Indexing (yellow)
   - Idle (green)
   - Error (red)
3. [ ] Manual re-index command:
   - `openakta index --force`
   - Full re-index (ignore hashes)
4. [ ] Index health checks:
   - Detect corrupted index
   - Auto-repair on startup
   - Alert user if index is stale
5. [ ] Logging/tracing:
   - Debug mode for troubleshooting
   - Log indexing decisions (why was this block indexed?)
   - Log retrieval decisions (why was this result returned?)

**Deliverables:**
- `apps/desktop/src/components/index-status.tsx` — Status UI
- `crates/openakta-daemon/src/commands/index.rs` — CLI commands
- Enhanced logging throughout

**Success Criteria:**
- [ ] User can see indexing progress in real-time
- [ ] User can manually trigger re-index
- [ ] User can diagnose indexing issues via logs
- [ ] Index auto-repairs on corruption

**Dependencies:** Sprint 5 (need working pipeline)

---

#### Sprint 8: Testing & Validation

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🔴 CRITICAL (ensures quality)

**Tasks:**
1. [ ] Unit tests:
   - Chunking logic (correct AST extraction)
   - Hash computation (deterministic, consistent)
   - Change detection (correct delta identification)
2. [ ] Integration tests:
   - Full pipeline (file save → chunk → embed → store → retrieve)
   - Incremental indexing (edit file → verify only changed blocks re-indexed)
3. [ ] Performance benchmarks:
   - Regression detection (fail if latency increases >10%)
   - Resource usage monitoring (fail if RAM >1.2GB)
4. [ ] Test on large codebases:
   - 100K+ LOC projects
   - Multi-language projects
   - Monorepos
5. [ ] Validate resource targets:
   - RAM usage (confirm <1GB)
   - CPU usage (confirm <50% during indexing)
   - Disk usage (confirm <1GB total)
   - Latency (confirm <100ms P95)

**Deliverables:**
- `tests/rag_integration_test.rs` — Integration tests
- `tests/performance_test.rs` — Performance benchmarks
- `docs/PERFORMANCE-VALIDATION.md` — Validation report

**Success Criteria:**
- [ ] All unit tests pass (100% code coverage for critical paths)
- [ ] All integration tests pass
- [ ] Performance benchmarks meet targets
- [ ] Validated on 3+ large codebases (100K+ LOC)

**Dependencies:** All previous sprints (need complete system)

---

## 📊 Resource Allocation

### Team

| Agent | Role | Sprints | Time Commitment |
|-------|------|---------|-----------------|
| **Agent B** | Storage/Context Specialist | 1-8 | 100% (4 weeks) |

### Infrastructure

| Resource | Requirement | Provided By |
|----------|-------------|-------------|
| Development Machine | Any modern laptop (16GB RAM) | User |
| Model Download | 550MB (one-time) | HuggingFace |
| CI/CD | Standard Rust CI | GitHub Actions |
| Benchmarking | Large codebases for testing | Open source repos |

---

## 📈 Success Metrics

### Technical Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| RAM Usage (peak) | <1GB | `htop` during embedding |
| RAM Usage (idle) | <300MB | `htop` at rest |
| Retrieval Latency (P95) | <100ms | End-to-end query time |
| Embedding Speed | >100 blocks/sec | Batch initial scan |
| Change Detection Accuracy | 100% | No false negatives |
| Re-indexing Reduction | 80-95% | vs full re-index |
| Disk Usage | <1GB | Total footprint |
| Cloud Costs | $0 | Monthly bill |

### User Experience Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Initial Scan Time | <5 sec for 10K LOC | User timing |
| File Save → Indexed | <100ms | Event timing |
| Query → Results | <100ms | User-perceived latency |
| Indexing Visibility | Real-time progress bar | User feedback |
| Error Recovery | Auto-repair on startup | User reports |

---

## 🚨 Risk Management

### High-Risk Items

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Jina model too slow on CPU | Medium | High | Use smaller model, reduce dimensions |
| Qdrant Embedded too heavy | Low | Medium | Switch to sqlite-vec |
| AST chunking too complex | Medium | Medium | Cache ASTs, incremental parsing |
| Resource usage too high | Low | High | Add "lite mode", lazy loading |

### Contingency Plans

**If Jina model is too slow (>50ms/block):**
1. Switch to Jina-Code-Tiny (if available)
2. Truncate dimensions to 512 (Matryoshka)
3. Batch inference for initial scan
4. Pre-compute on idle (background worker)

**If Qdrant Embedded uses too much RAM (>500MB):**
1. Switch to sqlite-vec (simpler, lighter)
2. Reduce HNSW parameters
3. Paginate vector loading (lazy loading)

**If AST chunking is too slow:**
1. Cache parsed ASTs (avoid re-parsing)
2. Use incremental parsing (Tree-sitter supports this)
3. Fall back to line-based chunking for unknown languages

---

## 🔗 Dependencies

### Internal Dependencies

| Sprint | Depends On | Blocked By |
|--------|------------|------------|
| 1 | None | None |
| 2 | 1 (for testing) | None |
| 3 | None | None |
| 4 | 3 | None |
| 5 | 1, 2, 3, 4 | None |
| 6 | 5 | None |
| 7 | 5 | None |
| 8 | 5, 6, 7 | None |

### External Dependencies

| Dependency | Purpose | Status |
|------------|---------|--------|
| Jina Code v2 weights | Embedding model | ✅ Available on HuggingFace |
| Candle framework | ML inference | ✅ In workspace deps |
| Qdrant client | Vector store | ✅ In workspace deps |
| Tree-sitter | Code parsing | ✅ In workspace deps |
| BLAKE3 | Hashing | ✅ In workspace deps |

---

## 📅 Timeline

### Week 1: Pillars 1-2
- Sprint 1: Jina Embeddings ✅
- Sprint 2: Qdrant Embedded ✅

### Week 2: Pillars 3-4
- Sprint 3: AST Chunking ✅
- Sprint 4: Merkle Trees ✅

### Week 3: Integration
- Sprint 5: RAG Pipeline ✅
- Sprint 6: Performance Optimization ✅

### Week 4: Polish & Testing
- Sprint 7: Developer Experience ✅
- Sprint 8: Testing & Validation ✅

**Total Duration:** 4 weeks (8 sprints)

---

## 📋 Sprint Templates

Each sprint should create:

1. **Sprint Plan:** `planning/agent-b/SPRINT-BX-*.md`
2. **Implementation:** Code in appropriate crate
3. **Tests:** Unit + integration tests
4. **Benchmarks:** Performance metrics
5. **Completion Report:** `planning/agent-b/done/SPRINT-BX-COMPLETION.md`

Use template: `planning/SPRINT-COMPLETION-TEMPLATE.md`

---

## ✅ Definition of Done

Phase 1 is complete when:
- [ ] All 4 sprints complete
- [ ] All tests passing
- [ ] All benchmarks meet targets
- [ ] Completion reports created

Phase 2 is complete when:
- [ ] All 4 sprints complete
- [ ] End-to-end latency <100ms
- [ ] RAM usage <1GB peak, <300MB idle
- [ ] Validated on 3+ large codebases
- [ ] User-facing features working (progress bar, status indicator)

---

**Ready to execute. This plan enables zero-cloud-cost RAG with <1GB RAM usage.**
