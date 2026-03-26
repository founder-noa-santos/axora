//! Core configuration

use openakta_agents::{
    FallbackPolicy, ProviderInstancesConfig, ProviderRuntimeConfig, RemoteRegistryConfig,
    TomlModelRegistryEntry,
};
use openakta_embeddings::{CodeEmbeddingConfig, FallbackEmbeddingConfig, SkillEmbeddingConfig};
use openakta_indexing::{CollectionSpec, VectorBackendKind};
use openakta_mcp_server::{
    ContainerExecutorConfig, MassRefactorExecutorConfig, SandboxedToolExecutionMode,
    WasiExecutorConfig,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Semantic vector backend selection.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticVectorBackend {
    /// sqlite-vec HNSW ANN (default, production local backend).
    /// Uses sqlite-vec extension for efficient approximate nearest neighbor search.
    ///
    /// **No installation required:** sqlite-vec is statically linked at build time.
    /// The extension is auto-registered at process startup before any SQLite usage.
    #[default]
    SqliteVec,
    /// External Qdrant or compatible endpoint (cloud tier / enterprise self-hosted).
    /// Cloud tier: Qdrant Cloud (Azure Marketplace) with Cohere embed-v3-multilingual.
    /// Self-hosted: bring-your-own Qdrant or compatible vector backend.
    External {
        endpoint: String,
        api_key: Option<String>,
    },
}

fn default_broadcast_lag_streak_limit() -> u32 {
    3
}

/// Shared retrieval configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Dense backend kind.
    pub backend: VectorBackendKind,
    /// Shared Qdrant endpoint for dense collections.
    pub qdrant_url: String,
    /// SQLite dense-store path.
    pub sqlite_path: PathBuf,
    /// Code retrieval settings.
    pub code: RetrievalDomainConfig,
    /// Skill retrieval settings.
    pub skills: SkillRetrievalDomainConfig,
    /// Optional remote fallback.
    pub fallback: FallbackEmbeddingConfig,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        let runtime_root = PathBuf::from(".openakta");
        Self {
            backend: VectorBackendKind::SqliteJson,
            qdrant_url: "http://127.0.0.1:6334".to_string(),
            sqlite_path: runtime_root.join("vectors.db"),
            code: RetrievalDomainConfig::default(),
            skills: SkillRetrievalDomainConfig::default(),
            fallback: FallbackEmbeddingConfig::default(),
        }
    }
}

/// Shared per-domain retrieval settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalDomainConfig {
    /// Collection name.
    pub collection: String,
    /// Model root.
    pub model_root: PathBuf,
    /// Tokenizer path.
    pub tokenizer_path: PathBuf,
    /// Device selection.
    pub device: String,
    /// Embedding dimensions.
    pub dimensions: usize,
    /// Maximum input length.
    pub max_length: usize,
    /// Batch size.
    pub batch_size: usize,
    /// Default token budget.
    pub token_budget: usize,
}

impl Default for RetrievalDomainConfig {
    fn default() -> Self {
        let embedding = CodeEmbeddingConfig::default();
        Self {
            collection: CollectionSpec::code_default().name,
            model_root: embedding.model_root,
            tokenizer_path: embedding.tokenizer_path,
            device: embedding.device,
            dimensions: embedding.dimensions,
            max_length: embedding.max_length,
            batch_size: embedding.batch_size,
            token_budget: 2_000,
        }
    }
}

impl RetrievalDomainConfig {
    /// Convert to a code embedding config.
    pub fn embedding_config(&self) -> CodeEmbeddingConfig {
        CodeEmbeddingConfig {
            model_name: "jina-code-embeddings-v2".to_string(),
            model_root: self.model_root.clone(),
            tokenizer_path: self.tokenizer_path.clone(),
            dimensions: self.dimensions,
            max_length: self.max_length,
            batch_size: self.batch_size,
            device: self.device.clone(),
        }
    }

