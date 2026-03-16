//! AXORA Indexing
//!
//! Code indexing with Tree-sitter chunking, Merkle tree sync, and SCIP protocol.

#![warn(missing_docs)]

pub mod chunker;
pub mod error;
pub mod influence;
pub mod indexer;
pub mod merkle;
pub mod repository_map;
pub mod scip;
pub mod task_queue;
pub mod traceability;
pub mod vector_store;

pub use chunker::{Chunker, CodeChunk};
pub use error::IndexingError;
pub use indexer::IncrementalIndexer;
pub use influence::{
    InfluenceGraph, InfluenceVector, InfluenceGraphStats, InfluenceError,
    FileId,
};
pub use merkle::MerkleTree;
pub use repository_map::{
    RepositoryMapper, RepositoryMap, Symbol as RepoSymbol, SymbolKind as RepoSymbolKind, RepositoryMapError,
};
pub use scip::{
    SCIPIndex, PackageInfo, Symbol, SymbolKind, Occurrence, ExternalSymbol,
    SymbolRelationship, RelationshipKind, SCIPError,
    CodeParser, ParserRegistry, Language,
};
pub use task_queue::{
    TaskQueue, Task, TaskStatus, TaskResult, TaskQueueStats, TaskQueueError,
};
pub use traceability::{
    TraceabilityMatrix, TraceabilityLink, TraceabilityStats, TraceabilityError,
    BusinessRule, LinkType,
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
