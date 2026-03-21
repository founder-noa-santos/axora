# Phase 1: Foundation Implementation

**Status:** 🔄 IN PROGRESS  
**Priority:** CRITICAL  
**Start Date:** 2026-03-16  
**Duration:** 4 weeks  

---

## Overview

Now that all 8 research areas are complete with 42 ADRs approved, we begin **implementation Phase 1: Foundation**.

This phase implements the core infrastructure that all other features build upon:
- RAG pipeline (context management)
- Vector indexing (local codebase understanding)
- Basic agent framework
- Token optimization foundation

---

## Goals

By the end of this phase, OPENAKTA will have:
1. ✅ Working RAG pipeline with hybrid retrieval
2. ✅ Local codebase indexing with Tree-sitter chunking
3. ✅ Vector search with Qdrant embedded
4. ✅ Basic agent framework with state machine
5. ✅ Token optimization (prefix caching, diff-based comms)
6. ✅ Merkle tree sync for incremental updates

**Performance Targets:**
- Query latency P95: <100ms
- Retrieval recall @10: >95%
- Indexing speed: >100 files/sec
- Incremental sync: <5s for typical changes

---

## Sprint Breakdown

### Sprint 1: Project Setup & Dependencies (Week 1)

**Goals:**
- [ ] Create new crate structure for all components
- [ ] Add all dependencies to workspace Cargo.toml
- [ ] Set up basic module structure
- [ ] Create test fixtures

**Tasks:**

#### 1.1 Create New Crates
```bash
# Create crate structure
cargo new crates/openakta-rag --lib
cargo new crates/openakta-indexing --lib
cargo new crates/openakta-embeddings --lib
cargo new crates/openakta-agents --lib
cargo new crates/openakta-cache --lib
```

#### 1.2 Update Workspace Cargo.toml
Add to `/Users/noasantos/Downloads/openakta/Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "crates/openakta-proto",
    "crates/openakta-storage",
    "crates/openakta-core",
    "crates/openakta-daemon",
    # New crates for Phase 1
    "crates/openakta-rag",
    "crates/openakta-indexing",
    "crates/openakta-embeddings",
    "crates/openakta-agents",
    "crates/openakta-cache",
]

[workspace.dependencies]
# ... existing deps ...

# New dependencies for Phase 1
# Embeddings
candle-core = "0.8"
candle-transformers = "0.8"
candle-nn = "0.8"

# Vector search
qdrant-client = "1.12"
lancedb = "0.10"

# Code parsing
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.22"
tree-sitter-python = "0.23"

# Full-text search
tantivy = "0.22"

# Caching
dashmap = "5.5"
rocksdb = "0.22"

# File watching
notify = "6.1"

# Hashing
blake3 = "1.5"
sha2 = "0.10"

# Inference
ollama-rs = "0.2"
llama-cpp-rs = "0.3"

# Diff generation
unified-diff = "0.3"
patch = "0.5"
```

#### 1.3 Create Basic Module Structure
For each new crate, create:
- `src/lib.rs` - Module root with public API
- `src/error.rs` - Error types
- `src/config.rs` - Configuration structs
- `tests/` - Integration tests

#### 1.4 Verify Build
```bash
cargo build --workspace
cargo test --workspace
```

**Deliverable:** Workspace compiles with all new crates

---

### Sprint 2: Embeddings & Vector Search (Week 2)

**Goals:**
- [ ] Implement Jina Code Embeddings v2 inference
- [ ] Set up Qdrant embedded mode
- [ ] Create vector index with HNSW
- [ ] Benchmark embedding latency

**Tasks:**

#### 2.1 Implement Embedding Engine (`openakta-embeddings`)
```rust
// crates/openakta-embeddings/src/lib.rs
use candle_core::{Device, Tensor};
use candle_transformers::models::jina_code::JinaCodeModel;

pub struct EmbeddingEngine {
    model: JinaCodeModel,
    device: Device,
    dimensions: usize,
}

impl EmbeddingEngine {
    pub fn new(model_path: &str, dimensions: usize) -> Result<Self> {
        // Load Jina Code Embeddings v2
        // Default: 768 dimensions, can truncate via Matryoshka
    }
    
    pub async fn embed(&self, code: &str) -> Result<Vec<f32>> {
        // Generate embedding for code snippet
        // Target: <25ms latency for 512 tokens
    }
    
    pub async fn embed_batch(&self, codes: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Batch embedding for indexing throughput
        // Target: >100 chunks/sec
    }
}
```

