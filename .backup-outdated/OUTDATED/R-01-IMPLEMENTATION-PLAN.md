# R-01 Research Summary & Implementation Plan

**Date:** 2026-03-16  
**Status:** ✅ Research Complete → 🔄 Ready for Implementation  
**Research:** [R-01 Findings](./findings/context-management/R-01-result.md)  
**Architecture:** [architecture-context-rag.md](../docs/architecture-context-rag.md)

---

## Executive Summary

R-01 research on **Context Management & RAG** is complete. Six architectural decisions have been made (ADR-012, ADR-006, ADR-007, ADR-013, ADR-014, ADR-015) defining a production-grade RAG system optimized for code.

**Key Outcome:** AXORA will implement a **Modular RAG** architecture with:
- Hybrid retrieval (BM25 + Jina embeddings)
- AST-based chunking (cAST algorithm)
- Context reordering (counters "Lost in the Middle")
- Merkle tree sync (sub-second incremental updates)

**Competitive Parity:** This architecture matches or exceeds Cursor's RAG system based on available information.

---

## Decisions Made

| ADR | Decision | Impact |
|-----|----------|--------|
| ADR-012 | Modular RAG with 6-stage pipeline | Foundation for all code retrieval |
| ADR-006 | Jina-code-embeddings-1.5b | Best-in-class code embeddings |
| ADR-007 | LanceDB (local) + Qdrant (cloud) | Dual-layer architecture |
| ADR-013 | AST-based chunking (cAST) | 20-30% better retrieval accuracy |
| ADR-014 | Context reordering algorithm | 15-25% better information extraction |
| ADR-015 | Merkle tree state sync | Sub-second incremental sync |

---

## Implementation Roadmap

### Sprint 0: Foundation (Week 1-2)

**Goal:** Set up core dependencies and infrastructure

**Tasks:**
- [ ] Add dependencies to `Cargo.toml`:
  ```toml
  [dependencies]
  # Embedding inference
  candle-core = "0.4"
  candle-transformers = "0.4"
  
  # Vector databases
  lancedb = "0.10"
  qdrant-client = "1.9"
  
  # BM25 / full-text search
  tantivy = "0.22"
  
  # AST parsing
  tree-sitter = "0.22"
  tree-sitter-rust = "0.22"
  tree-sitter-typescript = "0.21"
  tree-sitter-python = "0.22"
  
  # Hashing
  sha2 = "0.10"
  hex = "0.4"
  ```

- [ ] Create `crates/axora-rag/` crate structure
- [ ] Set up basic module structure
- [ ] Create test fixtures (sample codebases)

**Deliverable:** Empty crate with dependencies, compiles successfully

---

### Sprint 1: AST Chunking (Week 3-4)

**Goal:** Implement cAST chunking algorithm

**Tasks:**
- [ ] Implement `CodeChunk` struct with all metadata fields
- [ ] Implement Tree-sitter parser integration
- [ ] Implement cAST split-then-merge algorithm:
  ```rust
  pub fn cast_chunk(code: &str, language: Language) -> Vec<CodeChunk>;
  ```
- [ ] Add non-whitespace character counting
- [ ] Implement greedy sibling merge
- [ ] Write unit tests for chunking correctness
- [ ] Benchmark: chunking speed, chunk size distribution

**Deliverable:** `axora-cast` crate with working AST chunking

**Success Criteria:**
- Chunks align with AST boundaries (functions, classes)
- No chunks exceed 2048 non-whitespace characters
- Chunking speed: >1000 lines/min

---

### Sprint 2: Embedding Inference (Week 5-6)

**Goal:** Integrate Jina-code-embeddings-1.5b

**Tasks:**
- [ ] Download Jina model (ONNX format)
- [ ] Implement embedding inference with Candle:
  ```rust
  pub struct JinaEmbedder {
      model: candle::Model,
      dimensions: usize,  // Configurable via Matryoshka
  }
  
  impl JinaEmbedder {
      pub fn embed(&self, code: &str) -> Result<Vec<f32>>;
  }
  ```
- [ ] Implement Matryoshka truncation (1536 → 512-768)
- [ ] Batch embedding for throughput
- [ ] Benchmark: latency per embedding, GPU vs CPU

