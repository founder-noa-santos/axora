//! RAG errors

use thiserror::Error;

/// RAG operation errors
#[derive(Error, Debug)]
pub enum RagError {
    /// Retrieval failed
    #[error("retrieval failed: {0}")]
    Retrieval(String),

    /// Re-ranking failed
    #[error("re-ranking failed: {0}")]
    Rerank(String),

    /// Context building failed
    #[error("context building failed: {0}")]
    ContextBuild(String),
}
