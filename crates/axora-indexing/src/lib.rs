//! AXORA Indexing
//!
//! Code indexing with Tree-sitter chunking, Merkle tree sync, and SCIP protocol.

#![warn(missing_docs)]

pub mod chunker;
pub mod error;
pub mod indexer;
pub mod influence;
pub mod merkle;
pub mod repository_map;
pub mod scip;
pub mod task_queue;
pub mod traceability;
pub mod vector_store;

pub use chunker::{BlockId, ChunkMetadata, ChunkType, Chunker, CodeChunk};
pub use error::IndexingError;
pub use indexer::IncrementalIndexer;
pub use influence::{FileId, InfluenceError, InfluenceGraph, InfluenceGraphStats, InfluenceVector};
pub use merkle::{BlockHashEntry, FileHashEntry, IndexDelta, MerkleTree};
pub use repository_map::{
    RepositoryMap, RepositoryMapError, RepositoryMapper, Symbol as RepoSymbol,
    SymbolKind as RepoSymbolKind,
};
pub use scip::{
    CodeParser, ExternalSymbol, Language, Occurrence, PackageInfo, ParserRegistry,
    RelationshipKind, SCIPError, SCIPIndex, Symbol, SymbolKind, SymbolRelationship,
};
pub use task_queue::{Task, TaskQueue, TaskQueueError, TaskQueueStats, TaskResult, TaskStatus};
pub use traceability::{
    BusinessRule, LinkType, TraceabilityError, TraceabilityLink, TraceabilityMatrix,
    TraceabilityStats,
};
pub use vector_store::VectorStore;

use thiserror::Error;

/// Indexing-related errors
#[derive(Error, Debug)]
pub enum AxoraIndexingError {
    /// Indexing error
    #[error("indexing error: {0}")]
    Indexing(#[from] IndexingError),

    /// SCIP error
    #[error("SCIP error: {0}")]
    SCIP(#[from] SCIPError),

    /// Influence error
    #[error("influence error: {0}")]
    Influence(#[from] InfluenceError),
}

/// Result type for indexing operations
pub type Result<T> = std::result::Result<T, AxoraIndexingError>;