    /// Convert to a collection spec.
    pub fn collection_spec(&self) -> CollectionSpec {
        CollectionSpec {
            name: self.collection.clone(),
            dimensions: self.dimensions,
            ..CollectionSpec::code_default()
        }
    }
}

/// Skill-specific retrieval settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRetrievalDomainConfig {
    /// Collection name.
    pub collection: String,
    /// Model root.
    pub model_root: PathBuf,
    /// Tokenizer path.
    pub tokenizer_path: PathBuf,
    /// Device selection.
    pub device: String,
    /// Embedding dimensions.
    pub dimensions: usize,
    /// Maximum input length.
    pub max_length: usize,
    /// Batch size.
    pub batch_size: usize,
    /// Default token budget.
    pub token_budget: usize,
    /// Root directory containing authored skills.
    pub corpus_root: PathBuf,
    /// SQLite skill catalog path.
    pub catalog_db_path: PathBuf,
    /// BM25 index directory.
    pub bm25_dir: PathBuf,
}

impl Default for SkillRetrievalDomainConfig {
    fn default() -> Self {
        let runtime_root = PathBuf::from(".openakta");
        let embedding = SkillEmbeddingConfig::default();
        Self {
            collection: CollectionSpec::skill_default().name,
            model_root: embedding.model_root,
            tokenizer_path: embedding.tokenizer_path,
            device: embedding.device,
            dimensions: embedding.dimensions,
            max_length: embedding.max_length,
            batch_size: embedding.batch_size,
            token_budget: 1500,
            corpus_root: PathBuf::from("./skills"),
            catalog_db_path: runtime_root.join("skill-index").join("skill-catalog.db"),
            bm25_dir: runtime_root.join("skill-bm25"),
        }
    }
}

impl SkillRetrievalDomainConfig {
    /// Convert to a skill embedding config.
    pub fn embedding_config(&self) -> SkillEmbeddingConfig {
        SkillEmbeddingConfig {
            model_name: "bge-small-en-v1.5".to_string(),
            model_root: self.model_root.clone(),
            tokenizer_path: self.tokenizer_path.clone(),
            dimensions: self.dimensions,
            max_length: self.max_length,
            batch_size: self.batch_size,
            device: self.device.clone(),
        }
    }

    /// Convert to a collection spec.
    pub fn collection_spec(&self) -> CollectionSpec {
        CollectionSpec {
            name: self.collection.clone(),
            dimensions: self.dimensions,
            ..CollectionSpec::skill_default()
        }
    }
}

