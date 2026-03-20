//! Domain-aware dense vector storage backends.

use crate::error::IndexingError;
use crate::Result;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, DeletePointsBuilder, Distance, PointId, PointStruct,
    SearchPointsBuilder, Value as QdrantValue, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Dense retrieval domain.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalDomain {
    /// Source code and AST chunks.
    Code,
    /// Procedural `SKILL.md` documents.
    Skill,
}

/// Dense backend implementation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VectorBackendKind {
    /// Qdrant-backed collection.
    Qdrant,
    /// SQLite/sqlite-vec style local collection.
    SqliteVec,
}

/// Distance metric for dense similarity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    /// Cosine similarity.
    Cosine,
}

/// Search result returned by dense collections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    /// Stable identifier.
    pub id: String,
    /// Similarity score.
    pub score: f32,
    /// Opaque payload.
    pub payload: Value,
}

/// Dense collection specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionSpec {
    /// Retrieval domain for this collection.
    pub domain: RetrievalDomain,
    /// Collection or table name.
    pub name: String,
    /// Embedding dimensionality.
    pub dimensions: usize,
    /// Distance metric.
    pub distance: DistanceMetric,
}

impl CollectionSpec {
    /// Default code collection spec.
    pub fn code_default() -> Self {
        Self {
            domain: RetrievalDomain::Code,
            name: "axora-code-chunks".to_string(),
            dimensions: 768,
            distance: DistanceMetric::Cosine,
        }
    }

    /// Default skill collection spec.
    pub fn skill_default() -> Self {
        Self {
            domain: RetrievalDomain::Skill,
            name: "axora-skill-docs".to_string(),
            dimensions: 384,
            distance: DistanceMetric::Cosine,
        }
    }
}

/// Collection contract implemented by each dense backend.
#[async_trait::async_trait]
pub trait DenseVectorCollection: Send + Sync {
    /// Collection metadata.
    fn spec(&self) -> &CollectionSpec;

    /// Insert or replace a vector and payload.
    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> Result<()>;

    /// Delete a vector.
    async fn delete(&self, id: &str) -> Result<()>;

    /// Search by query vector.
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
}

/// Domain-aware store holding both code and skill collections.
pub struct DualVectorStore {
    code: Arc<dyn DenseVectorCollection>,
    skill: Arc<dyn DenseVectorCollection>,
    backend: VectorBackendKind,
}

impl DualVectorStore {
    /// Create a Qdrant-backed dual store.
    pub async fn new_qdrant(
        url: &str,
        code_spec: CollectionSpec,
        skill_spec: CollectionSpec,
    ) -> Result<Self> {
        Ok(Self {
            code: Arc::new(QdrantVectorCollection::new(url, code_spec).await?),
            skill: Arc::new(QdrantVectorCollection::new(url, skill_spec).await?),
            backend: VectorBackendKind::Qdrant,
        })
    }

    /// Create a SQLite-backed dual store.
    pub fn new_sqlite(
        path: impl AsRef<Path>,
        code_spec: CollectionSpec,
        skill_spec: CollectionSpec,
    ) -> Result<Self> {
        Ok(Self {
            code: Arc::new(SqliteVecCollection::new(path.as_ref(), code_spec)?),
            skill: Arc::new(SqliteVecCollection::new(path.as_ref(), skill_spec)?),
            backend: VectorBackendKind::SqliteVec,
        })
    }

    /// Access the code collection.
    pub fn code_collection(&self) -> Arc<dyn DenseVectorCollection> {
        self.code.clone()
    }

    /// Access the skill collection.
    pub fn skill_collection(&self) -> Arc<dyn DenseVectorCollection> {
        self.skill.clone()
    }

    /// Selected dense backend.
    pub fn backend(&self) -> VectorBackendKind {
        self.backend
    }
}

/// Qdrant-backed dense collection.
pub struct QdrantVectorCollection {
    client: Qdrant,
    spec: CollectionSpec,
}

