//! Pull-based procedural-memory retrieval for `SKILL.md` corpora.

use chrono::Utc;
use openakta_embeddings::{BgeSkillEmbedder, SkillEmbedder, SkillEmbeddingConfig};
use openakta_indexing::{
    CollectionSpec, DenseSkillHit, DenseVectorCollection, SkillDenseIndex, SkillIndexDocument,
    SparseSkillHit, TantivySkillIndex, VectorBackendKind,
};
use openakta_proto::mcp::v1::{
    CandidateScore, RetrievalDiagnostics, RetrieveSkillsRequest, RetrieveSkillsResponse,
    RetrievedSkill,
};
use openakta_rag::{
    CandleCrossEncoder, CrossEncoderScorer, MemgasResult as SharedMemgasResult, RankedHit,
    ReciprocalRankFusion, RetrievalDocument, SelectionResult as SharedSelectionResult,
    UnifiedFinalStage,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tiktoken_rs::cl100k_base;
use tokio::fs;
use tokio::sync::Mutex;

/// Procedural memory errors.
#[derive(Error, Debug)]
pub enum ProceduralError {
    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite error with path context.
    #[error("database error at {path}: {source}")]
    Database {
        /// Path to the database file.
        path: String,
        /// Underlying rusqlite error.
        #[source]
        source: rusqlite::Error,
    },

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Indexing error.
    #[error("indexing error: {0}")]
    Indexing(String),

    /// Retrieval error.
    #[error("retrieval error: {0}")]
    Retrieval(String),

    /// Invalid skill document.
    #[error("invalid skill document: {0}")]
    InvalidSkill(String),
}

/// Result type for procedural-memory operations.
pub type Result<T> = std::result::Result<T, ProceduralError>;

/// Helper to wrap rusqlite errors with path context
fn db_error(path: impl Into<String>, source: rusqlite::Error) -> ProceduralError {
    ProceduralError::Database {
        path: path.into(),
        source,
    }
}

impl From<rusqlite::Error> for ProceduralError {
    fn from(err: rusqlite::Error) -> Self {
        // For backward compatibility where path is not available
        ProceduralError::Database {
            path: "unknown".to_string(),
            source: err,
        }
    }
}

/// Skill outcome for utility tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillOutcome {
    /// Skill execution succeeded.
    Success,
    /// Skill execution failed.
    Failure,
}

/// Step in a procedural skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillStep {
    /// Step order.
    pub order: u32,
    /// Human-readable description.
    pub description: String,
    /// Optional command.
    pub command: Option<String>,
    /// Optional validation rule.
    pub validation: Option<String>,
}

/// Attached script block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Script {
    /// Script language.
    pub language: String,
    /// Script source.
    pub code: String,
}

/// Authoring metadata retained on skill creation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    /// Unique identifier.
    pub skill_id: String,
    /// Human-readable name.
    pub name: String,
    /// Domain category.
    pub domain: String,
    /// Retrieval summary.
    pub summary: String,
    /// Retrieval tags.
    pub tags: Vec<String>,
    /// Creation timestamp.
    pub created_at: u64,
    /// Update timestamp.
    pub updated_at: u64,
    /// Number of successful executions.
    pub success_count: u32,
    /// Number of failed executions.
    pub failure_count: u32,
    /// Utility score used as a prior.
    pub utility_score: f32,
}

impl SkillMetadata {
    /// Create new metadata.
    pub fn new(skill_id: &str, name: &str, domain: &str, tags: Vec<String>) -> Self {
        let now = current_unix_ts();
        Self {
            skill_id: skill_id.to_string(),
            name: name.to_string(),
            domain: domain.to_string(),
            summary: String::new(),
            tags,
            created_at: now,
            updated_at: now,
            success_count: 0,
            failure_count: 0,
            utility_score: 0.5,
        }
    }
}

/// Authored skill before canonicalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    /// Skill metadata.
    pub metadata: SkillMetadata,
    /// Ordered procedural steps.
    pub steps: Vec<SkillStep>,
    /// Optional scripts.
    pub scripts: Option<Vec<Script>>,
    /// Related skills.
    pub related_skills: Vec<String>,
    /// Raw markdown response, when available.
    pub raw_content: String,
}

impl Skill {
    /// Create a new authored skill.
    pub fn new(
        skill_id: &str,
        name: &str,
        domain: &str,
        tags: Vec<String>,
        steps: Vec<SkillStep>,
    ) -> Self {
        Self {
            metadata: SkillMetadata::new(skill_id, name, domain, tags),
            steps,
            scripts: None,
            related_skills: Vec::new(),
            raw_content: String::new(),
        }
    }

    /// Attach a script.
    pub fn with_script(mut self, language: &str, code: &str) -> Self {
        let script = Script {
            language: language.to_string(),
            code: code.to_string(),
        };
        self.scripts.get_or_insert_with(Vec::new).push(script);
        self
    }

    /// Attach a related skill id.
    pub fn with_related(mut self, skill_id: &str) -> Self {
        self.related_skills.push(skill_id.to_string());
        self
    }

    /// Convert the authored skill into a canonical document.
    pub fn to_document(&self, source_path: impl Into<String>) -> SkillDocument {
        let body_markdown = if self.raw_content.trim().is_empty() {
            render_skill_markdown(self)
        } else {
            self.raw_content.clone()
        };
        let summary = if self.metadata.summary.trim().is_empty() {
            self.steps
                .first()
                .map(|step| step.description.clone())
                .unwrap_or_else(|| self.metadata.name.clone())
        } else {
            self.metadata.summary.clone()
        };
        SkillDocument::new(
            self.metadata.skill_id.clone(),
            self.metadata.name.clone(),
            summary,
            body_markdown,
            source_path,
            self.metadata.domain.clone(),
            self.metadata.tags.clone(),
            self.metadata.updated_at,
        )
    }

    /// Render the skill as a canonical `SKILL.md` document with frontmatter.
    pub fn to_skill_markdown(&self) -> String {
        let frontmatter = serde_yaml::to_string(&serde_json::json!({
            "skill_id": &self.metadata.skill_id,
            "name": &self.metadata.name,
            "domain": &self.metadata.domain,
            "summary": &self.metadata.summary,
            "tags": &self.metadata.tags,
            "updated_at": self.metadata.updated_at,
        }))
        .unwrap_or_default();
        format!("---\n{}---\n\n{}", frontmatter, render_skill_markdown(self))
    }
}

/// Canonical procedural-memory record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDocument {
    /// Unique skill identifier.
    pub skill_id: String,
    /// Display title.
    pub title: String,
    /// Compact summary.
    pub summary: String,
    /// Markdown body without frontmatter.
    pub body_markdown: String,
    /// Source path on disk.
    pub source_path: String,
    /// Domain category.
    pub domain: String,
    /// Retrieval tags.
    pub tags: Vec<String>,
    /// Prompt token cost.
    pub token_cost: usize,
    /// Content checksum.
    pub checksum: String,
    /// Last update timestamp.
    pub updated_at: u64,
}

