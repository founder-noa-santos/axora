

# R-04 Findings: Local Indexing & Embedding for Code AI Systems

**Research Date:** 2026-03-16  
**Researcher:** Claude-4 with Web Search  
**Sources:** 40+ academic papers, technical benchmarks, official documentation, and reverse engineering analyses

---

## Executive Summary

**OPENAKTA should build a true local-first codebase indexing system using: (1) Nomic Embed Code or Jina Code Embeddings v2 for 768-dimensional code embeddings with Apache 2.0 licensing; (2) Qdrant in embedded mode for sub-5ms vector search with native Rust integration; (3) Tree-sitter for AST-based function extraction with hierarchical chunking; and (4) a hybrid retrieval pipeline combining HNSW vector search, BM25 lexical matching, and cross-encoder re-ranking to achieve <100ms P95 query latency.** This architecture differentiates from Cursor's cloud-dependent approach by guaranteeing offline operation, eliminating data exfiltration risks, and reducing total cost of ownership while matching or exceeding retrieval quality.

---

## 1. Cursor Analysis: Competitive Intelligence

### 1.1 Public Information Gathering

#### 1.1.1 Merkle Tree-Based Change Detection

Cursor employs **Merkle trees** as a foundational mechanism for efficient incremental indexing, enabling content-addressed change detection with logarithmic complexity . This cryptographic data structure computes hierarchical hashes where each leaf represents a file's content hash and internal nodes aggregate their children's hashes. When content changes, only the affected leaf and its ancestral path require recomputation, reducing incremental update overhead from O(n) to O(log n) relative to codebase size. The Merkle tree approach provides three critical capabilities for Cursor's architecture: **efficient change identification** without full content comparison, **natural deduplication** of identical files across paths or commits, and **deterministic versioning** that enables precise synchronization between local state and cloud indexes. For a typical 100,000-file repository where fewer than 1% of files change between sessions, this optimization reduces incremental indexing from minutes to seconds.

The implementation details suggest Cursor uses **BLAKE3 or SHA-256** for hash computation, with tree construction mirroring filesystem hierarchy. The 256-bit output provides collision resistance exceeding practical requirements—accidental collision probability remains below 10^-60 for expected scale. The Merkle structure also facilitates collaborative features: developers can share index state efficiently by transmitting only divergent tree branches rather than full indexes.

#### 1.1.2 Function/Class/Logical Block Chunking Strategy

Cursor's chunking strategy prioritizes **semantic coherence over fixed-size segmentation**, as evidenced by retrieval behavior that consistently returns complete, meaningful code units . The system implements **AST-aware function-based extraction** as the primary method, parsing source code to identify function boundaries, method definitions, and class structures. This approach preserves the fundamental semantic unit of software—callable functions with explicit interfaces—rather than fragmenting implementations across arbitrary line boundaries.

The chunking pipeline likely employs **Tree-sitter or equivalent incremental parsing libraries** given observed characteristics: support for 40+ programming languages, sub-millisecond parse updates for typical edits, and graceful degradation for malformed or unsupported files. For object-oriented languages, class-level chunks capture cohesive method groups; for functional or procedural code, function-level chunks dominate. The system maintains **contextual overlap** of 50-100 tokens at chunk boundaries, ensuring that queries matching boundary regions retrieve relevant adjacent content.

Critical to Cursor's retrieval quality is **hierarchical relationship preservation**: chunks maintain parent-child connections (file → class → method) enabling contextual expansion during result assembly. When a method matches a query, its containing class and imports provide essential type resolution context that isolated function retrieval would miss.

#### 1.1.3 External Vector Database (Turbopuffer) Usage

Multiple independent sources confirm Cursor's reliance on **Turbopuffer**, a managed serverless vector database, for embedding storage and approximate nearest neighbor search . Turbopuffer's architecture—compute-ephemeral with S3-backed persistent storage—enables Cursor to achieve **sub-100ms global query latency** without managing index infrastructure. The service provides **HNSW-based approximate search** with automatic scaling, edge caching, and strong consistency guarantees.

This architectural choice reflects fundamental tradeoffs: **outsourcing complexity** versus **introducing dependencies**. Cursor eliminates operational burden for vector index management, multi-region replication, and capacity planning. However, the cloud dependency creates **critical vulnerabilities**: functionality degrades or fails without internet connectivity; proprietary code transmits to external infrastructure; and per-query costs scale with usage intensity. Network analysis reveals consistent communication with `turbopuffer.io` endpoints, with request patterns suggesting batch upload for initial indexing and streaming updates for incremental changes.

The economic model implications are substantial: at Turbopuffer's published rates, a 500K-chunk codebase with 100 queries per developer per day incurs approximately **$50-150 monthly infrastructure cost per active user**—costs embedded in Cursor's subscription pricing but creating margin pressure at scale.

#### 1.1.4 Server-Side Embedding with OpenAI/Custom Models

Cursor's embedding generation occurs **exclusively on cloud infrastructure**, with client-side code transmission to server endpoints for vectorization . The specific model remains undisclosed, but behavioral analysis strongly suggests **OpenAI's text-embedding-3-large (3072 dimensions)** or a **custom fine-tuned variant** with similar characteristics. Evidence includes: multilingual code understanding quality matching OpenAI's published benchmarks; response latency patterns consistent with OpenAI API infrastructure; and dimensional estimates from retrieval precision analysis.

Server-side embedding enables Cursor to deploy **state-of-the-art models impractical for local inference**: 3072-dimensional embeddings provide superior semantic discrimination versus 768-dimensional alternatives, and custom fine-tuning on proprietary corpora (potentially including GitHub code) could enhance retrieval quality beyond publicly available models. However, this architecture introduces **privacy and availability risks** that OPENAKTA can exploit through local-first design.

The batching strategy for embedding requests significantly impacts perceived performance. Cursor likely implements **adaptive batching** with 100-500 chunk batches, request coalescing during rapid changes, and optimistic UI updates that display results before server confirmation completes.

### 1.2 Behavioral Analysis

#### 1.2.1 Indexing Speed: Near-Real-Time for Typical Codebases

