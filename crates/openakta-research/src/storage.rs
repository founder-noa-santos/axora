//! Local SQLite storage for research sessions, FTS5 keyword index, and embedding BLOBs.
//!
//! Semantic recall ranks by cosine similarity **while streaming rows** from SQLite: we keep only
//! a min-heap of size `limit` plus one row buffer (no full-table `Vec` of all candidates).

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use openakta_embeddings::{EmbeddingProvider, RESEARCH_EMBED_BYTES, RESEARCH_EMBED_DIM};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::types::SearchResult;
use crate::vector_math::{cosine_similarity_with_norms, l2_norm};

/// Errors from [`ResearchStorage`].
#[derive(Debug, Error)]
pub enum ResearchStorageError {
    /// Filesystem failure (e.g. creating DB parent directory).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// SQLite failure.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Embedding dimension mismatch.
    #[error("embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        actual: usize,
    },
    /// Invalid embedding BLOB.
    #[error("invalid embedding blob: {0}")]
    InvalidBlob(String),
    /// Embedding provider failure.
    #[error("embed: {0}")]
    Embed(#[from] anyhow::Error),
}

/// Result type for research storage operations.
pub type Result<T> = std::result::Result<T, ResearchStorageError>;

/// Local research memory (sessions + results + vectors + FTS).
pub struct ResearchStorage {
    conn: Connection,
}

struct PreparedSearchResultRow {
    rank: usize,
    title: String,
    url: String,
    snippet: String,
    blob: Vec<u8>,
    hash: String,
}

