//! Vector store using Qdrant

use crate::error::IndexingError;
use crate::Result;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, QueryPointsBuilder, SearchPointsBuilder, Value,
    VectorParamsBuilder,
};
use qdrant_client::Payload;
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Chunk ID
    pub id: String,
    /// Score
    pub score: f32,
    /// Payload metadata
    pub payload: serde_json::Value,
}

/// Vector store for embeddings
pub struct VectorStore {
    client: Qdrant,
    collection_name: String,
}

impl VectorStore {
    /// Create new vector store with Qdrant embedded
    pub async fn new(collection: &str) -> Result<Self> {
        info!("Creating vector store: {}", collection);

        // Connect to Qdrant (default: localhost:6334)
        // In production, configure for embedded mode
        let client = Qdrant::from_url("http://localhost:6334")
            .build()
            .map_err(|e| IndexingError::VectorStore(e.to_string()))?;

        let store = Self {
            client,
            collection_name: collection.to_string(),
        };

        // Create collection if not exists
        store.create_collection().await?;

        info!("Vector store ready: {}", collection);

        Ok(store)
    }

    /// Create collection with HNSW index
    async fn create_collection(&self) -> Result<()> {
        debug!("Creating collection: {}", self.collection_name);

        // Check if collection exists
        let exists = self
            .client
            .collection_exists(&self.collection_name)
            .await
            .map_err(|e| IndexingError::VectorStore(e.to_string()))?;

        if exists {
            debug!("Collection already exists");
            return Ok(());
        }

        // Create collection with HNSW configuration
        self.client
            .create_collection(
                CreateCollectionBuilder::new(self.collection_name.clone())
                    .vectors_config(VectorParamsBuilder::new(768, Distance::Cosine))
                    .hnsw_config(qdrant_client::qdrant::HnswConfigDiff {
                        m: Some(16),             // Connectivity
                        ef_construct: Some(128), // Build depth
                        ..Default::default()
                    }),
            )
            .await
            .map_err(|e| IndexingError::VectorStore(e.to_string()))?;

        info!("Collection created: {}", self.collection_name);

        Ok(())
    }

    /// Insert vector with payload
    pub async fn insert(
        &self,
        id: &str,
        vector: &[f32],
        payload: &serde_json::Value,
    ) -> Result<()> {
        debug!("Inserting vector: {}", id);

        // Convert payload to Qdrant format
        let qdrant_payload: Payload = payload
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), Value::from(v.to_string())))
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .unwrap_or_default()
            .into();

        // Create point
        let point = PointStruct::new(id.to_string(), vector.to_vec(), qdrant_payload);

        // Upsert point
        self.client
            .upsert_points(
                qdrant_client::qdrant::UpsertPointsBuilder::new(&self.collection_name, vec![point])
                    .wait(true),
            )
            .await
            .map_err(|e| IndexingError::VectorStore(e.to_string()))?;

        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        debug!("Searching with limit: {}", limit);

        // Search
        let result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection_name, query.to_vec(), limit as u64)
                    .with_payload(true),
            )
            .await
            .map_err(|e| IndexingError::VectorStore(e.to_string()))?;

        // Convert to SearchResult
        let results = result
            .result
            .into_iter()
            .map(|scored_point| {
                // Extract ID - use debug format as fallback
                let id_str = format!("{:?}", scored_point.id);

                SearchResult {
                    id: id_str,
                    score: scored_point.score,
                    payload: serde_json::Value::Object(
                        scored_point
                            .payload
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v.to_string())))
                            .collect(),
                    ),
                }
            })
            .collect();

        Ok(results)
    }

    /// Get collection name
    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_creation() {
        // Skip if Qdrant not running
        if std::env::var("QDRANT_URL").is_err() {
            println!("Qdrant not running, skipping test");
            return;
        }

        let store = VectorStore::new("test-collection").await;
        assert!(store.is_ok());
    }
}
