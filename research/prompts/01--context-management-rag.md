# R-01: Context Management & RAG (Retrieval-Augmented Generation)

## Research Prompt

Copy and paste the following into Claude/GPT-4/Perplexity with web search enabled:

---

```
# Deep Research: Context Management & RAG for Multi-Agent AI Systems

## Context
I'm building AXORA, a multi-agent AI coding system that competes with Cursor, GitHub Copilot, and similar tools. We need SCIENTIFIC-LEVEL understanding of context management. This is not casual research - we need production-grade, SOTA knowledge.

## Core Research Questions

### 1. RAG Architecture Deep Dive
Research and explain with technical depth:

a) **Naive RAG vs Advanced RAG vs Modular RAG**
   - What are the specific architectural differences?
   - What papers define each approach?
   - What are the quantitative performance differences?
   - Which companies use which approach?

b) **Retrieval Strategies**
   - Dense retrieval vs sparse retrieval vs hybrid
   - What embedding models are SOTA for code retrieval in 2025-2026?
   - Compare: cosine similarity, dot product, learned similarity functions
   - What does "multi-vector retrieval" mean and when is it useful?

c) **Re-ranking Strategies**
   - What are SOTA re-ranking algorithms?
   - Compare: Cross-encoders, listwise ranking, LLM-as-reranker
   - What latency/accuracy tradeoffs exist?
   - What does Cohere/Cross-encoder/BGE-reranker offer?

### 2. Context Window Optimization

a) **Context Selection Algorithms**
   - Research: "Lost in the Middle" phenomenon - what's the solution?
   - What's the optimal context size for different LLM sizes?
   - How do we handle contexts larger than model's window?
   - Research sliding window, hierarchical attention, and retrieval strategies

b) **Hierarchical Context Organization**
   - How should code be chunked? (functions, classes, files, modules?)
   - What metadata should accompany each chunk?
   - How do we preserve code structure in embeddings?
   - Research: AST-based chunking vs line-based vs semantic

c) **Relevance Scoring**
   - What algorithms determine "relevance" for code?
   - BM25 for code - does it work?
   - Embedding similarity thresholds - what values?
   - How to combine lexical + semantic similarity?

### 3. Cursor-Specific Research

**CRITICAL:** Research how Cursor's indexing works:

a) Search for:
   - "Cursor IDE indexing architecture"
   - "Cursor codebase embedding local"
   - "Cursor vector database implementation"
   - Any patents, blog posts, or technical writing from Cursor team

b) Analyze:
   - Do they index locally or in cloud?
   - What vector database do they use (if known)?
   - How fast is their indexing?
   - How do they handle incremental updates?

c) Reverse engineer (from public info):
   - What's their likely architecture?
   - What embedding model would make sense?
   - How do they achieve sub-second retrieval?

### 4. Code-Specific Considerations

a) **Code is Different from Text**
   - Why is code retrieval harder than text retrieval?
   - What papers address code-specific embedding challenges?
   - How do we handle: imports, dependencies, type definitions?
   - Research: CodeBERT, GraphCodeBERT, UniXcoder, StarCoder embeddings

b) **Dependency-Aware Retrieval**
   - How do we retrieve not just similar code, but RELATED code?
   - Call graph-based retrieval
   - Import dependency traversal
   - Type-aware retrieval

c) **Temporal Aspects**
   - How do we handle code that changes frequently?
   - Version-aware embeddings?
   - How to invalidate stale embeddings?

### 5. Production Implementations

Research these specific systems:

a) **LangChain RAG**
   - What retrieval types do they support?
   - What's their ParentDocumentRetriever?
   - MultiQueryRetriever - how does it work?

b) **LlamaIndex**
   - Their vector store index architecture
   - Knowledge graphs + vectors
   - Sub-question query engine

c) **Enterprise Systems**
   - How does Sourcegraph Cody do retrieval?
   - What about GitHub's retrieval for Copilot?
   - Any public architecture docs from these companies?

### 6. Quantitative Benchmarks

Find and report:

a) **Standard Datasets**
   - What benchmarks exist for code retrieval?
   - CodeSearchNet - what are SOTA scores?
   - Any newer benchmarks (2024-2026)?

b) **Performance Metrics**
   - What's "good" retrieval accuracy? (MRR, NDCG, Recall@K)
   - What latency is acceptable for interactive use?
   - What's the token/cost impact of different strategies?

c) **Ablation Studies**
   - What components matter most?
   - Chunk size impact?
   - Embedding model impact?
   - Re-ranking impact?

## Required Output Format

### Section 1: Executive Summary
- 3-5 key findings that will guide our architecture
- Clear recommendations with confidence levels

### Section 2: Technical Deep Dive
- Detailed explanations with diagrams where helpful
- Cite specific papers with links
- Include code snippets for key algorithms

### Section 3: Competitive Analysis
- What Cursor likely does (with evidence)
- What competitors do differently
- Where we can differentiate

### Section 4: Implementation Recommendations
- Specific embedding models to use (with benchmarks)
- Specific vector databases to consider
- Specific chunking strategies
- Specific relevance scoring algorithms

### Section 5: Open Questions
- What we still don't know
- What requires experimentation
- What requires proprietary data

## Sources Required

Must include:
- At least 5 academic papers (with links)
- At least 3 industry blog posts from relevant companies
- At least 2 open-source implementation references
- Quantitative benchmarks where available

## Quality Bar

This research will directly inform our architecture. It must be:
- Technically precise (no hand-waving)
- Evidence-based (cite sources)
- Actionable (we should know what to implement)
- Honest about uncertainty (don't speculate without labeling it)

If you cannot find definitive answers, clearly state:
- What is unknown
- What requires experimentation
- What the search space is
```