impl ResearchStorage {
    /// Open or create the database at `path`, apply schema, enable WAL and foreign keys.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;",
        )?;
        Self::apply_schema(&conn)?;
        Ok(Self { conn })
    }

    fn apply_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(include_str!("../migrations/research_v1.sql"))?;
        Ok(())
    }

    /// Insert a session and its search results with embeddings computed via `embedder`.
    ///
    /// All `embed_text` calls complete **before** the SQLite transaction opens, avoiding WAL
    /// contention while inference runs.
    pub fn append_session(
        &mut self,
        workspace_root: &str,
        query_text: &str,
        provider_used: Option<&str>,
        results: &[SearchResult],
        embedder: &dyn EmbeddingProvider,
    ) -> Result<String> {
        let prepared = Self::prepare_search_rows(results, embedder)?;
        self.append_session_prepared(workspace_root, query_text, provider_used, &prepared)
    }

    /// Like [`Self::append_session`], but runs each `embed_text` on the blocking thread pool so
    /// Candle/ONNX inference cannot starve async Tokio workers.
    pub async fn append_session_async(
        &mut self,
        workspace_root: &str,
        query_text: &str,
        provider_used: Option<&str>,
        results: &[SearchResult],
        embedder: std::sync::Arc<dyn EmbeddingProvider + Send + Sync>,
    ) -> Result<String> {
        let mut prepared = Vec::with_capacity(results.len());
        for (rank, hit) in results.iter().enumerate() {
            let canonical = embedder.canonicalize(&hit.title, &hit.url, &hit.snippet);
            let hash = sha256_hex(canonical.as_bytes());
            let embedding = tokio::task::spawn_blocking({
                let embedder = embedder.clone();
                let canonical = canonical.clone();
                move || embedder.embed_text(&canonical)
            })
            .await
            .map_err(|e| anyhow::anyhow!("join embedding task: {e}"))??;
            if embedding.len() != embedder.dimensions() {
                return Err(ResearchStorageError::DimensionMismatch {
                    expected: embedder.dimensions(),
                    actual: embedding.len(),
                });
            }
            let blob = embedding_to_blob(&embedding)?;
            prepared.push(PreparedSearchResultRow {
                rank,
                title: hit.title.clone(),
                url: hit.url.clone(),
                snippet: hit.snippet.clone(),
                blob,
                hash,
            });
        }
        self.append_session_prepared(workspace_root, query_text, provider_used, &prepared)
    }

    fn prepare_search_rows(
        results: &[SearchResult],
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Vec<PreparedSearchResultRow>> {
        let mut rows = Vec::with_capacity(results.len());
        for (rank, hit) in results.iter().enumerate() {
            let canonical = embedder.canonicalize(&hit.title, &hit.url, &hit.snippet);
            let hash = sha256_hex(canonical.as_bytes());
            let embedding = embedder.embed_text(&canonical)?;
            if embedding.len() != embedder.dimensions() {
                return Err(ResearchStorageError::DimensionMismatch {
                    expected: embedder.dimensions(),
                    actual: embedding.len(),
                });
            }
            let blob = embedding_to_blob(&embedding)?;
            rows.push(PreparedSearchResultRow {
                rank,
                title: hit.title.clone(),
                url: hit.url.clone(),
                snippet: hit.snippet.clone(),
                blob,
                hash,
            });
        }
        Ok(rows)
    }

    fn append_session_prepared(
        &mut self,
        workspace_root: &str,
        query_text: &str,
        provider_used: Option<&str>,
        prepared: &[PreparedSearchResultRow],
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = now_ms();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO research_sessions (id, workspace_root, query_text, created_at_ms, provider_used, raw_metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                &session_id,
                workspace_root,
                query_text,
                now as i64,
                provider_used,
            ],
        )?;
        for row in prepared {
            tx.execute(
                "INSERT INTO search_results (session_id, rank_in_session, title, url, snippet, embedding, embedded_text_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &session_id,
                    row.rank as i64,
                    &row.title,
                    &row.url,
                    &row.snippet,
                    &row.blob,
                    &row.hash,
                ],
            )?;
        }
        tx.commit()?;
        Ok(session_id)
    }

    /// Offline semantic search: cosine similarity vs stored embeddings.
    ///
    /// Rows are processed **one at a time** from SQLite; only a heap of size `limit` and one
    /// decoded embedding buffer are retained (no `Vec` of all rows).
    pub fn search_historical_research(
        &self,
        workspace_root: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        if query_embedding.len() != RESEARCH_EMBED_DIM {
            return Err(ResearchStorageError::DimensionMismatch {
                expected: RESEARCH_EMBED_DIM,
                actual: query_embedding.len(),
            });
        }
        if limit == 0 {
            return Ok(Vec::new());
        }

        let q_norm = l2_norm(query_embedding);
        let mut heap: BinaryHeap<Reverse<HeapItem>> = BinaryHeap::new();

        {
            let mut stmt = self.conn.prepare(
                "SELECT r.title, r.url, r.snippet, r.embedding
                 FROM search_results r
                 JOIN research_sessions s ON r.session_id = s.id
                 WHERE s.workspace_root = ?1",
            )?;
            let rows = stmt.query_map(params![workspace_root], |row| {
                Ok(CandidateRow {
                    title: row.get(0)?,
                    url: row.get(1)?,
                    snippet: row.get(2)?,
                    blob: row.get(3)?,
                })
            })?;

            for row in rows {
                let c = row?;
                let doc = blob_to_embedding(&c.blob)?;
                let d_norm = l2_norm(&doc);
                let mut score =
                    cosine_similarity_with_norms(query_embedding, q_norm, &doc, d_norm);
                if !score.is_finite() {
                    score = 0.0;
                }

                let item = HeapItem {
                    score,
                    title: c.title,
                    url: c.url,
                    snippet: c.snippet,
                };

                if heap.len() < limit {
                    heap.push(Reverse(item));
                } else if let Some(Reverse(min_item)) = heap.peek() {
                    if item.score > min_item.score {
                        heap.pop();
                        heap.push(Reverse(item));
                    }
                }
            }
        } // `stmt` dropped: no statement held during post-processing

        let mut picked: Vec<HeapItem> = heap.into_iter().map(|Reverse(h)| h).collect();
        picked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.url.cmp(&b.url))
        });

        Ok(picked
            .into_iter()
            .map(|h| SearchResult {
                title: h.title,
                url: h.url,
                snippet: h.snippet,
            })
            .collect())
    }

    /// Keyword search via FTS5 (no embedding required).
    pub fn search_keywords(
        &self,
        workspace_root: &str,
        fts_query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.title, r.url, r.snippet
             FROM search_results_fts
             JOIN search_results AS r ON r.id = search_results_fts.rowid
             JOIN research_sessions AS s ON r.session_id = s.id
             WHERE s.workspace_root = ?1 AND search_results_fts MATCH ?2
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![workspace_root, fts_query, limit as i64], |row| {
            Ok(SearchResult {
                title: row.get(0)?,
                url: row.get(1)?,
                snippet: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

struct CandidateRow {
    title: String,
    url: String,
    snippet: String,
    blob: Vec<u8>,
}

/// Heap entry: `Ord` only by `score` (used inside `Reverse` for a bounded min-heap of top scores).
#[derive(Debug, Clone)]
struct HeapItem {
    score: f32,
    title: String,
    url: String,
    snippet: String,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.score.to_bits() == other.score.to_bits()
    }
}

impl Eq for HeapItem {}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.score.total_cmp(&other.score))
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.total_cmp(&other.score)
    }
}

fn now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

/// Serialize `f32` embedding to little-endian BLOB bytes.
pub fn embedding_to_blob(v: &[f32]) -> Result<Vec<u8>> {
    if v.len() != RESEARCH_EMBED_DIM {
        return Err(ResearchStorageError::DimensionMismatch {
            expected: RESEARCH_EMBED_DIM,
            actual: v.len(),
        });
    }
    let mut out = Vec::with_capacity(RESEARCH_EMBED_BYTES);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    Ok(out)
}

