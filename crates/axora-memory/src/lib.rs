//! AXORA Memory System
//!
//! Tripartite memory architecture for AXORA agents:
//! - **Semantic Memory** — Factual knowledge (API contracts, schemas, docs)
//! - **Episodic Memory** — Experience logs (conversation history, debugging sessions)
//! - **Procedural Memory** — Skills and workflows (SKILL.md files)
//!
//! # Example
//!
//! ```rust,no_run
//! use axora_memory::{SemanticMemory, SemanticMetadata, DocType, InMemorySemanticStore};
//! use axora_memory::{Skill, SkillStep, ProceduralStore, SkillRepository};
//! use axora_memory::{MemoryLifecycle, LifecycleConfig, PruningWorker};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create in-memory semantic store
//! let semantic_store = InMemorySemanticStore::new(384);
//!
//! // Create procedural store
//! let temp_dir = std::env::temp_dir();
//! let proc_store = ProceduralStore::new(&temp_dir).await?;
//!
//! // Create lifecycle manager
//! let lifecycle = MemoryLifecycle::with_defaults();
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod lifecycle;
pub mod procedural_store;
pub mod semantic_store;

// Re-export main types
pub use lifecycle::{
    ConflictDetail, ConflictResolutionReport, ConflictType, EbbinghausDecay, LifecycleConfig,
    MemoryConflict, MemoryInfo, MemoryLifecycle, MemoryTrait, PruningReport, PruningWorker,
    TestMemory, UtilityTracker,
};
pub use procedural_store::{
    ProceduralError, ProceduralStore, Script, Skill, SkillMetadata, SkillOutcome,
    SkillRepository, SkillStep,
};
pub use semantic_store::{
    CollectionStats, DocType, InMemorySemanticStore, SearchResult, SemanticError, SemanticMemory,
    SemanticMetadata,
};
