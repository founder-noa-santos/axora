//! VectorStore trait: the load-bearing seam for local-first vector infrastructure.
//!
//! This trait abstracts over different vector storage backends:
//! - `SqliteVecStore` — sqlite-vec extension, HNSW ANN (default, production local)
//! - `ExternalVectorStore` — Qdrant Cloud or self-hosted endpoint (enterprise/cloud tier)
//!
//! Cloud tier uses Cohere embed-v3-multilingual embeddings via api.openakta.dev.
//! Local tier uses Candle embeddings (JinaCode 768-dim, BGE-Skill 384-dim).

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::sync::Arc;

use crate::SemanticError;

fn bundled_sqlite_version() -> &'static str {
    std::ffi::CStr::from_bytes_with_nul(rusqlite::ffi::SQLITE_VERSION)
        .ok()
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
}

/// Canonical, idempotent sqlite-vec initialization for the entire process.
///
/// This is the single source of truth for sqlite-vec registration across all
/// OPENAKTA binaries (CLI, daemon, tests). It uses `OnceLock` to ensure
/// thread-safe, one-time initialization with two-tier verification.
///
/// **MUST** be called before any SQLite connections are opened in your process.
///
/// # Verification
///
/// Tier A: `SELECT vec_version()` — proves extension is registered
/// Tier B: Minimal `vec0` virtual table creation — proves module is operational
///
/// # Example
///
/// ```rust,no_run
/// // In your main() function, before any SQLite usage:
/// openakta_memory::ensure_sqlite_vec_ready()
///     .expect("sqlite-vec initialization failed");
/// ```
pub fn ensure_sqlite_vec_ready() -> Result<(), SqliteVecInitError> {
    use std::sync::OnceLock;

    static INIT: OnceLock<Result<(), SqliteVecInitError>> = OnceLock::new();

    INIT.get_or_init(|| {
        // Step 0: verify the process is using the bundled SQLite we compiled against.
        let runtime_version = rusqlite::version();
        let runtime_version_number = rusqlite::version_number();
        let bundled_version = bundled_sqlite_version();
        let bundled_version_number = rusqlite::ffi::SQLITE_VERSION_NUMBER;

        if runtime_version_number != bundled_version_number {
            return Err(SqliteVecInitError::VersionMismatch {
                runtime_version: runtime_version.to_string(),
                runtime_version_number,
                bundled_version: bundled_version.to_string(),
                bundled_version_number,
            });
        }

        // Step 1: Register via sqlite3_auto_extension (matches sqlite-vec crate test)
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        // Step 2: Tier A verification - vec_version()
        let conn = Connection::open_in_memory()
            .map_err(|e| SqliteVecInitError::ConnectionFailed(e.to_string()))?;

        let version_result: Result<String, _> =
            conn.query_row("SELECT vec_version()", [], |row| row.get(0));

        match version_result {
            Ok(version) => {
                tracing::debug!("sqlite-vec Tier A check passed: {}", version);
            }
            Err(e) => {
                return Err(SqliteVecInitError::TierACheckFailed(format!(
                    "vec_version() query failed: {}",
                    e
                )));
            }
        }

        // Step 3: Tier B verification - vec0 smoke test
        let smoke_result: Result<(), rusqlite::Error> = (|| {
            conn.execute_batch(
                "CREATE VIRTUAL TABLE _vec_smoke_test USING vec0 (embedding float[1]);",
            )?;
            conn.execute_batch("DROP TABLE _vec_smoke_test;")?;
            Ok(())
        })();

        match smoke_result {
            Ok(_) => {
                tracing::debug!("sqlite-vec Tier B check passed: vec0 module operational");
            }
            Err(e) => {
                return Err(SqliteVecInitError::TierBCheckFailed(format!(
                    "vec0 smoke test failed: {}",
                    e
                )));
            }
        }

        tracing::info!("sqlite-vec initialized (statically linked, two-tier verification passed)");
        Ok(())
    })
    .clone()
}

