# OPENAKTA Technical Architecture: Context Management & RAG

**Created:** 2026-03-16  
**Status:** ✅ Approved  
**Based On:** R-01 Research, ADR-012, ADR-006, ADR-007, ADR-013, ADR-014, ADR-015

---

## System Overview

OPENAKTA's context management system implements a **Modular RAG** architecture optimized for multi-agent code generation and retrieval.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Context Management Pipeline                   │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐
│ User Query   │
│ or Agent     │
│ Request      │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 1: Query Reformulation                                    │
│  - LLM rewrites query for clarity                               │
│  - Expands with domain-specific terminology                     │
│  - May decompose into sub-queries                               │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 2: Hybrid Retrieval (Parallel)                           │
│  ┌────────────────────┐    ┌────────────────────┐               │
│  │  BM25 (Sparse)     │    │  Dense (Jina 1.5B) │               │
│  │  - Exact matches   │    │  - Semantic search │               │
│  │  - Variable names  │    │  - Conceptual      │               │
│  │  - Error codes     │    │  - Intent          │               │
│  └────────────────────┘    └────────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
       │                              │
       └──────────────┬───────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 3: Reciprocal Rank Fusion (RRF)                          │
│  - Merges BM25 + Dense results                                  │
│  - Rank-based fusion (not score-based)                          │
│  - Produces unified candidate set (~1000 chunks)                │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 4: Cross-Encoder Re-ranking                              │
│  - Concatenates query + each chunk                              │
│  - Full self-attention scoring                                  │
│  - Reduces 1000 → 100 candidates                                │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 5: Knapsack Selection                                    │
│  - Maximizes relevance score                                    │
│  - Respects token budget constraint                             │
│  - Selects optimal subset (~20-50 chunks)                       │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  Stage 6: Context Reordering ("Lost in the Middle")             │
│  - Alternates high-score chunks: beginning ←→ end               │
│  - Aligns with LLM attention biases                             │
│  - Produces final prompt context                                │
└─────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────┐
│  LLM Generation                                                  │
│  - Receives optimized context                                   │
│  - Generates code, answer, or action                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Specifications

### 1. Query Reformulation

**Purpose:** Improve retrieval quality by optimizing query before search

**Implementation:**
- Small local LLM (7B parameter) for reformulation
- Prompt template:
  ```
  Reformulate this coding query for better retrieval:
  - Add relevant technical terms
  - Clarify ambiguous references
  - Expand acronyms if needed
  
  Query: "{user_query}"
  Context: {file_language}, {current_file}
  
  Reformulated Query:
  ```

**Latency Budget:** <100ms (local model)

---

### 2. Hybrid Retrieval

#### 2.1 BM25 (Sparse Retrieval)

**Purpose:** Exact lexical matching for identifiers, error codes, variable names

**Implementation:**
- Use `tantivy` crate (Rust port of Lucene)
- Index configuration:
  ```rust
  IndexSettings {
      tokenize: "Code",  // Custom tokenizer for code
      stemmer: None,     // No stemming for code
      field_norms: true, // For BM25 scoring
  }
  ```

**Latency Budget:** <50ms for 100K chunks

#### 2.2 Dense Retrieval (Jina-code-embeddings-1.5b)

**Purpose:** Semantic/conceptual matching

**Model Specs:**
- Parameters: 1.54B
- Dimensions: 1536 (truncatable to 256-1536)
- Context: 32K tokens
- Backbone: Qwen2.5

**Implementation:**
- Inference: ONNX Runtime or Candle (Rust)
- Storage dimensions: 512-768 (Matryoshka truncation)
- Query dimensions: 1536 (full for max accuracy)

**Latency Budget:** <100ms for query embedding, <50ms for vector search

---

### 3. Reciprocal Rank Fusion (RRF)

**Purpose:** Merge BM25 and Dense results fairly