impl QdrantVectorCollection {
    /// Create or open a Qdrant collection.
    pub async fn new(url: &str, spec: CollectionSpec) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let collection = Self { client, spec };
        collection.ensure_collection().await?;
        Ok(collection)
    }

    async fn ensure_collection(&self) -> Result<()> {
        let exists = self
            .client
            .collection_exists(&self.spec.name)
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        if exists {
            return Ok(());
        }

        self.client
            .create_collection(
                CreateCollectionBuilder::new(self.spec.name.clone()).vectors_config(
                    VectorParamsBuilder::new(
                        self.spec.dimensions as u64,
                        match self.spec.distance {
                            DistanceMetric::Cosine => Distance::Cosine,
                        },
                    ),
                ),
            )
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    fn validate_dimensions(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.spec.dimensions {
            return Err(IndexingError::DimensionMismatch {
                expected: self.spec.dimensions,
                actual: vector.len(),
            }
            .into());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl DenseVectorCollection for QdrantVectorCollection {
    fn spec(&self) -> &CollectionSpec {
        &self.spec
    }

    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> Result<()> {
        self.validate_dimensions(vector)?;
        let payload = json_to_qdrant_payload(payload)?;
        self.client
            .upsert_points(
                qdrant_client::qdrant::UpsertPointsBuilder::new(
                    &self.spec.name,
                    vec![PointStruct::new(id.to_string(), vector.to_vec(), payload)],
                )
                .wait(true),
            )
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.spec.name)
                    .points(vec![PointId::from(id.to_string())])
                    .wait(true),
            )
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        self.validate_dimensions(query)?;
        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.spec.name, query.to_vec(), limit as u64)
                    .with_payload(true),
            )
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;

        Ok(response
            .result
            .into_iter()
            .map(|point| SearchResult {
                id: point
                    .id
                    .as_ref()
                    .map(|id| format!("{id:?}"))
                    .unwrap_or_default(),
                score: point.score,
                payload: qdrant_payload_to_json(point.payload),
            })
            .collect())
    }
}

/// SQLite-backed dense collection using separate tables per domain.
pub struct SqliteVecCollection {
    db: Arc<Mutex<Connection>>,
    spec: CollectionSpec,
    path: PathBuf,
}

impl SqliteVecCollection {
    /// Create a SQLite-backed collection.
    pub fn new(path: &Path, spec: CollectionSpec) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        }
        let connection = Connection::open(path).map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Self::run_migrations_on(&connection, &spec)?;
        let collection = Self {
            db: Arc::new(Mutex::new(connection)),
            spec,
            path: path.to_path_buf(),
        };
        Ok(collection)
    }

    fn run_migrations_on(connection: &Connection, spec: &CollectionSpec) -> Result<()> {
        let table = sqlite_metadata_table_name(spec);
        let vector_table = sqlite_vector_table_name(spec);
        let schema = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {table} (
                id TEXT PRIMARY KEY,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS {vector_table} (
                id TEXT PRIMARY KEY,
                embedding TEXT NOT NULL
            );
            "#
        );
        connection
            .execute_batch(&schema)
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    fn validate_dimensions(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.spec.dimensions {
            return Err(IndexingError::DimensionMismatch {
                expected: self.spec.dimensions,
                actual: vector.len(),
            }
            .into());
        }
        Ok(())
    }

    /// SQLite path backing this collection.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait::async_trait]
impl DenseVectorCollection for SqliteVecCollection {
    fn spec(&self) -> &CollectionSpec {
        &self.spec
    }

    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> Result<()> {
        self.validate_dimensions(vector)?;
        let table = sqlite_metadata_table_name(&self.spec);
        let vector_table = sqlite_vector_table_name(&self.spec);
        let payload = serde_json::to_string(&payload)
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let embedding = serde_json::to_string(vector)
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let connection = self.db.lock().await;
        connection
            .execute(
                &format!(
                    "INSERT INTO {table}(id, payload) VALUES (?1, ?2)
                     ON CONFLICT(id) DO UPDATE SET payload = excluded.payload"
                ),
                params![id, payload],
            )
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        connection
            .execute(
                &format!(
                    "INSERT INTO {vector_table}(id, embedding) VALUES (?1, ?2)
                     ON CONFLICT(id) DO UPDATE SET embedding = excluded.embedding"
                ),
                params![id, embedding],
            )
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let table = sqlite_metadata_table_name(&self.spec);
        let vector_table = sqlite_vector_table_name(&self.spec);
        let connection = self.db.lock().await;
        connection
            .execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        connection
            .execute(&format!("DELETE FROM {vector_table} WHERE id = ?1"), params![id])
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        Ok(())
    }

    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        self.validate_dimensions(query)?;
        let table = sqlite_metadata_table_name(&self.spec);
        let vector_table = sqlite_vector_table_name(&self.spec);
        let connection = self.db.lock().await;
        let mut stmt = connection
            .prepare(&format!(
                "SELECT m.id, m.payload, v.embedding
                 FROM {table} m
                 JOIN {vector_table} v ON v.id = m.id"
            ))
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (id, payload, embedding) =
                row.map_err(|err| IndexingError::VectorStore(err.to_string()))?;
            let embedding = serde_json::from_str::<Vec<f32>>(&embedding)
                .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
            if embedding.len() != self.spec.dimensions {
                continue;
            }
            results.push(SearchResult {
                id,
                score: cosine_similarity(query, &embedding),
                payload: serde_json::from_str(&payload)
                    .map_err(|err| IndexingError::VectorStore(err.to_string()))?,
            });
        }

        results.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }
}