**Deliverable:** `axora-embeddings` crate with Jina integration

**Success Criteria:**
- Embedding latency: <100ms per chunk (CPU), <20ms (GPU)
- Embedding dimensions: configurable (256-1536)
- Model size: ~3GB (FP16), ~1.5GB (INT8 quantized)

---

### Sprint 3: Hybrid Retrieval (Week 7-8)

**Goal:** Implement BM25 + Dense retrieval with RRF fusion

**Tasks:**
- [ ] Set up Tantivy index for BM25:
  ```rust
  pub struct BM25Index {
      index: tantivy::Index,
      // ...
  }
  ```
- [ ] Set up LanceDB index for dense vectors:
  ```rust
  pub struct DenseIndex {
      table: lancedb::Table,
      dimensions: usize,
  }
  ```
- [ ] Implement parallel retrieval
- [ ] Implement Reciprocal Rank Fusion:
  ```rust
  pub fn rrf_fusion(bm25: Vec<ChunkId>, dense: Vec<ChunkId>) -> Vec<ChunkId>;
  ```
- [ ] Benchmark: retrieval latency, recall metrics

**Deliverable:** `axora-retrieval` crate with hybrid search

**Success Criteria:**
- Retrieval latency: <200ms (BM25) + <100ms (Dense)
- Recall@10: >85% on test queries
- RRF produces unified, ranked candidate set

---

### Sprint 4: Re-ranking & Selection (Week 9-10)

**Goal:** Implement cross-encoder re-ranking and Knapsack selection

**Tasks:**
- [ ] Integrate cross-encoder model (BGE-reranker-large):
  ```rust
  pub struct CrossEncoderReranker {
      model: candle::Model,
  }
  
  impl CrossEncoderReranker {
      pub fn score(&self, query: &str, chunk: &str) -> f64;
  }
  ```
- [ ] Implement batched re-ranking (top 100 candidates)
- [ ] Implement Knapsack selection algorithm:
  ```rust
  pub fn knapsack_select(chunks: Vec<RankedChunk>, budget: usize) -> Vec<Chunk>;
  ```
- [ ] Integrate token counting for budgeting
- [ ] Benchmark: re-ranking latency, selection quality

**Deliverable:** `axora-rerank` crate with re-ranking + selection

**Success Criteria:**
- Re-ranking latency: <500ms for 100 chunks
- Knapsack respects token budget exactly
- MRR >0.85 on test queries

---

### Sprint 5: Context Reordering (Week 11)

**Goal:** Implement "Lost in the Middle" mitigation

**Tasks:**
- [ ] Implement reordering algorithm:
  ```rust
  pub fn reorder_context(chunks: Vec<RetrievedChunk>) -> Vec<RetrievedChunk>;
  ```
- [ ] Write unit tests for correct alternation pattern
- [ ] A/B test: with vs without reordering
- [ ] Measure impact on downstream LLM performance

**Deliverable:** Reordering function integrated into pipeline

**Success Criteria:**
- 15-25% improvement in information extraction (measured via LLM accuracy)
- Latency overhead: <1ms

---

### Sprint 6: Merkle Tree Sync (Week 12-13)

**Goal:** Implement incremental state synchronization

**Tasks:**
- [ ] Implement Merkle tree data structure:
  ```rust
  pub struct MerkleTree {
      root_hash: String,
      nodes: HashMap<PathBuf, HashNode>,
  }
  
  pub struct HashNode {
      hash: String,  // SHA-256
      children: Vec<PathBuf>,
  }
  ```
- [ ] Implement file hash computation (SHA-256)
- [ ] Implement folder hash computation (recursive)
- [ ] Implement diff algorithm (find divergent branches)
- [ ] Implement incremental sync protocol
- [ ] Benchmark: sync speed for various change scenarios

**Deliverable:** `axora-sync` crate with Merkle tree sync

**Success Criteria:**
- Full tree build: <5s for 50K files
- Incremental sync: <1s for single file change
- Bandwidth: <100KB for typical changes

---

### Sprint 7: Integration & End-to-End (Week 14-15)

