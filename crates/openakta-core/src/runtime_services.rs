//! Shared runtime-managed services for daemon and CLI entrypoints.

use openakta_docs::{DocReconciler, DocReconcilerConfig, ReconcileDecision};
use openakta_embeddings::{BgeSkillEmbedder, JinaCodeEmbedder};
use openakta_indexing::{DualVectorStore, MerkleTree, TantivySkillIndex};
use openakta_memory::{
    builtin_skill_root, ConsolidationPipeline, ConsolidationWorker, DocType, EpisodicStore,
    EpisodicStoreConfig, HybridSkillIndex, LightweightLLM, MemoryLifecycle,
    PersistentSemanticStore, SemanticMemory, SemanticMetadata, SkillCatalog, SkillCorpusIngestor,
    SkillRetrievalConfig, SkillRetrievalPipeline, SqliteVecStore, VectorStore,
};
use openakta_rag::{CodeRetrievalPipeline, OpenaktaReranker};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::{CoreConfig, SemanticVectorBackend};

/// Memory services hosted by the OPENAKTA runtime.
pub struct MemoryServices {
    /// Episodic store used by agents and lifecycle workers.
    pub episodic_store: Arc<EpisodicStore>,
    /// SQLite catalog for canonical skill documents.
    pub skill_catalog: Arc<SkillCatalog>,
    /// End-to-end pull-based retrieval pipeline.
    pub skill_retrieval: Arc<SkillRetrievalPipeline>,
    /// Dense code retrieval pipeline.
    pub code_retrieval: Arc<CodeRetrievalPipeline>,
    /// Persistent semantic store for synced docs and retrieved knowledge.
    pub semantic_store: Arc<PersistentSemanticStore>,
    /// Vector store trait object (Phase 1+ seam for backend swaps).
    pub vector_store: Arc<dyn VectorStore>,
}

impl MemoryServices {
    /// Create runtime-managed memory services and sync the built-in skill corpus.
    pub async fn new(config: &CoreConfig) -> anyhow::Result<Self> {
        let episodic_path = config.database_path.with_extension("episodic.db");
        info!("Opening episodic store at: {}", episodic_path.display());
        let episodic_store = EpisodicStore::new(EpisodicStoreConfig::persistent(
            &episodic_path.display().to_string(),
        ))
        .await?;
        let skill_catalog_path = config.skill_index_root.join("skill-catalog.db");
        info!("Opening skill catalog at: {}", skill_catalog_path.display());
        let skill_catalog = SkillCatalog::new(&skill_catalog_path)?;
        SkillCorpusIngestor::sync_builtin_skills(&skill_catalog, builtin_skill_root())
            .await
            .map_err(anyhow::Error::msg)?;
        info!(
            "Opening dual vector store (backend: {:?})",
            config.retrieval.backend
        );
        let dual_store = match config.retrieval.backend {
            openakta_indexing::VectorBackendKind::Qdrant => DualVectorStore::new_qdrant(
                &config.retrieval.qdrant_url,
                config.retrieval.code.collection_spec(),
                config.retrieval.skills.collection_spec(),
            )
            .await
            .map_err(anyhow::Error::msg)?,
            openakta_indexing::VectorBackendKind::SqliteJson => DualVectorStore::new_sqlite(
                &config.retrieval.sqlite_path,
                config.retrieval.code.collection_spec(),
                config.retrieval.skills.collection_spec(),
            )
            .map_err(anyhow::Error::msg)?,
        };
        let skill_embedder = Arc::new(
            BgeSkillEmbedder::new(config.retrieval.skills.embedding_config())
                .map_err(anyhow::Error::msg)?,
        );
        let code_embedder = Arc::new(
            JinaCodeEmbedder::new(config.retrieval.code.embedding_config())
                .map_err(anyhow::Error::msg)?,
        );
        let skill_config = SkillRetrievalConfig {
            corpus_root: config.retrieval.skills.corpus_root.clone(),
            catalog_db_path: config.retrieval.skills.catalog_db_path.clone(),
            dense_backend: config.retrieval.backend,
            dense_store_path: config.retrieval.sqlite_path.clone(),
            qdrant_url: config.retrieval.qdrant_url.clone(),
            dense_collection: config.retrieval.skills.collection_spec(),
            embedding: config.retrieval.skills.embedding_config(),
            bm25_dir: config.retrieval.skills.bm25_dir.clone(),
            skill_token_budget: config.retrieval.skills.token_budget,
            dense_limit: 64,
            bm25_limit: 64,
        };
        let skill_index = Arc::new(
            HybridSkillIndex::with_components(
                dual_store.skill_collection(),
                skill_embedder,
                TantivySkillIndex::new(&skill_config.bm25_dir).map_err(anyhow::Error::msg)?,
            )
            .map_err(anyhow::Error::msg)?,
        );
        let reranker = OpenaktaReranker::for_workspace(&config.workspace_root);
        let skill_retrieval: SkillRetrievalPipeline<HybridSkillIndex, OpenaktaReranker> =
            SkillRetrievalPipeline::with_components(
                skill_config.clone(),
                skill_catalog.clone(),
                SkillCorpusIngestor::new(&skill_config.corpus_root),
                skill_index,
                reranker,
            )
            .map_err(anyhow::Error::msg)?;
        let code_retrieval = Arc::new(CodeRetrievalPipeline::new(
            dual_store.code_collection(),
            code_embedder,
            OpenaktaReranker::for_workspace(&config.workspace_root),
        ));
        skill_retrieval
            .sync_if_needed()
            .await
            .map_err(anyhow::Error::msg)?;
        info!(
            "Opening semantic store at: {}",
            config.semantic_store_path.display()
        );
        let semantic_store = PersistentSemanticStore::with_scan_cap(
            &config.semantic_store_path,
            384,
            config.semantic_scan_cap,
        )
        .map_err(anyhow::Error::msg)?;

        // Wire up vector store - sqlite-vec is the only supported local backend
        let vector_store: Arc<dyn VectorStore> = match &config.semantic_vector_backend {
            SemanticVectorBackend::SqliteVec => {
                let path_str = config.semantic_store_path.display().to_string();
                let vec_store = SqliteVecStore::new(&path_str, 384, config.semantic_scan_cap)
                    .map_err(anyhow::Error::msg)?;
                Arc::new(vec_store)
            }
            SemanticVectorBackend::External { endpoint, api_key } => {
                return Err(anyhow::anyhow!(
                    "External vector backend not yet implemented: endpoint={}, api_key={}",
                    endpoint,
                    if api_key.is_some() { "present" } else { "none" }
                ));
            }
        };

        #[allow(clippy::arc_with_non_send_sync)]
        let out = Self {
            episodic_store: Arc::new(episodic_store),
            skill_catalog: Arc::new(skill_catalog),
            skill_retrieval: Arc::new(skill_retrieval),
            code_retrieval,
            semantic_store: Arc::new(semantic_store),
            vector_store,
        };
        Ok(out)
    }

