//! Incremental code indexer for dense and sparse retrieval backends.

use crate::chunker::Chunker;
use crate::code_index::{CodeIndexDocument, TantivyCodeIndex};
use crate::error::IndexingError;
use crate::merkle::{IndexDelta, MerkleTree};
use crate::vector_store::DenseVectorCollection;
use crate::Result;
use openakta_embeddings::CodeEmbedder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Statistics from indexing.
#[derive(Debug, Default)]
pub struct IndexStats {
    pub chunks_indexed: usize,
    pub files_processed: usize,
    pub deleted_chunks: usize,
    pub indexing_time_secs: f64,
}

/// Incremental indexer that keeps dense and sparse code indexes aligned.
pub struct IncrementalIndexer {
    chunker: Chunker,
    embedder: Arc<dyn CodeEmbedder>,
    dense_store: Arc<dyn DenseVectorCollection>,
    sparse_index: Arc<TantivyCodeIndex>,
    root_path: PathBuf,
    state_path: PathBuf,
}

impl IncrementalIndexer {
    /// Create a new incremental indexer.
    pub fn new(
        root_path: impl AsRef<Path>,
        state_path: impl AsRef<Path>,
        embedder: Arc<dyn CodeEmbedder>,
        dense_store: Arc<dyn DenseVectorCollection>,
        sparse_index: Arc<TantivyCodeIndex>,
    ) -> Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        info!("Creating incremental indexer for {:?}", root_path);
        Ok(Self {
            chunker: Chunker::new()?,
            embedder,
            dense_store,
            sparse_index,
            root_path,
            state_path: state_path.as_ref().to_path_buf(),
        })
    }

    /// Index the codebase incrementally and persist the Merkle baseline.
    pub async fn index(&mut self) -> Result<IndexStats> {
        let started_at = std::time::Instant::now();
        info!("Starting incremental code indexing");

        let current_tree = MerkleTree::build(&self.root_path)?;
        let previous_tree = if self.state_path.exists() {
            MerkleTree::load_from_path(&self.state_path)?
        } else {
            MerkleTree::empty(&self.root_path)
        };
        let deltas = current_tree.diff(&previous_tree);

        let mut changed_files = HashSet::new();
        let mut stats = IndexStats::default();

        for delta in &deltas {
            match delta {
                IndexDelta::UpsertBlock(entry) => {
                    changed_files.insert(entry.file_path.clone());
                }
                IndexDelta::DeleteBlock {
                    block_id,
                    file_path,
                } => {
                    self.dense_store.delete(block_id).await?;
                    self.sparse_index.delete(block_id)?;
                    stats.deleted_chunks += 1;
                    changed_files.insert(file_path.clone());
                }
                IndexDelta::Noop { .. } => {}
            }
        }

        for file_path in changed_files {
            let absolute_path = self.root_path.join(&file_path);
            if !absolute_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&absolute_path)
                .map_err(|err| IndexingError::FileRead(err.to_string()))?;
            let language =
                Chunker::detect_language(&file_path).unwrap_or_else(|| "unknown".to_string());
            let chunks = self
                .chunker
                .extract_chunks(&content, &file_path, &language)
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
                let document = CodeIndexDocument {
                    chunk_id: chunk.id.clone(),
                    file_path: file_path.display().to_string(),
                    symbol_path: chunk.metadata.symbol_path.clone(),
                    summary: format!(
                        "{}:{}-{}",
                        file_path.display(),
                        chunk.line_range.0,
                        chunk.line_range.1
                    ),
                    body_markdown: chunk.content.clone(),
                    language: Some(language.clone()),
                    chunk_type: Some(format!("{:?}", chunk.chunk_type)),
                    start_line: chunk.line_range.0,
                    end_line: chunk.line_range.1,
                    token_cost: chunk.token_count,
                };
                self.dense_store
                    .upsert(
                        &document.chunk_id,
                        &embedding,
                        serde_json::json!({
                            "chunk_id": document.chunk_id,
                            "file_path": document.file_path,
                            "symbol_path": document.symbol_path,
                            "summary": document.summary,
                            "language": document.language,
                            "chunk_type": document.chunk_type,
                            "start_line": document.start_line,
                            "end_line": document.end_line,
                            "token_cost": document.token_cost,
                            "checksum": chunk.metadata.content_hash,
                        }),
                    )
                    .await?;
                self.sparse_index.upsert(&document)?;
                stats.chunks_indexed += 1;
            }
            stats.files_processed += 1;
        }

        current_tree.save_to_path(&self.state_path)?;
        stats.indexing_time_secs = started_at.elapsed().as_secs_f64();
        info!(
            "Incremental indexing complete: {} chunks across {} files ({} deletions)",
            stats.chunks_indexed, stats.files_processed, stats.deleted_chunks
        );
        Ok(stats)
    }
}
