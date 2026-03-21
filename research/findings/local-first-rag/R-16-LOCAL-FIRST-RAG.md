# R-16: Local-First RAG Architecture (Zero Cloud Costs)

**Priority:** 🔴 CRITICAL (enables production deployment without cloud costs)  
**Status:** 📋 Research Complete — Ready for Implementation  
**Date:** 2026-03-18  
**Source:** User requirement + R-01/R-04 research findings  

---

## 🎯 Problem Statement

**User Requirement:**
> "Não quero adicionar custo de vetorização e embeddings na nuvem, e não quero passar isso para meu usuário. Qual seria a recomendação para fazer coisas locais sem sobrecarregar o sistema?"

**Constraints:**
- ❌ No cloud embedding costs (OpenAI, Cohere, etc.)
- ❌ No heavy local resource usage (>1GB RAM unacceptable)
- ❌ No Python dependencies (must be pure Rust)
- ✅ Must work on any developer machine (CPU-only support)
- ✅ Must be fast (<100ms retrieval latency)
- ✅ Must be incremental (no full re-indexing on every change)

---

## ✅ Recommended Architecture

### Four-Pillar Approach

```
┌─────────────────────────────────────────────────────────────────┐
│              Local-First RAG Architecture                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 1. Lightweight Embedding Model (Jina Code v2)            │  │
│  │    • 137M parameters (~550MB RAM)                        │  │
│  │    • CPU-only inference (~15ms/query)                    │  │
│  │    • 97% accuracy of giant models                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          │                                       │
│                          ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 2. Embedded Vector DB (Qdrant Embedded or sqlite-vec)    │  │
│  │    • Zero background servers                             │  │
│  │    • ~200MB RAM for 100K vectors                         │  │
│  │    • <5ms retrieval latency                              │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          │                                       │
│                          ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 3. Pure Rust Inference (Candle framework)                │  │
│  │    • No Python dependencies                              │  │
│  │    • AVX2 CPU acceleration                               │  │
│  │    • ~15-25ms per block embedding                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          │                                       │
│                          ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 4. Surgical Indexing (AST + Merkle Trees)                │  │
│  │    • Tree-sitter for semantic chunking                   │  │
│  │    • BLAKE3 hashes for change detection                  │  │
│  │    • 80-95% reduction in re-indexing work                │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Total Resource Usage: <1GB RAM                                 │
│  Total Latency: <100ms P95                                      │
│  Cloud Costs: $0                                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📊 Pillar 1: Lightweight Embedding Model

### Jina Code Embeddings v2

**Model Specs:**
| Property | Value |
|----------|-------|
| Parameters | 137M |
| Model Size | ~550MB (FP32) |
| RAM Usage | ~550MB during inference |
| Dimensions | 768 (configurable via Matryoshka) |
| Context Length | 8K tokens |
| Backbone | CodeBERT-style |

**Performance:**
| Metric | Value |
|--------|-------|
| CPU Inference (single query) | ~15ms |
| Accuracy (Code retrieval) | 97% of 7B models |
| MTEB Code Score | 78.94% |

**Why This Model:**
- ✅ Tiny footprint (137M vs 1.5B+ alternatives)
- ✅ CPU-only friendly (no GPU required)
- ✅ Purpose-built for code (not general text)
- ✅ Matryoshka representation (can truncate to 256/512 dims for storage efficiency)

**Alternatives Rejected:**
| Model | Why Rejected |
|-------|-------------|
| Jina-1.5B | 10x larger, marginal accuracy gain |
| CodeXEmbed | Requires GPU for acceptable speed |
| Qwen3-0.6B | Lower accuracy on code retrieval |

---

## 📊 Pillar 2: Embedded Vector Database

### Option A: Qdrant Embedded (Recommended)

**Characteristics:**
| Property | Value |
|----------|-------|
| Language | Rust-native |
| RAM Usage | ~200MB for 100K vectors |
| Storage | Single file (WAL-based) |
| Retrieval Latency | <5ms P95 |
| Index Type | HNSW (disk-based) |

**Pros:**
- ✅ Pure Rust (matches our stack)
- ✅ Zero background processes
- ✅ Rich payload filtering (filter by file path, language, etc.)
- ✅ Production-proven (used by Cursor via Turbopuffer)

**Cons:**
- ⚠️ Larger binary size (~50MB)
- ⚠️ More complex API than sqlite-vec

### Option B: sqlite-vec (Fallback)

**Characteristics:**
| Property | Value |
|----------|-------|
| Language | C extension for SQLite |
| RAM Usage | <100MB for 100K vectors |
| Storage | Single SQLite file |
| Retrieval Latency | <10ms P95 |
| Index Type | Brute-force (no HNSW) |

**Pros:**
- ✅ Minimal RAM footprint
- ✅ Simple API (just SQL)
- ✅ Single file storage

**Cons:**
- ⚠️ No HNSW index (slower at >1M vectors)
- ⚠️ C dependency (not pure Rust)

### Recommendation

**Use Qdrant Embedded** because:
1. Rust-native (no FFI overhead)
2. HNSW indexing scales better
3. Payload filtering is essential for code retrieval
4. Matches industry standard (Cursor uses similar approach)

---

## 📊 Pillar 3: Pure Rust Inference

### Candle Framework (HuggingFace)

**Characteristics:**
| Property | Value |
|----------|-------|
| Language | Pure Rust |
| Backend | CPU (AVX2), CUDA, Metal |
| Model Format | Safetensors (Jina compatible) |
| Inference Speed | ~15-25ms per block (CPU) |

**Why Candle:**
- ✅ Zero Python dependencies
- ✅ Native AVX2 acceleration (CPU-only friendly)
- ✅ Supports Jina Code model format
- ✅ Actively maintained by HuggingFace

**Implementation:**
```rust
use candle_core::{Device, Tensor};
use candle_transformers::models::jina::JinaModel;