/// Error types for sqlite-vec initialization
#[derive(Debug, Clone, thiserror::Error)]
pub enum SqliteVecInitError {
    #[error(
        "runtime SQLite {runtime_version} ({runtime_version_number}) does not match bundled SQLite {bundled_version} ({bundled_version_number})"
    )]
    VersionMismatch {
        runtime_version: String,
        runtime_version_number: i32,
        bundled_version: String,
        bundled_version_number: i32,
    },

    #[error("failed to open test connection: {0}")]
    ConnectionFailed(String),

    #[error("Tier A verification failed: {0}")]
    TierACheckFailed(String),

    #[error("Tier B verification failed: {0}")]
    TierBCheckFailed(String),
}

/// Legacy alias for backwards compatibility.
///
/// **Deprecated:** Use [`ensure_sqlite_vec_ready()`] instead.
/// This function will be removed in a future release.
#[deprecated(
    since = "0.1.0",
    note = "Use ensure_sqlite_vec_ready() instead - it provides idempotent, two-tier verified initialization"
)]
pub fn init_sqlite_vec_static() -> Result<(), VectorStoreError> {
    ensure_sqlite_vec_ready().map_err(|e| VectorStoreError::Internal(e.to_string()))
}

/// Initialize sqlite-vec extension via dynamic loading (DEV-ONLY fallback).
///
/// **WARNING:** This function is for development/debugging ONLY.
/// It is NOT part of the product path and requires manual sqlite-vec installation.
///
/// This function is compiled out by default. To enable for debugging:
/// ```toml
/// # In your Cargo.toml
/// openakta-memory = { path = "...", features = ["sqlite-vec-dynamic-dev"] }
/// ```
///
/// # Safety
///
/// This loads external shared libraries at runtime. End users do NOT need this.
#[cfg(feature = "sqlite-vec-dynamic-dev")]
#[allow(dead_code)]
fn _init_sqlite_vec_dynamic_debug(conn: &mut Connection) -> Result<(), VectorStoreError> {
    use std::path::Path;

    tracing::warn!("Using DEV-ONLY sqlite-vec dynamic loading - NOT for production!");

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let lib_name = if cfg!(target_os = "macos") {
        "sqlite_vec.dylib"
    } else {
        "sqlite_vec.so"
    };

    let search_paths = [
        format!("{}/.local/lib/{}", home, lib_name),
        format!("{}/.local/lib/sqlite_vec", home),
        "/usr/local/lib/sqlite_vec.dylib".to_string(),
        "/usr/local/lib/sqlite_vec.so".to_string(),
        "/opt/homebrew/lib/sqlite_vec.dylib".to_string(),
        "/opt/homebrew/lib/sqlite_vec.so".to_string(),
    ];

    for path in &search_paths {
        if Path::new(path).exists() {
            unsafe {
                match conn.load_extension(path, Some("sqlite3_vec_init")) {
                    Ok(_) => {
                        tracing::warn!("sqlite-vec loaded from {} (DEV MODE ONLY)", path);
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load sqlite-vec from {}: {}", path, e);
                    }
                }
            }
        }
    }

    Err(VectorStoreError::Internal(format!(
        "sqlite-vec dynamic load failed (DEV MODE). Searched: {:?}. \
             This is a dev-only feature - production uses static linking.",
        search_paths
    )))
}

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