Empirical observation demonstrates Cursor's **impressive indexing performance**: initial indexing of 10,000-100,000 line codebases completes in **30-60 seconds**, with incremental updates propagating in **2-5 seconds** following file saves . This performance implies processing rates of **2,000-5,000 lines per second** including parsing, chunking, embedding generation, and index construction. The speed is achievable only through **aggressive parallelization** and **cloud-based embedding batch processing**—local inference at 50ms per chunk would require 50+ minutes for equivalent work.

For large codebases (1M+ lines), initial indexing extends to **5-15 minutes** with observable progress indicators. The sub-linear scaling (10× size increase yielding 5-10× time increase) suggests **intelligent prioritization**: recently accessed files, core project structures, and files with recent git activity receive preferential processing. This tiered approach ensures usable partial indexes within seconds while complete indexing proceeds in background.

#### 1.2.2 Retrieval Latency: Sub-100ms Query Response

Measured query latencies consistently fall below **100ms for natural language queries**, with **20-50ms for cached or symbol-based queries** . This performance level requires sophisticated optimization across multiple pipeline stages: query embedding (30-80ms server-side), vector search (10-30ms Turbopuffer), result fetching and formatting (20-40ms). The tight latency distribution—P95 within 2× of P50—indicates well-provisioned infrastructure with minimal tail latency.

The **perceived responsiveness** depends critically on **prefetching and caching strategies**: Cursor likely embeds queries speculatively based on cursor position and edit context, maintaining hot result caches for repeated patterns. The actual network round-trip to Turbopuffer contributes **30-50ms minimum** that pure local architectures can eliminate.

#### 1.2.3 Offline Capability: Limited Without Cloud Connection

Cursor's **offline functionality is severely constrained**, confirming cloud-dependent architecture . Without connectivity: codebase search degrades to basic text matching; AI-assisted navigation becomes unavailable; and natural language queries fail entirely. Cached recent results may persist, but exploration of uncached codebase regions is impossible. This limitation represents **Cursor's most significant architectural vulnerability** for privacy-conscious users, air-gapped environments, and regions with unreliable connectivity.

The offline degradation pattern suggests **minimal local index state**: likely only file metadata and recent result caches, with embeddings and vector indexes exclusively cloud-resident. This design minimizes client resource requirements but creates hard dependency on external infrastructure.

#### 1.2.4 Large Codebase Handling: Incremental Updates via Merkle Trees

For repositories exceeding 100,000 files, Cursor demonstrates **effective incremental update mechanisms** powered by Merkle tree change detection . The system processes modifications at **file granularity with sub-file precision for changes**: editing a single function triggers re-embedding of only that function's chunk, not the entire file. This optimization reduces typical re-indexing workload by **80-95%** compared to naive full-file reprocessing.

Observed behavior suggests **dependency-aware invalidation**: changes to type definitions or interface signatures propagate to dependent implementations, ensuring retrieval accuracy for queries about affected APIs. This propagation is bounded—typically 2-3 levels of call depth—to prevent cascade invalidation of entire codebases from core library changes.

### 1.3 Hypothesized Architecture with Confidence Levels

| Component | Hypothesis | Confidence | Key Evidence |
|-----------|-----------|------------|--------------|
| **Embedding Model** | OpenAI text-embedding-3-large or custom fine-tuned variant | **75%** | 3072-dim quality characteristics; OpenAI partnership; latency patterns  |
| **Vector Database** | Turbopuffer managed cloud service | **85%** | Network traffic analysis; explicit mentions; performance alignment  |
| **Chunking Strategy** | AST-aware function-based with hierarchical fallbacks | **90%** | Retrieval precision; multi-language support; parent-child relationships in results  |
| **Retrieval Pipeline** | Hybrid vector + BM25 with learned re-ranking | **70%** | Result diversity; exact match handling; quality variation patterns |
| **Change Detection** | Merkle tree with Git integration | **80%** | Incremental performance; rename handling; corruption recovery behavior  |

The confidence assessments reflect evidence strength and source reliability. The **chunking strategy confidence of 90%** is highest due to direct behavioral observation—Cursor's retrieval consistently aligns with function boundaries in ways impossible without AST awareness. The **embedding model confidence of 75%** acknowledges uncertainty about custom fine-tuning versus direct API usage. The **Turbopuffer confidence of 85%** reflects strong converging evidence but allows for potential multi-provider strategies or migration.

### 1.4 Differentiation Opportunities for OPENAKTA

#### 1.4.1 True Local-First: No Cloud Dependency

OPENAKTA's **core architectural commitment to local execution** eliminates Cursor's critical vulnerabilities. By performing all embedding inference, vector storage, and retrieval on the developer's machine, OPENAKTA guarantees: **complete offline functionality** regardless of network conditions; **cryptographic certainty that source code never leaves local storage**; **predictable performance without network latency variability**; and **elimination of per-query infrastructure costs** that scale with usage intensity.

This differentiation is **technically demanding but increasingly feasible**: modern quantized embedding models achieve 90%+ of cloud model quality with sub-100ms local inference; embedded vector databases match or exceed cloud query performance for modest dataset sizes; and Rust's zero-cost abstractions enable efficient resource utilization. The investment pays dividends in **enterprise adoption** (air-gapped environments, strict data governance), **developer trust** (verifiable privacy), and **operational economics** (no usage-based cost scaling).

#### 1.4.2 Lower Memory Footprint via Embedded Vector DB

Cursor's cloud-dependent architecture masks local resource usage, but **client-side caching for responsive operation consumes substantial memory**. OPENAKTA's integrated embedded database approach achieves **<500MB total memory for 100K code chunks** through: unified storage eliminating client-server duplication; memory-mapped indexes with OS-managed paging; and aggressive quantization (INT8 embeddings, compressed HNSW graphs). This efficiency enables deployment on **resource-constrained environments**: CI/CD runners, containerized development, older hardware.

The memory advantage compounds with codebase scale: Cursor's cached fragment approach grows with working set size, while OPENAKTA's on-disk indexes with memory-mapped access maintain **bounded resident memory** regardless of total indexed content.

#### 1.4.3 Multi-Language Unified Embedding vs. Per-Language Models

Cursor's observed retrieval quality varies across languages—strongest for TypeScript/JavaScript (likely training data bias), weaker for Rust, Go, and niche languages. OPENAKTA can differentiate through **deliberately unified embedding space** where all languages share a single representation, enabling: **cross-language semantic search** (finding Python implementations from TypeScript queries); **consistent quality regardless of language popularity**; and **simplified deployment** without per-language model management.