fn sqlite_metadata_table_name(spec: &CollectionSpec) -> &'static str {
    match spec.domain {
        RetrievalDomain::Code => "code_chunks",
        RetrievalDomain::Skill => "skill_docs",
    }
}

fn sqlite_vector_table_name(spec: &CollectionSpec) -> &'static str {
    match spec.domain {
        RetrievalDomain::Code => "code_chunk_embeddings",
        RetrievalDomain::Skill => "skill_doc_embeddings",
    }
}

fn json_to_qdrant_payload(payload: Value) -> Result<Payload> {
    let object = payload.as_object().cloned().unwrap_or_default();
    let payload = object
        .into_iter()
        .map(|(key, value)| (key, qdrant_value(value)))
        .collect::<std::collections::HashMap<_, _>>();
    Ok(payload.into())
}

fn qdrant_value(value: Value) -> QdrantValue {
    match value {
        Value::Null => QdrantValue::from(""),
        Value::Bool(value) => QdrantValue::from(value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                QdrantValue::from(value)
            } else if let Some(value) = value.as_u64() {
                QdrantValue::from(value as i64)
            } else {
                QdrantValue::from(value.as_f64().unwrap_or_default())
            }
        }
        Value::String(value) => QdrantValue::from(value),
        other => QdrantValue::from(other.to_string()),
    }
}

fn qdrant_payload_to_json(payload: std::collections::HashMap<String, QdrantValue>) -> Value {
    Value::Object(
        payload
            .into_iter()
            .map(|(key, value)| (key, Value::String(value.to_string())))
            .collect(),
    )
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot = left.iter().zip(right.iter()).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn sqlite_dual_store_keeps_domains_separate() {
        let dir = tempdir().unwrap();
        let store = DualVectorStore::new_sqlite(
            dir.path().join("vectors.db"),
            CollectionSpec::code_default(),
            CollectionSpec::skill_default(),
        )
        .unwrap();

        let code = store.code_collection();
        let skill = store.skill_collection();

        code.upsert(
            "chunk-1",
            &vec![0.1; 768],
            serde_json::json!({"chunk_id":"chunk-1","file_path":"src/lib.rs"}),
        )
        .await
        .unwrap();
        skill
            .upsert(
                "skill-1",
                &vec![0.2; 384],
                serde_json::json!({"skill_id":"skill-1","source_path":"skills/SKILL.md"}),
            )
            .await
            .unwrap();

        assert_eq!(code.search(&vec![0.1; 768], 4).await.unwrap().len(), 1);
        assert_eq!(skill.search(&vec![0.2; 384], 4).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn sqlite_collection_rejects_dimension_mismatch() {
        let dir = tempdir().unwrap();
        let store = DualVectorStore::new_sqlite(
            dir.path().join("vectors.db"),
            CollectionSpec::code_default(),
            CollectionSpec::skill_default(),
        )
        .unwrap();
        let err = store
            .skill_collection()
            .upsert("skill-1", &vec![0.0; 768], serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("dimension mismatch"));
    }
}
