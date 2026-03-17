//! Incremental indexer

use crate::chunker::Chunker;
use crate::error::IndexingError;
use crate::merkle::MerkleTree;
use crate::vector_store::VectorStore;
use crate::Result;
use axora_embeddings::EmbeddingEngine;
use std::path::{Path, PathBuf};
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
    embedder: EmbeddingEngine,
    vector_store: VectorStore,
    merkle_tree: MerkleTree,
    root_path: PathBuf,
}

impl IncrementalIndexer {
    /// Create new incremental indexer
    pub async fn new(root_path: &Path, embedder: EmbeddingEngine) -> Result<Self> {
        info!("Creating indexer for {:?}", root_path);

        // Build or load Merkle tree
        let merkle_tree = MerkleTree::build(root_path)?;

        // Create vector store
        let vector_store = VectorStore::new("axora-codebase").await?;

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
            // TODO: Read file, chunk, embed, index
            stats.files_processed += 1;
        }

        info!("Indexing complete: {} chunks", stats.chunks_indexed);

        Ok(stats)
    }
}