**Algorithm:**
```rust
fn rrf_fusion(bm25_results: Vec<ChunkId>, dense_results: Vec<ChunkId>, k: usize) -> Vec<ChunkId> {
    let mut scores: HashMap<ChunkId, f64> = HashMap::new();
    
    // Score by reciprocal rank
    for (rank, chunk_id) in bm25_results.iter().enumerate() {
        *scores.entry(chunk_id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
    }
    for (rank, chunk_id) in dense_results.iter().enumerate() {
        *scores.entry(chunk_id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
    }
    
    // Sort by combined score
    let mut sorted: Vec<_> = scores.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    sorted.into_iter().map(|(id, _)| id).collect()
}
```

**Parameters:**
- k = 60 (standard RRF constant)

---

### 4. Cross-Encoder Re-ranking

**Purpose:** High-precision scoring of top candidates

**Model Options:**
- BGE-reranker-large (recommended)
- ms-marco-MiniLM-L-12-v2 (faster, slightly less accurate)

**Implementation:**
- Batch inference for latency optimization
- Process top 100 RRF results
- Output: Relevance score [0.0 - 1.0]

**Latency Budget:** <500ms for 100 chunks (batched)

---

### 5. Knapsack Selection

**Purpose:** Maximize relevance within token budget

**Algorithm:**
```rust
fn knapsack_select(chunks: Vec<RankedChunk>, token_budget: usize) -> Vec<Chunk> {
    let mut dp = vec![vec![0.0; token_budget + 1]; chunks.len() + 1];
    let mut selected = vec![vec![false; token_budget + 1]; chunks.len() + 1];
    
    // Dynamic programming
    for i in 1..=chunks.len() {
        let chunk_tokens = chunks[i-1].token_count;
        let chunk_score = chunks[i-1].relevance_score;
        
        for j in 0..=token_budget {
            dp[i][j] = dp[i-1][j];  // Don't take
            
            if j >= chunk_tokens {
                let take_score = dp[i-1][j - chunk_tokens] + chunk_score;
                if take_score > dp[i][j] {
                    dp[i][j] = take_score;
                    selected[i][j] = true;
                }
            }
        }
    }
    
    // Backtrack to find selected chunks
    let mut result = Vec::new();
    let mut j = token_budget;
    for i in (1..=chunks.len()).rev() {
        if selected[i][j] {
            result.push(chunks[i-1].clone());
            j -= chunks[i-1].token_count;
        }
    }
    
    result.reverse();
    result
}
```

**Token Budget:** Configurable (default: 8000-16000 tokens)

---

### 6. Context Reordering

**Purpose:** Counteract "Lost in the Middle" attention decay

**Algorithm:** (See ADR-014 for full spec)

```rust
fn reorder_context(chunks: Vec<RetrievedChunk>) -> Vec<RetrievedChunk> {
    let mut reordered = Vec::with_capacity(chunks.len());
    let mut left = 0;
    let mut right = chunks.len() - 1;
    let mut pick_left = true;
    
    while left <= right {
        if pick_left {
            reordered.push(chunks[left].clone());
            left += 1;
        } else {
            reordered.push(chunks[right].clone());
            right -= 1;
        }
        pick_left = !pick_left;
    }
    
    reordered
}
```

**Expected Improvement:** 15-25% better information extraction

---

## Data Structures

### Chunk Representation

```rust
/// A chunk of code with metadata
pub struct CodeChunk {
    /// Unique identifier
    pub id: ChunkId,
    
    /// Absolute file path
    pub file_path: PathBuf,
    
    /// Line range in original file
    pub line_range: (usize, usize),
    
    /// Chunk content (AST-aligned)
    pub content: String,
    
    /// Token count (for budgeting)
    pub token_count: usize,
    
    /// Non-whitespace character count
    pub nw_char_count: usize,
    
    /// Symbol definitions (function names, class names, etc.)
    pub symbols: Vec<Symbol>,
    
    /// Incoming call graph edges
    pub called_by: Vec<SymbolRef>,
    
    /// Outgoing call graph edges
    pub calls: Vec<SymbolRef>,
    
    /// Language identifier
    pub language: Language,
    
    /// Git hash of file at index time
    pub git_hash: Option<String>,
    
    /// Embedding vector (truncated)
    pub embedding: Vec<f32>,
}

/// Relevance-ranked chunk (after re-ranking)
pub struct RankedChunk {
    pub chunk: CodeChunk,
    pub relevance_score: f64,  // 0.0 - 1.0
    pub bm25_rank: usize,
    pub dense_rank: usize,
}
```

