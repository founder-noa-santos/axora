# R-04: Local Indexing & Embedding

## Research Prompt

Copy and paste the following into Claude/GPT-4/Perplexity with web search enabled:

---

```
# Deep Research: Local Indexing & Embedding for Code AI Systems

## Context
I'm building OPENAKTA, a multi-agent AI coding system that must run locally on developer machines. A key capability is fast codebase indexing and retrieval (similar to Cursor's "codebase understanding"). We need SCIENTIFIC-LEVEL understanding of how to build this. This research must be production-grade with specific implementation details.

## Core Research Questions

### 1. Cursor Indexing Analysis (CRITICAL)

This is our main competitor - understand their approach:

a) **Public Information Gathering**
   Search extensively for:
   - "Cursor IDE indexing architecture"
   - "Cursor codebase embedding how it works"
   - "Cursor local vector database"
   - Cursor blog posts about technical implementation
   - Cursor patents (if any)
   - Interviews with Cursor founders about architecture
   - Any reverse engineering attempts

b) **Behavioral Analysis**
   From using Cursor or watching demos:
   - How fast is indexing?
   - How fast is retrieval?
   - Does it work offline?
   - How does it handle large codebases?
   - Incremental updates?

c) **Hypothesis: Likely Architecture**
   Based on available info, propose:
   - What embedding model they likely use
   - What vector database they likely use
   - Their chunking strategy
   - Their retrieval pipeline
   - Confidence level for each claim

### 2. Embedding Models for Code

a) **Model Comparison**
   Research and compare these models (and any newer ones):

   | Model | Dimensions | Params | MRR@10 | License | Local? |
   |-------|------------|--------|--------|---------|--------|
   | CodeBERT | 768 | 125M | ? | MIT | Yes |
   | GraphCodeBERT | 768 | 125M | ? | MIT | Yes |
   | UniXcoder | 768 | 125M | ? | MIT | Yes |
   | BGE-code | ? | ? | ? | ? | ? |
   | StarCoder embeddings | ? | ? | ? | ? | ? |
   | nomic-embed-text | 768 | ? | ? | Apache 2 | Yes |
   | mxbai-embed-large | 1024 | ? | ? | Apache 2 | Yes |
   | Any 2025-2026 models | ... | ... | ... | ... | ... |

   For each:
   - CodeSearchNet benchmark scores
   - Inference latency (CPU, GPU)
   - Memory footprint
   - Commercial use allowed?
   - HuggingFace availability

b) **Embedding Model Selection Criteria**
   For our use case (local-first, Rust, code):
   - What dimensions are optimal? (tradeoff: quality vs size vs speed)
   - Single model or ensemble?
   - Fine-tuning opportunities?

c) **Running Embeddings Locally**
   - ONNX Runtime for embeddings?
   - llama.cpp support for embeddings?
   - Rust libraries: candle, burn, ort?
   - Performance benchmarks

### 3. Vector Databases for Local Use

a) **Candidate Evaluation**
   Deep-dive into each:

   1. **ChromaDB**
      - Rust client availability
      - Memory footprint
      - Performance at 1M vectors
      - Persistence format
      - License (Apache 2?)
      
   2. **Qdrant**
      - Local/embedded mode
      - Rust native (big plus)
      - Performance benchmarks
      - Memory-mapped indexes
      
   3. **LanceDB**
      - Embedded mode
      - Columnar storage
      - Rust support
      - Performance
      
   4. **SQLite + Vector Extension**
      - sqlite-vec extension
      - Performance vs dedicated vector DBs
      - Simplicity advantage
      - Already using SQLite for other data
      
   5. **FAISS (Facebook)**
      - Raw library (no server)
      - Rust bindings (faiss-rs)
      - Index types (IVF, HNSW, etc.)
      - Performance/accuracy tradeoffs
      
   6. **HNSWlib**
      - HNSW algorithm implementation
      - Rust bindings
      - Memory usage
      - Query speed

b) **Benchmark Requirements**
   For our use case:
   - Codebase size: 10K-100K code chunks
   - Embedding dimension: 768-1024
   - Query latency target: <100ms p95
   - Memory budget: <500MB
   - Build time: <1 minute for full reindex

c) **Index Types**
   Compare index algorithms:
   - Flat (brute force) - baseline
   - HNSW (graph-based)
   - IVF (inverted file)
   - LSH (locality sensitive hashing)
   - Product quantization
   
   For each: build time, query time, recall, memory

### 4. Code Chunking Strategies

a) **Chunking Approaches**
   Research and compare:

   1. **Line-based**
      - Fixed N lines per chunk
      - Overlap of M lines
      - Simple but loses structure

   2. **Function-based**
      - One chunk per function
      - Preserves semantic unit
      - Variable chunk sizes

   3. **Class-based**
      - One chunk per class
      - Good for OOP codebases
      - May be too large

   4. **AST-based**
      - Parse code into AST
      - Chunk by AST nodes
      - Preserves structure
      - More complex

   5. **File-based**
      - One chunk per file
      - Simple
      - May exceed context limits

   6. **Hybrid**
      - Combine approaches
      - Hierarchical chunks
      - Parent-child relationships

b) **Chunk Metadata**
   What metadata to store with each chunk:
   - File path
   - Line numbers
   - Function/class names
   - Imports/dependencies
   - Language
   - Last modified time
   - Git hash (for invalidation)

c) **Chunk Size Optimization**
   - What's the optimal chunk size for code?
   - Any empirical studies?
   - Tradeoffs: too small (lost context) vs too large (diluted embeddings)

### 5. Incremental Indexing

a) **Change Detection**
   - Watch file system for changes
   - Git-based change detection
   - Which files changed?
   - Which chunks are affected?

b) **Selective Re-indexing**
   - Only re-index changed chunks
   - Update vector index incrementally
   - Delete removed chunks
   - Handle renamed files

c) **Index Consistency**
   - Handle concurrent modifications
   - Atomic index updates
   - Rollback on failure

### 6. Retrieval Strategies

a) **Query Processing**
   - How to embed the query?
   - Same model as chunks?
   - Query expansion techniques?

b) **Hybrid Search**
   Combine:
   - Vector similarity (semantic)
   - BM25 (lexical)
   - File path matching
   - Symbol matching
   
   How to combine scores?

c) **Re-ranking**
   - Initial retrieval: fast, approximate
   - Re-ranking: slower, more accurate
   - Cross-encoder re-ranker?
   - LLM-as-reranker?

d) **Multi-Stage Retrieval**
   1. Candidate generation (fast, high recall)
   2. Scoring/ranking (slower, better precision)
   3. Re-ranking (slowest, best quality)
   
   Is this overkill for our use case?

### 7. Code-Specific Challenges

a) **Cross-File Dependencies**
   - Function in file A calls function in file B
   - How to retrieve both together?
   - Dependency graph traversal
   - Import-aware retrieval

b) **Symbol Resolution**
   - User asks about "the User class"
   - Which User class? (may be multiple)
   - Disambiguation strategies

c) **Language Diversity**
   - Codebases have multiple languages
   - Single embedding model for all?
   - Language-specific models?
   - Language as filter?

d) **Generated Code**
   - Handle generated files?
   - Exclude from indexing?
   - Mark as generated?

### 8. Rust Implementation

a) **Rust Ecosystem**
   Identify Rust crates for:
   - Embedding inference: candle, ort, burn?
   - Vector search: qdrant-client, chromadb, hnsw?
   - Code parsing: tree-sitter?
   - File watching: notify?

b) **Performance Optimization**
   - Multi-threaded indexing
   - Memory-mapped files
   - Zero-copy where possible
   - SIMD for embeddings

c) **Integration with Existing Stack**
   - We use SQLite for storage
   - Can we add vector search to same DB?
   - sqlite-vec extension?

## Required Output Format

### Section 1: Cursor Analysis
- What we know about Cursor's approach
- Likely architecture (with confidence levels)
- Where we can differentiate

### Section 2: Embedding Model Recommendation
- Specific model to use
- Benchmarks supporting choice
- Implementation approach

### Section 3: Vector Database Recommendation
- Specific database to embed
- Comparison table
- Rust integration details

### Section 4: Chunking Strategy
- Recommended approach
- Chunk size parameters
- Metadata schema

### Section 5: Retrieval Pipeline
- End-to-end retrieval flow
- Optimization techniques
- Latency budget

### Section 6: Implementation Plan
- Specific Rust crates
- Architecture diagram
- Phased implementation

## Sources Required

Must include:
- At least 5 papers on code embeddings/retrieval
- At least 3 vector database benchmarks
- At least 2 Cursor-related sources
- Rust crate documentation

## Quality Bar

This research determines our core "code understanding" capability. It must be:
- Specific (model names, version numbers, benchmarks)
- Practical (Rust implementation details)
- Quantitative (latency, memory, accuracy numbers)
- Actionable (we should know exactly what to build)
```

