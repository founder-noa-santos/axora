//! Code-oriented dense and sparse indexes.

use crate::error::IndexingError;
use crate::vector_store::{DenseVectorCollection, SearchResult as DenseSearchResult};
use crate::Result;
use openakta_embeddings::CodeEmbedder;
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

/// Canonical chunk document used by the code indexes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeIndexDocument {
    pub chunk_id: String,
    pub file_path: String,
    pub symbol_path: Option<String>,
    pub summary: String,
    pub body_markdown: String,
    pub language: Option<String>,
    pub chunk_type: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub token_cost: usize,
}

/// Dense hit from the dense code collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DenseCodeHit {
    pub chunk_id: String,
    pub rank: u32,
    pub score: f32,
}

/// Sparse hit from Tantivy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SparseCodeHit {
    pub chunk_id: String,
    pub rank: u32,
    pub score: f32,
}

/// Dense code index backed by an injected collection and code embedder.
pub struct CodeDenseIndex {
    collection: Arc<dyn DenseVectorCollection>,
    embedder: Arc<dyn CodeEmbedder>,
}

impl CodeDenseIndex {
    pub fn new(
        collection: Arc<dyn DenseVectorCollection>,
        embedder: Arc<dyn CodeEmbedder>,
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

    pub async fn upsert(&self, document: &CodeIndexDocument) -> Result<()> {
        let embedding = self
            .embedder
            .embed(&format!(
                "{}\n{}\n{}\n{}",
                document.file_path,
                document
                    .symbol_path
                    .clone()
                    .unwrap_or_else(|| document.summary.clone()),
                document.summary,
                document.body_markdown
            ))
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        self.collection
            .upsert(
                &document.chunk_id,
                &embedding,
                serde_json::json!({
                    "chunk_id": document.chunk_id,
                    "file_path": document.file_path,
                    "symbol_path": document.symbol_path,
                    "summary": document.summary,
                    "language": document.language,
                    "chunk_type": document.chunk_type,
                    "start_line": document.start_line,
                    "end_line": document.end_line,
                    "token_cost": document.token_cost,
                }),
            )
            .await
    }

    pub async fn delete(&self, chunk_id: &str) -> Result<()> {
        self.collection.delete(chunk_id).await
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<DenseCodeHit>> {
        let query_embedding = self
            .embedder
            .embed(query)
            .await
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let hits = self.collection.search(&query_embedding, limit).await?;
        Ok(hits
            .into_iter()
            .enumerate()
            .map(|(index, hit)| DenseCodeHit {
                chunk_id: dense_hit_id(&hit),
                rank: (index + 1) as u32,
                score: hit.score,
            })
            .collect())
    }
}

fn dense_hit_id(hit: &DenseSearchResult) -> String {
    hit.payload
        .get("chunk_id")
        .and_then(|value| value.as_str())
        .unwrap_or(&hit.id)
        .to_string()
}

/// Tantivy-backed sparse code index.
pub struct TantivyCodeIndex {
    index: Index,
    reader: IndexReader,
    schema_fields: CodeSchemaFields,
    writer_memory_bytes: usize,
    root: PathBuf,
}

#[derive(Clone)]
struct CodeSchemaFields {
    chunk_id: Field,
    file_path: Field,
    symbol_path: Field,
    summary: Field,
    body_markdown: Field,
    language: Field,
    chunk_type: Field,
    start_line: Field,
    end_line: Field,
    token_cost: Field,
}

impl TantivyCodeIndex {
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

    pub fn upsert(&self, document: &CodeIndexDocument) -> Result<()> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(self.writer_memory_bytes)
            .map_err(IndexingError::Tantivy)?;
        writer.delete_term(Term::from_field_text(
            self.schema_fields.chunk_id,
            &document.chunk_id,
        ));
        writer
            .add_document(doc!(
                self.schema_fields.chunk_id => document.chunk_id.clone(),
                self.schema_fields.file_path => document.file_path.clone(),
                self.schema_fields.symbol_path => document.symbol_path.clone().unwrap_or_default(),
                self.schema_fields.summary => document.summary.clone(),
                self.schema_fields.body_markdown => document.body_markdown.clone(),
                self.schema_fields.language => document.language.clone().unwrap_or_default(),
                self.schema_fields.chunk_type => document.chunk_type.clone().unwrap_or_default(),
                self.schema_fields.start_line => document.start_line as u64,
                self.schema_fields.end_line => document.end_line as u64,
                self.schema_fields.token_cost => document.token_cost as u64,
            ))
            .map_err(IndexingError::Tantivy)?;
        writer.commit().map_err(IndexingError::Tantivy)?;
        self.reader.reload().map_err(IndexingError::Tantivy)?;
        Ok(())
    }

    pub fn delete(&self, chunk_id: &str) -> Result<()> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(self.writer_memory_bytes)
            .map_err(IndexingError::Tantivy)?;
        writer.delete_term(Term::from_field_text(self.schema_fields.chunk_id, chunk_id));
        writer.commit().map_err(IndexingError::Tantivy)?;
        self.reader.reload().map_err(IndexingError::Tantivy)?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SparseCodeHit>> {
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(
            &self.index,
            vec![
                self.schema_fields.file_path,
                self.schema_fields.symbol_path,
                self.schema_fields.summary,
                self.schema_fields.body_markdown,
                self.schema_fields.language,
                self.schema_fields.chunk_type,
            ],
        );
        let parsed = parser
            .parse_query(query)
            .map_err(|err| IndexingError::VectorStore(err.to_string()))?;
        let terms = query
            .split_whitespace()
            .flat_map(|term| {
                [
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(
                            Term::from_field_text(self.schema_fields.chunk_id, term),
                            IndexRecordOption::Basic,
                        )) as Box<dyn tantivy::query::Query>,
                    ),
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(
                            Term::from_field_text(self.schema_fields.file_path, term),
                            IndexRecordOption::Basic,
                        )) as Box<dyn tantivy::query::Query>,
                    ),
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(
                            Term::from_field_text(self.schema_fields.symbol_path, term),
                            IndexRecordOption::Basic,
                        )) as Box<dyn tantivy::query::Query>,
                    ),
                ]
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
            let Some(OwnedValue::Str(chunk_id)) =
                retrieved.get_first(self.schema_fields.chunk_id).cloned()
            else {
                continue;
            };
            hits.push(SparseCodeHit {
                chunk_id,
                rank: (index + 1) as u32,
                score,
            });
        }
        Ok(hits)
    }

    pub fn get_document(&self, chunk_id: &str) -> Result<Option<CodeIndexDocument>> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema_fields.chunk_id, chunk_id),
            IndexRecordOption::Basic,
        );
        let docs = searcher
            .search(&query, &TopDocs::with_limit(1))
            .map_err(IndexingError::Tantivy)?;
        let Some((_, address)) = docs.into_iter().next() else {
            return Ok(None);
        };
        let retrieved: TantivyDocument = searcher.doc(address).map_err(IndexingError::Tantivy)?;
        Ok(Some(CodeIndexDocument {
            chunk_id: owned_string(retrieved.get_first(self.schema_fields.chunk_id))
                .unwrap_or_default(),
            file_path: owned_string(retrieved.get_first(self.schema_fields.file_path))
                .unwrap_or_default(),
            symbol_path: owned_string(retrieved.get_first(self.schema_fields.symbol_path))
                .filter(|value| !value.is_empty()),
            summary: owned_string(retrieved.get_first(self.schema_fields.summary))
                .unwrap_or_default(),
            body_markdown: owned_string(retrieved.get_first(self.schema_fields.body_markdown))
                .unwrap_or_default(),
            language: owned_string(retrieved.get_first(self.schema_fields.language))
                .filter(|value| !value.is_empty()),
            chunk_type: owned_string(retrieved.get_first(self.schema_fields.chunk_type))
                .filter(|value| !value.is_empty()),
            start_line: owned_usize(retrieved.get_first(self.schema_fields.start_line))
                .unwrap_or(1),
            end_line: owned_usize(retrieved.get_first(self.schema_fields.end_line)).unwrap_or(1),
            token_cost: owned_usize(retrieved.get_first(self.schema_fields.token_cost))
                .unwrap_or(0),
        }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn build_schema() -> (Schema, CodeSchemaFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let chunk_id = builder.add_text_field("chunk_id", STRING | STORED);
    let file_path = builder.add_text_field("file_path", text_options.clone().set_stored());
    let symbol_path = builder.add_text_field("symbol_path", text_options.clone().set_stored());
    let summary = builder.add_text_field("summary", text_options.clone().set_stored());
    let body_markdown = builder.add_text_field("body_markdown", text_options.clone().set_stored());
    let language = builder.add_text_field("language", text_options.clone().set_stored());
    let chunk_type = builder.add_text_field("chunk_type", text_options.clone().set_stored());
    let start_line = builder.add_u64_field("start_line", STORED);
    let end_line = builder.add_u64_field("end_line", STORED);
    let token_cost = builder.add_u64_field("token_cost", STORED);
    (
        builder.build(),
        CodeSchemaFields {
            chunk_id,
            file_path,
            symbol_path,
            summary,
            body_markdown,
            language,
            chunk_type,
            start_line,
            end_line,
            token_cost,
        },
    )
}

fn owned_string(value: Option<&OwnedValue>) -> Option<String> {
    match value.cloned()? {
        OwnedValue::Str(value) => Some(value),
        OwnedValue::PreTokStr(value) => Some(value.text),
        _ => None,
    }
}

fn owned_usize(value: Option<&OwnedValue>) -> Option<usize> {
    match value.cloned()? {
        OwnedValue::U64(value) => Some(value as usize),
        OwnedValue::I64(value) if value >= 0 => Some(value as usize),
        _ => None,
    }
}