---

## Follow-up Prompts

After receiving initial research, use these for deeper dives:

### Follow-up 1: Embedding Models
```
Based on the R-01 research, now deep-dive specifically into embedding models for code:

1. Compare these specific models with benchmarks:
   - CodeBERT
   - GraphCodeBERT
   - UniXcoder
   - StarCoder (embedding head)
   - BGE-code
   - Any 2025-2026 models

2. For each model, report:
   - Model size (parameters, MB)
   - Embedding dimension
   - Inference latency (on CPU and GPU)
   - MRR/Recall@K on CodeSearchNet
   - License (can we use commercially?)
   - Available implementations (HuggingFace, etc.)

3. Recommendation: Which model should we use for a local-first product?
```

### Follow-up 2: Vector Databases
```
Deep-dive into vector databases for local-first code retrieval:

1. Compare:
   - ChromaDB
   - Qdrant (local mode)
   - LanceDB
   - SQLite with vector extension
   - FAISS (raw)
   - ScaNN
   - HNSWlib

2. For each, report:
   - Memory footprint for 1M vectors (1024-dim)
   - Index build time
   - Query latency (p50, p95, p99)
   - Recall vs exact search
   - Disk persistence format
   - Language bindings (Rust specifically)
   - License

3. Recommendation: Which should we embed in our Rust application?
```

### Follow-up 3: Chunking Strategies
```
Deep-dive into optimal chunking strategies for code:

1. Research chunking approaches:
   - Line-based (fixed N lines)
   - Function-based (one chunk per function)
   - Class-based (one chunk per class)
   - AST-based (semantic nodes)
   - File-based with overlap
   - Hybrid approaches

2. For each, analyze:
   - Pros/cons for retrieval quality
   - Pros/cons for embedding quality
   - Impact on context assembly
   - Handling of cross-chunk dependencies
   - Implementation complexity

3. Find any papers that empirically compare chunking strategies

4. Recommendation: What chunking strategy for our use case?
```

---

## Findings Template

Save research findings in `research/findings/context-management/` using this template:

```markdown
# R-01 Findings: Context Management & RAG

**Research Date:** YYYY-MM-DD  
**Researcher:** [AI Model Used]  
**Sources:** [List of papers, articles, etc.]

## Key Findings

### Finding 1: [Title]
**Description:** ...
**Source:** [Link]
**Confidence:** High/Medium/Low
**Implication for AXORA:** ...

### Finding 2: [Title]
...

## Recommended Architecture

Based on this research, we recommend:

1. **Embedding Model:** [specific model]
2. **Vector Database:** [specific DB]
3. **Chunking Strategy:** [specific approach]
4. **Re-ranking:** [specific algorithm]

## Open Questions

- [ ] Question 1
- [ ] Question 2

## Next Steps

1. [Action item]
2. [Action item]
```

---

## Related Research

- [R-04: Local Indexing & Embedding](./04-local-indexing-embedding.md) - Overlaps with vector DB research
- [R-03: Token Efficiency](./03-token-efficiency-compression.md) - Context size optimization
- [R-07: Memory & State](./07-memory-state-management.md) - Long-term context storage