    /// Start pruning and consolidation workers.
    pub fn start(&self, config: &CoreConfig) -> Vec<std::thread::JoinHandle<()>> {
        let pruning_interval = Duration::from_secs(config.pruning_interval_secs);
        let episodic_path = config.database_path.with_extension("episodic.db");
        let pruning_handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("pruning runtime");
            runtime.block_on(async move {
                let lifecycle = MemoryLifecycle::with_defaults();
                loop {
                    match EpisodicStore::new(EpisodicStoreConfig::persistent(
                        &episodic_path.display().to_string(),
                    ))
                    .await
                    {
                        Ok(store) => match store.list_all().await {
                            Ok(memories) => {
                                let report = lifecycle
                                    .prune(memories, |memory| {
                                        tokio::runtime::Handle::current()
                                            .block_on(store.delete(&memory.id))
                                            .map_err(|e| {
                                                openakta_memory::lifecycle::LifecycleError::Io(
                                                    std::io::Error::other(e.to_string()),
                                                )
                                            })
                                    })
                                    .await;
                                if let Err(err) = report {
                                    error!("Pruning iteration failed: {}", err);
                                }
                            }
                            Err(err) => error!("Failed to list episodic memories: {}", err),
                        },
                        Err(err) => error!("Failed to open episodic store for pruning: {}", err),
                    }
                    tokio::time::sleep(pruning_interval).await;
                }
            });
        });

        let episodic_path = config.database_path.with_extension("episodic.db");
        let skill_catalog_path = config.skill_index_root.join("skill-catalog.db");
        let skill_corpus_root = config.retrieval.skills.corpus_root.clone();
        let interval = Duration::from_secs(config.pruning_interval_secs.max(60));
        let consolidation_handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("consolidation runtime");
            runtime.block_on(async move {
                let episodic_store = match EpisodicStore::new(EpisodicStoreConfig::persistent(
                    &episodic_path.display().to_string(),
                ))
                .await
                {
                    Ok(store) => store,
                    Err(err) => {
                        error!("Failed to start consolidation episodic store: {}", err);
                        return;
                    }
                };
                let skill_catalog = match SkillCatalog::new(skill_catalog_path) {
                    Ok(catalog) => catalog,
                    Err(err) => {
                        error!("Failed to open skill catalog: {}", err);
                        return;
                    }
                };

                let pipeline = ConsolidationPipeline::new(
                    episodic_store,
                    skill_catalog,
                    LightweightLLM::new("claude-haiku"),
                    skill_corpus_root,
                );
                let worker = ConsolidationWorker::new(pipeline, interval);
                worker.start().await;
            });
        });

        vec![pruning_handle, consolidation_handle]
    }
}

