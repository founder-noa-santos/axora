//! AXORA Embeddings
//!
//! Embedding inference for code using Candle and Jina Code Embeddings v2.

#![warn(missing_docs)]

pub mod config;
pub mod embedder;
pub mod error;

pub use config::EmbeddingConfig;
pub use embedder::EmbeddingEngine;
pub use error::EmbeddingError;

use thiserror::Error;

/// Embedding-related errors
#[derive(Error, Debug)]
pub enum AxoraEmbeddingsError {
    /// Candle error
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),

    /// Embedding error
    #[error("embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
}

/// Result type for embedding operations
pub type Result<T> = std::result::Result<T, AxoraEmbeddingsError>;
