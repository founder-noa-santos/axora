//! AXORA Cache
//!
//! Multi-tier caching (L1/L2/L3) and token optimization.

#![warn(missing_docs)]

pub mod concurrency;
pub mod context;
pub mod context_pruning;
pub mod diff;
pub mod l1_cache;
pub mod l2_cache;
pub mod l3_cache;
pub mod minifier;
pub mod prefix_cache;
pub mod rag;
pub mod toon;

pub use l1_cache::L1Cache;
pub use l2_cache::L2Cache;
pub use l3_cache::L3Cache;
pub use prefix_cache::{
    CachedPrefix, CachedPromptBuilder, CacheStats, PrefixCache,
};
pub use diff::{
    apply_patch, calculate_token_savings, DiffLine, Hunk, TokenSavings,
    UnifiedDiff, PatchResult,
};
pub use minifier::{
    CodeMinifier, MinifiedCode, MinifierConfig, MinifierError,
};
pub use toon::{
    Schema, ToonSerializer, ToonStats, ToonError,
};
pub use context::{
    ContextManager as RAGContextManager, TaskContext as RAGTaskContext, SharedContext, Task as RAGTask, Agent as RAGAgent, Document,
    CodeFile, TaskResult as RAGTaskResult, AgentState, ContextSavings,
};
pub use context_pruning::{
    ContextManager, Task as PruningTask, Agent as PruningAgent, TaskContext, BusinessRule,
    TraceabilityMatrix, TraceabilityLink, TokenReductionBenchmark,
};
pub use concurrency::{
    ConcurrentExecutor, ConcurrencyConfig, Task as ConcurrentTask, TaskResult,
    TokenCalculator, BatchExecutor, ConcurrencyError,
};
pub use rag::{DomainRagStore, Experience, RagResult, RetrievalStrategy};

use thiserror::Error;

/// Cache errors
#[derive(Error, Debug)]
pub enum CacheError {
    /// Cache miss
    #[error("cache miss")]
    Miss,

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Database error
    #[error("database error: {0}")]
    Database(String),
}

/// Result type for cache operations
pub type Result<T> = std::result::Result<T, CacheError>;
