//! VectorStore trait: the load-bearing seam for local-first vector infrastructure.
//!
//! This trait abstracts over different vector storage backends:
//! - `SqliteVecStore` — sqlite-vec extension, HNSW ANN (default, production local)
//! - `SqliteLinearVectorStore` — JSON text, linear scan (fallback/migration path)
//! - `ExternalVectorStore` — Qdrant Cloud or self-hosted endpoint (enterprise/cloud tier)
//!
//! Cloud tier uses Cohere embed-v3-multilingual embeddings via api.openakta.dev.
//! Local tier uses Candle embeddings (JinaCode 768-dim, BGE-Skill 384-dim).

use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::Arc;

use crate::SemanticError;

/// Result type for vector store operations.
pub type VectorResult<T> = Result<T, VectorStoreError>;

/// Vector store error types.
#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("storage error: {0}")]
    Storage(#[from] SemanticError),

    #[error("backend not available: {0}")]
    BackendNotAvailable(String),

    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// A hit returned by vector search.
#[derive(Debug, Clone)]
pub struct VectorHit {
    /// Stable identifier for the vector entry.
    pub id: String,
    /// Similarity score (higher = more similar).
    pub score: f32,
    /// Opaque payload associated with the vector.
    pub payload: Value,
}

/// Candidate returned by `scan_for_pruning`.
#[derive(Debug, Clone)]
pub struct PruneCandidate {
    /// Stable identifier.
    pub id: String,
    /// Creation timestamp (UNIX epoch ms).
    pub created_at: i64,
    /// Number of times retrieved.
    pub retrieval_count: u32,
    /// Importance score (0.0–1.0).
    pub importance: f32,
}

/// The core vector store trait.
///
/// All vector backends must implement this trait. The trait is designed to be
/// stateless per call — connection details belong in the constructor.
#[async_trait]
pub trait VectorStore: Send + Sync + 'static {
    /// Insert or overwrite a vector entry.
    ///
    /// `id` is a stable content-addressed key. `payload` is arbitrary JSON.
    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> VectorResult<()>;

    /// Find top-K most similar vectors to `query`.
    ///
    /// `filter` is reserved for future metadata filtering (not implemented in Phase 1).
    async fn search(
        &self,
        query: &[f32],
        limit: usize,
        filter: Option<Value>,
    ) -> VectorResult<Vec<VectorHit>>;

    /// Delete a single entry by id.
    async fn delete(&self, id: &str) -> VectorResult<()>;

    /// Return approximate count of stored vectors.
    async fn count(&self) -> VectorResult<u64>;

    /// Scan entries for pruning (returns oldest/least-used first).
    ///
    /// This is used by the memory lifecycle for pruning decisions.
    async fn scan_for_pruning(&self, limit: usize) -> VectorResult<Vec<PruneCandidate>>;

    /// Backend identifier string for observability.
    ///
    /// Examples: `"sqlite-json-linear"`, `"sqlite-vec-ann"`, `"qdrant-cloud"`.
    fn backend_id(&self) -> &'static str;
}

/// Linear-scan SQLite vector store (fallback/migration path).
///
/// Wraps `PersistentSemanticStore` to implement `VectorStore`.
/// Uses JSON text storage and full scan in Rust — no ANN.
/// This is a legacy compatibility layer; prefer `SqliteVecStore` for production use.

pub struct SqliteLinearVectorStore {
    db: Arc<tokio::sync::Mutex<Connection>>,
    embedding_dim: usize,
    path: String,
    scan_cap: usize,
}