**Goal:** Integrate all components into unified pipeline

**Tasks:**
- [ ] Create `RAGPipeline` struct orchestrating all stages:
  ```rust
  pub struct RAGPipeline {
      query_reformulator: QueryReformulator,
      bm25_index: BM25Index,
      dense_index: DenseIndex,
      reranker: CrossEncoderReranker,
      // ...
  }
  
  impl RAGPipeline {
      pub async fn retrieve(&self, query: &str, budget: usize) -> Result<Vec<CodeChunk>>;
  }
  ```
- [ ] Implement query reformulation (small local LLM)
- [ ] Wire all stages together
- [ ] End-to-end latency profiling
- [ ] Optimize bottlenecks
- [ ] Integration tests

**Deliverable:** Fully functional RAG pipeline

**Success Criteria:**
- End-to-end latency (p50): <500ms
- End-to-end latency (p95): <1000ms
- Retrieval Recall@10: >90%

---

### Sprint 8: Benchmarking & Optimization (Week 16-17)

**Goal:** Validate performance against targets

**Tasks:**
- [ ] Run SWE-Bench style retrieval benchmarks
- [ ] Run CodeSearchNet retrieval evaluation
- [ ] Profile memory usage
- [ ] Optimize hot paths
- [ ] Document performance results
- [ ] A/B test: old vs new retrieval

**Deliverable:** Performance report, optimized pipeline

**Success Criteria:**
- All performance targets met (see architecture doc)
- MRR >0.85 on benchmarks
- Memory usage: <500MB for typical codebase

---

## Testing Strategy

### Unit Tests
- [ ] cAST chunking correctness (preserves AST boundaries)
- [ ] RRF fusion correctness
- [ ] Knapsack optimality
- [ ] Context reordering pattern
- [ ] Merkle tree hash computation

### Integration Tests
- [ ] End-to-end retrieval latency
- [ ] Incremental sync correctness
- [ ] Indexing speed benchmarks

### Evaluation Benchmarks
- [ ] SWE-Bench Verified (adapted)
- [ ] CodeSearchNet retrieval metrics
- [ ] Custom "Find the bug" tasks

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| cAST chunking too slow | High | Medium | Profile early, optimize Tree-sitter parsing |
| Jina embeddings too large for local | High | Medium | Test INT8 quantization, reduce dimensions |
| LanceDB/Qdrant integration complex | Medium | High | Start with simpler DB (SQLite-vec), migrate later |
| Merkle sync has edge case bugs | Medium | High | Extensive testing with file moves/renames |
| Reordering doesn't help in practice | Low | Low | A/B test early, be ready to disable |

---

## Success Metrics

**After Implementation:**
- ✅ Retrieval Recall@10: >90%
- ✅ Query Latency (p50): <500ms
- ✅ Query Latency (p95): <1000ms
- ✅ Indexing Speed: >1000 files/min
- ✅ Incremental Sync: <1s for single file change
- ✅ MRR on benchmarks: >0.85

**Business Impact:**
- ✅ Token costs reduced 5-10x (better retrieval = less context waste)
- ✅ User experience: sub-second code retrieval
- ✅ Competitive with Cursor on retrieval quality

---

## Next Steps

1. **Start Sprint 0** (Foundation) - Week 1-2
2. **Await R-04 research** (Local Indexing) - May refine vector DB choice
3. **Parallel: R-02 research** (Communication) - Agent communication protocol
4. **Parallel: R-05 research** (Local Models) - Model selection for reformulation

---

## Related Documents

- [ADR-012: Context Management & RAG Strategy](../research/DECISIONS.md#adr-012)
- [ADR-006: Embedding Model](../research/DECISIONS.md#adr-006)
- [ADR-007: Vector Database](../research/DECISIONS.md#adr-007)
- [ADR-013: Code Chunking Strategy](../research/DECISIONS.md#adr-013)
- [ADR-014: Context Reordering](../research/DECISIONS.md#adr-014)
- [ADR-015: State Synchronization](../research/DECISIONS.md#adr-015)
- [Architecture: Context & RAG](../docs/architecture-context-rag.md)
- [R-01 Research Findings](./findings/context-management/R-01-result.md)