impl SkillDocument {
    /// Create a canonical skill document.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        skill_id: String,
        title: String,
        summary: String,
        body_markdown: String,
        source_path: impl Into<String>,
        domain: String,
        tags: Vec<String>,
        updated_at: u64,
    ) -> Self {
        let source_path = source_path.into();
        let checksum = blake3::hash(body_markdown.as_bytes()).to_hex().to_string();
        let token_cost = estimate_tokens(&format!("{title}\n{summary}\n{body_markdown}"));
        Self {
            skill_id,
            title,
            summary,
            body_markdown,
            source_path,
            domain,
            tags,
            token_cost,
            checksum,
            updated_at,
        }
    }

    fn as_index_document(&self) -> SkillIndexDocument {
        SkillIndexDocument {
            skill_id: self.skill_id.clone(),
            title: self.title.clone(),
            summary: self.summary.clone(),
            body_markdown: self.body_markdown.clone(),
            source_path: self.source_path.clone(),
            domain: self.domain.clone(),
            tags: self.tags.clone(),
        }
    }
}

impl RetrievalDocument for SkillDocument {
    fn id(&self) -> &str {
        &self.skill_id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn summary(&self) -> &str {
        &self.summary
    }

    fn body_markdown(&self) -> &str {
        &self.body_markdown
    }

    fn token_cost(&self) -> usize {
        self.token_cost
    }
}

/// SQLite-backed skill catalog.
#[derive(Debug, Clone)]
pub struct SkillCatalog {
    db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillSourceState {
    skill_id: String,
    source_path: String,
    checksum: String,
    modified_at_ns: u64,
    file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FastFileSnapshot {
    source_path: PathBuf,
    modified_at_ns: u64,
    file_size_bytes: u64,
}

/// Incremental sync summary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSyncSummary {
    /// Number of files metadata-checked.
    pub checked_files: usize,
    /// Number of changed or inserted skills.
    pub upserts: usize,
    /// Number of deleted skills.
    pub deletions: usize,
    /// Number of files skipped through the metadata fast path.
    pub fast_path_skips: usize,
}

impl SkillCatalog {
    /// Open or create the skill catalog.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let catalog = Self { db_path: path };
        catalog.ensure_schema()?;
        Ok(catalog)
    }

    fn ensure_schema(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS skill_documents (
                skill_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                body_markdown TEXT NOT NULL,
                source_path TEXT NOT NULL,
                domain TEXT NOT NULL,
                tags TEXT NOT NULL,
                token_cost INTEGER NOT NULL,
                checksum TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                indexed_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS skill_source_state (
                source_path TEXT PRIMARY KEY,
                skill_id TEXT NOT NULL,
                checksum TEXT NOT NULL,
                modified_at_ns INTEGER NOT NULL,
                file_size_bytes INTEGER NOT NULL
            );
            "#,
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        Ok(())
    }

    /// Upsert a canonical document.
    pub fn upsert_document(&self, document: &SkillDocument) -> Result<()> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute(
            r#"
            INSERT INTO skill_documents (
                skill_id, title, summary, body_markdown, source_path, domain,
                tags, token_cost, checksum, updated_at, indexed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
            ON CONFLICT(skill_id) DO UPDATE SET
                title = excluded.title,
                summary = excluded.summary,
                body_markdown = excluded.body_markdown,
                source_path = excluded.source_path,
                domain = excluded.domain,
                tags = excluded.tags,
                token_cost = excluded.token_cost,
                checksum = excluded.checksum,
                updated_at = excluded.updated_at,
                indexed_at = NULL
            "#,
            params![
                document.skill_id,
                document.title,
                document.summary,
                document.body_markdown,
                document.source_path,
                document.domain,
                serde_json::to_string(&document.tags)
                    .map_err(|err| ProceduralError::Serialization(err.to_string()))?,
                document.token_cost as i64,
                document.checksum,
                document.updated_at as i64,
            ],
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        Ok(())
    }

    /// Fetch a document by id.
    pub fn get_document(&self, skill_id: &str) -> Result<Option<SkillDocument>> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.query_row(
            r#"
            SELECT skill_id, title, summary, body_markdown, source_path, domain,
                   tags, token_cost, checksum, updated_at
            FROM skill_documents
            WHERE skill_id = ?1
            "#,
            [skill_id],
            map_row_to_document,
        )
        .optional()
        .map_err(|e| db_error(self.db_path.display().to_string(), e))
    }

    /// List all documents.
    pub fn list_documents(&self) -> Result<Vec<SkillDocument>> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        let mut stmt = conn
            .prepare(
                r#"
            SELECT skill_id, title, summary, body_markdown, source_path, domain,
                   tags, token_cost, checksum, updated_at
            FROM skill_documents
            ORDER BY skill_id
            "#,
            )
            .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        let rows = stmt
            .query_map([], map_row_to_document)
            .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        let mut documents = Vec::new();
        for row in rows {
            documents.push(row.map_err(|e| db_error(self.db_path.display().to_string(), e))?);
        }
        Ok(documents)
    }

    /// Delete a document.
    pub fn delete_document(&self, skill_id: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute(
            "DELETE FROM skill_documents WHERE skill_id = ?1",
            [skill_id],
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute(
            "DELETE FROM skill_source_state WHERE skill_id = ?1",
            [skill_id],
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        Ok(())
    }

    /// Mark a document as indexed.
    pub fn mark_indexed(&self, skill_id: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute(
            "UPDATE skill_documents SET indexed_at = ?2 WHERE skill_id = ?1",
            params![skill_id, current_unix_ts() as i64],
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        Ok(())
    }

    fn upsert_source_state(&self, state: &SkillSourceState) -> Result<()> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        conn.execute(
            r#"
            INSERT INTO skill_source_state (
                source_path, skill_id, checksum, modified_at_ns, file_size_bytes
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(source_path) DO UPDATE SET
                skill_id = excluded.skill_id,
                checksum = excluded.checksum,
                modified_at_ns = excluded.modified_at_ns,
                file_size_bytes = excluded.file_size_bytes
            "#,
            params![
                state.source_path,
                state.skill_id,
                state.checksum,
                state.modified_at_ns as i64,
                state.file_size_bytes as i64,
            ],
        )
        .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        Ok(())
    }

    fn list_source_states(&self) -> Result<Vec<SkillSourceState>> {
        let conn = Connection::open(&self.db_path).map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        let mut stmt = conn
            .prepare(
                r#"
            SELECT skill_id, source_path, checksum, modified_at_ns, file_size_bytes
            FROM skill_source_state
            ORDER BY source_path
            "#,
            )
            .map_err(|e| db_error(self.db_path.display().to_string(), e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SkillSourceState {
                skill_id: row.get(0)?,
                source_path: row.get(1)?,
                checksum: row.get(2)?,
                modified_at_ns: row.get::<_, i64>(3)? as u64,
                file_size_bytes: row.get::<_, i64>(4)? as u64,
            })
        })?;
        let mut states = Vec::new();
        for row in rows {
            states.push(row?);
        }
        Ok(states)
    }
}

/// Scans `SKILL.md` sources and canonicalizes them.
#[derive(Debug, Clone)]
pub struct SkillCorpusIngestor {
    root: PathBuf,
}

impl SkillCorpusIngestor {
    /// Create a new corpus ingestor.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Synchronize the corpus into the catalog.
    pub async fn sync(&self, catalog: &SkillCatalog) -> Result<Vec<SkillDocument>> {
        let mut documents = Vec::new();
        for snapshot in self.discover_skill_files().await? {
            let (document, state) = self.parse_snapshot(&snapshot).await?;
            catalog.upsert_document(&document)?;
            catalog.upsert_source_state(&state)?;
            documents.push(document);
        }
        Ok(documents)
    }

    async fn discover_skill_files(&self) -> Result<Vec<FastFileSnapshot>> {
        let mut snapshots = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(path) = stack.pop() {
            let Ok(mut entries) = fs::read_dir(&path).await else {
                continue;
            };
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    stack.push(entry_path);
                    continue;
                }
                if entry_path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
                    continue;
                }
                let metadata = entry.metadata().await?;
                let modified_at_ns = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_nanos() as u64)
                    .unwrap_or_default();
                snapshots.push(FastFileSnapshot {
                    source_path: entry_path,
                    modified_at_ns,
                    file_size_bytes: metadata.len(),
                });
            }
        }
        snapshots.sort_by(|left, right| left.source_path.cmp(&right.source_path));
        Ok(snapshots)
    }

    async fn parse_snapshot(
        &self,
        snapshot: &FastFileSnapshot,
    ) -> Result<(SkillDocument, SkillSourceState)> {
        let content = fs::read_to_string(&snapshot.source_path).await?;
        let checksum = blake3::hash(content.as_bytes()).to_hex().to_string();
        let mut document = self.parse_content(&snapshot.source_path, &content)?;
        document.checksum = checksum.clone();
        let state = SkillSourceState {
            skill_id: document.skill_id.clone(),
            source_path: snapshot.source_path.to_string_lossy().to_string(),
            checksum,
            modified_at_ns: snapshot.modified_at_ns,
            file_size_bytes: snapshot.file_size_bytes,
        };
        Ok((document, state))
    }

    fn parse_content(&self, path: &Path, content: &str) -> Result<SkillDocument> {
        let (frontmatter, body_markdown) = split_frontmatter(content)?;
        let title = frontmatter
            .get("name")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| first_heading(&body_markdown))
            .unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
        let skill_id = frontmatter
            .get("skill_id")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| title.to_uppercase().replace([' ', '-'], "_"));
        let domain = frontmatter
            .get("domain")
            .and_then(|value| value.as_str())
            .unwrap_or("general")
            .to_string();
        let summary = frontmatter
            .get("summary")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| first_paragraph(&body_markdown))
            .unwrap_or_else(|| title.clone());
        let tags = extract_tags(&frontmatter);
        let metadata_updated_at = frontmatter
            .get("updated_at")
            .and_then(|value| value.as_u64())
            .unwrap_or_else(current_unix_ts);
        let mut document = SkillDocument::new(
            skill_id,
            title,
            summary,
            body_markdown,
            path.to_string_lossy().to_string(),
            domain,
            tags,
            metadata_updated_at,
        );
        document.checksum = blake3::hash(content.as_bytes()).to_hex().to_string();
        Ok(document)
    }
}