impl SqliteLinearVectorStore {
    /// Create a new linear vector store from an existing `PersistentSemanticStore` path.
    pub fn new(path: &str, embedding_dim: usize, scan_cap: usize) -> VectorResult<Self> {
        let conn = Connection::open(path).map_err(|e| VectorStoreError::Storage(
            SemanticError::Storage {
                path: path.to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("failed to open SQLite: {}", e)),
                ),
            }
        ))?;

        Ok(Self {
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            embedding_dim,
            path: path.to_string(),
            scan_cap,
        })
    }

    /// Guard: warn if table size exceeds scan_cap.
    async fn check_scan_cap(&self) {
        let db = self.db.lock().await;
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM semantic_memories",
                [],
                |row: &rusqlite::Row| -> rusqlite::Result<i64> { row.get::<_, i64>(0) },
            )
            .unwrap_or(0);
        drop(db);
        if count as usize > self.scan_cap {
            tracing::warn!(
                count = count,
                cap = self.scan_cap,
                "semantic_memories table exceeds scan_cap; consider pruning or migrating to sqlite-vec backend"
            );
        }
    }
}

#[async_trait]
impl VectorStore for SqliteLinearVectorStore {
    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> VectorResult<()> {
        if vector.len() != self.embedding_dim {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.embedding_dim,
                actual: vector.len(),
            });
        }

        // Extract content from payload for semantic memory compatibility
        let content = payload.get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let embedding = serde_json::to_string(vector)
            .map_err(|e| VectorStoreError::Internal(format!("serialization failed: {}", e)))?;
        let metadata = serde_json::to_string(&payload)
            .map_err(|e| VectorStoreError::Internal(format!("serialization failed: {}", e)))?;

        let db = self.db.lock().await;
        db.execute(
            r#"
            INSERT INTO semantic_memories (id, content, embedding, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                embedding = excluded.embedding,
                metadata = excluded.metadata,
                updated_at = excluded.updated_at
            "#,
            rusqlite::params![
                id,
                content,
                embedding,
                metadata,
                chrono::Utc::now().timestamp_millis(),
                chrono::Utc::now().timestamp_millis(),
            ],
        )
        .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("upsert failed: {}", e)),
            ),
        }))?;

        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        limit: usize,
        _filter: Option<Value>,
    ) -> VectorResult<Vec<VectorHit>> {
        if query.len() != self.embedding_dim {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.embedding_dim,
                actual: query.len(),
            });
        }

        self.check_scan_cap().await;

        let db = self.db.lock().await;
        let mut stmt = db
            .prepare("SELECT id, content, embedding, metadata FROM semantic_memories")
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("search prepare failed: {}", e)),
                ),
            }))?;

        let rows: Vec<rusqlite::Result<(String, String, Vec<f32>, Value)>> = stmt
            .query_map(
                [],
                |row: &rusqlite::Row| -> rusqlite::Result<(String, String, Vec<f32>, Value)> {
                    let embedding: String = row.get(2)?;
                    let metadata: String = row.get(3)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        serde_json::from_str::<Vec<f32>>(&embedding).unwrap_or_default(),
                        serde_json::from_str::<Value>(&metadata).unwrap_or(Value::Null),
                    ))
                },
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("search query failed: {}", e)),
                ),
            }))?
            .collect();

        let mut results = Vec::new();
        for row_result in rows {
            let row: rusqlite::Result<(String, String, Vec<f32>, Value)> = row_result;
            let (id, content, embedding, metadata): (String, String, Vec<f32>, Value) =
                row.map_err(|e: rusqlite::Error| -> VectorStoreError {
                    VectorStoreError::Storage(SemanticError::Storage {
                        path: self.path.clone(),
                        source: rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                            Some(format!("row fetch failed: {}", e)),
                        ),
                    })
                })?;

            if embedding.len() != self.embedding_dim {
                continue;
            }

            results.push(VectorHit {
                id,
                score: cosine_similarity(query, &embedding),
                payload: {
                    let mut map = serde_json::Map::new();
                    map.insert("content".to_string(), Value::String(content));
                    if let Value::Object(mut m) = metadata {
                        map.extend(m);
                    }
                    Value::Object(map)
                },
            });
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        let db = self.db.lock().await;
        db.execute(
            "DELETE FROM semantic_memories WHERE id = ?",
            rusqlite::params![id],
        )
        .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("delete failed: {}", e)),
            ),
        }))?;
        Ok(())
    }

    async fn count(&self) -> VectorResult<u64> {
        let db = self.db.lock().await;
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM semantic_memories",
                [],
                |row: &rusqlite::Row| row.get::<_, i64>(0),
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("count failed: {}", e)),
                ),
            }))?;
        Ok(count as u64)
    }

    async fn scan_for_pruning(&self, limit: usize) -> VectorResult<Vec<PruneCandidate>> {
        let db = self.db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT id, metadata, created_at FROM semantic_memories ORDER BY created_at ASC LIMIT ?",
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("scan prepare failed: {}", e)),
                ),
            }))?;

        let rows: Vec<rusqlite::Result<(String, i64, Value)>> = stmt
            .query_map(
                rusqlite::params![limit as i64],
                |row: &rusqlite::Row| -> rusqlite::Result<(String, i64, Value)> {
                    let metadata: String = row.get(1)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(2)?,
                        serde_json::from_str::<Value>(&metadata).unwrap_or(Value::Null),
                    ))
                },
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("scan query failed: {}", e)),
                ),
            }))?
            .collect();

        let mut candidates = Vec::new();
        for row_result in rows {
            let row: rusqlite::Result<(String, i64, Value)> = row_result;
            let (id, created_at, metadata): (String, i64, Value) =
                row.map_err(|e: rusqlite::Error| -> VectorStoreError {
                    VectorStoreError::Storage(SemanticError::Storage {
                        path: self.path.clone(),
                        source: rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                            Some(format!("scan row fetch failed: {}", e)),
                        ),
                    })
                })?;

            // Extract retrieval_count and importance from metadata
            let retrieval_count = metadata
                .get("retrieval_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let importance = metadata
                .get("importance")
                .and_then(Value::as_f64)
                .unwrap_or(0.5) as f32;

            candidates.push(PruneCandidate {
                id,
                created_at,
                retrieval_count,
                importance,
            });
        }

        Ok(candidates)
    }

    fn backend_id(&self) -> &'static str {
        "sqlite-json-linear"
    }
}