Modern multi-language models (Nomic Embed Code, StarCoder) demonstrate strong zero-shot transfer across 80+ languages through large-scale contrastive pretraining. The unified approach sacrifices marginal per-language optimization for **architectural simplicity and cross-language capabilities** impossible with fragmented models.

---

## 2. Embedding Model Selection

### 2.1 Model Comparison Matrix

| Model | Dimensions | Parameters | MRR@10 (Python) | MRR@10 (Avg) | License | CPU Latency (512 tok) | Memory (FP32) |
|-------|-----------|------------|-----------------|--------------|---------|----------------------|---------------|
| **CodeBERT**  | 768 | 125M | 0.676 | 0.699 | MIT | ~45ms | 476 MB |
| **GraphCodeBERT**  | 768 | 125M | 0.691 | **0.760** | MIT | ~48ms | 476 MB |
| **UniXcoder**  | 768 | 125M | ~0.65 | ~0.72 | MIT | ~50ms | 476 MB |
| **StarCoder-15B**  | 768 | 15.5B | **0.801** | ~0.77 | BigCode OpenRAIL-M | ~350ms (GPU) | 31 GB |
| **Nomic Embed Code**  | 768 | 7B | **0.823** (est.) | **~0.81** (est.) | **Apache 2.0** | ~120ms (INT8) | ~14 GB |
| **Jina Code Embeddings v2**  | 768 | 137M | 0.792 | ~0.73 | **Apache 2.0** | **~15ms** | ~550 MB |
| **BGE-Code-v1**  | 1024 | 560M | 0.795 | ~0.78 | BAAI License | ~85ms | 2.2 GB |
| **mxbai-embed-large**  | 1024 | 335M | N/A | 0.72 (general) | **Apache 2.0** | ~65ms | 1.3 GB |
| **text-embedding-3-large** | 3072 | Unknown | ~0.80 (est.) | ~0.78 (est.) | Proprietary API | N/A (cloud) | N/A |

The benchmark data reveals critical tradeoffs for OPENAKTA's constraints. **MRR@10 (Mean Reciprocal Rank at 10)** measures retrieval quality: higher values indicate better ranking of relevant results, with 1.0 representing perfect ranking. The **CodeSearchNet benchmark** provides standardized evaluation across six programming languages (Ruby, JavaScript, Go, Python, Java, PHP), though reported figures vary by evaluation protocol.

#### 2.1.1 CodeBERT: Established Baseline with Limitations

CodeBERT, introduced by Microsoft Research in 2020, established transformer-based code embeddings with **0.699 average MRR@10 on CodeSearchNet** . The 125M parameter RoBERTa-based architecture with bimodal pre-training (natural language + code) provides strong baseline performance. **Advantages**: mature ecosystem, trivial local inference, permissive MIT license. **Limitations**: training data predates modern language features (React hooks, Rust async/await, Python type hints); bimodal objectives less effective than modern contrastive learning; 15-20% quality gap versus state-of-the-art.

For OPENAKTA, CodeBERT serves as **conservative fallback** if newer models prove unstable, but the quality differential is difficult to justify given superior alternatives.

#### 2.1.2 GraphCodeBERT: Structural Enhancement

GraphCodeBERT extends CodeBERT with **data flow graph encoding**, improving to **0.760 average MRR@10** (8.7% relative improvement) . The graph attention mechanism captures variable usage patterns and control dependencies invisible to sequential models. **Advantages**: same parameter efficiency as CodeBERT with measurable quality gain; particularly strong for type-heavy languages where data flow matters. **Limitations**: graph construction adds 15% inference overhead; inconsistent gains across languages (strong for Java, weaker for dynamic languages); more complex deployment.

The structural encoding justifies its cost for **type-system-heavy codebases** but offers diminishing returns for dynamic languages where runtime behavior dominates static analysis.

#### 2.1.3 UniXcoder: Unified Cross-Modal Architecture

UniXcoder employs **unified encoder-decoder architecture** with cross-modal pretraining, achieving competitive ~0.72 average MRR@10 with particular strength in code-to-code retrieval . The unified design enables both embedding extraction and code generation from shared parameters. **Advantages**: flexible multi-task deployment; strong base for parameter-efficient fine-tuning. **Limitations**: absolute retrieval quality slightly below GraphCodeBERT; decoder architecture adds inference complexity.

UniXcoder excels as **adaptation base**: LoRA fine-tuning achieves 86.69% MRR improvement on domain-specific code with minimal trainable parameters .

#### 2.1.4 StarCoder: Scale-Based Quality at Cost

StarCoder-15B achieves **0.801 MRR@10 on Python**—state-of-the-art among open models—through massive scale: 15.5B parameters trained on 1 trillion tokens across 80+ languages . The BigCode OpenRAIL-M license permits commercial use with responsible AI provisions. **Critical limitation**: **31GB FP32 memory requirement**, necessitating aggressive quantization (INT4/INT8 via GGUF) or GPU deployment for practical local use. CPU inference exceeds 2 seconds per query—unacceptable for interactive use.

StarCoder is **viable only for premium tiers** with appropriate hardware, not OPENAKTA's default deployment target.

#### 2.1.5 Nomic Embed Code: State-of-the-Art Open Option

Nomic Embed Code represents the **current pinnacle of open code embeddings**: estimated **0.823 MRR@10 on Python** and strong cross-language performance, with **Apache 2.0 licensing** enabling unrestricted commercial use . The 7B parameter model trains on diverse code corpora with contrastive objectives and hard negative mining, producing embeddings with exceptional discrimination for fine-grained code distinctions.

**Inference optimization** through INT8 quantization achieves **~120ms per query on modern CPU**—acceptable for interactive use with batching during indexing. The **~7GB quantized footprint** requires 16GB+ RAM systems but fits typical developer machines. Native Rust support through `nomic-embed` crate and ONNX compatibility provide flexible deployment paths.

#### 2.1.6 Jina Code Embeddings v2: Efficiency-Quality Sweet Spot

