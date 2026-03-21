//! Embedding-related errors

use thiserror::Error;

/// Embedding operation errors
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Model loading failed
    #[error("failed to load model: {0}")]
    ModelLoad(String),

    /// Inference failed
    #[error("inference failed: {0}")]
    Inference(String),

    /// Invalid input
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Dimension mismatch
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected vector dimension for the loaded model
        expected: usize,
        /// Actual dimension produced by inference or input
        actual: usize,
    },
}