/// Decode BLOB to an embedding array (portable, alignment-safe).
pub fn blob_to_embedding(blob: &[u8]) -> Result<[f32; RESEARCH_EMBED_DIM]> {
    if blob.len() != RESEARCH_EMBED_BYTES {
        return Err(ResearchStorageError::InvalidBlob(format!(
            "expected {} bytes, got {}",
            RESEARCH_EMBED_BYTES,
            blob.len()
        )));
    }
    let mut out = [0f32; RESEARCH_EMBED_DIM];
    for (i, chunk) in blob.chunks_exact(4).enumerate() {
        let arr: [u8; 4] = chunk.try_into().expect("chunks_exact(4)");
        out[i] = f32::from_le_bytes(arr);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_math::{cosine_similarity, dot_f32};

    fn unit_axis(dim: usize, axis: usize) -> Vec<f32> {
        let mut v = vec![0f32; dim];
        v[axis] = 1.0;
        v
    }

    #[test]
    fn blob_roundtrip() {
        let v = unit_axis(RESEARCH_EMBED_DIM, 3);
        let blob = embedding_to_blob(&v).unwrap();
        let back = blob_to_embedding(&blob).unwrap();
        assert_eq!(back[3], 1.0);
        assert_eq!(back[0], 0.0);
    }

    #[test]
    fn top_k_prefers_higher_cosine() {
        let q = unit_axis(RESEARCH_EMBED_DIM, 0);
        let qn = l2_norm(&q);
        let rows = vec![
            CandidateRow {
                title: "best".into(),
                url: "u0".into(),
                snippet: "s".into(),
                blob: embedding_to_blob(&unit_axis(RESEARCH_EMBED_DIM, 0)).unwrap(),
            },
            CandidateRow {
                title: "worse".into(),
                url: "u1".into(),
                snippet: "s".into(),
                blob: embedding_to_blob(&unit_axis(RESEARCH_EMBED_DIM, 1)).unwrap(),
            },
        ];
        let mut heap: BinaryHeap<Reverse<HeapItem>> = BinaryHeap::new();
        let limit = 1;
        for c in rows {
            let doc = blob_to_embedding(&c.blob).unwrap();
            let dn = l2_norm(&doc);
            let score = cosine_similarity_with_norms(&q, qn, &doc, dn);
            let item = HeapItem {
                score,
                title: c.title,
                url: c.url,
                snippet: c.snippet,
            };
            if heap.len() < limit {
                heap.push(Reverse(item));
            } else if let Some(Reverse(top)) = heap.peek() {
                if item.score > top.score {
                    heap.pop();
                    heap.push(Reverse(item));
                }
            }
        }
        let out: Vec<_> = heap.into_iter().map(|Reverse(h)| h).collect();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "best");
    }

    #[test]
    fn cosine_matches_vector_math_module() {
        let q = vec![1f32, 0., 0., 1.];
        let d = vec![0f32, 1., 0., 0.];
        let a = cosine_similarity(&q, &d);
        let b = {
            let qn = l2_norm(&q);
            let dn = l2_norm(&d);
            cosine_similarity_with_norms(&q, qn, &d, dn)
        };
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn dot_matches_manual_384() {
        let mut a = vec![0f32; RESEARCH_EMBED_DIM];
        let mut b = vec![0f32; RESEARCH_EMBED_DIM];
        a[0] = 2.0;
        a[1] = 3.0;
        b[0] = 5.0;
        b[1] = 7.0;
        assert!((dot_f32(&a, &b) - (10.0 + 21.0)).abs() < 1e-5);
    }

    #[test]
    fn search_historical_empty_db() {
        let dir = tempfile::tempdir().unwrap();
        let store = ResearchStorage::open(dir.path().join("db.sqlite")).unwrap();
        let q = unit_axis(RESEARCH_EMBED_DIM, 0);
        let out = store
            .search_historical_research("/ws", &q, 5)
            .unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn search_historical_limit_zero() {
        let dir = tempfile::tempdir().unwrap();
        let store = ResearchStorage::open(dir.path().join("db.sqlite")).unwrap();
        let q = unit_axis(RESEARCH_EMBED_DIM, 0);
        let out = store.search_historical_research("/ws", &q, 0).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn search_historical_ranks_three_rows() {
        use openakta_embeddings::DeterministicTestEmbeddingProvider;

        let dir = tempfile::tempdir().unwrap();
        let mut store = ResearchStorage::open(dir.path().join("db.sqlite")).unwrap();
        let embedder = DeterministicTestEmbeddingProvider::new();

        let hits = vec![
            SearchResult {
                title: "A".into(),
                url: "https://a".into(),
                snippet: "alpha".into(),
            },
            SearchResult {
                title: "B".into(),
                url: "https://b".into(),
                snippet: "beta".into(),
            },
            SearchResult {
                title: "C".into(),
                url: "https://c".into(),
                snippet: "gamma".into(),
            },
        ];
        store
            .append_session("/ws", "q", None, &hits, &embedder)
            .unwrap();

        let first = &hits[0];
        let canon = embedder.canonicalize(&first.title, &first.url, &first.snippet);
        let query = embedder.embed_text(&canon).unwrap();
        let out = store.search_historical_research("/ws", &query, 2).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].url, first.url, "exact query vector must match stored row first");
    }
}