/// Core system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// MCP server bind address
    pub mcp_bind_address: String,
    /// MCP server port
    pub mcp_port: u16,
    /// Database path
    pub database_path: PathBuf,
    /// Workspace root exposed to MCP and doc sync services.
    pub workspace_root: PathBuf,
    /// Nested retrieval configuration.
    pub retrieval: RetrievalConfig,
    /// Root directory for authored `SKILL.md` files.
    pub skill_corpus_root: PathBuf,
    /// Root directory for retrieval index artifacts.
    pub skill_index_root: PathBuf,
    /// Dense-index endpoint for skill retrieval.
    pub skill_qdrant_url: String,
    /// Sparse BM25 directory.
    pub skill_bm25_dir: PathBuf,
    /// Default skill-retrieval budget.
    pub skill_retrieval_budget_tokens: usize,
    /// Root directory for repository documentation.
    pub docs_root: PathBuf,
    /// Local semantic store database path.
    pub semantic_store_path: PathBuf,
    /// Provider instance configuration.
    #[serde(default)]
    pub providers: ProviderInstancesConfig,
    /// Flat HTTP client policy for provider transports.
    #[serde(default)]
    pub provider_runtime: ProviderRuntimeConfig,
    /// Optional remote model registry configuration.
    pub remote_registry: Option<RemoteRegistryConfig>,
    /// Local model registry extensions declared in TOML.
    #[serde(default)]
    pub registry_models: Vec<TomlModelRegistryEntry>,
    /// Fallback policy when the cloud lane is unavailable.
    pub fallback_policy: FallbackPolicy,
    /// Whether DAAO routing is enabled when both lanes exist.
    pub routing_enabled: bool,
    /// Retry budget for local validation failures before arbiter escalation.
    pub local_validation_retry_budget: u32,
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u64,
    /// MCP command allowlist enforced by the daemon.
    pub mcp_allowed_commands: Vec<String>,
    /// Maximum execution time for MCP commands.
    pub mcp_command_timeout_secs: u64,
    /// Sandboxed execution mode for mutating MCP tools (never raw host shell from config).
    pub execution_mode: SandboxedToolExecutionMode,
    /// Container backend configuration.
    pub container_executor: ContainerExecutorConfig,
    /// WASI backend configuration.
    pub wasi_executor: WasiExecutorConfig,
    /// Dedicated configuration for sandboxed mass-refactor scripts.
    #[serde(default)]
    pub mass_refactor_executor: MassRefactorExecutorConfig,
    /// Background pruning cadence in seconds.
    pub pruning_interval_secs: u64,
    /// Background doc sync cadence in seconds.
    pub doc_sync_interval_secs: u64,
    /// Fraction of model context window reserved for prompts.
    pub provider_context_use_ratio: f32,
    /// Fixed token safety margin for prompt budgeting.
    pub provider_context_margin_tokens: u32,
    /// Fraction of prompt budget allocated to retrieval.
    pub provider_retrieval_share: f32,
    /// Enable debug mode
    pub debug: bool,
    /// Consecutive `RecvError::Lagged` on broadcast streams before failing (V-004); Collective + ReAct interrupt channel.
    #[serde(default = "default_broadcast_lag_streak_limit")]
    pub broadcast_lag_streak_limit: u32,
    /// Optional outbound web search (Serper / Tavily BYOK); see [`openakta_research::ResearchConfig`].
    #[serde(default)]
    pub research: Option<openakta_research::ResearchConfig>,
    /// Defensive cap on semantic store rows before warning (default 50_000).
    #[serde(default = "default_semantic_scan_cap")]
    pub semantic_scan_cap: usize,
    /// Semantic vector backend selection (Phase 1-2).
    #[serde(default)]
    pub semantic_vector_backend: SemanticVectorBackend,
}

fn default_semantic_scan_cap() -> usize {
    50_000
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 50051,
            mcp_bind_address: "127.0.0.1".to_string(),
            mcp_port: 50061,
            database_path: PathBuf::from("openakta.db"),
            workspace_root: PathBuf::from("."),
            retrieval: RetrievalConfig::default(),
            skill_corpus_root: PathBuf::from("./skills"),
            skill_index_root: PathBuf::from("./.openakta/skill-index"),
            skill_qdrant_url: "http://127.0.0.1:6334".to_string(),
            skill_bm25_dir: PathBuf::from("./.openakta/skill-bm25"),
            skill_retrieval_budget_tokens: 1500,
            docs_root: PathBuf::from("./docs"),
            semantic_store_path: PathBuf::from("./semantic-memory.db"),
            providers: ProviderInstancesConfig::default(),
            provider_runtime: ProviderRuntimeConfig::default(),
            remote_registry: None,
            registry_models: Vec::new(),
            fallback_policy: FallbackPolicy::Explicit,
            routing_enabled: false,
            local_validation_retry_budget: 1,
            max_concurrent_agents: 10,
            frame_duration_ms: 16, // ~60 FPS
            mcp_allowed_commands: vec![
                "cargo".to_string(),
                "git".to_string(),
                "rg".to_string(),
                "rustc".to_string(),
            ],
            mcp_command_timeout_secs: 30,
            execution_mode: SandboxedToolExecutionMode::Hybrid,
            container_executor: ContainerExecutorConfig::default(),
            wasi_executor: WasiExecutorConfig::default(),
            mass_refactor_executor: MassRefactorExecutorConfig::default(),
            pruning_interval_secs: 3600,
            doc_sync_interval_secs: 60,
            provider_context_use_ratio: 0.8,
            provider_context_margin_tokens: 512,
            provider_retrieval_share: 0.35,
            debug: false,
            broadcast_lag_streak_limit: default_broadcast_lag_streak_limit(),
            research: None,
            semantic_scan_cap: default_semantic_scan_cap(),
            semantic_vector_backend: SemanticVectorBackend::SqliteVec,
        }
    }
}