/// Cosine similarity helper.
fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot = left.iter().zip(right.iter()).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|v| v * v).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|v| v * v).sum::<f32>().sqrt();
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
    async fn sqlite_linear_upsert_and_search() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.display().to_string();

        // Create the table first (mimic PersistentSemanticStore migrations)
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .unwrap();

        let store = SqliteLinearVectorStore::new(&path_str, 384, 50_000).unwrap();

        // Upsert
        store
            .upsert(
                "test-1",
                &vec![0.1; 384],
                serde_json::json!({
                    "content": "test content",
                    "retrieval_count": 0,
                    "importance": 0.5
                }),
            )
            .await
            .unwrap();

        // Search
        let hits = store.search(&vec![0.1; 384], 10, None).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "test-1");
        assert!((hits[0].score - 1.0).abs() < 0.001); // cosine of identical = 1.0
    }

    #[tokio::test]
    async fn sqlite_linear_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.display().to_string();

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .unwrap();

        let store = SqliteLinearVectorStore::new(&path_str, 384, 50_000).unwrap();

        assert_eq!(store.count().await.unwrap(), 0);

        store
            .upsert(
                "test-1",
                &vec![0.1; 384],
                serde_json::json!({"content": "test"}),
            )
            .await
            .unwrap();

        assert_eq!(store.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn sqlite_linear_scan_for_pruning() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.display().to_string();

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .unwrap();

        let store = SqliteLinearVectorStore::new(&path_str, 384, 50_000).unwrap();

        store
            .upsert(
                "test-1",
                &vec![0.1; 384],
                serde_json::json!({
                    "content": "test",
                    "retrieval_count": 0,
                    "importance": 0.3
                }),
            )
            .await
            .unwrap();

        let candidates = store.scan_for_pruning(10).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "test-1");
        assert_eq!(candidates[0].retrieval_count, 0);
        assert!((candidates[0].importance - 0.3).abs() < 0.001);
    }

    #[tokio::test]
    async fn sqlite_linear_rejects_dimension_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.display().to_string();

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .unwrap();

        let store = SqliteLinearVectorStore::new(&path_str, 384, 50_000).unwrap();

        let err = store
            .upsert("test-1", &vec![0.1; 768], serde_json::json!({}))
            .await
            .unwrap_err();

        assert!(matches!(err, VectorStoreError::DimensionMismatch { .. }));
    }
}

