//! OPENAKTA Cache
//!
//! Multi-tier caching (L1/L2/L3) and token optimization.

pub mod blackboard;
pub mod compactor;
pub mod concurrency;
pub mod context;
pub mod context_pruning;
pub mod diff;
pub mod l1_cache;
pub mod l2_cache;
pub mod l3_cache;
pub mod latent_context;
pub mod minifier;
pub mod prefix_cache;
pub mod rag;
pub mod toon;

pub use blackboard::v2::v2_pubsub::{PubSubHub, Subscriber, Subscription, SubscriptionId};
pub use blackboard::v2::v2_versioning::{
    Update as BlackboardUpdate, VersionedContext, VersionedContextError, VersionedValue,
};
pub use blackboard::v2::{BlackboardV2, BlackboardV2Error, Result as BlackboardV2Result};
pub use blackboard::{
    Blackboard, BlackboardError, BlackboardMetadata, BlackboardSchema, BlackboardSnapshot,
    ChangeType, ReflectionPhase, Result as BlackboardResult,
};
pub use compactor::hierarchical_memory::{
    HierarchicalContext, HierarchicalMemory, MemoryEntry, MemorySummary,
};
pub use compactor::importance_scorer::{ImportanceScorer, ItemKind, ScoredItem};
pub use compactor::rolling_summary::{RollingSummary, Turn};
pub use compactor::{
    CompactContext, CompactorConfig, CompactorError, Context, ContextCompactor, ContextEntry,
    Result as CompactorResult,
};
pub use concurrency::{
    BatchExecutor, ConcurrencyConfig, ConcurrencyError, ConcurrentExecutor, Task as ConcurrentTask,
    TaskResult, TokenCalculator,
};
pub use context::{
    Agent as RAGAgent, AgentState, CodeFile, ContextManager as RAGContextManager, ContextSavings,
    Document, SharedContext, Task as RAGTask, TaskContext as RAGTaskContext,
    TaskResult as RAGTaskResult,
};
pub use context_pruning::{
    Agent as PruningAgent, BusinessRule, ContextManager, Task as PruningTask, TaskContext,
    TokenReductionBenchmark, TraceabilityLink, TraceabilityMatrix,
};
pub use diff::{
    apply_patch, calculate_token_savings, parse_unified_diff, DiffLine, Hunk, PatchResult,
    TokenSavings, UnifiedDiff,
};
pub use l1_cache::L1Cache;
pub use l2_cache::L2Cache;
pub use l3_cache::L3Cache;
pub use latent_context::{LatentContextRecord, LatentContextStore};
pub use minifier::{CodeMinifier, MinifiedCode, MinifierConfig, MinifierError};
pub use prefix_cache::{
    CacheStats, CachedPrefix, CachedPromptBuilder, PrefixCache, PrefixCacheLookup,
};
pub use rag::{DomainRagStore, Experience, RagResult, RetrievalStrategy};
pub use toon::{Schema, ToonError, ToonSerializer, ToonStats};

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