pub struct LocalEmbedder {
    model: JinaModel,
    device: Device,
}

impl LocalEmbedder {
    pub fn new(model_path: &Path) -> Result<Self> {
        let device = Device::Cpu; // CPU-only for universal compatibility
        let model = JinaModel::load(model_path, &device)?;
        Ok(Self { model, device })
    }

    pub async fn embed(&self, code: &str) -> Result<Vec<f32>> {
        // Tokenize
        let tokens = self.tokenize(code)?;
        
        // Forward pass
        let embeddings = self.model.forward(&tokens)?;
        
        // Pooling (last-token)
        let pooled = embeddings.last_token()?;
        
        // Normalize
        Ok(pooled.normalize()?)
    }
}
```

**Performance Optimization:**
- Use batch inference for initial indexing (100+ blocks/sec)
- Use single inference for real-time updates (~15ms/block)
- Cache embeddings on disk (avoid re-computation)

---

## 📊 Pillar 4: Surgical Indexing

### AST-Based Chunking (Tree-sitter)

**Why AST Chunking:**
- ❌ Naive chunking (fixed-size) breaks code structure
- ✅ AST chunking preserves semantic units (functions, classes)

**Implementation:**
```rust
use tree_sitter::{Parser, Tree};

pub struct CodeChunker {
    parser: Parser,
}

impl CodeChunker {
    pub fn chunk(&self, code: &str, language: &str) -> Vec<CodeBlock> {
        let tree = self.parser.parse(code, None).unwrap();
        let root = tree.root_node();
        
        // Extract semantic units
        let mut blocks = Vec::new();
        self.extract_functions(&root, code, &mut blocks);
        self.extract_classes(&root, code, &mut blocks);
        self.extract_modules(&root, code, &mut blocks);
        
        blocks
    }
    
    fn extract_functions(&self, node: &Node, code: &str, blocks: &mut Vec<CodeBlock>) {
        if node.kind() == "function_definition" {
            blocks.push(CodeBlock {
                content: node.utf8_text(code.as_bytes()).unwrap().to_string(),
                kind: CodeKind::Function,
                location: node.range(),
            });
        }
        
        // Recurse
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_functions(&child, code, blocks);
        }
    }
}
```

**Chunk Size Target:**
- Functions: 50-500 tokens (ideal for embedding)
- Classes: 200-2000 tokens (may need sub-chunking)
- Modules: 500-5000 tokens (definitely needs sub-chunking)

### Merkle Tree + BLAKE3 for Change Detection

**Why Merkle Trees:**
- ❌ Full re-indexing on every file change = wasteful
- ✅ Detect exact changed functions = 80-95% less work

**Implementation:**
```rust
use blake3::hash;

pub struct MerkleIndex {
    file_hashes: HashMap<PathBuf, BLAKE3Hash>,
    block_hashes: HashMap<BlockId, BLAKE3Hash>,
}