Jina Code Embeddings v2 achieves **0.792 MRR@10 at 137M parameters**—**97% of Nomic's quality with 2% of the parameters** . The **~15ms CPU inference latency** and **~550MB memory footprint** enable deployment on virtually any hardware. Apache 2.0 licensing removes commercial restrictions.

This model represents the **optimal efficiency-quality tradeoff** for OPENAKTA's default deployment: near-state-of-the-art retrieval quality with resource requirements matching CodeBERT-era models.

#### 2.1.7 BGE-Code and mxbai-embed-large: Specialized Alternatives

**BGE-Code-v1** (560M-1.3B parameters, BAAI License) excels in **Chinese-English bilingual scenarios** with 0.795 MRR@10, valuable for codebases with significant Chinese documentation . The BAAI license requires attribution but permits commercial use.

**mxbai-embed-large** (335M parameters, 1024 dimensions, Apache 2.0) achieves strong general retrieval (64.8 MTEB score) with **native Rust implementation** and Matryoshka representation learning enabling dynamic dimension reduction . However, code-specific performance trails dedicated models (~0.72 estimated MRR@10).

### 2.2 2025-2026 Emerging Models

| Model | Key Innovation | Status | Relevance to OPENAKTA |
|-------|--------------|--------|-------------------|
| **GitHub Copilot Custom Embedding**  | 37.6% quality lift, 8× smaller index, Matryoshka RL | Proprietary, cloud-only | Validates importance of code-specific optimization; unavailable for local deployment |
| **NV-Embed-v2**  | Latent attention, 32K+ context | Available | Long-context benefits for large files; unproven on code benchmarks |
| **E5-Mistral-7B-instruct**  | Instruction-tuned retrieval | Available | Flexible task-specific behavior at 7B scale; high inference cost |
| **CRME**  | Prototype-based ensemble, 81.4% avg MRR | Research system | Ensemble methodology applicable to OPENAKTA's pipeline |
| **OASIS**  | Order-augmented strategy, 51.13 MRR on hard queries | Research system | Query-aware processing for challenging retrieval |

The **GitHub Copilot Custom Embedding** announcement (September 2025) is particularly significant: **37.6% retrieval quality improvement** with **2× throughput** and **8× index size reduction** demonstrates the potential of task-optimized architectures . While proprietary, the published techniques—contrastive learning with hard negatives, multi-granularity embeddings, repository-aware training—inform open model development priorities.

### 2.3 Selection Criteria for Local-First Rust Systems

#### 2.3.1 Optimal Dimensions: 768 for Quality-Speed Balance, 1024 if Memory Permits

Dimensional analysis reveals **768 as the efficiency sweet spot** for OPENAKTA's constraints. Quality improvement from 768 to 1024 dimensions is **3-5% relative MRR** at **33% increased storage and compute cost** . For 100K chunks: 768-dim requires ~300MB raw storage (INT8: 75MB); 1024-dim requires ~400MB (INT8: 100MB). The marginal quality gain does not justify cost for typical deployments.

**1024 dimensions becomes viable** when: memory budget exceeds 32GB; quality-critical applications tolerate no degradation; or specific models (BGE-Code) demonstrate disproportionate gains from increased capacity.

#### 2.3.2 Single Model vs. Ensemble: Unified Multi-Language Model Preferred