/// Hybrid skill index configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRetrievalConfig {
    /// Root directory containing authoring `SKILL.md` files.
    pub corpus_root: PathBuf,
    /// SQLite catalog path.
    pub catalog_db_path: PathBuf,
    /// Dense backend selection.
    pub dense_backend: VectorBackendKind,
    /// SQLite dense-store path for local fallback/backends.
    pub dense_store_path: PathBuf,
    /// Qdrant endpoint.
    pub qdrant_url: String,
    /// Dense skill collection spec.
    pub dense_collection: CollectionSpec,
    /// Skill embedder configuration.
    pub embedding: SkillEmbeddingConfig,
    /// BM25 index directory.
    pub bm25_dir: PathBuf,
    /// Default token budget for returned skills.
    pub skill_token_budget: usize,
    /// Dense candidate limit.
    pub dense_limit: usize,
    /// Sparse candidate limit.
    pub bm25_limit: usize,
}

impl Default for SkillRetrievalConfig {
    fn default() -> Self {
        let runtime_root = PathBuf::from(".openakta");
        Self {
            corpus_root: runtime_root.join("skills"),
            catalog_db_path: runtime_root.join("skill-catalog.db"),
            dense_backend: VectorBackendKind::Qdrant,
            dense_store_path: runtime_root.join("skill-vectors.db"),
            qdrant_url: "http://127.0.0.1:6334".to_string(),
            dense_collection: CollectionSpec::skill_default(),
            embedding: SkillEmbeddingConfig::default(),
            bm25_dir: runtime_root.join("skill-bm25"),
            skill_token_budget: 1500,
            dense_limit: 64,
            bm25_limit: 64,
        }
    }
}

async fn build_skill_dense_collection(
    config: &SkillRetrievalConfig,
) -> Result<Arc<dyn DenseVectorCollection>> {
    match config.dense_backend {
        VectorBackendKind::Qdrant => Ok(Arc::new(
            openakta_indexing::QdrantVectorCollection::new(
                &config.qdrant_url,
                config.dense_collection.clone(),
            )
            .await
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?,
        )),
        VectorBackendKind::SqliteJson => Ok(Arc::new(
            openakta_indexing::SqliteJsonVectorCollection::new(
                &config.dense_store_path,
                config.dense_collection.clone(),
            )
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?,
        )),
    }
}

/// Dense + BM25 orchestrator.
pub struct HybridSkillIndex {
    dense: SkillDenseIndex,
    sparse: TantivySkillIndex,
    fusion: ReciprocalRankFusion,
}

impl HybridSkillIndex {
    /// Create a new hybrid index.
    pub async fn new(config: &SkillRetrievalConfig) -> Result<Self> {
        let dense_collection = build_skill_dense_collection(config).await?;
        let sparse = TantivySkillIndex::new(&config.bm25_dir)
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        let embedder = Arc::new(
            BgeSkillEmbedder::new(config.embedding.clone())
                .map_err(|err| ProceduralError::Indexing(err.to_string()))?,
        );
        Self::with_components(dense_collection, embedder, sparse)
    }

    /// Create a hybrid index from injected components.
    pub fn with_components(
        dense_collection: Arc<dyn DenseVectorCollection>,
        embedder: Arc<dyn SkillEmbedder>,
        sparse: TantivySkillIndex,
    ) -> Result<Self> {
        let dense = SkillDenseIndex::new(dense_collection, embedder)
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        Ok(Self {
            dense,
            sparse,
            fusion: ReciprocalRankFusion::default(),
        })
    }

    /// Upsert a document in both indexes.
    pub async fn upsert_document(&self, document: &SkillDocument) -> Result<()> {
        self.dense
            .upsert(&document.as_index_document())
            .await
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        self.sparse
            .upsert(&document.as_index_document())
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        Ok(())
    }

    /// Upsert all catalog documents.
    pub async fn sync_catalog(&self, catalog: &SkillCatalog) -> Result<()> {
        for document in catalog.list_documents()? {
            self.upsert_document(&document).await?;
            catalog.mark_indexed(&document.skill_id)?;
        }
        Ok(())
    }