// ============================================================================
// sqlite-vec ANN Backend
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
    ///
    /// **Important:** sqlite-vec must be initialized before calling this function.
    /// Call [`ensure_sqlite_vec_ready()`][crate::ensure_sqlite_vec_ready] at process startup,
    /// before any SQLite connections are opened. This is typically done in your `main()` function.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // In main(), before any SQLite usage:
    /// openakta_memory::ensure_sqlite_vec_ready()
    ///     .expect("sqlite-vec initialization failed");
    ///
    /// // Now you can safely create SqliteVecStore:
    /// let store = openakta_memory::SqliteVecStore::new("memory.db", 384, 1000)?;
    /// # Ok(()) }
    /// ```
    pub fn new(path: &str, dim: usize, scan_cap: usize) -> VectorResult<Self> {
        // Ensure sqlite-vec is initialized (idempotent via OnceLock)
        // This delegates to the canonical initializer - no duplicate registration
        ensure_sqlite_vec_ready().map_err(|e| {
            VectorStoreError::Internal(format!("sqlite-vec not initialized: {}", e))
        })?;

        let mut conn = Connection::open(path).map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: path.to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("failed to open SQLite: {}", e)),
                ),
            })
        })?;

        // Run migrations (creates vec0 virtual table)
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
        // Note: vec0 syntax requires column definitions AFTER "USING vec0"
        conn.execute_batch(&format!(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS semantic_vec USING vec0 (
                id TEXT PRIMARY KEY,
                embedding FLOAT[{dim}]
            );
            "#
        ))
        .map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration failed: {}", e)),
                ),
            })
        })?;

        Self::ensure_payload_table_schema(conn)?;

        Ok(())
    }

    fn ensure_payload_table_schema(conn: &Connection) -> VectorResult<()> {
        let existing_sql: Option<String> = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='semantic_vec_payload'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: "semantic_vec_payload".to_string(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("payload schema introspection failed: {}", e)),
                    ),
                })
            })?;

        match existing_sql {
            None => conn.execute_batch(
                r#"
                CREATE TABLE semantic_vec_payload (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_svp_updated ON semantic_vec_payload(updated_at);
                "#,
            ),
            Some(sql) if sql.to_ascii_lowercase().contains("references semantic_vec") => {
                conn.execute_batch(
                    r#"
                    DROP INDEX IF EXISTS idx_svp_updated;
                    CREATE TABLE semantic_vec_payload_new (
                        id TEXT PRIMARY KEY,
                        content TEXT NOT NULL,
                        payload TEXT NOT NULL,
                        created_at INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    );
                    INSERT INTO semantic_vec_payload_new(id, content, payload, created_at, updated_at)
                    SELECT id, content, payload, created_at, updated_at
                    FROM semantic_vec_payload;
                    DROP TABLE semantic_vec_payload;
                    ALTER TABLE semantic_vec_payload_new RENAME TO semantic_vec_payload;
                    CREATE INDEX IF NOT EXISTS idx_svp_updated ON semantic_vec_payload(updated_at);
                    "#,
                )
            }
            Some(_) => conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_svp_updated ON semantic_vec_payload(updated_at);",
            ),
        }
        .map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec_payload".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("payload schema migration failed: {}", e)),
                ),
            })
        })?;

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
            .query_row("SELECT COUNT(*) FROM semantic_memories", [], |row| {
                row.get(0)
            })
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
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: "semantic_memories".to_string(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("migration query failed: {}", e)),
                    ),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        let mut migrated = 0u64;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: "semantic_vec".to_string(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("migration transaction failed: {}", e)),
                    ),
                })
            })?;

        for (id, content, embedding_json, metadata, created_at, updated_at) in rows_data {
            // Parse JSON embedding to Vec<f32>
            let embedding: Vec<f32> = serde_json::from_str(&embedding_json).map_err(|e| {
                VectorStoreError::Internal(format!("failed to parse embedding JSON: {}", e))
            })?;

            // Convert to bytes for sqlite-vec
            let embedding_blob: &[u8] = bytemuck::cast_slice(&embedding);

            // Insert into semantic_vec (virtual table)
            tx.execute(
                "INSERT INTO semantic_vec(id, embedding) VALUES (?, ?)",
                rusqlite::params![id, embedding_blob],
            )
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: "semantic_vec".to_string(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("migration insert to vec failed: {}", e)),
                    ),
                })
            })?;

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

        tx.commit().map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: "semantic_vec".to_string(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("migration commit failed: {}", e)),
                ),
            })
        })?;

        // Drop legacy table after successful migration
        conn.execute("DROP TABLE IF EXISTS semantic_memories", [])
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: "semantic_memories".to_string(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("drop legacy table failed: {}", e)),
                    ),
                })
            })?;

        Ok(migrated)
    }

    /// Guard: warn if table size exceeds scan_cap.
    async fn check_scan_cap(&self) {
        let db = self.db.lock().await;
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM semantic_vec",
                [],
                |row: &rusqlite::Row| -> rusqlite::Result<i64> { row.get(0) },
            )
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

        let content = payload
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let metadata = serde_json::to_string(&payload)
            .map_err(|e| VectorStoreError::Internal(format!("serialization failed: {}", e)))?;

        let mut db = self.db.lock().await;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("upsert transaction failed: {}", e)),
                    ),
                })
            })?;

        // Convert f32 slice to bytes for sqlite-vec
        let embedding_bytes: &[u8] = bytemuck::cast_slice(vector);

        // sqlite-vec virtual tables implement INSERT/UPDATE, but SQLite does not
        // support `ON CONFLICT .. DO UPDATE` against this virtual table.
        let updated = tx
            .execute(
                "UPDATE semantic_vec SET embedding = ?2 WHERE id = ?1",
                rusqlite::params![id, embedding_bytes],
            )
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("vec update failed: {}", e)),
                    ),
                })
            })?;

        if updated == 0 {
            tx.execute(
                "INSERT INTO semantic_vec(id, embedding) VALUES (?, ?)",
                rusqlite::params![id, embedding_bytes],
            )
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("vec insert failed: {}", e)),
                    ),
                })
            })?;
        }

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
        .map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("payload upsert failed: {}", e)),
                ),
            })
        })?;

        tx.commit().map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("upsert commit failed: {}", e)),
                ),
            })
        })?;

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
            .prepare(
                r#"
                SELECT sv.id, sv.distance, svp.content, svp.payload
                FROM semantic_vec sv
                JOIN semantic_vec_payload svp ON sv.id = svp.id
                WHERE sv.embedding MATCH ?1
                  AND sv.k = ?2
                ORDER BY sv.distance
                "#,
            )
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("search prepare failed: {}", e)),
                    ),
                })
            })?;

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
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("search query failed: {}", e)),
                    ),
                })
            })?
            .collect();

        let mut results = Vec::new();
        for row_result in rows {
            let (id, distance, content, payload_json): (String, f32, String, String) =
                row_result.map_err(|e| {
                    VectorStoreError::Storage(SemanticError::Storage {
                        path: self.path.clone(),
                        source: rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                            Some(format!("search row fetch failed: {}", e)),
                        ),
                    })
                })?;

            // sqlite-vec returns distance (0 = identical), convert to similarity score
            let score = 1.0 - distance;

            let payload: Value = serde_json::from_str(&payload_json).unwrap_or_else(|_| {
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
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("delete transaction failed: {}", e)),
                    ),
                })
            })?;

        // Delete from payload first (FK cascade will handle vec, but explicit is clearer)
        tx.execute(
            "DELETE FROM semantic_vec_payload WHERE id = ?",
            rusqlite::params![id],
        )
        .map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("payload delete failed: {}", e)),
                ),
            })
        })?;

        // Delete from virtual table
        tx.execute(
            "DELETE FROM semantic_vec WHERE id = ?",
            rusqlite::params![id],
        )
        .map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("vec delete failed: {}", e)),
                ),
            })
        })?;

        tx.commit().map_err(|e| {
            VectorStoreError::Storage(SemanticError::Storage {
                path: self.path.clone(),
                source: rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(format!("delete commit failed: {}", e)),
                ),
            })
        })?;

        Ok(())
    }

    async fn count(&self) -> VectorResult<u64> {
        let db = self.db.lock().await;
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM semantic_vec",
                [],
                |row: &rusqlite::Row| -> rusqlite::Result<i64> { row.get(0) },
            )
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("count failed: {}", e)),
                    ),
                })
            })?;
        Ok(count as u64)
    }

    async fn scan_for_pruning(&self, limit: usize) -> VectorResult<Vec<PruneCandidate>> {
        let db = self.db.lock().await;
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
            .map_err(|e| {
                VectorStoreError::Storage(SemanticError::Storage {
                    path: self.path.clone(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("scan query failed: {}", e)),
                    ),
                })
            })?
            .collect();

        let mut candidates = Vec::new();
        for row_result in rows {
            let (id, payload_json, created_at): (String, String, i64) =
                row_result.map_err(|e| {
                    VectorStoreError::Storage(SemanticError::Storage {
                        path: self.path.clone(),
                        source: rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                            Some(format!("scan row fetch failed: {}", e)),
                        ),
                    })
                })?;

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
//
// External vector store stub for cloud tier or self-hosted Qdrant.
//
// This stub is reserved for future implementation connecting to:
// - Qdrant Cloud (Azure Marketplace) for paid tier
// - Self-hosted Qdrant or compatible endpoints for enterprise
//
// Cloud tier uses Cohere embed-v3-multilingual embeddings.
// Communication is via HTTPS proxy to api.openakta.dev with token auth.
//
// Future migration path: Turbopuffer via `VectorStore` trait compatibility.
//
// pub struct ExternalVectorStore {
//     endpoint: String,
//     api_key: Option<String>,
//     client: reqwest::Client,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_runtime_matches_bundled_build() {
        assert_eq!(
            rusqlite::version_number(),
            rusqlite::ffi::SQLITE_VERSION_NUMBER
        );
        assert_eq!(rusqlite::version(), bundled_sqlite_version());
    }

    #[test]
    fn test_ensure_sqlite_vec_ready_two_tier_verification() {
        // This test verifies the canonical initialization works correctly
        // Tier A: vec_version() check
        // Tier B: vec0 smoke test

        let result = ensure_sqlite_vec_ready();
        assert!(
            result.is_ok(),
            "sqlite-vec initialization failed: {:?}",
            result.err()
        );

        // Verify it's idempotent - calling again should succeed
        let result2 = ensure_sqlite_vec_ready();
        assert!(
            result2.is_ok(),
            "sqlite-vec re-initialization failed: {:?}",
            result2.err()
        );
    }

    #[test]
    fn test_sqlite_vec_store_requires_init() {
        // SqliteVecStore::new should succeed when canonical init was called
        let temp_path = tempfile::NamedTempFile::new()
            .expect("failed to create temp file")
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        // First initialize (this happens in main() normally)
        ensure_sqlite_vec_ready().expect("init failed");

        // Now create the store
        let store_result = SqliteVecStore::new(&temp_path, 384, 1000);
        assert!(
            store_result.is_ok(),
            "SqliteVecStore::new failed: {:?}",
            store_result.err()
        );
    }

    #[test]
    fn test_sqlite_vec_store_round_trip_search_and_delete() {
        let temp_path = tempfile::NamedTempFile::new()
            .expect("failed to create temp file")
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        ensure_sqlite_vec_ready().expect("init failed");
        let store = SqliteVecStore::new(&temp_path, 1, 1000).expect("store init failed");

        tokio_test::block_on(async {
            store
                .upsert(
                    "doc-1",
                    &[1.0],
                    serde_json::json!({
                        "content": "alpha"
                    }),
                )
                .await
                .expect("upsert failed");

            let hits = store.search(&[1.0], 1, None).await.expect("search failed");
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].id, "doc-1");
            assert!(hits[0].score > 0.99, "unexpected score: {}", hits[0].score);

            let count = store.count().await.expect("count failed");
            assert_eq!(count, 1);

            store.delete("doc-1").await.expect("delete failed");
            let count_after_delete = store.count().await.expect("count after delete failed");
            assert_eq!(count_after_delete, 0);
        });
    }
}