#### 2.2 Set Up Qdrant Embedded (`openakta-indexing`)
```rust
// crates/openakta-indexing/src/vector_store.rs
use qdrant_client::{Qdrant, QdrantConfig};
use qdrant_client::qdrant::{
    CreateCollection, Distance, HnswConfigDiff, VectorParams,
};

pub struct VectorStore {
    client: Qdrant,
    collection_name: String,
}

impl VectorStore {
    pub async fn new(collection: &str) -> Result<Self> {
        // Qdrant embedded mode (in-memory or disk-backed)
        let config = QdrantConfig::from_url("http://localhost:6334");
        let client = Qdrant::new(config)?;
        
        // Create collection with HNSW index
        client.create_collection(&CreateCollection {
            collection_name: collection.to_string(),
            vectors_config: Some(VectorParams {
                size: 768,
                distance: Distance::Cosine.into(),
                hnsw_config: Some(HnswConfigDiff {
                    m: Some(16),  // connectivity
                    ef_construct: Some(128),  // build depth
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }).await?;
        
        Ok(Self { client, collection_name: collection.to_string() })
    }
    
    pub async fn insert(&self, id: &str, vector: &[f32], payload: &Payload) -> Result<()> {
        // Insert vector with metadata
    }
    
    pub async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        // HNSW search, target: <5ms for 100K vectors
    }
}
```

#### 2.3 Benchmark Performance
```rust
// crates/openakta-embeddings/benches/embed_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_embedding(c: &mut Criterion) {
    let engine = EmbeddingEngine::new("jina-code-v2", 768).unwrap();
    
    c.bench_function("embed_512_tokens", |b| {
        b.iter(|| {
            let code = generate_test_code(512);
            engine.embed(&code).unwrap()
        })
    });
}

fn bench_batch(c: &mut Criterion) {
    let engine = EmbeddingEngine::new("jina-code-v2", 768).unwrap();
    
    c.bench_function("embed_batch_100", |b| {
        b.iter(|| {
            let codes: Vec<_> = (0..100).map(|i| generate_test_code(512)).collect();
            engine.embed_batch(&codes).unwrap()
        })
    });
}

criterion_group!(benches, bench_embedding, bench_batch);
criterion_main!(benches);
```

**Success Criteria:**
- Single embedding: <25ms (512 tokens)
- Batch embedding: >100 chunks/sec
- Vector search: <5ms for 100K vectors

**Deliverable:** Working embedding + vector search pipeline

---

### Sprint 3: Code Chunking & Indexing (Week 3)

**Goals:**
- [ ] Implement Tree-sitter based chunking
- [ ] Create Merkle tree for change detection
- [ ] Implement incremental indexing
- [ ] Benchmark indexing speed

**Tasks:**

#### 3.1 Implement AST Chunking (`openakta-indexing`)
```rust
// crates/openakta-indexing/src/chunker.rs
use tree_sitter::{Parser, Query, QueryCursor};

pub struct Chunker {
    parser: Parser,
    language_queries: HashMap<Language, Query>,
}

pub struct CodeChunk {
    pub id: String,
    pub file_path: PathBuf,
    pub line_range: (usize, usize),
    pub content: String,
    pub language: Language,
    pub chunk_type: ChunkType,  // Function, Class, Module, etc.
    pub metadata: ChunkMetadata,
}

impl Chunker {
    pub fn extract_chunks(&self, code: &str, file_path: &Path, language: Language) -> Result<Vec<CodeChunk>> {
        let tree = self.parser.parse(code, None).ok_or(Error::ParseFailed)?;
        let query = self.language_queries.get(&language).ok_or(Error::NoQuery)?;
        
        let mut chunks = Vec::new();
        let mut cursor = QueryCursor::new();
        
        for m in cursor.matches(query, tree.root_node(), code.as_bytes()) {
            let chunk = CodeChunk::from_match(m, file_path, language);
            
            // Apply size constraints
            if chunk.token_count >= 256 && chunk.token_count <= 512 {
                chunks.push(chunk);
            } else if chunk.token_count > 512 {
                // Split large chunks
                chunks.extend(self.split_chunk(chunk));
            }
        }
        
        Ok(chunks)
    }
}
```