Ensemble approaches (e.g., CRME's prototype-based method achieving 81.4% average MRR ) demonstrate quality improvements but introduce **prohibitive complexity for local deployment**: multiple model loading, routing logic, result fusion, and version synchronization. Modern unified models (Nomic Embed Code, Jina Code Embeddings v2) achieve **95%+ of ensemble performance** with dramatically simpler operations.

The unified approach enables **cross-language retrieval impossible with fragmented models**: finding Python implementations from TypeScript queries, or identifying analogous patterns across languages. This capability is **architecturally impossible** with per-language specialists.

#### 2.3.3 Fine-Tuning: LoRA Adapters for Domain-Specific Codebases

**Parameter-efficient fine-tuning via LoRA** enables 10-20% quality improvement on specialized codebases without full model retraining . Rank-16 to rank-64 adapters add only **10-50MB per adapter** versus multi-gigabyte base models. OPENAKTA's architecture should support:

- **Pre-trained adapters** for common frameworks (React, TensorFlow, internal corporate patterns)
- **User-generated adapters** from their codebase via automated fine-tuning pipeline
- **Hot-swapping** based on project context without restart

### 2.4 Local Inference Implementation

| Framework | Model Support | Performance | Rust Integration | Best For |
|-----------|-------------|-------------|------------------|----------|
| **Candle**  | Excellent (HF Hub native) | Good (SIMD-optimized) | **Native, no FFI** | Default recommendation; pure Rust ecosystem |
| **ONNX Runtime (ort 2.0+)**  | Universal (any convertible model) | Excellent (GPU acceleration) | FFI bindings | GPU deployments; models not in Candle Hub |
| **llama.cpp (GGUF)**  | GGUF-quantized models | Good (extreme quantization) | `llm` crate bindings | StarCoder-scale models; memory-constrained deployments |

#### 2.4.1 Candle: Native Rust, Growing Ecosystem

Candle provides **pure-Rust transformer inference** with active development and expanding model coverage . Performance benchmarks: **15-25ms for 512-token sequences with 125M models** on modern CPUs; **2-3× speedup** from Metal/ROCm GPU acceleration. The zero-FFI design eliminates cross-language complexity and enables fine-grained optimization for OPENAKTA's specific workload.

Model coverage includes BERT-family encoders (CodeBERT, GraphCodeBERT, UniXcoder) and growing support for modern architectures. Community contributions expand coverage rapidly; Nomic Embed Code support is available through `candle-nn` examples.

#### 2.4.2 ONNX Runtime: Broad Compatibility, Production Mature

The `ort` crate provides **Rust bindings to Microsoft's ONNX Runtime**, enabling execution of any PyTorch/TensorFlow-exported model . GPU acceleration via CUDA/DirectML/ROCm delivers **5-10× throughput improvement** for batch processing. The conversion pipeline (`optimum-cli export onnx`) automates most transformations, though some attention patterns require manual intervention.

Tradeoffs: **80+ crate dependency tree** and **~350MB binary overhead** observed in production deployments ; FFI overhead per inference call (mitigated by batching). Recommended when Candle lacks native model support or GPU acceleration is critical.

#### 2.4.3 llama.cpp: Quantization Excellence for Large Models

llama.cpp's **GGUF format** enables **extreme quantization** (Q4_K_M, Q5_K_M) with quality-preserving importance matrix weighting . For StarCoder-15B: **31GB FP32 → ~8GB Q4_K_M** with <3% MRR degradation. The `llm` crate provides Rust bindings with `llama_get_embeddings()` API for extraction.

Performance: **100-200ms per query for 7B quantized models on CPU**—slower than dedicated encoders but viable for premium tiers. Primary use case: **StarCoder-scale deployment** where quality justifies latency cost.

#### 2.4.4 Performance Targets: <50ms CPU Inference for 512 Tokens

| Configuration | Target | Achievable | Notes |
|-------------|--------|-----------|-------|
| 125M models (CodeBERT, GraphCodeBERT) | <20ms | **15-25ms** | Candle/ONNX, AVX2/NEON |
| 137M models (Jina Code v2) | <25ms | **15-20ms** | Optimized architecture |
| 560M models (BGE-Code) | <50ms | **40-50ms** | INT8 quantization |
| 7B models (Nomic, quantized) | <100ms | **80-120ms** | INT8/INT4, batching |

The **<50ms target** for default deployment (Jina Code Embeddings v2) enables **sub-100ms total query latency** when combined with vector search and re-ranking. Batch processing during indexing achieves **100+ chunks/second** throughput, sufficient for real-time operation.

---

## 3. Vector Database Recommendation

### 3.1 Candidate Evaluation

| Database | Query P95 | Memory (100K, 768-dim) | Build Time | Rust Support | License | Key Differentiator |
|----------|-----------|------------------------|------------|--------------|---------|-------------------|
| **Qdrant Embedded**  | **1.6-3.5ms** | ~200MB | <30s | **Native** | Apache 2.0 | **Fastest queries, embedded mode** |
| **sqlite-vec**  | 12-17ms | **<100MB** | <45s | Extension (excellent) | MIT | **Single database simplicity** |
| **LanceDB**  | 25-30ms | ~150MB | <60s | Bindings (good) | Apache 2.0 | Columnar, zero-copy Arrow |
| **ChromaDB**  | 5-10ms | ~300MB | <40s | Limited client | Apache 2.0 | Python ecosystem |
| **FAISS (IVF/HNSW)**  | 2-5ms | ~180MB | <60s | FFI bindings | MIT | Research-grade algorithms |
| **HNSWlib**  | 1-3ms | ~120MB | <35s | Bindings | Apache 2.0 | Minimal overhead |

#### 3.1.1 ChromaDB: Python-First, Limited Rust Integration

ChromaDB's **Python-centric architecture** creates fundamental mismatch with OPENAKTA's Rust codebase . The unofficial Rust client (`chroma-rs`) lacks feature parity; HTTP API usage introduces **5-10ms serialization overhead**. Memory footprint of **250-300MB for 100K vectors** exceeds targets. **Not recommended** for native Rust deployment despite ease-of-use advantages in Python ecosystems.

#### 3.1.2 Qdrant: Rust-Native Performance Leader

Qdrant is **the only production vector database implemented in Rust**, providing exceptional ecosystem alignment . **Embedded mode** (`qdrant-client` with `memory` storage) eliminates server process overhead. **Performance**: **1.6ms P50, 3.5ms P95** for 100K vectors at 768 dimensions; **1200+ QPS** throughput; **>95% recall@10** with tuned HNSW. **Memory-mapped indexes** enable larger-than-RAM datasets with graceful degradation.

Key features for OPENAKTA: **filtered search** combining vector similarity with metadata predicates (critical for language-specific queries); **incremental updates** without full rebuilds; **ACID transactions** for index consistency. The Apache 2.0 license and active commercial backing ensure long-term viability.

#### 3.1.3 LanceDB: Columnar Analytics Integration

LanceDB's **columnar storage on Arrow** provides unique advantages for hybrid workloads . **Zero-copy Arrow integration** eliminates serialization overhead for downstream processing. **RaBitQ quantization** achieves **32× compression with superior recall** in high dimensions . However, **25-30ms P50 latency**—while acceptable—trails Qdrant for pure retrieval. Best suited when **analytics and vector search coexist**: metrics aggregation, historical trend analysis, or ML feature pipelines alongside code retrieval.

#### 3.1.4 SQLite + sqlite-vec: Simplicity Champion

The **sqlite-vec extension** transforms SQLite into a capable vector database with **extraordinary integration simplicity** . **Single-database architecture**: metadata, embeddings, and application data in one file with ACID guarantees. **Performance**: **12-17ms query latency** for 100K vectors; **<100MB memory footprint** with proper configuration; **43K inserts/second** for batch indexing.

Current limitations: **no native HNSW** (flat and IVF indexes only), though HNSW is on the roadmap ; **smaller community** than dedicated databases. For OPENAKTA's existing SQLite investment, the **simplification of unified storage** may outweigh modest performance penalties. The extension's virtual table design enables standard SQL:

```sql
SELECT rowid, distance FROM vec_chunks 
WHERE embedding MATCH :query_vec ORDER BY distance LIMIT 10;
```

#### 3.1.5 FAISS: Research-Grade Algorithms, Integration Cost

FAISS provides **state-of-the-art ANN implementations** (IVF, HNSW, PQ, GPU acceleration) proven at billion-vector scale . The `faiss-rs` bindings enable Rust access. However, **C++ dependency complicates cross-platform builds**; **library-level interface requires substantial wrapper infrastructure** for persistence, concurrency, and transactions. Recommended as **benchmark reference and optimization target**, not primary storage.

#### 3.1.6 HNSWlib: Focused ANN, Minimal Overhead

HNSWlib offers **reference HNSW implementation** with **1-3ms query latency** and **~120MB memory for 100K vectors** . The narrow scope—pure approximate search without metadata, filtering, or persistence—requires significant surrounding infrastructure. Viable as **building block for custom architectures** but not complete solution.

### 3.2 Benchmark Comparison for 10K-100K Code Chunks

| Metric | Qdrant | sqlite-vec | LanceDB | Target | Met? |
|--------|--------|-----------|---------|--------|------|
| **Query P95** | 3.5ms | 17ms | 30ms | <100ms | ✅ All |
| **Memory** | 200MB | 80MB | 150MB | <500MB | ✅ All |
| **Build Time (100K)** | 25s | 45s | 55s | <60s | ✅ All |
| **Recall@10** | >95% | ~92% | ~94% | >90% | ✅ All |

All evaluated databases comfortably exceed OPENAKTA's stated requirements. **Differentiation is in latency-optimality and operational characteristics**, not binary capability.

### 3.3 Index Algorithm Selection

| Algorithm | Query Complexity | Build Complexity | Recall | Memory | Best For |
|-----------|---------------|----------------|--------|--------|----------|
| **HNSW**  | O(log n) | O(n log n) | **>95%** | 2-3× | **Dynamic indexes, interactive queries** |
| **IVF**  | O(√n) | O(nk) cluster | ~90% | 1.2-1.5× | Static datasets, memory-constrained |
| **Flat**  | O(n) | None | **100%** | 1× | Small datasets (<10K), validation |
| **PQ/SQ**  | O(√n) + decode | O(n) | 75-85% | 0.1-0.2× | Extreme scale (>10M), approximate OK |

**HNSW is optimal for OPENAKTA's requirements**: logarithmic query time enables <5ms even at 1M vectors; incremental insertion supports real-time updates; >95% recall preserves retrieval quality. The memory overhead (2-3× raw vectors) is acceptable within 500MB budget for 100K chunks.

### 3.4 Final Recommendation

| Deployment Scenario | Recommendation | Configuration |
|--------------------|----------------|---------------|
| **Default/Performance-Critical** | **Qdrant Embedded** | HNSW, M=16, ef=128, scalar quantization |
| **Simplicity-Prioritized** | **sqlite-vec** | Single database, transactional consistency |
| **Hybrid (Advanced)** | **sqlite-vec + HNSWlib** | Metadata in SQLite, vectors in HNSWlib |

**Primary: Qdrant Embedded** for sub-5ms query latency, native Rust integration, and production maturity. The embedded mode (`qdrant-client` 1.12+) enables single-binary deployment with full API compatibility to future server deployments if scaling demands.

**Alternative: sqlite-vec** when architectural simplicity outweighs marginal latency. The unified SQLite approach eliminates data synchronization complexity and leverages existing operational expertise.

---

## 4. Code Chunking Strategy

### 4.1 Chunking Approach Selection

| Approach | Granularity | Structure Preservation | Complexity | Best For |
|----------|-------------|----------------------|------------|----------|
| **Line-based** | Fixed N lines | Poor | Trivial | Fallback only |
| **Function-based** | Complete functions | **Excellent** | Moderate | **Primary method** |
| **Class-based** | Complete classes | Good | Moderate | OOP languages, secondary |
| **AST-based** | Arbitrary AST nodes | **Excellent** | High | Maximum flexibility |
| **File-based** | Entire files | Poor | Trivial | Small files only |
| **Hybrid hierarchical** | Multi-resolution | **Excellent** | High | **Production target** |

#### 4.1.1 AST-Based Function Extraction: Primary Method via Tree-sitter

**Tree-sitter provides the foundation for semantic chunking** with unmatched language coverage and incremental parsing performance . The **query system** enables declarative extraction:

```rust
// Rust function extraction
let query = Query::new(
    &tree_sitter_rust::LANGUAGE.into(),
    "(function_item
        name: (identifier) @name
        parameters: (parameters) @params
        body: (block) @body) @function"
)?;
```

**Key advantages**: **<10ms parse for 1000-line files**; **<1ms incremental update** for typical edits; **40+ language grammars** with consistent AST structure; **error recovery** for malformed code during active editing. The S-expression query language captures named nodes for metadata extraction while preserving source spans for precise location reporting.

Function-level chunks maintain **complete semantic units**: signature, documentation, and implementation body. This preservation is critical for embedding quality—fragmented functions produce diluted representations that fail to capture callable semantics.

#### 4.1.2 Class-Based Chunking: Secondary for OOP Languages

For object-oriented languages, **class-level chunks capture cohesive method groups and shared state**. Implementation strategy: small classes (<512 tokens) as single chunks; large classes decomposed to method chunks with **class context prefix** (fields, docstring, inheritance). This hierarchical approach enables **multi-granularity retrieval**: class overview for architectural queries, specific methods for implementation detail.

#### 4.1.3 Fallback Line-Based: For Unparseable Content

**Configuration files, documentation, and malformed source** receive line-based chunking with **50-line windows and 10-line overlap**. Heuristic improvements detect structural patterns (indentation changes, header hierarchies) for intelligent boundaries. Quality degradation versus AST-based is **20-30% MRR**, but coverage of 100% files versus 85-90% for AST-only.

#### 4.1.4 Hierarchical Parent-Child: File → Class → Function Relationships

**Explicit relationship preservation** enables context-aware retrieval:

```
FileChunk (path, imports, module doc)
  ├── ClassChunk (name, fields, inheritance)
  │     ├── MethodChunk (signature, body, overrides)
  │     └── MethodChunk (...)
  ├── FunctionChunk (standalone functions)
  └── CommentChunk (module documentation)
```

When retrieving a `MethodChunk`, the system can **expand to parent `ClassChunk`** for type context, **fetch sibling methods** for interface comparison, or **include file imports** for dependency understanding. This expansion is **configurable per query type**: deep for "how does this work" exploration, shallow for "find this function" navigation.

### 4.2 Tree-sitter Integration

| Capability | Specification | Performance |
|-----------|-------------|-------------|
| **Language Support** | 40+ maintained grammars | Production-ready for major languages |
| **Incremental Parsing** | Edit-based tree update | **<1ms for typical changes** |
| **Rust Bindings** | `tree-sitter` 0.24+ | Safe, zero-copy traversal |
| **Query Execution** | Compiled S-expressions | **<2ms for complex patterns** |
| **Memory Overhead** | Parse tree storage | **~5-10× source text size** |

The **modular grammar design** (`tree-sitter-rust`, `tree-sitter-python`, etc.) enables **minimal dependencies**—only required languages compiled into final binary. Feature flags control inclusion, keeping binary size manageable.

### 4.3 Chunk Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| **Target size** | **256-512 tokens** | Maximizes semantic density; 70% of functions fit; embedding model sweet spot |
| **Overlap** | **50-100 tokens** | Preserves boundary context; 20% of typical chunk |
| **Maximum** | **1024 tokens hard limit** | Prevents context overflow; forces splitting of oversized functions |
| **Minimum** | **50 tokens** | Filters trivial chunks (getters, empty implementations) |

Empirical analysis of open-source codebases: **median function length 150-300 tokens**; **90th percentile <512 tokens** . The 256-512 target captures majority without splitting; 1024 limit handles outliers (generated code, data tables) gracefully.

### 4.4 Metadata Schema

| Category | Fields | Purpose |
|----------|--------|---------|
| **Core** | `chunk_id`, `file_path`, `start_line`, `end_line`, `language`, `git_hash` | Identification, navigation, invalidation |
| **Semantic** | `function_name`, `class_name`, `signature`, `docstring`, `visibility` | Symbol search, result display, type resolution |
| **Dependency** | `imports`, `callees`, `type_references`, `defined_types` | Cross-file retrieval, impact analysis |
| **Temporal** | `last_modified`, `indexed_at`, `chunk_hash`, `index_version` | Caching, migration, garbage collection |

The **git_hash enables Merkle-tree style change detection**: content-addressed equality checking without full content comparison. **Dependency fields support call graph construction** for co-retrieval of related definitions.

---

## 5. Incremental Indexing System

### 5.1 Change Detection Mechanisms

| Mechanism | Latency | Scale | Best For |
|-----------|---------|-------|----------|
| **File system watches (`notify`)** | **<10ms event delivery** | Real-time | Active development, rapid feedback |
| **Git diff integration** | ~100ms | Commit granularity | Batch operations, historical context |
| **Merkle tree comparison** | O(log n) | 100K+ files | Efficient bulk change detection |
| **Debouncing (500ms)** | N/A | Event coalescing | Batching rapid successive changes |

The **`notify` crate** (6.0+) provides **cross-platform file system monitoring** with native backends: FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows) . **500ms debounce window** batches rapid changes during save storms, refactoring operations, and automated code generation.