// ============================================================================
// Phase 2: sqlite-vec ANN Backend
// ============================================================================

/// sqlite-vec ANN vector store (default production backend).
///
/// Uses the sqlite-vec extension for efficient approximate nearest neighbor search.
/// Provides HNSW-based ANN with significantly better performance than linear scan
/// for large vector collections (>10k vectors).
///
/// This is the default backend for local-first semantic memory in OPENAKTA.
pub struct SqliteVecStore {
    db: Arc<tokio::sync::Mutex<Connection>>,
    embedding_dim: usize,
    path: String,
    scan_cap: usize,
}

impl SqliteVecStore {
    /// Create a new sqlite-vec store.
    ///
    /// `dim` must match the embedding dimension (384 for semantic, 768 for code).
    pub fn new(path: &str, dim: usize, scan_cap: usize) -> VectorResult<Self> {
        let mut conn = Connection::open(path).map_err(|e| VectorStoreError::Storage(
            SemanticError::Storage {
                path: path.to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("failed to open SQLite: {}", e)),
                ),
            }
        ))?;

        // Load sqlite-vec extension
        unsafe {
            conn.load_extension_enable()
                .map_err(|e| VectorStoreError::Internal(format!("failed to enable extension loading: {}", e)))?;
        }

        conn.execute(
            "SELECT load_extension('sqlite_vec')",
            [],
        )
        .map_err(|e| VectorStoreError::Internal(format!("failed to load sqlite-vec: {}", e)))?;

        unsafe {
            conn.load_extension_disable()
                .map_err(|e| VectorStoreError::Internal(format!("failed to disable extension loading: {}", e)))?;
        }

        // Run migrations
        Self::run_migrations(&mut conn, dim)?;

        Ok(Self {
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            embedding_dim: dim,
            path: path.to_string(),
            scan_cap,
        })
    }

    fn run_migrations(conn: &mut Connection, dim: usize) -> VectorResult<()> {
        // Create virtual table for ANN using sqlite-vec
        conn.execute_batch(&format!(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS semantic_vec (
                id TEXT PRIMARY KEY,
                embedding FLOAT[{dim}]
            ) USING vec0;

            CREATE TABLE IF NOT EXISTS semantic_vec_payload (
                id TEXT PRIMARY KEY REFERENCES semantic_vec(id) ON DELETE CASCADE,
                content TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_svp_updated ON semantic_vec_payload(updated_at);
            "#
        ))
        .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: "semantic_vec".to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("migration failed: {}", e)),
            ),
        }))?;

        Ok(())
    }

    /// Migrate from legacy JSON table to sqlite-vec format.
    pub fn migrate_from_json(conn: &mut Connection) -> VectorResult<u64> {
        // Check if legacy table exists
        let legacy_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='semantic_memories')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !legacy_exists {
            return Ok(0);
        }

        // Count rows to migrate
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM semantic_memories", [], |row| row.get(0))
            .unwrap_or(0);

        if count == 0 {
            return Ok(0);
        }

        // Read all legacy rows into memory first to avoid borrow conflicts
        let mut stmt = conn
            .prepare("SELECT id, content, embedding, metadata, created_at, updated_at FROM semantic_memories")
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_memories".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("prepare migration select failed: {}", e)),
                ),
            }))?;

        let rows_data: Vec<(String, String, String, String, i64, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_memories".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration query failed: {}", e)),
                ),
            }))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        let mut migrated = 0u64;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration transaction failed: {}", e)),
                ),
            }))?;

        for (id, content, embedding_json, metadata, created_at, updated_at) in rows_data {
            // Parse JSON embedding to Vec<f32>
            let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                .map_err(|e| VectorStoreError::Internal(format!("failed to parse embedding JSON: {}", e)))?;

            // Convert to bytes for sqlite-vec
            let embedding_blob: &[u8] = bytemuck::cast_slice(&embedding);
            
            // Insert into semantic_vec (virtual table)
            tx.execute(
                "INSERT INTO semantic_vec(id, embedding) VALUES (?, ?)",
                rusqlite::params![id, embedding_blob],
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration insert to vec failed: {}", e)),
                ),
            }))?;

            // Insert payload
            tx.execute(
                "INSERT INTO semantic_vec_payload(id, content, payload, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![id, content, metadata, created_at, updated_at],
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec_payload".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration insert to payload failed: {}", e)),
                ),
            }))?;

            migrated += 1;
        }

        tx.commit().map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: "semantic_vec".to_string(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("migration commit failed: {}", e)),
            ),
        }))?;

        // Drop legacy table after successful migration
        conn.execute("DROP TABLE IF EXISTS semantic_memories", [])
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_memories".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("drop legacy table failed: {}", e)),
                ),
            }))?;

        Ok(migrated)
    }

    /// Guard: warn if table size exceeds scan_cap.
    async fn check_scan_cap(&self) {
        let mut db = self.db.lock().await;
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM semantic_vec", [], |row: &rusqlite::Row| -> rusqlite::Result<i64> {
                row.get(0)
            })
            .unwrap_or(0);
        drop(db);
        if count as usize > self.scan_cap {
            tracing::warn!(
                count = count,
                cap = self.scan_cap,
                "semantic_vec table exceeds scan_cap; consider pruning or sharding"
            );
        }
    }
}