#### 3.2 Implement Merkle Tree Sync (`openakta-indexing`)
```rust
// crates/openakta-indexing/src/merkle.rs
use blake3::Hash;

pub struct MerkleTree {
    root_hash: Hash,
    nodes: HashMap<PathBuf, HashNode>,
}

pub struct HashNode {
    hash: Hash,
    children: Vec<PathBuf>,
    content_hash: Option<Hash>,
}

impl MerkleTree {
    pub fn build(root_path: &Path) -> Result<Self> {
        // Recursively build tree from filesystem
        // Leaf nodes: file content hash (BLAKE3)
        // Internal nodes: hash of children hashes
    }
    
    pub fn find_changed(&self, old_tree: &MerkleTree) -> Vec<PathBuf> {
        // O(log n) comparison
        // Only traverse divergent branches
        // Return list of changed files
    }
    
    pub fn update(&mut self, file_path: &Path, new_content: &[u8]) -> Result<()> {
        // Update leaf node
        // Propagate hash changes up the tree
        // Update root hash
    }
}
```

#### 3.3 Implement Incremental Indexer
```rust
// crates/openakta-indexing/src/indexer.rs
pub struct IncrementalIndexer {
    chunker: Chunker,
    embedder: EmbeddingEngine,
    vector_store: VectorStore,
    merkle_tree: MerkleTree,
}

impl IncrementalIndexer {
    pub async fn index(&mut self, root_path: &Path) -> Result<IndexStats> {
        // Build or load Merkle tree
        let changed_files = self.merkle_tree.find_changed(&self.previous_tree)?;
        
        let mut stats = IndexStats::default();
        
        for file_path in changed_files {
            // Read file
            let content = std::fs::read_to_string(&file_path)?;
            
            // Chunk with Tree-sitter
            let chunks = self.chunker.extract_chunks(&content, &file_path, detect_language(&file_path))?;
            
            // Generate embeddings
            let embeddings = self.embedder.embed_batch(&chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>()).await?;
            
            // Insert into vector store
            for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
                self.vector_store.insert(&chunk.id, embedding, chunk.to_payload()).await?;
                stats.chunks_indexed += 1;
            }
        }
        
        // Update Merkle tree
        self.previous_tree = self.merkle_tree.clone();
        
        Ok(stats)
    }
}
```

**Success Criteria:**
- Indexing speed: >100 files/sec
- Incremental sync: <5s for typical changes
- Parse rate: >85% for supported languages

**Deliverable:** Working incremental indexing pipeline

---

### Sprint 4: RAG Pipeline & Integration (Week 4)

**Goals:**
- [ ] Implement hybrid retrieval (vector + BM25 + symbol)
- [ ] Implement cross-encoder re-ranking
- [ ] Implement context reordering
- [ ] End-to-end integration testing

**Tasks:**