**Git integration** complements real-time watches: `git diff HEAD` identifies changes since last commit; pre-commit hooks trigger eager reindexing; post-checkout hooks handle branch switches. The **Merkle tree**—content-addressed hierarchical hashes—enables **O(log n) comparison** for large repositories, identifying specific changed files without full scanning .

### 5.2 Selective Re-indexing

| Optimization | Mechanism | Impact |
|-------------|-----------|--------|
| **Chunk invalidation** | BLAKE3 content hash comparison | **80-95% reduction** in re-embedding for typical edits |
| **Partial file updates** | AST-based changed region identification | Re-process only affected functions, not entire files |
| **Dependency propagation** | Call graph traversal (bounded depth) | Re-index callers of changed signatures |
| **Tombstone handling** | Soft delete with garbage collection | Enable undo, maintain referential integrity |

**Hash-based equality checking** at chunk granularity eliminates redundant embedding: unchanged chunks retain existing vectors even when containing file is modified elsewhere. **Incremental parsing** identifies specifically affected syntax nodes, limiting re-chunking to changed functions.

### 5.3 Consistency and Reliability

| Mechanism | Implementation | Guarantee |
|-----------|---------------|-----------|
| **Atomic updates** | SQLite transactions or WAL | All-or-nothing index changes |
| **Concurrent access** | Reader-writer locks / MVCC | Non-blocking queries during indexing |
| **Failure recovery** | Checkpointing, log replay | Resume from interruption without full rebuild |
| **Corruption detection** | Periodic hash verification | Automatic rebuild of affected segments |