---

## Follow-up Prompts

### Follow-up 1: sqlite-vec Evaluation
```
Deep-dive into sqlite-vec for our use case:

1. **Capabilities**
   - What vector operations does it support?
   - HNSW, IVF, or just flat search?
   - Performance vs dedicated vector DBs?

2. **Integration Benefits**
   - Single database for all data
   - Transactional consistency
   - Simplified deployment

3. **Limitations**
   - Scale limits?
   - Missing features?
   - Performance concerns?

4. **Recommendation**
   Should we use sqlite-vec or a dedicated vector DB?
```

### Follow-up 2: Tree-sitter for Code Parsing
```
Research using Tree-sitter for code chunking:

1. **Tree-sitter Capabilities**
   - Languages supported
   - AST quality
   - Incremental parsing
   - Rust bindings

2. **Chunking Implementation**
   - How to extract functions/classes from AST?
   - Handle nested definitions?
   - Preserve comments?

3. **Performance**
   - Parse time for large files?
   - Memory usage?
   - Incremental update support?

4. **Code Example**
   Show Rust code for Tree-sitter based chunking.
```

### Follow-up 3: Embedding Inference in Rust
```
Compare options for running embeddings in Rust:

1. **Candle (Hugging Face)**
   - Model support
   - Performance
   - Ease of use

2. **ONNX Runtime (ort crate)**
   - Model conversion required?
   - Performance
   - CPU/GPU support

3. **llama.cpp (via bindings)**
   - Support for embedding models?
   - Performance
   - Quantization options

4. **Recommendation**
   Which for our use case?
```

