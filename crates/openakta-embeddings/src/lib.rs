//! OPENAKTA Embeddings
//!
//! Domain-specific embedding inference for OPENAKTA retrieval.

pub mod config;
pub mod embedder;
pub mod error;
pub mod registry;
pub mod research_provider;
pub mod runtime_registry;

pub use config::{
    CodeEmbeddingConfig, DualEmbeddingConfig, EmbeddingDomain, EmbeddingProfile,
    FallbackEmbeddingConfig, FallbackPolicy, SkillEmbeddingConfig,
};
pub use embedder::{
    BgeSkillEmbedder, CodeEmbedder, EmbeddingModel, JinaCodeEmbedder, RemoteEmbeddingProvider,
    RemoteFallbackEmbedder, SkillEmbedder,
};
pub use error::EmbeddingError;
pub use registry::EmbeddingRegistry;
pub use research_provider::{
    DeterministicTestEmbeddingProvider, EmbeddingProvider, ResearchMinilmConfig,
    ResearchMinilmEmbedder, MAX_EMBED_TEXT_CHARS, RESEARCH_EMBED_BYTES, RESEARCH_EMBED_DIM,
};
pub use runtime_registry::{
    cache_size, get_or_load_runtime, CachedEmbeddingRuntime, ModelCacheKey,
};

use thiserror::Error;

/// Embedding-related errors
#[derive(Error, Debug)]
pub enum OpenaktaEmbeddingsError {
    /// Candle error
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),

    /// Embedding error
    #[error("embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
}

/// Result type for embedding operations
pub type Result<T> = std::result::Result<T, OpenaktaEmbeddingsError>;
