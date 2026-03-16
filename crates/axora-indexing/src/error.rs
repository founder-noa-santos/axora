//! Indexing errors

use thiserror::Error;

/// Indexing operation errors
#[derive(Error, Debug)]
pub enum IndexingError {
    /// Parse failed
    #[error("parse failed: {0}")]
    ParseFailed(String),

    /// No query for language
    #[error("no query defined for language")]
    NoQuery,

    /// File read error
    #[error("file read error: {0}")]
    FileRead(String),

    /// Vector store error
    #[error("vector store error: {0}")]
    VectorStore(String),

    /// Merkle tree error
    #[error("merkle tree error: {0}")]
    MerkleTree(String),
}
