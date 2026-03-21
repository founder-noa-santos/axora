//! Skill-oriented dense and sparse indexes.

use crate::error::IndexingError;
use crate::vector_store::{DenseVectorCollection, SearchResult as DenseSearchResult};
use crate::Result;
use openakta_embeddings::SkillEmbedder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::TantivyDocument;
use tantivy::schema::{
    Field, IndexRecordOption, OwnedValue, Schema, TextFieldIndexing, TextOptions, STORED, STRING,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};

/// Canonical document used by the skill indexes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillIndexDocument {
    /// Unique skill identifier.
    pub skill_id: String,
    /// Human-readable title.
    pub title: String,
    /// Compact summary used in prompt payloads.
    pub summary: String,
    /// Markdown body with frontmatter removed.
    pub body_markdown: String,
    /// Source path on disk.
    pub source_path: String,
    /// Domain or category.
    pub domain: String,
    /// Retrieval tags.
    pub tags: Vec<String>,
}

/// Dense hit from the dense skill collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DenseSkillHit {
    /// Skill identifier.
    pub skill_id: String,
    /// 1-based rank in the dense list.
    pub rank: u32,
    /// Raw dense score.
    pub score: f32,
}

/// Sparse hit from Tantivy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SparseSkillHit {
    /// Skill identifier.
    pub skill_id: String,
    /// 1-based rank in the sparse list.
    pub rank: u32,
    /// Raw BM25 score.
    pub score: f32,
}

/// Dense skill index backed by an injected collection and skill embedder.
pub struct SkillDenseIndex {
    collection: Arc<dyn DenseVectorCollection>,
    embedder: Arc<dyn SkillEmbedder>,
}

impl SkillDenseIndex {
    /// Create a dense skill index.
    pub fn new(
        collection: Arc<dyn DenseVectorCollection>,
        embedder: Arc<dyn SkillEmbedder>,
    ) -> Result<Self> {
        if collection.spec().dimensions != embedder.profile().dimensions {
            return Err(IndexingError::DimensionMismatch {
                expected: collection.spec().dimensions,
                actual: embedder.profile().dimensions,
            }
            .into());
        }

        Ok(Self {
            collection,
            embedder,
        })
    }

    /// Upsert a skill embedding.
    pub async fn upsert(&self, document: &SkillIndexDocument) -> Result<()> {
        let embedding = self
            .embedder
            .embed(&format!(
                "{}\n{}\n{}\n{}",
                document.title, document.summary, document.domain, document.body_markdown
            ))
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        self.collection
            .upsert(
                &document.skill_id,
                &embedding,
                serde_json::json!({
                    "skill_id": document.skill_id,
                    "source_path": document.source_path,
                    "title": document.title,
                    "domain": document.domain,
                    "tags": document.tags,
                }),
            )
            .await
    }

    /// Delete a skill embedding.
    pub async fn delete(&self, skill_id: &str) -> Result<()> {
        self.collection.delete(skill_id).await
    }

    /// Search for top-k similar skills.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<DenseSkillHit>> {
        let query_embedding = self
            .embedder
            .embed(query)
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let hits = self.collection.search(&query_embedding, limit).await?;
        Ok(hits
            .into_iter()
            .enumerate()
            .map(|(index, hit)| DenseSkillHit {
                skill_id: dense_hit_id(&hit),
                rank: (index + 1) as u32,
                score: hit.score,
            })
            .collect())
    }
}

fn dense_hit_id(hit: &DenseSearchResult) -> String {
    hit.payload
        .get("skill_id")
        .and_then(|value| value.as_str())
        .unwrap_or(&hit.id)
        .to_string()
}

/// Tantivy-backed sparse skill index.
pub struct TantivySkillIndex {
    index: Index,
    reader: IndexReader,
    schema_fields: SkillSchemaFields,
    writer_memory_bytes: usize,
    root: PathBuf,
}

#[derive(Clone)]
struct SkillSchemaFields {
    skill_id: Field,
    title: Field,
    summary: Field,
    domain: Field,
    tags: Field,
    body_markdown: Field,
}

impl TantivySkillIndex {
    /// Create or open a sparse skill index at the given directory.
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(IndexingError::Io)?;

        let (schema, fields) = build_schema();
        let index = match Index::open_in_dir(&root) {
            Ok(index) => index,
            Err(_) => Index::create_in_dir(&root, schema).map_err(IndexingError::Tantivy)?,
        };
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(IndexingError::Tantivy)?;