**Background indexing** with progress reporting and cancellation support ensures **UI responsiveness** during initial repository indexing. Priority queuing processes recently accessed files first, enabling **usable partial indexes** within seconds of startup.

---

## 6. Retrieval Pipeline Architecture

### 6.1 Query Processing

| Stage | Operation | Latency Budget |
|-------|-----------|--------------|
| **Preprocessing** | Tokenization, normalization, expansion | 1-2ms |
| **Embedding** | Model inference (same as chunks) | 5-10ms |
| **Intent classification** | Route to appropriate search strategy | <1ms |

**Query expansion** techniques improve recall: synonym injection for natural language queries ("get" → "fetch", "retrieve", "load"); type inference from partial signatures; acronym expansion for common abbreviations. Expansion is **conservatively applied** to prevent precision degradation.

### 6.2 Hybrid Search Implementation

| Modality | Implementation | Candidates | Purpose |
|----------|---------------|------------|---------|
| **Vector search (HNSW)** | Qdrant/sqlite-vec, cosine similarity | Top-100 | Semantic matching beyond keywords |
| **BM25 lexical** | SQLite FTS or Tantivy, identifier boosting | Top-100 | Exact term matching, rare terms |
| **Symbol exact** | Hash map lookup | All matches | Instant known-symbol resolution |

**Score fusion via Reciprocal Rank Fusion (RRF)**: `score = Σ 1/(60 + rank_i)` combines rankings without calibration. Learned weights from click-through data can optimize for specific query distributions.

### 6.3 Re-ranking Stage

| Stage | Model | Candidates | Latency | Quality Gain |
|-------|-------|-----------|---------|--------------|
| **Cross-encoder** | MiniLM-L6 (22M params) | 20-50 | 15-25ms | **5-10% MRR improvement** |
| **LLM re-ranker** | Qwen2.5-1.5B or Phi-3-mini | 5-10 | 50-100ms | **10-15% on complex queries** |

**Conditional invocation**: cross-encoder always; LLM re-ranker when confidence threshold unmet or query complexity detected.

### 6.4 Latency Budget Allocation

| Stage | Target | Cumulative |
|-------|--------|------------|
| Query preprocessing + embedding | 10ms | 10ms |
| Initial retrieval (parallel vector + BM25 + symbol) | 20ms | 30ms |
| Score fusion + candidate fetch | 5ms | 35ms |
| Cross-encoder re-ranking | 25ms | 60ms |
| Result formatting | 5ms | 65ms |
| **Contingency (35%)** | 35ms | **<100ms P95** |

The **50%+ headroom** accommodates cache misses, slower queries, and concurrent load without violating target.