#[async_trait]
impl VectorStore for SqliteVecStore {
    async fn upsert(&self, id: &str, vector: &[f32], payload: Value) -> VectorResult<()> {
        if vector.len() != self.embedding_dim {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.embedding_dim,
                actual: vector.len(),
            });
        }

        let content = payload.get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let metadata = serde_json::to_string(&payload)
            .map_err(|e| VectorStoreError::Internal(format!("serialization failed: {}", e)))?;

        let mut db = self.db.lock().await;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("upsert transaction failed: {}", e)),
                ),
            }))?;

        // Convert f32 slice to bytes for sqlite-vec
        let embedding_bytes: &[u8] = bytemuck::cast_slice(vector);
        
        // Insert into virtual table (sqlite-vec)
        tx.execute(
            "INSERT INTO semantic_vec(id, embedding) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET embedding = excluded.embedding",
            rusqlite::params![id, embedding_bytes],
        )
        .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("vec upsert failed: {}", e)),
            ),
        }))?;

        // Insert/update payload
        let now = chrono::Utc::now().timestamp_millis();
        tx.execute(
            r#"
            INSERT INTO semantic_vec_payload(id, content, payload, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                payload = excluded.payload,
                updated_at = excluded.updated_at
            "#,
            rusqlite::params![id, content, metadata, now, now],
        )
        .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("payload upsert failed: {}", e)),
            ),
        }))?;

        tx.commit().map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("upsert commit failed: {}", e)),
            ),
        }))?;

        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        limit: usize,
        _filter: Option<Value>,
    ) -> VectorResult<Vec<VectorHit>> {
        if query.len() != self.embedding_dim {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.embedding_dim,
                actual: query.len(),
            });
        }

        self.check_scan_cap().await;

        let mut db = self.db.lock().await;
        let mut stmt = db
            .prepare(
                r#"
                SELECT sv.id, sv.distance, svp.content, svp.payload
                FROM semantic_vec sv
                JOIN semantic_vec_payload svp ON sv.id = svp.id
                WHERE sv.embedding MATCH ?1
                ORDER BY sv.distance
                LIMIT ?2
                "#,
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("search prepare failed: {}", e)),
                ),
            }))?;

        // Convert query f32 slice to bytes for sqlite-vec MATCH
        let query_bytes: &[u8] = bytemuck::cast_slice(query);

        let rows: Vec<rusqlite::Result<(String, f32, String, String)>> = stmt
            .query_map(
                rusqlite::params![query_bytes, limit as i64],
                |row: &rusqlite::Row| -> rusqlite::Result<(String, f32, String, String)> {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f32>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("search query failed: {}", e)),
                ),
            }))?
            .collect();

        let mut results = Vec::new();
        for row_result in rows {
            let (id, distance, content, payload_json): (String, f32, String, String) =
                row_result.map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("search row fetch failed: {}", e)),
                    ),
                }))?;

            // sqlite-vec returns distance (0 = identical), convert to similarity score
            let score = 1.0 - distance;

            let payload: Value = serde_json::from_str(&payload_json)
                .unwrap_or_else(|_| {
                    let mut map = serde_json::Map::new();
                    map.insert("content".to_string(), Value::String(content.clone()));
                    Value::Object(map)
                });

            results.push(VectorHit { id, score, payload });
        }

        Ok(results)
    }

    async fn delete(&self, id: &str) -> VectorResult<()> {
        let mut db = self.db.lock().await;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("delete transaction failed: {}", e)),
                ),
            }))?;

        // Delete from payload first (FK cascade will handle vec, but explicit is clearer)
        tx.execute("DELETE FROM semantic_vec_payload WHERE id = ?", rusqlite::params![id])
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("payload delete failed: {}", e)),
                ),
            }))?;

        // Delete from virtual table
        tx.execute("DELETE FROM semantic_vec WHERE id = ?", rusqlite::params![id])
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("vec delete failed: {}", e)),
                ),
            }))?;

        tx.commit().map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
            path: self.path.clone(),
            source: rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                Some(format!("delete commit failed: {}", e)),
            ),
        }))?;

        Ok(())
    }

    async fn count(&self) -> VectorResult<u64> {
        let mut db = self.db.lock().await;
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM semantic_vec", [], |row: &rusqlite::Row| -> rusqlite::Result<i64> {
                row.get(0)
            })
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("count failed: {}", e)),
                ),
            }))?;
        Ok(count as u64)
    }

    async fn scan_for_pruning(&self, limit: usize) -> VectorResult<Vec<PruneCandidate>> {
        let mut db = self.db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT sv.id, svp.payload, svp.created_at FROM semantic_vec sv JOIN semantic_vec_payload svp ON sv.id = svp.id ORDER BY svp.created_at ASC LIMIT ?",
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("scan prepare failed: {}", e)),
                ),
            }))?;

        let rows: Vec<rusqlite::Result<(String, String, i64)>> = stmt
            .query_map(
                rusqlite::params![limit as i64],
                |row: &rusqlite::Row| -> rusqlite::Result<(String, String, i64)> {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("scan query failed: {}", e)),
                ),
            }))?
            .collect();

        let mut candidates = Vec::new();
        for row_result in rows {
            let (id, payload_json, created_at): (String, String, i64) =
                row_result.map_err(|e| VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("scan row fetch failed: {}", e)),
                    ),
                }))?;

            let payload: Value = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
            let retrieval_count = payload
                .get("retrieval_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let importance = payload
                .get("importance")
                .and_then(Value::as_f64)
                .unwrap_or(0.5) as f32;

            candidates.push(PruneCandidate {
                id,
                created_at,
                retrieval_count,
                importance,
            });
        }

        Ok(candidates)
    }

    fn backend_id(&self) -> &'static str {
        "sqlite-vec-ann"
    }
}

// ============================================================================
// External Backend Stub (Cloud Tier / Self-Hosted)
// ============================================================================

/// External vector store stub for cloud tier or self-hosted Qdrant.
///
/// This stub is reserved for future implementation connecting to:
/// - Qdrant Cloud (Azure Marketplace) for paid tier
/// - Self-hosted Qdrant or compatible endpoints for enterprise
///
/// Cloud tier uses Cohere embed-v3-multilingual embeddings.
/// Communication is via HTTPS proxy to api.openakta.dev with token auth.
///
/// Future migration path: Turbopuffer via VectorStore trait compatibility.
// pub struct ExternalVectorStore {
//     endpoint: String,
//     api_key: Option<String>,
//     client: reqwest::Client,
// }