impl CoreConfig {
    pub(crate) fn file_defaults() -> Self {
        Self::default()
    }

    /// Build a batteries-included runtime config rooted in the provided workspace.
    #[allow(clippy::field_reassign_with_default)]
    pub fn for_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let runtime_root = workspace_root.join(".openakta");
        let mut retrieval = RetrievalConfig::default();
        retrieval.sqlite_path = runtime_root.join("vectors.db");
        retrieval.qdrant_url = "http://127.0.0.1:6334".to_string();
        retrieval.code.collection = CollectionSpec::code_default().name;
        retrieval.code.token_budget = 2_000;
        retrieval.skills.collection = CollectionSpec::skill_default().name;
        retrieval.skills.corpus_root = workspace_root.join("skills");
        retrieval.skills.catalog_db_path =
            runtime_root.join("skill-index").join("skill-catalog.db");
        retrieval.skills.bm25_dir = runtime_root.join("skill-bm25");

        Self {
            workspace_root: workspace_root.clone(),
            database_path: runtime_root.join("openakta.db"),
            retrieval,
            skill_corpus_root: workspace_root.join("skills"),
            skill_index_root: runtime_root.join("skill-index"),
            skill_qdrant_url: "http://127.0.0.1:6334".to_string(),
            skill_bm25_dir: runtime_root.join("skill-bm25"),
            skill_retrieval_budget_tokens: 1500,
            docs_root: workspace_root.join("docs"),
            semantic_store_path: runtime_root.join("semantic-memory.db"),
            routing_enabled: false,
            ..Default::default()
        }
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut base = toml::Value::try_from(Self::file_defaults())?;
        let overlay: toml::Value = toml::from_str(&content)?;
        merge_toml(&mut base, overlay);
        let config: Self = base.try_into()?;
        Ok(config)
    }

    /// Load `openakta.toml` at the project root merged with [`Self::for_workspace`] defaults.
    ///
    /// Prefer this for `workspace_root/openakta.toml` so unset paths (for example `database_path`)
    /// resolve under `.openakta/` instead of being pulled from [`Self::file_defaults`] (`openakta.db`
    /// in the current directory).
    pub fn from_project_file(path: &PathBuf) -> anyhow::Result<Self> {
        let workspace_root = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let content = std::fs::read_to_string(path)?;
        let mut base = toml::Value::try_from(Self::for_workspace(workspace_root))?;
        let overlay: toml::Value = toml::from_str(&content)?;
        merge_toml(&mut base, overlay);
        let config: Self = base.try_into()?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the full server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }

    /// Get the MCP server address.
    pub fn mcp_server_address(&self) -> String {
        format!("{}:{}", self.mcp_bind_address, self.mcp_port)
    }

    /// Directory for append-only execution JSONL logs.
    pub fn execution_log_dir(&self) -> PathBuf {
        self.workspace_root.join(".openakta/logs/execution")
    }

    /// Ensure runtime directories exist before bootstrapping services.
    pub fn ensure_runtime_layout(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = self.semantic_store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = self.retrieval.sqlite_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(self.workspace_root.join(".openakta/secrets"))?;
        std::fs::create_dir_all(self.workspace_root.join(".openakta/cache"))?;
        std::fs::create_dir_all(self.execution_log_dir())?;
        std::fs::create_dir_all(&self.skill_corpus_root)?;
        std::fs::create_dir_all(&self.skill_index_root)?;
        std::fs::create_dir_all(&self.skill_bm25_dir)?;
        std::fs::create_dir_all(&self.retrieval.skills.corpus_root)?;
        if let Some(parent) = self.retrieval.skills.catalog_db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&self.retrieval.skills.bm25_dir)?;
        if self.docs_root.starts_with(&self.workspace_root) {
            std::fs::create_dir_all(&self.docs_root)?;
        }
        Ok(())
    }

    /// Build [`openakta_research::ResearchRuntime`] when `[research]` is present in config.
    pub fn research_runtime(&self) -> anyhow::Result<Option<openakta_research::ResearchRuntime>> {
        match &self.research {
            None => Ok(None),
            Some(rc) => {
                openakta_research::ResearchRuntime::from_workspace(&self.workspace_root, rc)
                    .map(Some)
            }
        }
    }
}

fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                match base_table.get_mut(&key) {
                    Some(existing) => merge_toml(existing, value),
                    None => {
                        base_table.insert(key, value);
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CoreConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 50051);
        assert_eq!(config.mcp_port, 50061);
        assert_eq!(config.max_concurrent_agents, 10);
        assert_eq!(config.frame_duration_ms, 16);
        assert_eq!(config.mcp_command_timeout_secs, 30);
        assert!(config.research.is_none());
        assert!(config.research_runtime().unwrap().is_none());
    }

    #[test]
    fn test_server_address() {
        let config = CoreConfig::default();
        assert_eq!(config.server_address(), "127.0.0.1:50051");
    }

    #[test]
    fn test_mcp_server_address() {
        let config = CoreConfig::default();
        assert_eq!(config.mcp_server_address(), "127.0.0.1:50061");
    }

    #[test]
    fn test_from_file_merges_partial_config_without_forcing_cloud_lane() {
        let tempdir = tempfile::tempdir().unwrap();
        let config_path = tempdir.path().join("openakta.toml");
        std::fs::write(
            &config_path,
            r#"
fallback_policy = "automatic"
routing_enabled = true
local_validation_retry_budget = 3

[providers]
default_local_instance = "local"

[providers.instances.local]
profile = "open_ai_compatible"
base_url = "http://127.0.0.1:11434"
is_local = true
default_model = "qwen2.5-coder:7b"
"#,
        )
        .unwrap();

        let config = CoreConfig::from_file(&config_path).unwrap();

        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 50051);
        assert!(config.providers.default_cloud_instance.is_none());
        assert_eq!(config.fallback_policy, FallbackPolicy::Automatic);
        assert!(config.routing_enabled);
        assert_eq!(config.local_validation_retry_budget, 3);
        assert_eq!(
            config
                .providers
                .instances
                .get(&openakta_agents::ProviderInstanceId("local".to_string()))
                .and_then(|local| local.default_model.as_deref()),
            Some("qwen2.5-coder:7b")
        );
    }

    #[test]
    fn test_from_project_file_defaults_database_under_openakta_dir() {
        let tempdir = tempfile::tempdir().unwrap();
        let config_path = tempdir.path().join("openakta.toml");
        std::fs::write(
            &config_path,
            r#"
fallback_policy = "explicit"
routing_enabled = false
"#,
        )
        .unwrap();

        let config = CoreConfig::from_project_file(&config_path).unwrap();
        assert_eq!(
            config.database_path,
            tempdir.path().join(".openakta").join("openakta.db")
        );
        assert_eq!(config.workspace_root, tempdir.path());
    }

    #[test]
    fn test_default_local_validation_retry_budget() {
        let config = CoreConfig::default();
        assert_eq!(config.local_validation_retry_budget, 1);
    }
}