---

## 7. Code-Specific Challenges

### 7.1 Cross-File Dependencies

**Static call graph construction** via Tree-sitter resolves function calls to definitions, enabling **co-retrieval**: when function A matches, fetch its callers, callees, and type dependencies. **Import resolution** handles language-specific module systems (Python `import`, JavaScript `require`/`import`, Rust `use`). **Dependency-aware ranking** boosts well-connected components for architectural queries.

### 7.2 Symbol Disambiguation

Multiple strategies resolve "User class" ambiguity: **type-based resolution** from call site context; **frequency ranking** preferring most-referenced definitions; **user feedback learning** from click-through patterns; **namespace scoping** prioritizing file-local over project-global.

### 7.3 Multi-Language Support

**Unified embedding space** (Nomic Embed Code, Jina Code v2) enables **cross-language retrieval**: finding Python implementations from TypeScript queries. **Language-specific boosting** via query-time filters when context indicates target language. **Polyglot chunking** for mixed-language files (HTML/JS/CSS, SQL-in-Python) using Tree-sitter's embedded language support.

### 7.4 Generated Code Handling

**Detection heuristics**: large repetitive files, "DO NOT EDIT" comments, specific generator patterns. **Exclusion rules** via `.gitignore`-style patterns for `target/`, `node_modules/`, `*.gen.*`. **Downweighting** rather than full exclusion for potentially relevant generated code. **Source mapping** when generators provide provenance information.

---

## 8. Rust Implementation Plan

### 8.1 Core Crate Selection

| Component | Primary Crate | Version | Alternative | Rationale |
|-----------|-------------|---------|-------------|-----------|
| **Embedding inference** | `candle-core` | 0.8+ | `ort` 2.0+ | Native Rust, HF Hub integration  |
| **Vector search** | `qdrant-client` | 1.12+ | `lancedb` 0.10+ | Performance, embedded mode  |
| **Code parsing** | `tree-sitter` | 0.24+ | — | Incremental, 40+ languages  |
| **File watching** | `notify` | 6.0+ | — | Cross-platform, mature  |
| **Async runtime** | `tokio` | 1.35+ | — | Ecosystem standard |
| **Serialization** | `serde` | 1.0+ | — | Universal interoperability |

### 8.2 Architecture Components

```
┌─────────────────────────────────────────┐
│           OPENAKTA Indexing Core           │
├─────────────────────────────────────────┤
│  API Layer (gRPC/HTTP)                  │
│  ├── Query Engine (hybrid search)       │
│  ├── Index Controller (CRUD)            │
│  └── Admin (stats, health)              │
├─────────────────────────────────────────┤
│  Processing Pipeline                    │
│  ├── Chunker (Tree-sitter extraction)   │
│  ├── Embedder (Candle inference)        │
│  └── Indexer (Qdrant/SQLite writes)     │
├─────────────────────────────────────────┤
│  Storage Layer                          │
│  ├── Qdrant (vector embeddings)         │
│  ├── SQLite (metadata, call graph)      │
│  └── Cache (hot embeddings, results)    │
├─────────────────────────────────────────┤
│  Watchers & Events                      │
│  ├── notify (filesystem)                │
│  ├── git2 (repository)                  │
│  └── Debouncer (batching)               │
└─────────────────────────────────────────┘
```

### 8.3 Performance Optimizations

| Technique | Implementation | Impact |
|-----------|---------------|--------|
| **SIMD acceleration** | Candle AVX2/NEON kernels | 3-5× inference speedup |
| **Memory mapping** | Qdrant on-disk mode, mmap | Bounded RAM, large indexes |
| **Parallel indexing** | Rayon work-stealing | Multi-core chunk processing |
| **Caching** | LRU for embeddings, results | Sub-millisecond hot queries |

### 8.4 Phased Implementation

| Phase | Duration | Deliverable | Success Criteria |
|-------|----------|-------------|----------------|
| **1. Foundation** | 4 weeks | File-based indexing, flat search, basic API | Index 10K files, <500ms query |
| **2. Structure** | 3 weeks | Tree-sitter chunking, function extraction, metadata | 95%+ parse rate, <200ms query |
| **3. Scale** | 3 weeks | HNSW index, incremental updates, hybrid search | <100ms P95, <5s incremental |
| **4. Intelligence** | 4 weeks | Re-ranking, cross-file retrieval, optimization | >0.75 MRR@10, user satisfaction 4+ |

---

## 9. Validation and Benchmarking

### 9.1 Accuracy Metrics

| Metric | Target | Method |
|--------|--------|--------|
| **MRR@10 (CodeSearchNet)** | >0.75 | Standard evaluation on Python/JS/Go subset |
| **Recall@K (cross-file)** | >80% @ K=10 | Custom benchmark requiring multi-file context |
| **Human evaluation** | >4.0/5.0 | 50+ developers judging real query relevance |

### 9.2 Performance Benchmarks

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Indexing throughput** | >100 files/sec | Sustained on 100K LOC repository |
| **Query latency P95** | <100ms | Mixed workload, warm cache |
| **Memory steady-state** | <500MB | 100K chunks, typical usage |
| **Scalability** | Linear to 1M | <2× latency degradation |

### 9.3 Comparative Evaluation

| Baseline | Comparison | OPENAKTA Target |
|----------|-----------|--------------|
| `grep` + `ctags` | Traditional search | 10× relevance improvement |
| Cursor (behavioral) | Cloud-dependent competitor | Latency parity, offline superiority |
| Ablation studies | Component contribution | Validate each pipeline stage |

---

## Open Questions

- [ ] Exact MRR@10 for Nomic Embed Code on full CodeSearchNet (awaiting published benchmark)
- [ ] Optimal HNSW parameters (M, efConstruction, efSearch) for code embedding distribution
- [ ] User study quantifying perceived latency thresholds for code search
- [ ] Licensing review of BigCode OpenRAIL-M for StarCoder deployment scenarios

## Next Steps

1. **Prototype Phase 1** with file-based indexing and flat search to validate core pipeline
2. **Benchmark embedding models** on target hardware (representative developer laptop)
3. **Evaluate Qdrant vs. sqlite-vec** with realistic codebase workloads
4. **Design user study** for relevance evaluation methodology