---

## Indexing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    Codebase Indexing Flow                        │
└─────────────────────────────────────────────────────────────────┘

1. File System Watch
   │
   ▼
2. Merkle Tree Update (incremental)
   │
   ▼
3. Identify Changed Files
   │
   ▼
4. Parse with Tree-sitter → AST
   │
   ▼
5. cAST Chunking (AST-aligned)
   │
   ▼
6. Compute Embeddings (Candle local: JinaCode 768-dim, BGE-Skill 384-dim)
   │
   ▼
7. Store in sqlite-vec (local HNSW ANN) + SqliteJson fallback
   │
   ▼
8. Update BM25 Index (tantivy)
   │
   ▼
9. Index Complete
```

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Indexing Speed | 1000 files/min | Full repo scan |
| Incremental Sync | <1s for 1 file change | Merkle tree diff |
| Query Latency (p50) | <500ms | End-to-end retrieval |
| Query Latency (p95) | <1000ms | End-to-end retrieval |
| Retrieval Recall@10 | >90% | Benchmark dataset |
| MRR (Mean Reciprocal Rank) | >0.85 | SWE-Bench style eval |

---

## Dependencies (Rust Crates)

```toml
[dependencies]
# Embedding inference (Candle)
candle-core = "0.4"
candle-transformers = "0.4"

# Vector databases
sqlite-vec = "0.1"
rusqlite = "0.31"

# BM25 / full-text search
tantivy = "0.22"

# AST parsing
tree-sitter = "0.22"
tree-sitter-rust = "0.22"
tree-sitter-typescript = "0.21"
tree-sitter-python = "0.22"
# ... add grammars for all supported languages

# Hashing (Merkle tree)
blake3 = "1.5"

# Utilities
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## Open Questions (Requires Experimentation)

### 1. MUVERA Integration
**Question:** Should we implement MUVERA for multi-vector retrieval proxy?

**Pros:**
- ColBERT-level accuracy with single-vector speed
- Theoretical guarantees on similarity approximation

**Cons:**
- Engineering complexity (Product Quantization)
- May not be worth it vs cross-encoder reranking

**Decision:** Defer until after R-04 research

---

### 2. GraphRAG vs SCIP
**Question:** Should we implement full GraphRAG traversal or use SCIP-style metadata filtering?

**Pros (GraphRAG):**
- Deep dependency-aware retrieval
- Multi-hop reasoning

**Cons:**
- Complex graph construction
- Slower indexing

**Decision:** Start with SCIP-style metadata, add GraphRAG after benchmarks

---

### 3. Shadow Workspace Scaling
**Question:** How many parallel agents can run without I/O bottlenecks?

**Unknown:**
- Disk I/O limits on consumer hardware
- CPU thermal throttling impact

**Decision:** Profile with 1, 2, 4, 8 parallel agents on M2 Pro and RTX 4070

---

## Testing Strategy

### Unit Tests
- RRF fusion correctness
- Knapsack selection optimality
- Context reordering algorithm
- Merkle tree hash computation

### Integration Tests
- End-to-end retrieval latency
- Indexing speed benchmarks
- Incremental sync correctness

### Evaluation Benchmarks
- SWE-Bench Verified (adapted for retrieval)
- CodeSearchNet retrieval metrics
- Custom benchmark: "Find the bug" tasks

---

## Related documents

- [Architecture ledger](./ARCHITECTURE-LEDGER.md) — recorded ADRs and baseline  
- [Active architecture](./active_architecture/) — current narrative  
- [Architecture communication](./architecture-communication.md) — protocol-oriented view  

---

## Next Steps

1. **Implement cAST chunking** - Core foundation
2. **Benchmark Jina embeddings** - Validate accuracy claims
3. **Prototype Merkle sync** - Test incremental update speed
4. **A/B test context reordering** - Measure real-world impact
5. **Profile end-to-end latency** - Identify bottlenecks