    /// Search both indexes and fuse the results.
    pub async fn search(
        &self,
        catalog: &SkillCatalog,
        query: &str,
        dense_limit: usize,
        bm25_limit: usize,
    ) -> Result<Vec<FusedCandidate>> {
        let dense_hits = self
            .dense
            .search(query, dense_limit)
            .await
            .map_err(|err| ProceduralError::Retrieval(err.to_string()))?;
        let sparse_hits = self
            .sparse
            .search(query, bm25_limit)
            .map_err(|err| ProceduralError::Retrieval(err.to_string()))?;

        let fused = self.fusion.fuse(&[
            dense_hits_to_ranked(&dense_hits, "dense"),
            sparse_hits_to_ranked(&sparse_hits, "bm25"),
        ]);

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for rank in fused.into_iter().take(96) {
            if !seen.insert(rank.document_id.clone()) {
                continue;
            }
            if let Some(document) = catalog.get_document(&rank.document_id)? {
                let dense = dense_hits
                    .iter()
                    .find(|hit| hit.skill_id == rank.document_id)
                    .cloned();
                let sparse = sparse_hits
                    .iter()
                    .find(|hit| hit.skill_id == rank.document_id)
                    .cloned();
                candidates.push(FusedCandidate {
                    skill: document,
                    rrf_score: rank.score,
                    dense_rank: dense.as_ref().map(|hit| hit.rank),
                    dense_score: dense.as_ref().map(|hit| hit.score),
                    bm25_rank: sparse.as_ref().map(|hit| hit.rank),
                    bm25_score: sparse.as_ref().map(|hit| hit.score),
                });
            }
        }
        Ok(candidates)
    }

    /// Delete a document from both indexes.
    pub async fn delete_document(&self, skill_id: &str) -> Result<()> {
        self.dense
            .delete(skill_id)
            .await
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        self.sparse
            .delete(skill_id)
            .map_err(|err| ProceduralError::Indexing(err.to_string()))?;
        Ok(())
    }
}

/// Storage backend used by the retrieval pipeline.
#[async_trait::async_trait]
pub trait SkillIndexBackend: Send + Sync {
    /// Upsert a document into the retrieval indexes.
    async fn upsert_document(&self, document: &SkillDocument) -> Result<()>;
    /// Delete a document from the retrieval indexes.
    async fn delete_document(&self, skill_id: &str) -> Result<()>;
    /// Search the retrieval indexes and return fused candidates.
    async fn search(
        &self,
        catalog: &SkillCatalog,
        query: &str,
        dense_limit: usize,
        bm25_limit: usize,
    ) -> Result<Vec<FusedCandidate>>;
}

#[async_trait::async_trait]
impl SkillIndexBackend for HybridSkillIndex {
    async fn upsert_document(&self, document: &SkillDocument) -> Result<()> {
        HybridSkillIndex::upsert_document(self, document).await
    }

    async fn delete_document(&self, skill_id: &str) -> Result<()> {
        HybridSkillIndex::delete_document(self, skill_id).await
    }

    async fn search(
        &self,
        catalog: &SkillCatalog,
        query: &str,
        dense_limit: usize,
        bm25_limit: usize,
    ) -> Result<Vec<FusedCandidate>> {
        HybridSkillIndex::search(self, catalog, query, dense_limit, bm25_limit).await
    }
}

struct SkillCorpusSynchronizer<I> {
    ingestor: SkillCorpusIngestor,
    catalog: SkillCatalog,
    index: Arc<I>,
    sync_lock: Mutex<()>,
}

impl<I> SkillCorpusSynchronizer<I>
where
    I: SkillIndexBackend,
{
    fn new(ingestor: SkillCorpusIngestor, catalog: SkillCatalog, index: Arc<I>) -> Self {
        Self {
            ingestor,
            catalog,
            index,
            sync_lock: Mutex::new(()),
        }
    }

    async fn sync_if_needed(&self) -> Result<SkillSyncSummary> {
        let _guard = self.sync_lock.lock().await;
        let mut summary = SkillSyncSummary::default();
        let snapshots = self.ingestor.discover_skill_files().await?;
        summary.checked_files = snapshots.len();

        let existing = self
            .catalog
            .list_source_states()?
            .into_iter()
            .map(|state| (state.source_path.clone(), state))
            .collect::<HashMap<_, _>>();
        let mut seen_paths = HashSet::new();

        for snapshot in snapshots {
            let source_path = snapshot.source_path.to_string_lossy().to_string();
            seen_paths.insert(source_path.clone());
            if let Some(state) = existing.get(&source_path) {
                if state.modified_at_ns == snapshot.modified_at_ns
                    && state.file_size_bytes == snapshot.file_size_bytes
                {
                    summary.fast_path_skips += 1;
                    continue;
                }
            }

            let (document, source_state) = self.ingestor.parse_snapshot(&snapshot).await?;
            if existing
                .get(&source_path)
                .map(|state| state.checksum == source_state.checksum)
                .unwrap_or(false)
            {
                self.catalog.upsert_source_state(&source_state)?;
                summary.fast_path_skips += 1;
                continue;
            }

            self.catalog.upsert_document(&document)?;
            self.catalog.upsert_source_state(&source_state)?;
            self.index.upsert_document(&document).await?;
            self.catalog.mark_indexed(&document.skill_id)?;
            summary.upserts += 1;
        }

        for (source_path, state) in existing {
            if seen_paths.contains(&source_path) {
                continue;
            }
            self.catalog.delete_document(&state.skill_id)?;
            self.index.delete_document(&state.skill_id).await?;
            summary.deletions += 1;
        }

        Ok(summary)
    }
}

/// Candidate after dense/BM25 fusion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusedCandidate {
    /// Canonical document.
    pub skill: SkillDocument,
    /// Reciprocal rank fusion score.
    pub rrf_score: f32,
    /// Dense rank, if present.
    pub dense_rank: Option<u32>,
    /// Dense score, if present.
    pub dense_score: Option<f32>,
    /// BM25 rank, if present.
    pub bm25_rank: Option<u32>,
    /// BM25 score, if present.
    pub bm25_score: Option<f32>,
}

/// Candidate accepted by MemGAS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptedCandidate {
    /// Base fused candidate.
    pub candidate: FusedCandidate,
    /// Posterior probability of belonging to the high-mean component.
    pub accept_posterior: f32,
}

/// Candidate after cross-encoder scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankedCandidate {
    /// Base accepted candidate.
    pub accepted: AcceptedCandidate,
    /// Local reranker score.
    pub cross_score: f32,
    /// Prompt token cost.
    pub token_cost: usize,
}

/// Selection result from the knapsack stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectionResult {
    /// Chosen skills.
    pub selected_skills: Vec<RerankedCandidate>,
    /// Rejected only because of the token budget.
    pub discarded_by_budget: Vec<RerankedCandidate>,
    /// Tokens used.
    pub used_tokens: usize,
    /// Objective value.
    pub objective_score: f32,
}

/// Result from the MemGAS classifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemgasResult {
    /// Accepted candidates.
    pub accept_set: Vec<AcceptedCandidate>,
    /// Rejected candidates.
    pub reject_set: Vec<AcceptedCandidate>,
    /// Gaussian means.
    pub component_means: [f32; 2],
    /// Gaussian variances.
    pub component_variances: [f32; 2],
    /// Whether EM converged.
    pub converged: bool,
    /// Whether a degenerate fallback path was used.
    pub degenerate: bool,
}

/// MemGAS classifier contract.
pub trait MemgasClassifier {
    /// Split fused candidates into accept/reject sets.
    fn classify(&self, candidates: &[FusedCandidate]) -> MemgasResult;
}

/// Default two-component GMM classifier over RRF scores.
#[derive(Debug, Clone)]
pub struct GaussianMemgasClassifier {
    /// Maximum EM iterations.
    pub max_iterations: usize,
    /// Convergence epsilon.
    pub epsilon: f32,
    /// Variance floor.
    pub variance_floor: f32,
}