#### 4.1 Implement Hybrid Retriever (`openakta-rag`)
```rust
// crates/openakta-rag/src/retriever.rs
use tantivy::Index as TantivyIndex;

pub struct HybridRetriever {
    vector_index: VectorStore,
    bm25_index: TantivyIndex,
    symbol_index: HashMap<String, Vec<ChunkId>>,
    cross_encoder: CrossEncoder,
}

pub struct RetrievalResult {
    pub chunk: CodeChunk,
    pub score: f32,
    pub source: RetrievalSource,  // Vector, BM25, or Symbol
}

impl HybridRetriever {
    pub async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<RetrievalResult>> {
        // Parallel retrieval
        let (vector_results, bm25_results, symbol_results) = tokio::join!(
            self.vector_search(query, 100),
            self.bm25_search(query, 100),
            Ok(self.symbol_search(query)),
        );
        
        // Reciprocal Rank Fusion
        let fused = self.rrf_fusion(vec![
            vector_results?,
            bm25_results?,
            symbol_results?,
        ]);
        
        // Cross-encoder re-ranking
        let reranked = self.cross_encoder.rerank(&fused, query).await?;
        
        Ok(reranked.into_iter().take(limit).collect())
    }
    
    fn rrf_fusion(&self, result_lists: Vec<Vec<RetrievalResult>>) -> Vec<RetrievalResult> {
        // Reciprocal Rank Fusion: score = Σ 1/(60 + rank)
        let mut scores: HashMap<ChunkId, f32> = HashMap::new();
        
        for results in result_lists {
            for (rank, result) in results.iter().enumerate() {
                *scores.entry(result.chunk.id.clone()).or_insert(0.0) += 1.0 / (60.0 + rank as f32);
            }
        }
        
        // Sort by fused score
        let mut fused: Vec<_> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Return sorted results
        fused.into_iter()
            .filter_map(|(id, _)| {
                result_lists.iter()
                    .flatten()
                    .find(|r| r.chunk.id == id)
                    .cloned()
            })
            .collect()
    }
}
```

#### 4.2 Implement Cross-Encoder Re-ranker
```rust
// crates/openakta-rag/src/reranker.rs
use candle_core::{Device, Tensor};
use candle_transformers::models::minilm::MiniLM;

pub struct CrossEncoder {
    model: MiniLM,
    device: Device,
}

impl CrossEncoder {
    pub async fn rerank(&self, results: &[RetrievalResult], query: &str) -> Result<Vec<RetrievalResult>> {
        // Score each result with cross-encoder
        let mut scored_results = Vec::new();
        
        for result in results {
            let score = self.score(query, &result.chunk.content).await?;
            scored_results.push((result.clone(), score));
        }
        
        // Sort by score
        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Return sorted results
        Ok(scored_results.into_iter().map(|(r, _)| r).collect())
    }
    
    async fn score(&self, query: &str, document: &str) -> Result<f32> {
        // Concatenate query + document
        // Pass through cross-encoder
        // Extract relevance score [0.0 - 1.0]
        // Target: <25ms per pair
    }
}
```

#### 4.3 Implement Context Reordering
```rust
// crates/openakta-rag/src/context.rs
pub struct ContextBuilder {
    max_tokens: usize,
}

impl ContextBuilder {
    pub fn build(&self, results: Vec<RetrievalResult>, query: &str) -> Result<String> {
        // Token budget management
        let mut context = String::new();
        let mut token_count = 0;
        
        // "Lost in the Middle" reordering
        // Place highest-scoring at beginning and end
        let reordered = self.reorder_results(results);
        
        for result in reordered {
            let chunk_tokens = count_tokens(&result.chunk.content);
            
            if token_count + chunk_tokens > self.max_tokens {
                break;
            }
            
            context.push_str(&format!(
                "\n\n=== {} (Line {}-{}) ===\n{}\n",
                result.chunk.file_path.display(),
                result.chunk.line_range.0,
                result.chunk.line_range.1,
                result.chunk.content
            ));
            
            token_count += chunk_tokens;
        }
        
        Ok(context)
    }
    
    fn reorder_results(&self, results: Vec<RetrievalResult>) -> Vec<RetrievalResult> {
        // Alternate: best → worst → 2nd best → 2nd worst → ...
        // This places most relevant at beginning and end
        let mut reordered = Vec::with_capacity(results.len());
        let mut left = 0;
        let mut right = results.len() - 1;
        let mut pick_left = true;
        
        while left <= right {
            if pick_left {
                reordered.push(results[left].clone());
                left += 1;
            } else {
                reordered.push(results[right].clone());
                right -= 1;
            }
            pick_left = !pick_left;
        }
        
        reordered
    }
}
```

