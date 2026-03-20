//! AXORA RAG
//!
//! RAG pipeline with hybrid retrieval.

#![warn(missing_docs)]

pub mod code_pipeline;
pub mod context;
pub mod error;
pub mod final_stage;
pub mod reranker;
pub mod retriever;

pub use code_pipeline::{CodeChunkDocument, CodeRetrievalPipeline, CodeRetrievalQuery, CodeRetrievalResult};
pub use context::ContextBuilder;
pub use error::RagError;
pub use final_stage::{
    AcceptedCandidate, BudgetedSelector, FusedCandidate, GaussianMemgasClassifier,
    KnapsackBudgetedSelector, MemgasClassifier, MemgasResult, RerankedCandidate,
    RetrievalDocument, SelectionResult, UnifiedFinalStage, UnifiedFinalStageResult,
};
pub use reranker::{CandleCrossEncoder, CrossEncoderScorer, RerankDocument};
pub use retriever::{FusedRank, RankedHit, ReciprocalRankFusion};

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
