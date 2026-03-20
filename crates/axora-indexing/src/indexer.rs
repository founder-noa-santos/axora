//! Incremental indexer

use crate::chunker::Chunker;
use crate::error::IndexingError;
use crate::merkle::MerkleTree;
use crate::vector_store::DenseVectorCollection;
use crate::Result;
use axora_embeddings::CodeEmbedder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Statistics from indexing
#[derive(Debug, Default)]
pub struct IndexStats {
    /// Number of chunks indexed
    pub chunks_indexed: usize,
    /// Number of files processed
    pub files_processed: usize,
    /// Indexing time in seconds
    pub indexing_time_secs: f64,
}

/// Incremental indexer
pub struct IncrementalIndexer {
    chunker: Chunker,
    embedder: Arc<dyn CodeEmbedder>,
    vector_store: Arc<dyn DenseVectorCollection>,
    merkle_tree: MerkleTree,
    root_path: PathBuf,
}

impl IncrementalIndexer {
    /// Create new incremental indexer
    pub async fn new(
        root_path: &Path,
        embedder: Arc<dyn CodeEmbedder>,
        vector_store: Arc<dyn DenseVectorCollection>,
    ) -> Result<Self> {
        info!("Creating indexer for {:?}", root_path);

        // Build or load Merkle tree
        let merkle_tree = MerkleTree::build(root_path)?;

        Ok(Self {
            chunker: Chunker::new()?,
            embedder,
            vector_store,
            merkle_tree,
            root_path: root_path.to_path_buf(),
        })
    }

    /// Index codebase incrementally
    pub async fn index(&mut self) -> Result<IndexStats> {
        info!("Starting incremental indexing");

        // Find changed files
        let changed_files = self
            .merkle_tree
            .find_changed(&MerkleTree::build(&self.root_path)?);
        info!("Found {} changed files", changed_files.len());

        let mut stats = IndexStats::default();

        // Process changed files
        for file_path in changed_files {
            let content = std::fs::read_to_string(self.root_path.join(&file_path))
                .map_err(|err| IndexingError::FileRead(err.to_string()))?;
            let language = Chunker::detect_language(&file_path)
                .unwrap_or_else(|| "unknown".to_string());
            let chunks = self
                .chunker
                .extract_chunks(&content, &self.root_path.join(&file_path), &language)
                .map_err(|err| IndexingError::ParseFailed(err.to_string()))?;
            let batch = chunks
                .iter()
                .map(|chunk| format!("{}\n{}", chunk.metadata.signature, chunk.content))
                .collect::<Vec<_>>();
            let embeddings = self
                .embedder
                .embed_batch(&batch)
                .await
                .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
            for (chunk, embedding) in chunks.into_iter().zip(embeddings.into_iter()) {
                let payload = serde_json::json!({
                    "chunk_id": chunk.id,
                    "file_path": file_path.display().to_string(),
                    "symbol_path": chunk.metadata.symbol_path,
                    "language": language,
                    "chunk_type": format!("{:?}", chunk.chunk_type),
                    "start_line": chunk.line_range.0,
                    "end_line": chunk.line_range.1,
                    "checksum": blake3::hash(chunk.content.as_bytes()).to_hex().to_string(),
                    "token_cost": chunk.content.len() / 4,
                });
                let chunk_id = payload["chunk_id"].as_str().unwrap_or_default().to_string();
                self.vector_store.upsert(&chunk_id, &embedding, payload).await?;
                stats.chunks_indexed += 1;
            }
            stats.files_processed += 1;
        }

        info!("Indexing complete: {} chunks", stats.chunks_indexed);

        Ok(stats)
    }
}
