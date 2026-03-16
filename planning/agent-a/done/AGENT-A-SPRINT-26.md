# Agent A — Sprint 26: Semantic Memory Store

**Phase:** 2  
**Sprint:** 26 (Memory Architecture)  
**File:** `crates/axora-memory/src/semantic_store.rs`  
**Priority:** CRITICAL (foundation for tripartite memory)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Semantic Memory Store** using Vector Database for factual knowledge storage.

### Context

R-14 research provides CRITICAL implementation details:
- **Semantic Memory** — API contracts, schemas, architectural docs
- **Vector DB Storage** — Qdrant for high-dimensional embeddings
- **Top-K Similarity Retrieval** — Semantic search (not keyword)
- **Integration:** Living Docs (Sprint 6) → Semantic Vector Store

**Your job:** Implement semantic memory store (foundation for tripartite memory).

---

## 📋 Deliverables

### 1. Create semantic_store.rs

**File:** `crates/axora-memory/src/semantic_store.rs`

**Core Structure:**
```rust
//! Semantic Memory Store
//!
//! This module implements semantic memory storage:
//! - Vector Database (Qdrant) for high-dimensional embeddings
//! - Top-K similarity retrieval
//! - Integration with Living Docs

use qdrant_client::{
    qdrant::{PointStruct, SearchPoints, Value},
    Qdrant,
};

/// Semantic memory entity
#[derive(Debug, Clone)]
pub struct SemanticMemory {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: SemanticMetadata,
}

/// Semantic metadata
#[derive(Debug, Clone)]
pub struct SemanticMetadata {
    pub source: String, // Living Docs, API contract, etc.
    pub doc_type: DocType,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Document type
#[derive(Debug, Clone)]
pub enum DocType {
    ApiContract,
    DatabaseSchema,
    ArchitecturalDoc,
    CodingConvention,
    BusinessRule,
}

/// Semantic memory store
pub struct SemanticStore {
    client: Qdrant,
    collection_name: String,
}

impl SemanticStore {
    /// Create new semantic store
    pub async fn new(client: Qdrant, collection_name: &str) -> Result<Self> {
        // Create collection if not exists
        client.create_collection(collection_name).await?;
        
        Ok(Self {
            client,
            collection_name: collection_name.to_string(),
        })
    }
    
    /// Insert semantic memory
    pub async fn insert(&self, memory: SemanticMemory) -> Result<()> {
        let point = PointStruct::new(
            memory.id.clone(),
            memory.embedding,
            self.metadata_to_payload(&memory.metadata)?,
        );
        
        self.client
            .upsert_points(&self.collection_name, vec![point])
            .await?;
        
        Ok(())
    }
    
    /// Retrieve top-K similar memories
    pub async fn retrieve(&self, query_embedding: &[f32], k: usize) -> Result<Vec<SemanticMemory>> {
        let search_points = SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: query_embedding.to_vec(),
            limit: k as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        };
        
        let results = self.client.search_points(search_points).await?;
        
        // Convert results to SemanticMemory
        let memories = results
            .result
            .into_iter()
            .map(|scored| self.result_to_memory(scored))
            .collect::<Result<Vec<_>>>()?;
        
        Ok(memories)
    }
    
    /// Batch insert (for Living Docs integration)
    pub async fn insert_batch(&self, memories: Vec<SemanticMemory>) -> Result<()> {
        let points = memories
            .into_iter()
            .map(|m| {
                Ok(PointStruct::new(
                    m.id.clone(),
                    m.embedding,
                    self.metadata_to_payload(&m.metadata)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        
        self.client
            .upsert_points(&self.collection_name, points)
            .await?;
        
        Ok(())
    }
}
```

---

### 2. Integrate with Living Docs

**File:** `crates/axora-docs/src/living.rs` (UPDATE)

```rust
// Add to existing LivingDocs
impl LivingDocs {
    /// Sync updated docs to semantic memory store
    pub async fn sync_to_semantic_memory(
        &self,
        semantic_store: &SemanticStore,
    ) -> Result<()> {
        // Get all updated docs since last sync
        let updated_docs = self.get_updated_docs_since(self.last_sync_timestamp)?;
        
        // Convert to semantic memories
        let memories: Vec<SemanticMemory> = updated_docs
            .into_iter()
            .map(|doc| {
                SemanticMemory {
                    id: doc.id.clone(),
                    content: doc.content.clone(),
                    embedding: embed(doc.content), // Use embedding model
                    metadata: SemanticMetadata {
                        source: "living_docs".to_string(),
                        doc_type: DocType::ArchitecturalDoc,
                        created_at: doc.created_at,
                        updated_at: doc.updated_at,
                    },
                }
            })
            .collect();
        
        // Batch insert to semantic store
        semantic_store.insert_batch(memories).await?;
        
        // Update last sync timestamp
        self.last_sync_timestamp = Utc::now().timestamp() as u64;
        
        Ok(())
    }
}
```

---

### 3. Add Embedding Model Integration

**File:** `crates/axora-memory/src/semantic_store.rs` (add to existing)

```rust
/// Embedding model for semantic memory
pub struct EmbeddingModel {
    model: ONNXModel, // Or use external API
}

impl EmbeddingModel {
    pub fn new(model_path: &str) -> Result<Self> {
        let model = ONNXModel::load(model_path)?;
        Ok(Self { model })
    }
    
    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embedding = self.model.run(text)?;
        Ok(embedding)
    }
}

/// Helper function for embedding
pub fn embed(text: &str) -> Vec<f32> {
    // Use default embedding model
    // In production, inject via dependency injection
    let model = EmbeddingModel::new("models/embedding.onnx").unwrap();
    model.embed(text).unwrap()
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-memory/src/semantic_store.rs` (NEW)
- `crates/axora-memory/src/lib.rs` (NEW crate)

**Update:**
- `crates/axora-docs/src/living.rs` (integrate with semantic store)

**DO NOT Edit:**
- `crates/axora-agents/` (Agent C's domain)
- `crates/axora-indexing/` (Agent B's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_semantic_memory_insert() { }

#[test]
fn test_semantic_memory_retrieve() { }

#[test]
fn test_top_k_similarity() { }

#[test]
fn test_batch_insert() { }

#[test]
fn test_living_docs_sync() { }

#[test]
fn test_embedding_generation() { }

#[test]
fn test_metadata_payload_conversion() { }

#[test]
fn test_collection_creation() { }
```

---

## ✅ Success Criteria

- [ ] `semantic_store.rs` created (Vector DB integration)
- [ ] Qdrant client integration works
- [ ] Top-K similarity retrieval works
- [ ] Batch insert works (for Living Docs)
- [ ] Living Docs sync integration works
- [ ] Embedding model integration works
- [ ] 8+ tests passing
- [ ] Performance: <100ms for retrieval

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md) — Memory architecture
- Research document — R-14 Semantic Memory spec

---

**Start AFTER Sprint 25 (AGENTS.md Ledger) is complete.**

**Priority: CRITICAL — foundation for tripartite memory architecture.**

**Dependencies:**
- Sprint 25 (recommended but not required)
- Living Docs (Sprint 6) — must be complete

**Blocks:**
- Sprint 27 (Episodic Memory)
- Sprint 28 (Procedural Memory)
- Sprint 29 (Consolidation Pipeline)
