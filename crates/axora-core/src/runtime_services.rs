//! Shared runtime-managed services for daemon and CLI entrypoints.

use axora_docs::{DocReconciler, DocReconcilerConfig, ReconcileDecision};
use axora_indexing::MerkleTree;
use axora_memory::{
    ConsolidationPipeline, ConsolidationWorker, DocType, EpisodicStore, EpisodicStoreConfig,
    LightweightLLM, MemoryLifecycle, PersistentSemanticStore, ProceduralStore, SemanticMemory,
    SemanticMetadata, SkillSeeder,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::CoreConfig;

/// Memory services hosted by the AXORA runtime.
pub struct MemoryServices {
    /// Episodic store used by agents and lifecycle workers.
    pub episodic_store: Arc<EpisodicStore>,
    /// Procedural skill store.
    pub procedural_store: Arc<ProceduralStore>,
    /// Persistent semantic store for synced docs and retrieved knowledge.
    pub semantic_store: Arc<PersistentSemanticStore>,
}

impl MemoryServices {
    /// Create runtime-managed memory services and seed default skills on first run.
    pub async fn new(config: &CoreConfig) -> anyhow::Result<Self> {
        let episodic_path = config.database_path.with_extension("episodic.db");
        let episodic_store =
            EpisodicStore::new(EpisodicStoreConfig::persistent(&episodic_path.display().to_string()))
                .await?;
        let procedural_store = ProceduralStore::new(&config.skills_root).await?;
        let semantic_store = PersistentSemanticStore::new(&config.semantic_store_path, 384)
            .map_err(anyhow::Error::msg)?;

        SkillSeeder::seed_defaults(&procedural_store, &config.database_path)
            .await
            .map_err(anyhow::Error::msg)?;

        Ok(Self {
            episodic_store: Arc::new(episodic_store),
            procedural_store: Arc::new(procedural_store),
            semantic_store: Arc::new(semantic_store),
        })
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
                                                axora_memory::lifecycle::LifecycleError::Io(
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
        let procedural_root = config.skills_root.clone();
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
                let procedural_store = match ProceduralStore::new(&procedural_root).await {
                    Ok(store) => store,
                    Err(err) => {
                        error!("Failed to start consolidation procedural store: {}", err);
                        return;
                    }
                };

                let pipeline = ConsolidationPipeline::new(
                    episodic_store,
                    procedural_store,
                    LightweightLLM::new("claude-haiku"),
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
        Ok(Self {
            root: config.workspace_root.clone(),
            reconciler: DocReconciler::new(DocReconcilerConfig::new(config.workspace_root.clone())),
            semantic_store: Arc::new(
                PersistentSemanticStore::new(&config.semantic_store_path, 384)
                    .map_err(anyhow::Error::msg)?,
            ),
            last_tree: None,
        })
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
                    warn!("Skipping non UTF-8 file {}: {}", absolute_path.display(), err);
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
            let memory = SemanticMemory::from_content(&new_content, embed_text(&new_content, 384), metadata);
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
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("doc-sync runtime");
            runtime.block_on(async move {
                let mut service = match DocSyncService::new(&config) {
                    Ok(service) => service,
                    Err(err) => {
                        error!("Failed to initialize doc sync service: {}", err);
                        return;
                    }
                };
                let interval = Duration::from_secs(config.doc_sync_interval_secs);
                loop {
                    if let Err(err) = service.sync_once() {
                        error!("Doc sync iteration failed: {}", err);
                    }
                    tokio::time::sleep(interval).await;
                }
            });
        })
    }
}

fn embed_text(text: &str, dim: usize) -> Vec<f32> {
    let mut vector = vec![0.0; dim];
    for (idx, byte) in text.bytes().enumerate() {
        let bucket = idx % dim;
        vector[bucket] += (byte as f32) / 255.0;
    }
    vector
}