impl Default for GaussianMemgasClassifier {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            epsilon: 1e-4,
            variance_floor: 1e-6,
        }
    }
}

impl MemgasClassifier for GaussianMemgasClassifier {
    fn classify(&self, candidates: &[FusedCandidate]) -> MemgasResult {
        if candidates.len() < 3 {
            let accepted = candidates
                .iter()
                .cloned()
                .map(|candidate| AcceptedCandidate {
                    candidate,
                    accept_posterior: 1.0,
                })
                .collect::<Vec<_>>();
            return MemgasResult {
                accept_set: accepted,
                reject_set: Vec::new(),
                component_means: [0.0, 0.0],
                component_variances: [1.0, 1.0],
                converged: false,
                degenerate: true,
            };
        }

        let standardized = standardize_scores(candidates);
        let mut means = initial_quantiles(&standardized);
        let mut variances = [1.0f32, 1.0f32];
        let mut priors = [0.5f32, 0.5f32];
        let mut converged = false;

        for _ in 0..self.max_iterations {
            let responsibilities = standardized
                .iter()
                .map(|score| posterior(*score, means, variances, priors))
                .collect::<Vec<_>>();

            let previous_means = means;
            for component in 0..2 {
                let weight_sum = responsibilities
                    .iter()
                    .map(|resp| resp[component])
                    .sum::<f32>();
                if weight_sum <= self.variance_floor {
                    continue;
                }
                means[component] = responsibilities
                    .iter()
                    .zip(standardized.iter())
                    .map(|(resp, score)| resp[component] * score)
                    .sum::<f32>()
                    / weight_sum;
                variances[component] = (responsibilities
                    .iter()
                    .zip(standardized.iter())
                    .map(|(resp, score)| resp[component] * (score - means[component]).powi(2))
                    .sum::<f32>()
                    / weight_sum)
                    .max(self.variance_floor);
                priors[component] =
                    (weight_sum / standardized.len() as f32).max(self.variance_floor);
            }

            if (means[0] - previous_means[0]).abs() < self.epsilon
                && (means[1] - previous_means[1]).abs() < self.epsilon
            {
                converged = true;
                break;
            }
        }

        if (means[0] - means[1]).abs() < self.epsilon {
            let accepted = candidates
                .iter()
                .cloned()
                .map(|candidate| AcceptedCandidate {
                    candidate,
                    accept_posterior: 1.0,
                })
                .collect::<Vec<_>>();
            return MemgasResult {
                accept_set: accepted,
                reject_set: Vec::new(),
                component_means: means,
                component_variances: variances,
                converged,
                degenerate: true,
            };
        }

        let accept_component = if means[0] >= means[1] { 0 } else { 1 };
        let mut accept_set = Vec::new();
        let mut reject_set = Vec::new();
        for (candidate, score) in candidates.iter().cloned().zip(standardized.iter().copied()) {
            let resp = posterior(score, means, variances, priors);
            let accepted = AcceptedCandidate {
                candidate,
                accept_posterior: resp[accept_component],
            };
            if accepted.accept_posterior >= 0.5 {
                accept_set.push(accepted);
            } else {
                reject_set.push(accepted);
            }
        }

        MemgasResult {
            accept_set,
            reject_set,
            component_means: means,
            component_variances: variances,
            converged,
            degenerate: false,
        }
    }
}

/// Budgeted selection contract.
pub trait BudgetedSkillSelector {
    /// Pick the optimal subset under the given budget.
    fn select(&self, items: &[RerankedCandidate], budget_tokens: usize) -> SelectionResult;
}

/// Exact 0/1 knapsack selector.
#[derive(Debug, Default, Clone)]
pub struct KnapsackBudgetedSkillSelector;

impl BudgetedSkillSelector for KnapsackBudgetedSkillSelector {
    fn select(&self, items: &[RerankedCandidate], budget_tokens: usize) -> SelectionResult {
        let mut dp = vec![vec![0.0f32; budget_tokens + 1]; items.len() + 1];
        let mut keep = vec![vec![false; budget_tokens + 1]; items.len() + 1];

        for (index, item) in items.iter().enumerate() {
            let weight = item.token_cost.min(budget_tokens + 1);
            for budget in 0..=budget_tokens {
                let skip = dp[index][budget];
                let take = if weight <= budget {
                    dp[index][budget - weight] + item.cross_score
                } else {
                    f32::NEG_INFINITY
                };
                if take > skip {
                    dp[index + 1][budget] = take;
                    keep[index + 1][budget] = true;
                } else {
                    dp[index + 1][budget] = skip;
                }
            }
        }

        let mut selected = Vec::new();
        let mut budget = budget_tokens;
        for index in (1..=items.len()).rev() {
            if keep[index][budget] {
                let item = items[index - 1].clone();
                budget = budget.saturating_sub(item.token_cost);
                selected.push(item);
            }
        }
        selected.reverse();

        let selected_ids = selected
            .iter()
            .map(|item| item.accepted.candidate.skill.skill_id.clone())
            .collect::<HashSet<_>>();
        let discarded_by_budget = items
            .iter()
            .filter(|item| !selected_ids.contains(&item.accepted.candidate.skill.skill_id))
            .cloned()
            .collect::<Vec<_>>();

        SelectionResult {
            used_tokens: selected.iter().map(|item| item.token_cost).sum(),
            objective_score: selected.iter().map(|item| item.cross_score).sum(),
            selected_skills: selected,
            discarded_by_budget,
        }
    }
}

/// End-to-end retrieval pipeline.
pub struct SkillRetrievalPipeline<I = HybridSkillIndex, R = CandleCrossEncoder> {
    config: SkillRetrievalConfig,
    catalog: SkillCatalog,
    synchronizer: SkillCorpusSynchronizer<I>,
    index: Arc<I>,
    final_stage: UnifiedFinalStage<R>,
}

impl SkillRetrievalPipeline<HybridSkillIndex, CandleCrossEncoder> {
    /// Create a new retrieval pipeline.
    pub async fn new(config: SkillRetrievalConfig) -> Result<Self> {
        let catalog = SkillCatalog::new(&config.catalog_db_path)?;
        let ingestor = SkillCorpusIngestor::new(&config.corpus_root);
        let index = Arc::new(HybridSkillIndex::new(&config).await?);
        let reranker =
            CandleCrossEncoder::new().map_err(|err| ProceduralError::Retrieval(err.to_string()))?;
        Self::with_components(config, catalog, ingestor, index, reranker)
    }
}

