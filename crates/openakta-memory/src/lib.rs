//! OPENAKTA Memory System.
//!
//! Tripartite memory architecture for OPENAKTA agents:
//! - **Semantic Memory** — Factual knowledge (API contracts, schemas, docs)
//! - **Episodic Memory** — Experience logs (conversation history, debugging sessions)
//! - **Procedural Memory** — Pull-based `SKILL.md` retrieval

pub mod consolidation;
pub mod episodic_store;
pub mod lifecycle;
pub mod procedural_store;
pub mod semantic_store;
pub mod skill_seeder;
pub mod vector_backend;

// Re-export main types
pub use consolidation::{
    ConsolidationError, ConsolidationPipeline, ConsolidationTrigger, ConsolidationWorker,
    LightweightLLM, ObservationActionPair, TeacherVerifier, ValidationMode, ValidationReport,
};
pub use episodic_store::{
    EpisodicError, EpisodicMemory, EpisodicStore, EpisodicStoreConfig, MemoryType,
    MemoryType as EpisodicMemoryType, SessionStats,
};
pub use lifecycle::{
    ConflictDetail, ConflictResolutionReport, ConflictType, EbbinghausDecay, LifecycleConfig,
    MemoryConflict, MemoryInfo, MemoryLifecycle, MemoryTrait, PruningReport, PruningWorker,
    TestMemory, UtilityTracker,
};
pub use procedural_store::{
    AcceptedCandidate, BudgetedSkillSelector, FusedCandidate, GaussianMemgasClassifier,
    HybridSkillIndex, KnapsackBudgetedSkillSelector, MemgasClassifier, MemgasResult,
    ProceduralError, RerankedCandidate, Script, SelectionResult, Skill, SkillCatalog,
    SkillCorpusIngestor, SkillDocument, SkillIndexBackend, SkillMetadata, SkillOutcome,
    SkillRetrievalConfig, SkillRetrievalPipeline, SkillStep, SkillSyncSummary,
};
pub use semantic_store::{
    CollectionStats, DocType, InMemorySemanticStore, PersistentSemanticStore, SearchResult,
    SemanticError, SemanticMemory, SemanticMetadata,
};
pub use skill_seeder::{builtin_skill_root, SkillSeedReport};
pub use vector_backend::{
    PruneCandidate, SqliteLinearVectorStore, SqliteVecStore, VectorHit, VectorResult, VectorStore,
    VectorStoreError,
};