---

## Findings Template

Save research findings in `research/findings/local-indexing/`:

```markdown
# R-04 Findings: Local Indexing & Embedding

**Research Date:** YYYY-MM-DD  
**Researcher:** [AI Model Used]  
**Sources:** [List of papers, articles, etc.]

## Cursor Analysis

**What we know:**
- ...

**Likely architecture:**
- Embedding model: [with confidence %]
- Vector DB: [with confidence %]
- Chunking: [with confidence %]

## Embedding Model Recommendation

| Model | Score | Size | Latency | License |
|-------|-------|------|---------|---------|
| ... | ... | ... | ... | ... |

**Recommended:** [Model name]
**Rationale:** ...

## Vector Database Recommendation

| Database | Query Time | Memory | Rust Support | License |
|----------|------------|--------|--------------|---------|
| ... | ... | ... | ... | ... |

**Recommended:** [DB name]
**Rationale:** ...

## Chunking Strategy

**Approach:** [Function-based / AST-based / etc.]
**Chunk size:** ~N tokens
**Overlap:** M tokens
**Metadata:** [list of fields]

## Retrieval Pipeline

```
[Query] → [Embed] → [Vector Search] → [Re-rank] → [Results]
            |           |                |
         5ms        50ms            100ms
```

## Rust Implementation

```rust
// Key crates to use
[dependencies]
qdrant-client = "..."
candle-core = "..."
tree-sitter = "..."
```

## Open Questions

- [ ] Question 1
- [ ] Question 2

## Next Steps

1. [Action item]
2. [Action item]
```

---

## Related Research

- [R-01: Context Management](./01-context-management-rag.md) - RAG retrieval
- [R-05: Model Optimization](./05-model-optimization-local.md) - Local model inference
- [R-03: Token Efficiency](./03-token-efficiency-compression.md) - Context optimization