/// Polling LivingDocs sync service backed by a Merkle diff.
pub struct DocSyncService {
    root: PathBuf,
    reconciler: DocReconciler,
    semantic_store: Arc<PersistentSemanticStore>,
    last_tree: Option<MerkleTree>,
}

impl DocSyncService {
    /// Create a new doc sync service.
    pub fn new(config: &CoreConfig) -> anyhow::Result<Self> {
        #[allow(clippy::arc_with_non_send_sync)]
        let out = Self {
            root: config.workspace_root.clone(),
            reconciler: DocReconciler::new(DocReconcilerConfig::new(config.workspace_root.clone())),
            semantic_store: Arc::new(
                PersistentSemanticStore::with_scan_cap(
                    &config.semantic_store_path,
                    384,
                    config.semantic_scan_cap,
                )
                .map_err(anyhow::Error::msg)?,
            ),
            last_tree: None,
        };
        Ok(out)
    }

    /// Run one reconciliation pass.
    pub fn sync_once(&mut self) -> anyhow::Result<usize> {
        let next_tree = MerkleTree::build(&self.root)?;
        let changed = if let Some(previous) = &self.last_tree {
            next_tree.find_changed(previous)
        } else {
            next_tree.file_hashes.keys().cloned().collect()
        };

        let mut synced = 0usize;
        for relative_path in changed {
            let absolute_path = self.root.join(&relative_path);
            if !absolute_path.is_file() {
                continue;
            }

            let new_content = match std::fs::read_to_string(&absolute_path) {
                Ok(content) => content,
                Err(err) => {
                    warn!(
                        "Skipping non UTF-8 file {}: {}",
                        absolute_path.display(),
                        err
                    );
                    continue;
                }
            };

            let (decision, patches) =
                self.reconciler
                    .reconcile_change(relative_path.as_path(), "", &new_content);
            if decision == ReconcileDecision::Noop {
                continue;
            }

            let doc_type = if relative_path.to_string_lossy().contains("README") {
                DocType::UserGuide
            } else {
                DocType::ArchitecturalDoc
            };
            let metadata = SemanticMetadata::new("living_docs", doc_type)
                .with_tag("doc_sync")
                .with_tag(match decision {
                    ReconcileDecision::Noop => "noop",
                    ReconcileDecision::UpdateRequired => "update_required",
                    ReconcileDecision::ReviewRequired => "review_required",
                })
                .with_related(&relative_path.to_string_lossy());
            let memory =
                SemanticMemory::from_content(&new_content, embed_text(&new_content, 384), metadata);
            self.semantic_store
                .insert(memory)
                .map_err(anyhow::Error::msg)?;

            synced += 1;
            for patch in patches {
                info!(
                    "Doc sync candidate for {} -> {}",
                    relative_path.display(),
                    patch.target.display()
                );
            }
        }

        self.last_tree = Some(next_tree);
        Ok(synced)
    }

    /// Start the periodic doc sync task.
    pub fn start(config: CoreConfig) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut service = match Self::new(&config) {
                Ok(service) => service,
                Err(err) => {
                    error!("Failed to initialize doc sync service: {}", err);
                    return;
                }
            };
            loop {
                if let Err(err) = service.sync_once() {
                    error!("Doc sync iteration failed: {}", err);
                }
                std::thread::sleep(Duration::from_secs(config.doc_sync_interval_secs));
            }
        })
    }
}

fn embed_text(content: &str, dim: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; dim];
    for (index, byte) in content.bytes().enumerate() {
        embedding[index % dim] += byte as f32 / 255.0;
    }
    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }
    embedding
}