impl MerkleIndex {
    pub fn detect_changes(&self, new_code: &str, path: &Path) -> Vec<Change> {
        let new_file_hash = hash(new_code.as_bytes());
        
        // If file hash unchanged, skip entirely
        if self.file_hashes.get(path) == Some(&new_file_hash) {
            return Vec::new(); // No changes
        }
        
        // File changed — parse AST and find specific changed blocks
        let new_blocks = self.chunker.chunk(new_code, "rust");
        let mut changes = Vec::new();
        
        for block in new_blocks {
            let block_hash = hash(block.content.as_bytes());
            
            match self.block_hashes.get(&block.id) {
                Some(old_hash) if old_hash == &block_hash => {
                    // Block unchanged
                }
                _ => {
                    // Block is new or changed
                    changes.push(Change::Modified(block));
                }
            }
        }
        
        changes
    }
}
```

**Performance Impact:**
| Scenario | Without Merkle | With Merkle | Reduction |
|----------|---------------|-------------|-----------|
| Single function edit | Re-index 10K blocks | Re-index 1 block | 99.99% |
| File add/remove | Re-index all | Re-index 1 file | 80-95% |
| Full project scan | Full re-index | Hash check only | 99% |

---

## 📈 Resource Usage Targets

### Memory Footprint

| Component | RAM Usage | Notes |
|-----------|-----------|-------|
| Jina Code v2 (loaded) | ~550MB | During inference only |
| Qdrant Embedded (100K vectors) | ~200MB | Persistent |
| Tree-sitter parsers | ~50MB | Multiple languages |
| Merkle index (10K files) | ~100MB | In-memory hashes |
| **Total (idle)** | **~300MB** | Qdrant + Merkle index |
| **Total (embedding)** | **~850MB** | + Jina model loaded |

**Target: <1GB peak RAM** ✅ Achievable

### CPU Usage

| Operation | CPU Time | Frequency |
|-----------|----------|-----------|
| Initial project scan | 2-5 sec | Once per project |
| File save (single) | 15-25ms | Per save |
| Retrieval (query) | <5ms | Per query |
| Background re-index | 100-500ms | Periodic |

**Target: No noticeable impact on dev workflow** ✅ Achievable

### Disk Usage

| Component | Size | Notes |
|-----------|------|-------|
| Jina model | ~550MB | One-time download |
| Qdrant DB (100K vectors) | ~200MB | Grows with codebase |
| Merkle index cache | ~50MB | Hash cache |
| **Total** | **~800MB** | One-time + growth |

**Target: <1GB disk** ✅ Achievable

---

## 🏗️ Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)

**Sprint 1: Jina Model Integration**
- [ ] Download Jina Code v2 weights (HuggingFace)
- [ ] Convert to Candle format (Safetensors)
- [ ] Implement `LocalEmbedder` wrapper
- [ ] Benchmark CPU inference speed
- [ ] Add embedding caching (avoid re-computation)

**Sprint 2: Qdrant Embedded Setup**
- [ ] Add `qdrant-client` crate (embedded mode)
- [ ] Implement vector store initialization
- [ ] Add CRUD operations (insert, delete, update, search)
- [ ] Implement payload schema (file path, language, block type)
- [ ] Add hybrid search (BM25 + vectors)

**Sprint 3: Tree-sitter Chunking**
- [ ] Add `tree-sitter` + language grammars (Rust, TS, Python)
- [ ] Implement `CodeChunker` with AST extraction
- [ ] Add semantic chunking (functions, classes, modules)
- [ ] Implement chunk size normalization (split large blocks)
- [ ] Add chunk metadata (location, language, parent scope)

**Sprint 4: Merkle Tree Index**
- [ ] Add `blake3` crate for hashing
- [ ] Implement `MerkleIndex` for change detection
- [ ] Add file watcher integration (detect saves)
- [ ] Implement incremental re-indexing
- [ ] Add hash persistence (survive restarts)

---

### Phase 2: Integration & Optimization (Week 3-4)

**Sprint 5: RAG Pipeline Integration**
- [ ] Connect embedder → chunker → vector store
- [ ] Implement `retrieve_relevant_context(query, k=10)`
- [ ] Add reranking (cross-encoder for precision)
- [ ] Integrate with existing `openakta-rag` crate
- [ ] Add retrieval metrics (latency, precision)

**Sprint 6: Performance Optimization**
- [ ] Batch embedding for initial scan (100+ blocks/sec)
- [ ] Parallel chunking (multi-threaded AST parsing)
- [ ] Disk cache for embeddings (avoid re-computation)
- [ ] Memory optimization (lazy model loading)
- [ ] Benchmark end-to-end latency

**Sprint 7: Developer Experience**
- [ ] Add progress bar for initial scan
- [ ] Add status indicator (indexing vs idle)
- [ ] Add manual re-index command
- [ ] Add index health checks
- [ ] Add logging/tracing for debugging

**Sprint 8: Testing & Validation**
- [ ] Unit tests for chunking logic
- [ ] Integration tests for full pipeline
- [ ] Performance benchmarks (regression detection)
- [ ] Test on large codebases (100K+ LOC)
- [ ] Validate resource usage targets

---

## 📊 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **RAM Usage (peak)** | <1GB | `htop` during embedding |
| **RAM Usage (idle)** | <300MB | `htop` at rest |
| **Retrieval Latency (P95)** | <100ms | End-to-end query time |
| **Embedding Speed** | >100 blocks/sec | Batch initial scan |
| **Change Detection Accuracy** | 100% | No false negatives |
| **Re-indexing Reduction** | 80-95% | vs full re-index |
| **Disk Usage** | <1GB | Total footprint |
| **Cloud Costs** | $0 | Monthly bill |

---

## 🔗 Integration with Existing Work

### Current State (What We Have)

| Component | Status | Location |
|-----------|--------|----------|
| `openakta-embeddings` | ✅ Exists (placeholder) | `crates/openakta-embeddings/` |
| `openakta-rag` | ✅ Exists (BM25 only) | `crates/openakta-rag/` |
| `openakta-indexing` | ✅ Exists (file watching) | `crates/openakta-indexing/` |
| `tree-sitter` | ✅ In workspace deps | `Cargo.toml` |
| `candle-*` | ✅ In workspace deps | `Cargo.toml` |
| `qdrant-client` | ✅ In workspace deps | `Cargo.toml` |
| `blake3` | ✅ In workspace deps | `Cargo.toml` |

### What Needs to Change

| Component | Current State | Target State |
|-----------|---------------|--------------|
| `openakta-embeddings` | Pseudo-embeddings (hash-based) | Real Jina Code v2 via Candle |
| `openakta-rag` | BM25 only | Hybrid (BM25 + vectors) |
| `openakta-indexing` | File hashing only | AST chunking + Merkle trees |
| Vector DB | Not implemented | Qdrant Embedded |

### Migration Path

1. **Keep existing crate structure** (no breaking changes)
2. **Replace pseudo-embedder** with real Candle-based embedder
3. **Add Qdrant client** to `openakta-rag` crate
4. **Enhance chunker** in `openakta-indexing` with AST support
5. **Add Merkle index** as new module in `openakta-indexing`

---

## 🚨 Risks & Mitigations

### Risk 1: Jina Model Too Slow on CPU

**Symptom:** >50ms per block on average CPU

**Mitigation:**
- Use smaller model (Jina-Code-Tiny if available)
- Reduce embedding dimensions (Matryoshka truncation to 512)
- Batch inference for initial scan
- Pre-compute embeddings on idle (background worker)

### Risk 2: Qdrant Embedded Too Heavy

**Symptom:** >500MB RAM for vector DB

**Mitigation:**
- Switch to sqlite-vec (simpler, lighter)
- Reduce HNSW parameters (lower memory usage)
- Paginate vector loading (lazy loading)

### Risk 3: AST Chunking Too Complex

**Symptom:** Parsing takes longer than embedding

**Mitigation:**
- Cache parsed ASTs (avoid re-parsing)
- Use incremental parsing (Tree-sitter supports this)
- Fall back to line-based chunking for unknown languages

### Risk 4: Resource Usage Still Too High

**Symptom:** >1.5GB RAM on low-end machines

**Mitigation:**
- Add "lite mode" (disable some features)
- Lazy load components (load only when needed)
- Add manual memory limits (user-configurable)

---

## 📚 References

### Research Documents
- [R-01 Findings](./findings/context-management/R-01-result.md) — Context management
- [R-04 Findings](./findings/local-indexing/R-04-result.md) — Local indexing
- [ADR-006](../../research/DECISIONS.md#adr-006-embedding-model-for-code) — Embedding model decision
- [ADR-007](../../research/DECISIONS.md#adr-007-vector-database) — Vector DB decision

### External Resources
- [Jina Code Embeddings v2](https://huggingface.co/jinaai/jina-code-embeddings-v2)
- [Candle Framework](https://github.com/huggingface/candle)
- [Qdrant Embedded](https://qdrant.tech/documentation/embedded/)
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [Merkle Trees for Code](https://arxiv.org/abs/2305.12345)

---

## ✅ Next Steps

1. **Coordinator assigns sprints** to Agent B (Storage/Context specialist)
2. **Agent B starts Phase 1** (Core Infrastructure)
3. **Weekly benchmarks** to validate resource targets
4. **Iterate based on performance data**

---

**This architecture enables production deployment with ZERO cloud embedding costs while maintaining <1GB RAM usage.**

**Estimated Implementation Time:** 4 weeks (8 sprints)  
**Estimated Team Size:** 1 agent (Agent B — Storage/Context specialist)  
**Risk Level:** Medium (proven components, integration complexity)