impl<I, R> SkillRetrievalPipeline<I, R>
where
    I: SkillIndexBackend,
    R: CrossEncoderScorer,
{
    /// Create a new retrieval pipeline with injected index and reranker implementations.
    pub fn with_components(
        config: SkillRetrievalConfig,
        catalog: SkillCatalog,
        ingestor: SkillCorpusIngestor,
        index: Arc<I>,
        reranker: R,
    ) -> Result<Self> {
        let synchronizer = SkillCorpusSynchronizer::new(ingestor, catalog.clone(), index.clone());
        Ok(Self {
            config,
            catalog,
            synchronizer,
            index,
            final_stage: UnifiedFinalStage::new(reranker),
        })
    }

    /// Synchronize only the changed corpus deltas.
    pub async fn sync_if_needed(&self) -> Result<SkillSyncSummary> {
        self.synchronizer.sync_if_needed().await
    }

    /// Execute the full retrieval pipeline.
    pub async fn retrieve(
        &self,
        request: &RetrieveSkillsRequest,
    ) -> Result<RetrieveSkillsResponse> {
        self.sync_if_needed().await?;
        let dense_limit = if request.dense_limit == 0 {
            self.config.dense_limit
        } else {
            request.dense_limit as usize
        };
        let bm25_limit = if request.bm25_limit == 0 {
            self.config.bm25_limit
        } else {
            request.bm25_limit as usize
        };
        let budget = if request.skill_token_budget == 0 {
            self.config.skill_token_budget
        } else {
            request.skill_token_budget as usize
        };

        let fused_candidates = self
            .index
            .search(&self.catalog, &request.query, dense_limit, bm25_limit)
            .await?;
        let shared_candidates = fused_candidates
            .iter()
            .cloned()
            .map(|candidate| openakta_rag::FusedCandidate {
                document: candidate.skill,
                rrf_score: candidate.rrf_score,
                dense_rank: candidate.dense_rank,
                dense_score: candidate.dense_score,
                bm25_rank: candidate.bm25_rank,
                bm25_score: candidate.bm25_score,
            })
            .collect::<Vec<_>>();
        let final_result = self
            .final_stage
            .run(&request.query, &shared_candidates, budget)
            .await
            .map_err(|err| ProceduralError::Retrieval(err.to_string()))?;
        let memgas = final_result.memgas;
        let selection = final_result.selection;

        Ok(RetrieveSkillsResponse {
            request_id: request.request_id.clone(),
            skills: selection
                .selected_documents
                .iter()
                .map(|candidate| RetrievedSkill {
                    skill_id: candidate.accepted.candidate.document.skill_id.clone(),
                    title: candidate.accepted.candidate.document.title.clone(),
                    source_path: candidate.accepted.candidate.document.source_path.clone(),
                    content: format!(
                        "# {}\n\n{}\n\n{}",
                        candidate.accepted.candidate.document.title,
                        candidate.accepted.candidate.document.summary,
                        candidate.accepted.candidate.document.body_markdown
                    ),
                    token_cost: candidate.token_cost as u32,
                    rrf_score: candidate.accepted.candidate.rrf_score,
                    accept_posterior: candidate.accepted.accept_posterior,
                    cross_score: candidate.cross_score,
                })
                .collect(),
            diagnostics: if request.include_diagnostics {
                Some(RetrievalDiagnostics {
                    dense_hits: fused_candidates
                        .iter()
                        .filter(|item| item.dense_rank.is_some())
                        .count() as u32,
                    bm25_hits: fused_candidates
                        .iter()
                        .filter(|item| item.bm25_rank.is_some())
                        .count() as u32,
                    fused_candidates: fused_candidates.len() as u32,
                    accept_count: memgas.accept_set.len() as u32,
                    reject_count: memgas.reject_set.len() as u32,
                    selected_count: selection.selected_documents.len() as u32,
                    used_tokens: selection.used_tokens as u32,
                    memgas_converged: memgas.converged,
                    memgas_degenerate: memgas.degenerate,
                    scores: build_candidate_scores(&fused_candidates, &memgas, &selection),
                    generated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                })
            } else {
                None
            },
        })
    }

    /// Expose the catalog for runtime wiring.
    pub fn catalog(&self) -> &SkillCatalog {
        &self.catalog
    }
}

