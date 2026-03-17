//! AXORA RAG
//!
//! RAG pipeline with hybrid retrieval.

#![warn(missing_docs)]

pub mod context;
pub mod error;
pub mod reranker;
pub mod retriever;

pub use context::ContextBuilder;
pub use error::RagError;
pub use reranker::CrossEncoder;
pub use retriever::{HybridRetriever, RetrievalResult};

use thiserror::Error;

/// RAG-related errors
#[derive(Error, Debug)]
pub enum AxoraRagError {
    /// RAG error
    #[error("rag error: {0}")]
    Rag(#[from] RagError),
}

/// Result type for RAG operations
pub type Result<T> = std::result::Result<T, AxoraRagError>;
