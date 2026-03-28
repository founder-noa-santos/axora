//! OPENAKTA RAG
//!
//! RAG pipeline with hybrid retrieval.

pub mod code_pipeline;
pub mod context;
pub mod contract;
pub mod error;
pub mod final_stage;
pub mod reranker;
pub mod retriever;
pub mod structural_code;

pub use code_pipeline::{
    CodeChunkDocument, CodeRetrievalPipeline, CodeRetrievalQuery, CodeRetrievalResult,
};
pub use context::ContextBuilder;
pub use contract::{
    build_candidate_scores, build_channel_stats, stage_stat, RetrievalCandidateScore,
    RetrievalChannelScore, RetrievalChannelStat, RetrievalContract, RetrievalDiagnosticsData,
    RetrievalStageStat, CHUNK_SCHEMA_VERSION, DIAGNOSTICS_SCHEMA_VERSION, EMBEDDING_SCHEMA_VERSION,
    FUSION_POLICY_V1, PAYLOAD_SCHEMA_VERSION, RERANK_POLICY_V1, RETRIEVAL_CONTRACT_VERSION,
    SELECTION_POLICY_V1,
};
pub use error::RagError;
pub use final_stage::{
    AcceptedCandidate, BudgetedSelector, FusedCandidate, GaussianMemgasClassifier,
    KnapsackBudgetedSelector, MemgasClassifier, MemgasResult, RerankedCandidate, RetrievalDocument,
    SelectionResult, UnifiedFinalStage, UnifiedFinalStageResult,
};
pub use reranker::{
    CandleCrossEncoder, CrossEncoderScorer, HeuristicCrossEncoder, OpenaktaReranker, RerankDocument,
};
pub use retriever::{FusedRank, RankedHit, ReciprocalRankFusion};
pub use structural_code::{
    StructuralCodeRetrievalConfig, StructuralCodeRetrievalRequest, StructuralCodeRetrievalResult,
    StructuralCodeRetriever, StructuralHydratedDocument, StructuralRetrievalDiagnostic,
};

use thiserror::Error;

/// RAG-related errors
#[derive(Error, Debug)]
pub enum OpenaktaRagError {
    /// RAG error
    #[error("rag error: {0}")]
    Rag(#[from] RagError),
}

/// Result type for RAG operations
pub type Result<T> = std::result::Result<T, OpenaktaRagError>;