fn build_candidate_scores(
    fused: &[FusedCandidate],
    memgas: &SharedMemgasResult<SkillDocument>,
    selection: &SharedSelectionResult<SkillDocument>,
) -> Vec<CandidateScore> {
    let selected_ids = selection
        .selected_documents
        .iter()
        .map(|item| item.accepted.candidate.document.skill_id.clone())
        .collect::<HashSet<_>>();
    let accepted_scores = memgas
        .accept_set
        .iter()
        .map(|item| {
            (
                item.candidate.document.skill_id.clone(),
                item.accept_posterior,
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let rerank_scores = selection
        .selected_documents
        .iter()
        .chain(selection.discarded_by_budget.iter())
        .map(|item| {
            (
                item.accepted.candidate.document.skill_id.clone(),
                (item.cross_score, item.token_cost),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    fused
        .iter()
        .map(|candidate| {
            let (cross_score, token_cost) = rerank_scores
                .get(&candidate.skill.skill_id)
                .copied()
                .unwrap_or((0.0, candidate.skill.token_cost));
            CandidateScore {
                skill_id: candidate.skill.skill_id.clone(),
                dense_rank: candidate.dense_rank.unwrap_or_default(),
                dense_score: candidate.dense_score.unwrap_or_default(),
                bm25_rank: candidate.bm25_rank.unwrap_or_default(),
                bm25_score: candidate.bm25_score.unwrap_or_default(),
                rrf_score: candidate.rrf_score,
                accept_posterior: accepted_scores
                    .get(&candidate.skill.skill_id)
                    .copied()
                    .unwrap_or(0.0),
                cross_score,
                token_cost: token_cost as u32,
                selected: selected_ids.contains(&candidate.skill.skill_id),
            }
        })
        .collect()
}

fn map_row_to_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillDocument> {
    let tags: String = row.get(6)?;
    Ok(SkillDocument {
        skill_id: row.get(0)?,
        title: row.get(1)?,
        summary: row.get(2)?,
        body_markdown: row.get(3)?,
        source_path: row.get(4)?,
        domain: row.get(5)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        token_cost: row.get::<_, i64>(7)? as usize,
        checksum: row.get(8)?,
        updated_at: row.get::<_, i64>(9)? as u64,
    })
}

fn render_skill_markdown(skill: &Skill) -> String {
    let mut markdown = format!(
        "# {}\n\n{}\n\n",
        skill.metadata.name, skill.metadata.summary
    );
    for step in &skill.steps {
        markdown.push_str(&format!("## Step {}\n{}\n\n", step.order, step.description));
        if let Some(command) = &step.command {
            markdown.push_str("```bash\n");
            markdown.push_str(command);
            markdown.push_str("\n```\n\n");
        }
    }
    if let Some(scripts) = &skill.scripts {
        markdown.push_str("## Scripts\n\n");
        for script in scripts {
            markdown.push_str(&format!("```{}\n{}\n```\n\n", script.language, script.code));
        }
    }
    markdown
}

fn split_frontmatter(content: &str) -> Result<(serde_json::Value, String)> {
    if !content.starts_with("---\n") {
        return Ok((
            serde_json::Value::Object(Default::default()),
            content.to_string(),
        ));
    }

    let Some(rest) = content.strip_prefix("---\n") else {
        return Ok((
            serde_json::Value::Object(Default::default()),
            content.to_string(),
        ));
    };
    let Some((yaml, body)) = rest.split_once("\n---\n") else {
        return Err(ProceduralError::InvalidSkill(
            "missing closing frontmatter delimiter".to_string(),
        ));
    };
    let value = serde_yaml::from_str::<serde_yaml::Value>(yaml)
        .map_err(|err| ProceduralError::Serialization(err.to_string()))?;
    let json = serde_json::to_value(value)
        .map_err(|err| ProceduralError::Serialization(err.to_string()))?;
    Ok((json, body.to_string()))
}

fn first_heading(content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
}

fn first_paragraph(content: &str) -> Option<String> {
    let mut paragraph = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph)
    }
}

fn extract_tags(frontmatter: &serde_json::Value) -> Vec<String> {
    let mut tags = Vec::new();
    for key in ["tags", "triggers"] {
        if let Some(values) = frontmatter.get(key).and_then(|value| value.as_array()) {
            for value in values {
                if let Some(tag) = value.as_str() {
                    let normalized = tag.trim().to_lowercase();
                    if !normalized.is_empty() && !tags.contains(&normalized) {
                        tags.push(normalized);
                    }
                }
            }
        }
    }
    tags
}

fn estimate_tokens(content: &str) -> usize {
    cl100k_base()
        .ok()
        .map(|bpe| bpe.encode_with_special_tokens(content).len())
        .unwrap_or_else(|| content.len() / 4)
}

fn current_unix_ts() -> u64 {
    Utc::now().timestamp().max(0) as u64
}

fn dense_hits_to_ranked(hits: &[DenseSkillHit], source: &str) -> Vec<RankedHit> {
    hits.iter()
        .map(|hit| RankedHit {
            document_id: hit.skill_id.clone(),
            rank: hit.rank,
            score: hit.score,
            source: source.to_string(),
        })
        .collect()
}

fn sparse_hits_to_ranked(hits: &[SparseSkillHit], source: &str) -> Vec<RankedHit> {
    hits.iter()
        .map(|hit| RankedHit {
            document_id: hit.skill_id.clone(),
            rank: hit.rank,
            score: hit.score,
            source: source.to_string(),
        })
        .collect()
}

fn standardize_scores(candidates: &[FusedCandidate]) -> Vec<f32> {
    let scores = candidates
        .iter()
        .map(|candidate| candidate.rrf_score)
        .collect::<Vec<_>>();
    let mean = scores.iter().sum::<f32>() / scores.len() as f32;
    let variance = scores
        .iter()
        .map(|score| (score - mean).powi(2))
        .sum::<f32>()
        / scores.len() as f32;
    let stddev = variance.sqrt().max(1e-6);
    scores
        .into_iter()
        .map(|score| (score - mean) / stddev)
        .collect()
}

fn initial_quantiles(values: &[f32]) -> [f32; 2] {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let q25 = sorted[((sorted.len() as f32 * 0.25).floor() as usize).min(sorted.len() - 1)];
    let q75 = sorted[((sorted.len() as f32 * 0.75).floor() as usize).min(sorted.len() - 1)];
    [q25, q75]
}

fn posterior(score: f32, means: [f32; 2], variances: [f32; 2], priors: [f32; 2]) -> [f32; 2] {
    let mut probs = [0.0f32; 2];
    for component in 0..2 {
        let variance = variances[component].max(1e-6);
        let coefficient = 1.0 / (2.0 * std::f32::consts::PI * variance).sqrt();
        let exponent = -((score - means[component]).powi(2)) / (2.0 * variance);
        probs[component] = priors[component] * coefficient * exponent.exp();
    }
    let total = probs[0] + probs[1];
    if total <= 1e-6 {
        [0.5, 0.5]
    } else {
        [probs[0] / total, probs[1] / total]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_rag::{CrossEncoderScorer, RerankDocument};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    #[derive(Default)]
    struct MockIndexBackend {
        upserts: AtomicUsize,
        deletions: AtomicUsize,
        results: Mutex<Vec<FusedCandidate>>,
    }

    impl MockIndexBackend {
        fn with_results(results: Vec<FusedCandidate>) -> Self {
            Self {
                upserts: AtomicUsize::new(0),
                deletions: AtomicUsize::new(0),
                results: Mutex::new(results),
            }
        }
    }

    #[async_trait::async_trait]
    impl SkillIndexBackend for MockIndexBackend {
        async fn upsert_document(&self, _document: &SkillDocument) -> Result<()> {
            self.upserts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn delete_document(&self, _skill_id: &str) -> Result<()> {
            self.deletions.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn search(
            &self,
            _catalog: &SkillCatalog,
            _query: &str,
            _dense_limit: usize,
            _bm25_limit: usize,
        ) -> Result<Vec<FusedCandidate>> {
            Ok(self.results.lock().await.clone())
        }
    }

    #[derive(Clone)]
    struct MockCrossEncoder {
        scores: Vec<f32>,
    }

    #[async_trait::async_trait]
    impl CrossEncoderScorer for MockCrossEncoder {
        async fn score_pairs(
            &self,
            _query: &str,
            docs: &[RerankDocument],
        ) -> openakta_rag::Result<Vec<f32>> {
            Ok(self.scores.iter().copied().take(docs.len()).collect())
        }
    }

    #[test]
    fn skill_document_uses_frontmatter_without_exposing_it() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let skill_dir = temp_dir.path().join("debug-auth");
            fs::create_dir_all(&skill_dir).await.unwrap();
            let skill_path = skill_dir.join("SKILL.md");
            fs::write(
                &skill_path,
                "---\nskill_id: DEBUG_AUTH\nname: Debug Auth\ndomain: security\ntriggers:\n  - JWT\nsummary: Inspect auth failures\n---\n# Debug Auth\n\nCheck headers first.\n",
            )
            .await
            .unwrap();

            let catalog = SkillCatalog::new(temp_dir.path().join("skills.db")).unwrap();
            let ingestor = SkillCorpusIngestor::new(temp_dir.path());
            let docs = ingestor.sync(&catalog).await.unwrap();

            assert_eq!(docs.len(), 1);
            assert!(!docs[0].body_markdown.contains("skill_id:"));
            assert!(docs[0].tags.contains(&"jwt".to_string()));
        });
    }

    #[test]
    fn memgas_marks_small_inputs_degenerate() {
        let classifier = GaussianMemgasClassifier::default();
        let candidates = vec![
            FusedCandidate {
                skill: SkillDocument::new(
                    "a".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                    "Body".to_string(),
                    "/tmp/a".to_string(),
                    "general".to_string(),
                    vec![],
                    current_unix_ts(),
                ),
                rrf_score: 0.1,
                dense_rank: Some(1),
                dense_score: Some(0.9),
                bm25_rank: None,
                bm25_score: None,
            },
            FusedCandidate {
                skill: SkillDocument::new(
                    "b".to_string(),
                    "B".to_string(),
                    "B".to_string(),
                    "Body".to_string(),
                    "/tmp/b".to_string(),
                    "general".to_string(),
                    vec![],
                    current_unix_ts(),
                ),
                rrf_score: 0.09,
                dense_rank: Some(2),
                dense_score: Some(0.8),
                bm25_rank: None,
                bm25_score: None,
            },
        ];

        let result = classifier.classify(&candidates);
        assert!(result.degenerate);
        assert_eq!(result.accept_set.len(), 2);
    }

    #[test]
    fn knapsack_obeys_budget() {
        let selector = KnapsackBudgetedSkillSelector;
        let items = vec![
            RerankedCandidate {
                accepted: AcceptedCandidate {
                    candidate: FusedCandidate {
                        skill: SkillDocument::new(
                            "a".to_string(),
                            "A".to_string(),
                            "A".to_string(),
                            "Body".to_string(),
                            "/tmp/a".to_string(),
                            "general".to_string(),
                            vec![],
                            current_unix_ts(),
                        ),
                        rrf_score: 0.3,
                        dense_rank: Some(1),
                        dense_score: Some(0.9),
                        bm25_rank: Some(1),
                        bm25_score: Some(4.0),
                    },
                    accept_posterior: 0.9,
                },
                cross_score: 0.8,
                token_cost: 900,
            },
            RerankedCandidate {
                accepted: AcceptedCandidate {
                    candidate: FusedCandidate {
                        skill: SkillDocument::new(
                            "b".to_string(),
                            "B".to_string(),
                            "B".to_string(),
                            "Body".to_string(),
                            "/tmp/b".to_string(),
                            "general".to_string(),
                            vec![],
                            current_unix_ts(),
                        ),
                        rrf_score: 0.2,
                        dense_rank: Some(2),
                        dense_score: Some(0.7),
                        bm25_rank: Some(2),
                        bm25_score: Some(3.0),
                    },
                    accept_posterior: 0.8,
                },
                cross_score: 0.7,
                token_cost: 700,
            },
        ];

        let selection = selector.select(&items, 1000);
        assert_eq!(selection.selected_skills.len(), 1);
        assert!(selection.used_tokens <= 1000);
    }

    #[test]
    fn incremental_sync_only_indexes_changed_files() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let skill_root = temp_dir.path().join("skills");
            std::fs::create_dir_all(skill_root.join("auth")).unwrap();
            let skill_path = skill_root.join("auth").join("SKILL.md");
            std::fs::write(
                &skill_path,
                "---\nskill_id: DEBUG_AUTH\nname: Debug Auth\ndomain: security\ntags: [jwt]\nsummary: Inspect auth failures\n---\n# Debug Auth\n\nCheck headers first.\n",
            )
            .unwrap();

            let catalog = SkillCatalog::new(temp_dir.path().join("skills.db")).unwrap();
            let ingestor = SkillCorpusIngestor::new(&skill_root);
            let index = Arc::new(MockIndexBackend::default());
            let synchronizer = SkillCorpusSynchronizer::new(ingestor, catalog, index.clone());

            let first = synchronizer.sync_if_needed().await.unwrap();
            let second = synchronizer.sync_if_needed().await.unwrap();

            assert_eq!(first.upserts, 1);
            assert_eq!(second.upserts, 0);
            assert_eq!(second.fast_path_skips, 1);
            assert_eq!(index.upserts.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn pipeline_respects_token_budget_and_rejects_noise() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let skill_root = temp_dir.path().join("skills");
            std::fs::create_dir_all(skill_root.join("auth")).unwrap();
            std::fs::create_dir_all(skill_root.join("noise")).unwrap();
            std::fs::write(
                skill_root.join("auth").join("SKILL.md"),
                "---\nskill_id: AUTH_DEBUG\nname: Auth Debug\ndomain: security\ntags: [jwt, auth]\nsummary: Debug auth failures\n---\n# Auth Debug\n\nInspect JWT headers and issuer claims.\n",
            )
            .unwrap();
            std::fs::write(
                skill_root.join("noise").join("SKILL.md"),
                "---\nskill_id: CSS_THEME\nname: CSS Theme\ndomain: frontend\ntags: [css]\nsummary: Change colors\n---\n# CSS Theme\n\nAdjust button colors.\n",
            )
            .unwrap();

            let auth_skill = SkillDocument::new(
                "AUTH_DEBUG".to_string(),
                "Auth Debug".to_string(),
                "Debug auth failures".to_string(),
                "Inspect JWT headers and issuer claims.".to_string(),
                skill_root.join("auth").join("SKILL.md").display().to_string(),
                "security".to_string(),
                vec!["jwt".to_string(), "auth".to_string()],
                current_unix_ts(),
            );
            let noise_skill = SkillDocument::new(
                "CSS_THEME".to_string(),
                "CSS Theme".to_string(),
                "Change colors".to_string(),
                "Adjust button colors.".to_string(),
                skill_root.join("noise").join("SKILL.md").display().to_string(),
                "frontend".to_string(),
                vec!["css".to_string()],
                current_unix_ts(),
            );
            let fallback_skill = SkillDocument::new(
                "AUTH_CHECKLIST".to_string(),
                "Auth Checklist".to_string(),
                "Review auth config".to_string(),
                "Review issuer configuration, clock skew, and audience values before rotating keys. Document every mismatch and compare against production settings.".to_string(),
                skill_root.join("fallback").join("SKILL.md").display().to_string(),
                "security".to_string(),
                vec!["auth".to_string(), "config".to_string()],
                current_unix_ts(),
            );

            let index = Arc::new(MockIndexBackend::with_results(vec![
                FusedCandidate {
                    skill: auth_skill.clone(),
                    rrf_score: 0.80,
                    dense_rank: Some(1),
                    dense_score: Some(0.95),
                    bm25_rank: Some(1),
                    bm25_score: Some(9.8),
                },
                FusedCandidate {
                    skill: noise_skill.clone(),
                    rrf_score: 0.01,
                    dense_rank: Some(32),
                    dense_score: Some(0.05),
                    bm25_rank: Some(28),
                    bm25_score: Some(0.1),
                },
                FusedCandidate {
                    skill: fallback_skill.clone(),
                    rrf_score: 0.42,
                    dense_rank: Some(6),
                    dense_score: Some(0.55),
                    bm25_rank: Some(8),
                    bm25_score: Some(1.4),
                },
            ]));

            let catalog = SkillCatalog::new(temp_dir.path().join("catalog.db")).unwrap();
            let ingestor = SkillCorpusIngestor::new(&skill_root);
            let pipeline = SkillRetrievalPipeline::with_components(
                SkillRetrievalConfig {
                    corpus_root: skill_root.clone(),
                    catalog_db_path: temp_dir.path().join("catalog.db"),
                    dense_backend: VectorBackendKind::SqliteJson,
                    dense_store_path: temp_dir.path().join("skill-vectors.db"),
                    qdrant_url: "http://127.0.0.1:6334".to_string(),
                    dense_collection: CollectionSpec {
                        name: "test".to_string(),
                        ..CollectionSpec::skill_default()
                    },
                    embedding: SkillEmbeddingConfig::default(),
                    bm25_dir: temp_dir.path().join("bm25"),
                    skill_token_budget: auth_skill.token_cost,
                    dense_limit: 64,
                    bm25_limit: 64,
                },
                catalog,
                ingestor,
                index,
                MockCrossEncoder { scores: vec![1.0, 0.2] },
            )
            .unwrap();

            let response = pipeline
                .retrieve(&RetrieveSkillsRequest {
                    request_id: "req-1".to_string(),
                    agent_id: "agent-1".to_string(),
                    role: "coder".to_string(),
                    task_id: "task-1".to_string(),
                    workspace_root: temp_dir.path().display().to_string(),
                    query: "debug JWT auth failures".to_string(),
                    focal_files: vec![],
                    focal_symbols: vec![],
                    skill_token_budget: auth_skill.token_cost as u32,
                    dense_limit: 64,
                    bm25_limit: 64,
                    include_diagnostics: true,
                })
                .await
                .unwrap();

            assert_eq!(response.skills.len(), 1);
            assert_eq!(response.skills[0].skill_id, "AUTH_DEBUG");
            assert!(response.skills[0].token_cost as usize <= auth_skill.token_cost);
            let diagnostics = response.diagnostics.unwrap();
            assert_eq!(diagnostics.reject_count, 1);
            assert_eq!(diagnostics.selected_count, 1);
            assert!(diagnostics.used_tokens <= auth_skill.token_cost as u32);
        });
    }
}