#### 4.4 End-to-End Integration Test
```rust
// crates/openakta-rag/tests/integration.rs
use openakta_rag::{RAGPipeline, RAGConfig};

#[tokio::test]
async fn test_full_rag_pipeline() {
    // Setup
    let config = RAGConfig {
        embedding_model: "jina-code-v2".to_string(),
        vector_store: "qdrant".to_string(),
        max_context_tokens: 8192,
    };
    
    let pipeline = RAGPipeline::new(config).await.unwrap();
    
    // Index a test codebase
    let stats = pipeline.index("/path/to/test/repo").await.unwrap();
    assert!(stats.chunks_indexed > 100);
    
    // Query
    let results = pipeline.retrieve("how is authentication handled?", 10).await.unwrap();
    
    // Validate
    assert!(results.len() >= 5);
    assert!(results[0].score > 0.7);
    
    // Build context
    let context = pipeline.build_context(&results, "how is authentication handled?").unwrap();
    assert!(context.contains("auth"));
    assert!(context.contains("authentication"));
}

#[tokio::test]
async fn test_query_latency() {
    let pipeline = setup_test_pipeline().await;
    
    let start = std::time::Instant::now();
    pipeline.retrieve("test query", 10).await.unwrap();
    let elapsed = start.elapsed();
    
    // P95 target: <100ms
    assert!(elapsed.as_millis() < 100);
}
```

**Success Criteria:**
- Query latency P95: <100ms
- Retrieval recall @10: >95%
- Cross-encoder adds <25ms per result

**Deliverable:** Complete RAG pipeline ready for agent integration

---

## Dependencies

### Rust Crates (Add to `Cargo.toml`)

```toml
[dependencies]
# Embeddings
candle-core = "0.8"
candle-transformers = "0.8"
candle-nn = "0.8"

# Vector search
qdrant-client = "1.12"
lancedb = "0.10"

# Code parsing
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.22"
tree-sitter-python = "0.23"

# Full-text search
tantivy = "0.22"

# Caching
dashmap = "5.5"
rocksdb = "0.22"

# File watching
notify = "6.1"

# Hashing
blake3 = "1.5"
sha2 = "0.10"

# Inference
ollama-rs = "0.2"

# Diff generation
unified-diff = "0.3"
patch = "0.5"

# Utilities
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
thiserror = "1.0"
anyhow = "1.0"
```

---

## Testing Strategy

### Unit Tests
- Embedding generation
- Chunk extraction
- Merkle tree operations
- RRF fusion
- Context reordering

### Integration Tests
- End-to-end RAG pipeline
- Incremental indexing
- Query latency benchmarks

### Benchmark Tests
- Embedding throughput
- Vector search latency
- Indexing speed
- Query P95 latency

---

## Success Criteria

**Phase 1 is complete when:**
- ✅ All 4 sprints completed
- ✅ Query latency P95 <100ms
- ✅ Retrieval recall @10 >95%
- ✅ Indexing speed >100 files/sec
- ✅ Incremental sync <5s
- ✅ All tests passing
- ✅ Documentation complete

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Qdrant embedded mode unstable | High | Low | Fallback to sqlite-vec |
| Tree-sitter parse rate <85% | Medium | Medium | Fallback to line-based chunking |
| Embedding latency >50ms | High | Low | Use smaller model, batch more |
| Merkle tree sync bugs | Medium | Medium | Extensive testing, rollback support |

---

## Next Steps After Phase 1

**Phase 2: Agent Framework** (4 weeks)
- Implement state machine orchestration
- Create 10 native agents
- Implement capability-based task assignment
- Add conflict resolution

**Phase 3: Token Optimization** (4 weeks)
- Implement prefix caching
- Add diff-based communication
- Code minification pipeline
- TOON serialization

**Phase 4: Desktop App** (4 weeks)
- Tauri v2 setup
- gRPC client
- React UI
- Integration with daemon

---

## Notes

- This phase is **foundational** - all other phases depend on it
- Performance targets are **aggressive but achievable** based on research
- If targets are missed, **document why** and adjust future phases
- **Test early, test often** - don't wait until end of phase