        Ok(Self {
            index,
            reader,
            schema_fields: fields,
            writer_memory_bytes: 50_000_000,
            root,
        })
    }

    /// Upsert a document into the BM25 index.
    pub fn upsert(&self, document: &SkillIndexDocument) -> Result<()> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(self.writer_memory_bytes)
            .map_err(IndexingError::Tantivy)?;
        writer.delete_term(Term::from_field_text(
            self.schema_fields.skill_id,
            &document.skill_id,
        ));
        writer
            .add_document(doc!(
                self.schema_fields.skill_id => document.skill_id.clone(),
                self.schema_fields.title => document.title.clone(),
                self.schema_fields.summary => document.summary.clone(),
                self.schema_fields.domain => document.domain.clone(),
                self.schema_fields.body_markdown => document.body_markdown.clone(),
                self.schema_fields.tags => document.tags.join(" "),
            ))
            .map_err(IndexingError::Tantivy)?;
        writer.commit().map_err(IndexingError::Tantivy)?;
        self.reader.reload().map_err(IndexingError::Tantivy)?;
        Ok(())
    }

    /// Delete a document from the BM25 index.
    pub fn delete(&self, skill_id: &str) -> Result<()> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(self.writer_memory_bytes)
            .map_err(IndexingError::Tantivy)?;
        writer.delete_term(Term::from_field_text(self.schema_fields.skill_id, skill_id));
        writer.commit().map_err(IndexingError::Tantivy)?;
        self.reader.reload().map_err(IndexingError::Tantivy)?;
        Ok(())
    }

    /// Search the BM25 index.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SparseSkillHit>> {
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(
            &self.index,
            vec![
                self.schema_fields.title,
                self.schema_fields.summary,
                self.schema_fields.tags,
                self.schema_fields.body_markdown,
            ],
        );
        let parsed = parser
            .parse_query(query)
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let terms = query
            .split_whitespace()
            .map(|term| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(self.schema_fields.skill_id, term),
                        IndexRecordOption::Basic,
                    )) as Box<dyn tantivy::query::Query>,
                )
            })
            .collect::<Vec<_>>();
        let combined = BooleanQuery::new(
            std::iter::once((
                Occur::Should,
                Box::new(parsed) as Box<dyn tantivy::query::Query>,
            ))
            .chain(terms)
            .collect(),
        );
        let docs = searcher
            .search(&combined, &TopDocs::with_limit(limit))
            .map_err(IndexingError::Tantivy)?;

        let mut hits = Vec::new();
        for (index, (score, address)) in docs.into_iter().enumerate() {
            let retrieved: TantivyDocument =
                searcher.doc(address).map_err(IndexingError::Tantivy)?;
            let Some(OwnedValue::Str(skill_id)) =
                retrieved.get_first(self.schema_fields.skill_id).cloned()
            else {
                continue;
            };
            hits.push(SparseSkillHit {
                skill_id,
                rank: (index + 1) as u32,
                score,
            });
        }
        Ok(hits)
    }

    /// Root directory of this index.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn build_schema() -> (Schema, SkillSchemaFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let skill_id = builder.add_text_field("skill_id", STRING | STORED);
    let title = builder.add_text_field("title", text_options.clone());
    let summary = builder.add_text_field("summary", text_options.clone());
    let domain = builder.add_text_field("domain", text_options.clone());
    let tags = builder.add_text_field("tags", text_options.clone());
    let body_markdown = builder.add_text_field("body_markdown", text_options);
    (
        builder.build(),
        SkillSchemaFields {
            skill_id,
            title,
            summary,
            domain,
            tags,
            body_markdown,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_store::{CollectionSpec, DualVectorStore};
    use openakta_embeddings::{BgeSkillEmbedder, SkillEmbeddingConfig};
    use tempfile::tempdir;

    #[tokio::test]
    async fn dense_skill_index_uses_skill_embedding_dimensions() {
        let dir = tempdir().unwrap();
        let store = DualVectorStore::new_sqlite(
            dir.path().join("vectors.db"),
            CollectionSpec::code_default(),
            CollectionSpec::skill_default(),
        )
        .unwrap();
        let index = SkillDenseIndex::new(
            store.skill_collection(),
            Arc::new(BgeSkillEmbedder::new(SkillEmbeddingConfig::default()).unwrap()),
        )
        .unwrap();

        index
            .upsert(&SkillIndexDocument {
                skill_id: "skill-1".to_string(),
                title: "Cargo Repair".to_string(),
                summary: "Recover a broken cargo workspace".to_string(),
                body_markdown: "Run cargo metadata and inspect the lockfile.".to_string(),
                source_path: "skills/CARGO_REPAIR/SKILL.md".to_string(),
                domain: "rust".to_string(),
                tags: vec!["cargo".to_string()],
            })
            .await
            .unwrap();

        let hits = index.search("cargo repair workspace", 5).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].skill_id, "skill-1");
    }
}
