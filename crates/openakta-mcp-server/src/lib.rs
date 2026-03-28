//! MCP gRPC tool sandbox server for OPENAKTA.

pub mod execution;
pub mod mass_refactor;

use execution::{CommandRequest, ExecutorRouter, PatchRequest, ToolExecutor};
use glob::Pattern;
use mass_refactor::{
    next_mass_refactor_session_id, MassRefactorRequest, MassRefactorTool,
    MASS_REFACTOR_CONSENT_APPROVED,
};
use openakta_agents::hitl::MissionHitlGate;
use openakta_cache::UnifiedDiff;
use openakta_embeddings::{CodeEmbeddingConfig, JinaCodeEmbedder};
use openakta_indexing::{
    Chunker, CollectionSpec, Language, ParserRegistry, TantivyCodeIndex, VectorBackendKind,
};
use openakta_memory::{SkillRetrievalConfig, SkillRetrievalPipeline};
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalService;
use openakta_proto::mcp::v1::retrieval_service_server::RetrievalService;
use openakta_proto::mcp::v1::tool_service_server::ToolService;
use openakta_proto::mcp::v1::{
    AuditEvent, CandidateScore, CapabilityPolicy, ListToolsRequest, ListToolsResponse,
    RetrievalDiagnostics, RetrieveCodeContextRequest, RetrieveCodeContextResponse,
    RetrieveSkillsRequest, RetrieveSkillsResponse, RetrievedCodeContext, RetrievedSkill,
    StreamAuditRequest, ToolCallRequest, ToolCallResult, ToolDefinition,
};
use openakta_rag::{CodeRetrievalPipeline, CodeRetrievalQuery, RetrievalDiagnosticsData};
use parking_lot::RwLock;
use prost_types::{value::Kind, ListValue, Struct, Timestamp, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openakta_proto::collective::v1::{
    QuestionConstraints, QuestionEnvelope, QuestionKind, QuestionOption,
};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub use execution::{
    ContainerExecutorConfig, ExecutionMode, MassRefactorExecutorConfig, SandboxedToolExecutionMode,
    WasiExecutorConfig,
};

/// MCP errors.
#[derive(Debug, Error)]
pub enum McpError {
    /// Request violates machine-local tool capability / workspace bounds (not tenant RBAC).
    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    /// Unknown tool requested.
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    /// Tool execution failure.
    #[error("tool execution failed: {0}")]
    ToolExecution(String),
}

/// Runtime configuration for the MCP sandbox boundary.
#[derive(Debug, Clone)]
pub struct McpServiceConfig {
    /// Canonical workspace root exposed by the server.
    pub workspace_root: PathBuf,
    /// Allowed executable names for `run_command`.
    pub allowed_commands: Vec<String>,
    /// Default timeout for command execution.
    pub default_max_execution_seconds: u32,
    /// Sandboxed execution routing for mutating tools (never raw host shell from config).
    pub execution_mode: SandboxedToolExecutionMode,
    /// Container execution settings.
    pub container_executor: ContainerExecutorConfig,
    /// WASI execution settings.
    pub wasi_executor: WasiExecutorConfig,
    /// Dedicated container settings for sandboxed mass-refactor scripts.
    pub mass_refactor_executor: MassRefactorExecutorConfig,
    /// Dense backend selection.
    pub dense_backend: VectorBackendKind,
    /// Shared Qdrant endpoint.
    pub dense_qdrant_url: String,
    /// Shared SQLite dense-store path.
    pub dense_store_path: PathBuf,
    /// Code collection specification.
    pub code_collection: CollectionSpec,
    /// Code embedding configuration.
    pub code_embedding: CodeEmbeddingConfig,
    /// Tantivy BM25 index directory for code retrieval.
    pub code_bm25_dir: PathBuf,
    /// Persisted Merkle state for incremental code indexing.
    pub code_index_state_path: PathBuf,
    /// Default code retrieval budget.
    pub code_retrieval_budget_tokens: usize,
    /// Skill retrieval configuration.
    pub skill_config: SkillRetrievalConfig,
    /// Human-in-the-loop gate (enables `request_user_input` when set).
    pub hitl_gate: Option<Arc<MissionHitlGate>>,
}

impl Default for McpServiceConfig {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            allowed_commands: vec![
                "cargo".to_string(),
                "git".to_string(),
                "rg".to_string(),
                "rustc".to_string(),
            ],
            default_max_execution_seconds: 30,
            execution_mode: SandboxedToolExecutionMode::Hybrid,
            container_executor: ContainerExecutorConfig::default(),
            wasi_executor: WasiExecutorConfig::default(),
            mass_refactor_executor: MassRefactorExecutorConfig::default(),
            dense_backend: VectorBackendKind::Qdrant,
            dense_qdrant_url: "http://127.0.0.1:6334".to_string(),
            dense_store_path: PathBuf::from(".openakta/vectors.db"),
            code_collection: CollectionSpec::code_default(),
            code_embedding: CodeEmbeddingConfig::default(),
            code_bm25_dir: PathBuf::from(".openakta/code-bm25"),
            code_index_state_path: PathBuf::from(".openakta/code-index-state.json"),
            code_retrieval_budget_tokens: 2_000,
            skill_config: SkillRetrievalConfig::default(),
            hitl_gate: None,
        }
    }
}

/// Enforces machine-local tool capabilities and workspace-relative tool scopes from [`CapabilityPolicy`].
#[derive(Debug, Default, Clone)]
pub struct MachineToolPolicyEngine;

impl MachineToolPolicyEngine {
    /// Validate tool invocation against optional policy bounds.
    pub fn validate(
        &self,
        policy: Option<&CapabilityPolicy>,
        tool_name: &str,
        workspace_root: &Path,
        scope: &Path,
    ) -> Result<(), McpError> {
        let Some(policy) = policy else {
            return Ok(());
        };

        let profile = policy.role.as_str();

        if !policy.allowed_actions.is_empty()
            && !policy
                .allowed_actions
                .iter()
                .any(|action| action == tool_name)
        {
            return Err(McpError::CapabilityDenied(format!(
                "tool '{tool_name}' not allowed for capability profile '{profile}'",
            )));
        }

        if !scope.starts_with(workspace_root) {
            return Err(McpError::CapabilityDenied(format!(
                "tool scope '{}' escapes workspace '{}'",
                scope.display(),
                workspace_root.display()
            )));
        }

        let scope_string = scope.to_string_lossy();
        if policy
            .denied_scope_patterns
            .iter()
            .any(|pattern| scope_string.contains(pattern.trim_start_matches('!')))
        {
            return Err(McpError::CapabilityDenied(format!(
                "tool scope '{}' denied by capability policy",
                scope.display()
            )));
        }

        if !policy.allowed_scope_patterns.is_empty()
            && !policy
                .allowed_scope_patterns
                .iter()
                .any(|pattern| scope_string.contains(pattern.trim_matches('*')))
        {
            return Err(McpError::CapabilityDenied(format!(
                "tool scope '{}' not in capability allowlist",
                scope.display()
            )));
        }

        Ok(())
    }
}

/// In-memory audit log for tool calls.
#[derive(Clone)]
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEvent>>>,
    tx: broadcast::Sender<AuditEvent>,
}

impl AuditLog {
    /// Create a new audit log.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    /// Append an audit event and notify subscribers.
    pub fn push(&self, event: AuditEvent) {
        self.entries.write().push(event.clone());
        let _ = self.tx.send(event);
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution context shared by embedded tool handlers.
pub struct ToolExecutionContext<'a> {
    request: &'a ToolCallRequest,
    workspace_root: &'a Path,
    scope: PathBuf,
    config: &'a McpServiceConfig,
    executor: &'a ExecutorRouter,
    retrieval_router: &'a RetrievalRouter,
}

/// Native, in-process MCP tool contract.
#[tonic::async_trait]
pub trait EmbeddedTool: Send + Sync {
    /// Tool definition surfaced via gRPC.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool against the request.
    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError>;
}

/// Embedded tool registry used by the gRPC adapter.
#[derive(Clone, Default)]
pub struct EmbeddedToolRegistry {
    tools: Arc<HashMap<String, Arc<dyn EmbeddedTool>>>,
}

impl EmbeddedToolRegistry {
    /// Create the default native tool registry.
    pub fn builtin() -> Self {
        Self::builtin_with_hitl(None, MassRefactorExecutorConfig::default())
    }

    /// Built-in tools plus optional `request_user_input` when `hitl_gate` is set.
    pub fn builtin_with_hitl(
        hitl_gate: Option<Arc<MissionHitlGate>>,
        mass_refactor_config: MassRefactorExecutorConfig,
    ) -> Self {
        let mut tools: HashMap<String, Arc<dyn EmbeddedTool>> = HashMap::new();
        for tool in [
            Arc::new(ReadFileTool) as Arc<dyn EmbeddedTool>,
            Arc::new(ListDirTool),
            Arc::new(GlobPathsTool),
            Arc::new(GenerateDiffTool),
            Arc::new(ApplyPatchTool),
            Arc::new(AstChunkTool),
            Arc::new(SymbolLookupTool),
            Arc::new(RunCommandTool),
            Arc::new(MassRefactorTool::new(mass_refactor_config)),
            Arc::new(GraphRetrieveSkillsTool),
            Arc::new(GraphRetrieveCodeTool),
        ] {
            tools.insert(tool.definition().name.clone(), tool);
        }
        if let Some(gate) = hitl_gate {
            let t: Arc<dyn EmbeddedTool> = Arc::new(RequestUserInputTool { gate });
            tools.insert(t.definition().name.clone(), t);
        }
        Self {
            tools: Arc::new(tools),
        }
    }

    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddedTool>> {
        self.tools.get(name).cloned()
    }

    fn definitions_for_capability_profile(&self, capability_profile: &str) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|tool| {
                capability_profile_allows_tool(capability_profile, &tool.definition().name)
            })
            .map(|tool| tool.definition())
            .collect()
    }
}

fn make_tool_definition(
    name: &str,
    description: &str,
    required_actions: Vec<String>,
    allowed_scope_patterns: Vec<String>,
    supports_streaming: bool,
    is_destructive: bool,
    read_only: bool,
    parameters: Struct,
    result_schema: Option<Struct>,
    tool_kind: &str,
    requires_approval: bool,
    ui_renderer: &str,
) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        required_actions,
        allowed_scope_patterns,
        supports_streaming,
        is_destructive,
        read_only,
        parameters: Some(parameters),
        strict: false,
        tool_kind: tool_kind.to_string(),
        result_schema,
        requires_approval,
        ui_renderer: ui_renderer.to_string(),
    }
}

/// MCP tool sandbox service.
#[derive(Clone)]
pub struct McpService {
    registry: EmbeddedToolRegistry,
    tool_policy: MachineToolPolicyEngine,
    audit: AuditLog,
    config: McpServiceConfig,
    executor: Arc<ExecutorRouter>,
    retrieval_router: Arc<RetrievalRouter>,
}

#[tonic::async_trait]
trait SkillRetrieverService: Send + Sync {
    async fn retrieve_skills(
        &self,
        request: RetrieveSkillsRequest,
    ) -> Result<RetrieveSkillsResponse, McpError>;
}

#[tonic::async_trait]
trait CodeRetrieverService: Send + Sync {
    async fn retrieve_code(
        &self,
        request: RetrieveCodeContextRequest,
    ) -> Result<RetrieveCodeContextResponse, McpError>;
}

#[derive(Clone)]
struct RetrievalRouter {
    skill: Arc<dyn SkillRetrieverService>,
    code: Arc<dyn CodeRetrieverService>,
}

struct LazyPipelineSkillRetriever {
    config: SkillRetrievalConfig,
    pipeline: tokio::sync::OnceCell<Arc<SkillRetrievalPipeline>>,
}

struct RuntimePipelineSkillRetriever {
    pipeline: Arc<SkillRetrievalPipeline>,
}

struct LazyPipelineCodeRetriever {
    config: McpServiceConfig,
    pipeline: tokio::sync::OnceCell<Arc<CodeRetrievalPipeline>>,
}

struct RuntimePipelineCodeRetriever {
    pipeline: Arc<CodeRetrievalPipeline>,
}

async fn build_code_dense_collection(
    config: &McpServiceConfig,
) -> Result<Arc<dyn openakta_indexing::DenseVectorCollection>, McpError> {
    match config.dense_backend {
        VectorBackendKind::Qdrant => Ok(Arc::new(
            openakta_indexing::QdrantVectorCollection::new(
                &config.dense_qdrant_url,
                config.code_collection.clone(),
            )
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))?,
        )),
        VectorBackendKind::SqliteJson => Ok(Arc::new(
            openakta_indexing::SqliteJsonVectorCollection::new(
                &config.dense_store_path,
                config.code_collection.clone(),
            )
            .map_err(|err| McpError::ToolExecution(err.to_string()))?,
        )),
    }
}

impl LazyPipelineSkillRetriever {
    fn new(config: SkillRetrievalConfig) -> Self {
        Self {
            config,
            pipeline: tokio::sync::OnceCell::new(),
        }
    }

    async fn pipeline(&self) -> Result<Arc<SkillRetrievalPipeline>, McpError> {
        self.pipeline
            .get_or_try_init(|| async {
                SkillRetrievalPipeline::new(self.config.clone())
                    .await
                    .map(Arc::new)
                    .map_err(|err| McpError::ToolExecution(err.to_string()))
            })
            .await
            .map(Arc::clone)
    }
}

#[tonic::async_trait]
impl SkillRetrieverService for LazyPipelineSkillRetriever {
    async fn retrieve_skills(
        &self,
        request: RetrieveSkillsRequest,
    ) -> Result<RetrieveSkillsResponse, McpError> {
        let pipeline = self.pipeline().await?;
        pipeline
            .retrieve(&request)
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))
    }
}

#[tonic::async_trait]
impl SkillRetrieverService for RuntimePipelineSkillRetriever {
    async fn retrieve_skills(
        &self,
        request: RetrieveSkillsRequest,
    ) -> Result<RetrieveSkillsResponse, McpError> {
        self.pipeline
            .retrieve(&request)
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))
    }
}

impl LazyPipelineCodeRetriever {
    fn new(config: McpServiceConfig) -> Self {
        Self {
            config,
            pipeline: tokio::sync::OnceCell::new(),
        }
    }

    async fn pipeline(&self) -> Result<Arc<CodeRetrievalPipeline>, McpError> {
        self.pipeline
            .get_or_try_init(|| async {
                let dense_collection = build_code_dense_collection(&self.config).await?;
                let sparse_index = Arc::new(
                    TantivyCodeIndex::new(&self.config.code_bm25_dir)
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?,
                );
                let embedder = Arc::new(
                    JinaCodeEmbedder::new(self.config.code_embedding.clone())
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?,
                );
                CodeRetrievalPipeline::new(
                    self.config.workspace_root.clone(),
                    self.config.code_index_state_path.clone(),
                    dense_collection,
                    sparse_index,
                    embedder,
                    openakta_rag::OpenaktaReranker::for_workspace(&self.config.workspace_root),
                )
                .map(Arc::new)
                .map_err(|err| McpError::ToolExecution(err.to_string()))
            })
            .await
            .map(Arc::clone)
    }
}

#[tonic::async_trait]
impl CodeRetrieverService for RuntimePipelineCodeRetriever {
    async fn retrieve_code(
        &self,
        request: RetrieveCodeContextRequest,
    ) -> Result<RetrieveCodeContextResponse, McpError> {
        let budget = if request.token_budget == 0 {
            2_000
        } else {
            request.token_budget as usize
        };
        let candidate_limit = request.candidate_limit.max(request.dense_limit).max(32) as usize;
        let result = self
            .pipeline
            .retrieve(&CodeRetrievalQuery {
                workspace_root: PathBuf::from(&request.workspace_root),
                query: request.query.clone(),
                focal_files: request.focal_files.clone(),
                focal_symbols: request.focal_symbols.clone(),
                dense_limit: request.dense_limit.max(32) as usize,
                sparse_limit: candidate_limit,
                candidate_limit,
                token_budget: budget,
            })
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(code_response_from_result(
            request.request_id,
            &result,
            request.include_diagnostics,
        ))
    }
}

#[tonic::async_trait]
impl CodeRetrieverService for LazyPipelineCodeRetriever {
    async fn retrieve_code(
        &self,
        request: RetrieveCodeContextRequest,
    ) -> Result<RetrieveCodeContextResponse, McpError> {
        let pipeline = self.pipeline().await?;
        let budget = if request.token_budget == 0 {
            self.config.code_retrieval_budget_tokens
        } else {
            request.token_budget as usize
        };
        let candidate_limit = request.candidate_limit.max(request.dense_limit).max(32) as usize;
        let result = pipeline
            .retrieve(&CodeRetrievalQuery {
                workspace_root: PathBuf::from(&request.workspace_root),
                query: request.query.clone(),
                focal_files: request.focal_files.clone(),
                focal_symbols: request.focal_symbols.clone(),
                dense_limit: request.dense_limit.max(32) as usize,
                sparse_limit: candidate_limit,
                candidate_limit,
                token_budget: budget,
            })
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(code_response_from_result(
            request.request_id,
            &result,
            request.include_diagnostics,
        ))
    }
}

impl McpService {
    /// Create a new MCP service with built-in tools.
    pub fn new() -> Self {
        Self::with_config(McpServiceConfig::default())
    }

    /// Create a new MCP service with explicit configuration.
    pub fn with_config(config: McpServiceConfig) -> Self {
        let registry = EmbeddedToolRegistry::builtin_with_hitl(
            config.hitl_gate.clone(),
            config.mass_refactor_executor.clone(),
        );
        Self::with_registry(config, registry)
    }

    /// Create a new MCP service backed by an explicit embedded registry.
    pub fn with_registry(config: McpServiceConfig, registry: EmbeddedToolRegistry) -> Self {
        let workspace_root = std::fs::canonicalize(&config.workspace_root)
            .unwrap_or_else(|_| config.workspace_root.clone());
        let mut config = config;
        config.workspace_root = workspace_root.clone();
        Self {
            registry,
            tool_policy: MachineToolPolicyEngine,
            audit: AuditLog::new(),
            executor: Arc::new(ExecutorRouter::new(
                config.execution_mode,
                config.container_executor.clone(),
                config.wasi_executor.clone(),
            )),
            retrieval_router: Arc::new(RetrievalRouter {
                skill: Arc::new(LazyPipelineSkillRetriever::new(config.skill_config.clone())),
                code: Arc::new(LazyPipelineCodeRetriever::new(config.clone())),
            }),
            config,
        }
    }

    /// Create an MCP service that reuses already constructed runtime retrieval services.
    pub fn with_runtime_retrievers(
        config: McpServiceConfig,
        skill_pipeline: Arc<SkillRetrievalPipeline>,
        code_pipeline: Arc<CodeRetrievalPipeline>,
    ) -> Self {
        let registry = EmbeddedToolRegistry::builtin_with_hitl(
            config.hitl_gate.clone(),
            config.mass_refactor_executor.clone(),
        );
        Self::with_retrievers(
            config,
            registry,
            Arc::new(RuntimePipelineSkillRetriever {
                pipeline: skill_pipeline,
            }),
            Arc::new(RuntimePipelineCodeRetriever {
                pipeline: code_pipeline,
            }),
        )
    }

    /// **Tests only** — same as [`Self::with_config`] but forces the insecure direct-host executor
    /// (ambient shell). Production code cannot obtain this through configuration.
    #[cfg(test)]
    pub fn with_config_insecure_direct_host_for_tests(config: McpServiceConfig) -> Self {
        let registry = EmbeddedToolRegistry::builtin_with_hitl(
            config.hitl_gate.clone(),
            config.mass_refactor_executor.clone(),
        );
        Self::with_registry_insecure_direct_host_for_tests(config, registry)
    }

    #[cfg(test)]
    fn with_registry_insecure_direct_host_for_tests(
        mut config: McpServiceConfig,
        registry: EmbeddedToolRegistry,
    ) -> Self {
        let workspace_root = std::fs::canonicalize(&config.workspace_root)
            .unwrap_or_else(|_| config.workspace_root.clone());
        config.workspace_root = workspace_root.clone();
        Self {
            registry,
            tool_policy: MachineToolPolicyEngine,
            audit: AuditLog::new(),
            executor: Arc::new(ExecutorRouter::new_insecure_direct_host_for_tests(
                config.container_executor.clone(),
                config.wasi_executor.clone(),
            )),
            retrieval_router: Arc::new(RetrievalRouter {
                skill: Arc::new(LazyPipelineSkillRetriever::new(config.skill_config.clone())),
                code: Arc::new(LazyPipelineCodeRetriever::new(config.clone())),
            }),
            config,
        }
    }

    fn with_retrievers(
        config: McpServiceConfig,
        registry: EmbeddedToolRegistry,
        skill_retriever: Arc<dyn SkillRetrieverService>,
        code_retriever: Arc<dyn CodeRetrieverService>,
    ) -> Self {
        let workspace_root = std::fs::canonicalize(&config.workspace_root)
            .unwrap_or_else(|_| config.workspace_root.clone());
        let mut config = config;
        config.workspace_root = workspace_root;
        let executor = Arc::new(ExecutorRouter::new(
            config.execution_mode,
            config.container_executor.clone(),
            config.wasi_executor.clone(),
        ));

        Self {
            registry,
            tool_policy: MachineToolPolicyEngine,
            audit: AuditLog::new(),
            config,
            executor,
            retrieval_router: Arc::new(RetrievalRouter {
                skill: skill_retriever,
                code: code_retriever,
            }),
        }
    }

    fn build_audit(
        &self,
        request: &ToolCallRequest,
        allowed: bool,
        detail: String,
        scope: String,
    ) -> AuditEvent {
        AuditEvent {
            event_id: format!("audit-{}", request.request_id),
            request_id: request.request_id.clone(),
            agent_id: request.agent_id.clone(),
            role: request.role.clone(),
            tool_name: request.tool_name.clone(),
            action: request.tool_name.clone(),
            scope,
            allowed,
            detail,
            created_at: Some(Timestamp::from(std::time::SystemTime::now())),
            mission_id: request.mission_id.clone(),
            task_id: request.task_id.clone(),
            turn_id: request.turn_id.clone(),
            tool_call_id: request.tool_call_id.clone(),
            phase: if allowed { "completed" } else { "denied" }.to_string(),
            status: if allowed { "completed" } else { "denied" }.to_string(),
            read_only: false,
            mutating: false,
            requires_approval: false,
            args_preview: request
                .arguments
                .as_ref()
                .map(|arguments| format!("{arguments:?}"))
                .unwrap_or_default(),
            result_preview: String::new(),
            error: String::new(),
        }
    }
}

impl Default for McpService {
    fn default() -> Self {
        Self::new()
    }
}

impl McpService {
    async fn retrieve_skills_internal(
        &self,
        request: RetrieveSkillsRequest,
    ) -> Result<RetrieveSkillsResponse, McpError> {
        self.retrieval_router.skill.retrieve_skills(request).await
    }

    async fn retrieve_code_internal(
        &self,
        request: RetrieveCodeContextRequest,
    ) -> Result<RetrieveCodeContextResponse, McpError> {
        self.retrieval_router.code.retrieve_code(request).await
    }
}

#[tonic::async_trait]
impl ToolService for McpService {
    async fn list_tools(
        &self,
        request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        let req = request.into_inner();
        Ok(Response::new(ListToolsResponse {
            tools: self.registry.definitions_for_capability_profile(&req.role),
        }))
    }

    async fn call_tool(
        &self,
        request: Request<ToolCallRequest>,
    ) -> Result<Response<ToolCallResult>, Status> {
        let req = request.into_inner();
        let tool = self
            .registry
            .get(&req.tool_name)
            .ok_or_else(|| Status::not_found(format!("tool '{}' not found", req.tool_name)))?;

        if !capability_profile_allows_tool(&req.role, &req.tool_name) {
            let workspace_root = self.config.workspace_root.clone();
            let audit = self.build_audit(
                &req,
                false,
                format!(
                    "tool '{}' is not registered for capability profile '{}'",
                    req.tool_name, req.role
                ),
                workspace_root.display().to_string(),
            );
            self.audit.push(audit);
            return Err(Status::permission_denied(format!(
                "tool '{}' forbidden for capability profile '{}'",
                req.tool_name, req.role
            )));
        }

        let workspace_root = self.config.workspace_root.clone();
        let scope = resolve_scope(&workspace_root, &req.arguments).map_err(|err| {
            let audit = self.build_audit(
                &req,
                false,
                err.to_string(),
                workspace_root.display().to_string(),
            );
            self.audit.push(audit);
            Status::invalid_argument(err.to_string())
        })?;

        self.tool_policy
            .validate(req.policy.as_ref(), &req.tool_name, &workspace_root, &scope)
            .map_err(|err| {
                let audit =
                    self.build_audit(&req, false, err.to_string(), scope.display().to_string());
                self.audit.push(audit);
                Status::permission_denied(err.to_string())
            })?;

        let result = tool
            .execute(ToolExecutionContext {
                request: &req,
                workspace_root: &workspace_root,
                scope: scope.clone(),
                config: &self.config,
                executor: &self.executor,
                retrieval_router: &self.retrieval_router,
            })
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let audit = self.build_audit(
            &req,
            result.success,
            if result.success {
                "tool executed".to_string()
            } else {
                "tool execution failed".to_string()
            },
            scope.display().to_string(),
        );
        self.audit.push(audit.clone());

        Ok(Response::new(ToolCallResult {
            audit_event: Some(audit),
            ..result
        }))
    }

    type StreamAuditStream = ReceiverStream<Result<AuditEvent, Status>>;

    async fn stream_audit(
        &self,
        _request: Request<StreamAuditRequest>,
    ) -> Result<Response<Self::StreamAuditStream>, Status> {
        let mut rx = self.audit.tx.subscribe();
        let (tx, stream_rx) = mpsc::channel(32);
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(stream_rx)))
    }
}

#[tonic::async_trait]
impl GraphRetrievalService for McpService {
    async fn retrieve_skills(
        &self,
        request: Request<RetrieveSkillsRequest>,
    ) -> Result<Response<RetrieveSkillsResponse>, Status> {
        let response = self
            .retrieve_skills_internal(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(response))
    }

    async fn retrieve_code_context(
        &self,
        request: Request<RetrieveCodeContextRequest>,
    ) -> Result<Response<RetrieveCodeContextResponse>, Status> {
        let response = self
            .retrieve_code_internal(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(response))
    }
}

#[tonic::async_trait]
impl RetrievalService for McpService {
    async fn retrieve_skills(
        &self,
        request: Request<RetrieveSkillsRequest>,
    ) -> Result<Response<RetrieveSkillsResponse>, Status> {
        let response = self
            .retrieve_skills_internal(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(response))
    }

    async fn retrieve_code_context(
        &self,
        request: Request<RetrieveCodeContextRequest>,
    ) -> Result<Response<RetrieveCodeContextResponse>, Status> {
        let response = self
            .retrieve_code_internal(request.into_inner())
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(response))
    }
}

struct ReadFileTool;
struct ListDirTool;
struct GlobPathsTool;
struct GenerateDiffTool;
struct ApplyPatchTool;
struct AstChunkTool;
struct SymbolLookupTool;
struct RunCommandTool;
struct GraphRetrieveSkillsTool;
struct GraphRetrieveCodeTool;

struct RequestUserInputTool {
    gate: Arc<MissionHitlGate>,
}

#[tonic::async_trait]
impl EmbeddedTool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "read_file",
            "Read a UTF-8 file inside the workspace",
            vec!["read_file".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([("path", string_value("string"))]),
            Some(struct_from_map([("content", string_value("string"))])),
            "filesystem",
            false,
            "file_read",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let content = std::fs::read_to_string(&ctx.scope)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([("content", string_value(content))])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "list_dir",
            "List directory entries inside the workspace",
            vec!["list_dir".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("path", string_value("string")),
                ("max_entries", string_value("integer")),
            ]),
            Some(struct_from_map([
                ("entries", string_value("string[]")),
                ("truncated", bool_value(true)),
            ])),
            "filesystem",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        if !ctx.scope.is_dir() {
            return Err(McpError::ToolExecution(format!(
                "'{}' is not a directory",
                ctx.scope.display()
            )));
        }

        let max_entries = integer_argument(&ctx.request.arguments, "max_entries")
            .unwrap_or(200)
            .clamp(1, 1_000);
        let mut entries = std::fs::read_dir(&ctx.scope)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect::<Vec<_>>();
        entries.sort();
        let truncated = entries.len() > max_entries;
        entries.truncate(max_entries);

        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                ("entries", string_list_value(entries)),
                ("truncated", bool_value(truncated)),
            ])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GlobPathsTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "glob_paths",
            "Find workspace paths matching a glob pattern",
            vec!["glob_paths".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("pattern", string_value("string")),
                ("path", string_value("string")),
                ("max_results", string_value("integer")),
            ]),
            Some(struct_from_map([
                ("paths", string_value("string[]")),
                ("truncated", bool_value(true)),
            ])),
            "filesystem",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let pattern = required_argument(&ctx.request.arguments, "pattern")?;
        let matcher =
            Pattern::new(&pattern).map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let max_results = integer_argument(&ctx.request.arguments, "max_results")
            .unwrap_or(200)
            .clamp(1, 1_000);
        let base = if ctx.scope.is_dir() {
            ctx.scope.clone()
        } else {
            ctx.scope
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| ctx.workspace_root.to_path_buf())
        };

        let mut paths = Vec::new();
        for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_entry(|entry| should_descend_tool_walk(entry))
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            let relative = relative_to_workspace(ctx.workspace_root, path);
            if matcher.matches_path(Path::new(&relative)) {
                paths.push(relative);
                if paths.len() >= max_results {
                    break;
                }
            }
        }
        paths.sort();
        let truncated = paths.len() >= max_results;

        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                ("paths", string_list_value(paths)),
                ("truncated", bool_value(truncated)),
            ])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GenerateDiffTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "generate_diff",
            "Generate a unified diff from current and updated file content",
            vec!["generate_diff".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("path", string_value("string")),
                ("updated_content", string_value("string")),
            ]),
            Some(struct_from_map([("diff", string_value("string"))])),
            "filesystem",
            false,
            "tool_call",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let current = std::fs::read_to_string(&ctx.scope).unwrap_or_default();
        let updated = required_argument(&ctx.request.arguments, "updated_content")?;
        let relative = relative_to_workspace(ctx.workspace_root, &ctx.scope);
        let diff = UnifiedDiff::generate(&current, &updated, &relative, &relative).to_string();
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([("diff", string_value(diff))])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for ApplyPatchTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "apply_patch",
            "Apply a unified diff patch to a file inside the workspace",
            vec!["apply_patch".to_string()],
            vec![".".to_string()],
            false,
            true,
            false,
            struct_from_map([
                ("path", string_value("string")),
                ("patch", string_value("string")),
            ]),
            Some(struct_from_map([
                ("path", string_value("string")),
                ("applied", bool_value(true)),
            ])),
            "filesystem",
            true,
            "tool_call",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let patch = required_argument(&ctx.request.arguments, "patch")?;
        let current = std::fs::read_to_string(&ctx.scope).unwrap_or_default();
        let outcome = ctx
            .executor
            .apply_patch(PatchRequest {
                workspace_root: ctx.workspace_root.to_path_buf(),
                scope: ctx.scope.clone(),
                current,
                patch,
            })
            .await?;
        if !outcome.success {
            return Ok(failure_result(ctx.request, outcome.stderr));
        }
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                (
                    "path",
                    string_value(relative_to_workspace(ctx.workspace_root, &ctx.scope)),
                ),
                ("applied", bool_value(true)),
            ])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for AstChunkTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "ast_chunk",
            "Chunk a source file using the native Tree-sitter chunker",
            vec!["ast_chunk".to_string()],
            vec!["src".to_string(), ".".to_string()],
            false,
            false,
            true,
            struct_from_map([("path", string_value("string"))]),
            None,
            "filesystem",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let content = std::fs::read_to_string(&ctx.scope)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let language =
            Chunker::detect_language(&ctx.scope).unwrap_or_else(|| "unknown".to_string());
        let mut chunker = Chunker::new().map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let chunks = chunker
            .extract_chunks(&content, &ctx.scope, &language)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let output = Struct {
            fields: [(
                "chunks".to_string(),
                Value {
                    kind: Some(Kind::ListValue(ListValue {
                        values: chunks
                            .into_iter()
                            .map(|chunk| Value {
                                kind: Some(Kind::StructValue(struct_from_map([
                                    ("id", string_value(chunk.id)),
                                    ("type", string_value(format!("{:?}", chunk.chunk_type))),
                                    ("start_line", number_value(chunk.line_range.0 as f64)),
                                    ("end_line", number_value(chunk.line_range.1 as f64)),
                                    ("signature", string_value(chunk.metadata.signature)),
                                ]))),
                            })
                            .collect(),
                    })),
                },
            )]
            .into_iter()
            .collect(),
        };
        Ok(success_result(ctx.request, Some(output)))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for SymbolLookupTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "symbol_lookup",
            "Lookup symbols through the native SCIP parser registry",
            vec!["symbol_lookup".to_string()],
            vec!["src".to_string(), ".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("path", string_value("string")),
                ("query", string_value("string")),
            ]),
            None,
            "retrieval",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let language = detect_language(&ctx.scope)?;
        let registry = ParserRegistry::new();
        let scip = registry
            .parse(language, ctx.workspace_root)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let query = extract_argument(&ctx.request.arguments, "query").unwrap_or_default();
        let symbols = scip
            .symbols
            .into_iter()
            .filter(|symbol| query.is_empty() || symbol.symbol.contains(&query))
            .take(20)
            .map(|symbol| Value {
                kind: Some(Kind::StructValue(struct_from_map([
                    ("symbol", string_value(symbol.symbol.clone())),
                    ("signature", string_value(symbol.signature.clone())),
                    ("kind", string_value(format!("{:?}", symbol.kind_enum()))),
                ]))),
            })
            .collect::<Vec<_>>();
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([(
                "symbols",
                Value {
                    kind: Some(Kind::ListValue(ListValue { values: symbols })),
                },
            )])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for RunCommandTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "run_command",
            "Run a bounded command in the workspace",
            vec!["run_command".to_string()],
            vec![".".to_string()],
            false,
            true,
            false,
            struct_from_map([
                ("program", string_value("string")),
                ("args", string_value("string[]")),
            ]),
            None,
            "command",
            true,
            "shell_command",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let command = extract_command(&ctx.request.arguments)?;
        if command.program.is_empty() {
            return Ok(failure_result(ctx.request, "missing command".to_string()));
        }
        if !ctx
            .config
            .allowed_commands
            .iter()
            .any(|allowed| allowed == &command.program)
        {
            return Ok(failure_result(
                ctx.request,
                format!("command '{}' is not on the MCP allowlist", command.program),
            ));
        }

        let timeout_secs = ctx
            .request
            .policy
            .as_ref()
            .map(|policy| policy.max_execution_seconds)
            .filter(|seconds| *seconds > 0)
            .unwrap_or(ctx.config.default_max_execution_seconds);
        let outcome = ctx
            .executor
            .run_command(CommandRequest {
                program: command.program.clone(),
                args: command.args.clone(),
                workspace_root: ctx.workspace_root.to_path_buf(),
                timeout_secs,
            })
            .await?;
        if outcome.exit_code == -1 {
            return Ok(timeout_result(ctx.request, &command.program, timeout_secs));
        }

        Ok(ToolCallResult {
            request_id: ctx.request.request_id.clone(),
            success: outcome.success,
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            exit_code: outcome.exit_code,
            output: Some(struct_from_map([
                ("program", string_value(command.program)),
                (
                    "args",
                    Value {
                        kind: Some(Kind::ListValue(ListValue {
                            values: command.args.into_iter().map(string_value).collect(),
                        })),
                    },
                ),
            ])),
            audit_event: None,
        })
    }
}

#[tonic::async_trait]
impl EmbeddedTool for MassRefactorTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "mass_refactor",
            "Run a sandboxed Python refactor against a staged workspace",
            vec!["mass_refactor".to_string()],
            vec![".".to_string()],
            false,
            true,
            false,
            struct_from_map([
                ("script", string_value("string")),
                ("target_paths", string_value("string[]")),
                ("consent_mode", string_value("string")),
                ("timeout_secs", number_value(30.0)),
            ]),
            None,
            "refactor",
            true,
            "tool_call",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let script = required_argument(&ctx.request.arguments, "script")?;
        let consent_mode = required_argument(&ctx.request.arguments, "consent_mode")?;
        let target_paths = required_list_argument(&ctx.request.arguments, "target_paths")?;
        let timeout_secs = extract_argument(&ctx.request.arguments, "timeout_secs")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|seconds| *seconds > 0)
            .unwrap_or(ctx.config.mass_refactor_executor.timeout_secs);

        let mut resolved_targets = Vec::new();
        let tool_policy = MachineToolPolicyEngine;
        for raw_path in &target_paths {
            let resolved = resolve_target_path(ctx.workspace_root, raw_path)?;
            tool_policy.validate(
                ctx.request.policy.as_ref(),
                "mass_refactor",
                ctx.workspace_root,
                &resolved,
            )?;
            resolved_targets.push(
                resolved
                    .strip_prefix(ctx.workspace_root)
                    .map_err(|err| McpError::ToolExecution(err.to_string()))?
                    .to_path_buf(),
            );
        }

        let result = self
            .execute(MassRefactorRequest {
                session_id: next_mass_refactor_session_id(),
                workspace_root: ctx.workspace_root.to_path_buf(),
                target_paths: resolved_targets,
                script,
                timeout_secs,
                consent_mode,
            })
            .await?;

        let changed_files = Value {
            kind: Some(Kind::ListValue(ListValue {
                values: result
                    .changed_files
                    .iter()
                    .cloned()
                    .map(string_value)
                    .collect(),
            })),
        };
        let mut output_fields = vec![
            (
                "rollback_performed".to_string(),
                bool_value(result.rollback_performed),
            ),
            ("changed_files".to_string(), changed_files),
        ];
        if result.success {
            output_fields.push(("diff".to_string(), string_value(result.diff.clone())));
        } else {
            output_fields.push((
                "consent_mode".to_string(),
                string_value(MASS_REFACTOR_CONSENT_APPROVED),
            ));
        }

        Ok(ToolCallResult {
            request_id: ctx.request.request_id.clone(),
            success: result.success,
            stdout: result.execution.stdout,
            stderr: result.stderr,
            exit_code: result.execution.exit_code,
            output: Some(Struct {
                fields: output_fields.into_iter().collect(),
            }),
            audit_event: None,
        })
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GraphRetrieveSkillsTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "graph_retrieve_skills",
            "Retrieve statistically relevant SKILL.md payloads for the active task",
            vec!["graph_retrieve_skills".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("query", string_value("string")),
                ("task_id", string_value("string")),
                ("focal_files", string_value("string[]")),
                ("focal_symbols", string_value("string[]")),
            ]),
            None,
            "retrieval",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let query = required_argument(&ctx.request.arguments, "query")?;
        let role = extract_argument(&ctx.request.arguments, "role")
            .unwrap_or_else(|| ctx.request.role.clone());
        let task_id = extract_argument(&ctx.request.arguments, "task_id")
            .unwrap_or_else(|| ctx.request.request_id.clone());
        let focal_files = list_argument(&ctx.request.arguments, "focal_files");
        let focal_symbols = list_argument(&ctx.request.arguments, "focal_symbols");
        let skill_token_budget = extract_argument(&ctx.request.arguments, "skill_token_budget")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1500);
        let dense_limit = extract_argument(&ctx.request.arguments, "dense_limit")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(64);
        let bm25_limit = extract_argument(&ctx.request.arguments, "bm25_limit")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(64);
        let include_diagnostics = extract_argument(&ctx.request.arguments, "include_diagnostics")
            .map(|value| value == "true")
            .unwrap_or(true);

        let response = ctx
            .retrieval_router
            .skill
            .retrieve_skills(RetrieveSkillsRequest {
                request_id: ctx.request.request_id.clone(),
                agent_id: ctx.request.agent_id.clone(),
                role,
                task_id,
                workspace_root: ctx.workspace_root.display().to_string(),
                query,
                focal_files,
                focal_symbols,
                skill_token_budget,
                dense_limit,
                bm25_limit,
                include_diagnostics,
            })
            .await?;

        Ok(success_result(
            ctx.request,
            Some(retrieve_skills_struct(&response)),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GraphRetrieveCodeTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "graph_retrieve_code",
            "Retrieve structurally reachable code context for the active task",
            vec!["graph_retrieve_code".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("query", string_value("string")),
                ("task_id", string_value("string")),
                ("focal_files", string_value("string[]")),
                ("focal_symbols", string_value("string[]")),
            ]),
            None,
            "retrieval",
            false,
            "search_results",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let query = required_argument(&ctx.request.arguments, "query")?;
        let role = extract_argument(&ctx.request.arguments, "role")
            .unwrap_or_else(|| ctx.request.role.clone());
        let task_id = extract_argument(&ctx.request.arguments, "task_id")
            .unwrap_or_else(|| ctx.request.request_id.clone());
        let focal_files = list_argument(&ctx.request.arguments, "focal_files");
        let focal_symbols = list_argument(&ctx.request.arguments, "focal_symbols");
        let token_budget = extract_argument(&ctx.request.arguments, "token_budget")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(ctx.config.code_retrieval_budget_tokens as u32);
        let dense_limit = extract_argument(&ctx.request.arguments, "dense_limit")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(32);
        let include_diagnostics = extract_argument(&ctx.request.arguments, "include_diagnostics")
            .map(|value| value == "true")
            .unwrap_or(true);

        let response = ctx
            .retrieval_router
            .code
            .retrieve_code(RetrieveCodeContextRequest {
                request_id: ctx.request.request_id.clone(),
                agent_id: ctx.request.agent_id.clone(),
                role,
                task_id,
                workspace_root: ctx.workspace_root.display().to_string(),
                query,
                focal_files,
                focal_symbols,
                token_budget,
                dense_limit,
                include_diagnostics,
                candidate_limit: dense_limit,
            })
            .await?;

        Ok(success_result(
            ctx.request,
            Some(retrieve_code_struct(&response)),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for RequestUserInputTool {
    fn definition(&self) -> ToolDefinition {
        make_tool_definition(
            "request_user_input",
            "Raise a structured HITL question; mission enters pending_answer until answered",
            vec!["request_user_input".to_string()],
            vec![".".to_string()],
            false,
            false,
            true,
            struct_from_map([
                ("mission_id", string_value("string")),
                ("turn_index", number_value(0.0)),
                ("kind", string_value("string")),
                ("text", string_value("string")),
                ("options_json", string_value("string")),
            ]),
            None,
            "approval",
            false,
            "approval_request",
        )
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let trusted_mission = ctx.request.mission_id.trim();
        let envelope = parse_hitl_question_from_arguments(
            ctx.request.agent_id.as_str(),
            &ctx.request.arguments,
            if trusted_mission.is_empty() {
                None
            } else {
                Some(trusted_mission)
            },
        )?;
        let mission_id = envelope.mission_id.clone();
        let qid = self
            .gate
            .raise_question(envelope, mission_id.as_str())
            .await
            .map_err(|e| McpError::ToolExecution(e.to_string()))?;
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                ("question_id", string_value(qid)),
                (
                    "mission_lifecycle",
                    string_value("pending_answer".to_string()),
                ),
            ])),
        ))
    }
}

#[derive(serde::Deserialize)]
struct HitlOptionJson {
    id: String,
    label: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(serde::Deserialize)]
struct HitlConstraintsJson {
    #[serde(default)]
    min_selections: u32,
    #[serde(default = "default_max_sel")]
    max_selections: u32,
    free_text_max_chars: Option<u32>,
}

fn default_max_sel() -> u32 {
    u32::MAX
}

fn parse_hitl_question_from_arguments(
    agent_id: &str,
    arguments: &Option<Struct>,
    trusted_mission_id: Option<&str>,
) -> Result<QuestionEnvelope, McpError> {
    let mission_id = if let Some(m) = trusted_mission_id.filter(|s| !s.is_empty()) {
        m.to_string()
    } else {
        required_argument(arguments, "mission_id")?
    };
    let session_id = extract_argument(arguments, "session_id")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| agent_id.to_string());
    let turn_index = extract_argument(arguments, "turn_index")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| McpError::ToolExecution("turn_index (u32) required".into()))?;
    let text = required_argument(arguments, "text")?;
    let kind_str = required_argument(arguments, "kind")?;
    let kind = parse_question_kind(&kind_str)?;

    let options_json = extract_argument(arguments, "options_json").unwrap_or_else(|| "[]".into());
    let parsed_options: Vec<HitlOptionJson> = serde_json::from_str(&options_json)
        .map_err(|e| McpError::ToolExecution(format!("invalid options_json: {e}")))?;
    let options: Vec<QuestionOption> = parsed_options
        .into_iter()
        .map(|o| QuestionOption {
            id: o.id,
            label: o.label,
            description: o.description,
            is_default: o.is_default,
        })
        .collect();

    let constraints: Option<QuestionConstraints> = extract_argument(arguments, "constraints_json")
        .filter(|s| !s.is_empty())
        .map(|s| {
            let c: HitlConstraintsJson = serde_json::from_str(&s)
                .map_err(|e| McpError::ToolExecution(format!("invalid constraints_json: {e}")))?;
            Ok(QuestionConstraints {
                min_selections: c.min_selections,
                max_selections: c.max_selections,
                free_text_max_chars: c.free_text_max_chars,
            })
        })
        .transpose()?;

    let expiry_token = extract_argument(arguments, "expiry_token");
    let sensitive = extract_argument(arguments, "sensitive")
        .map(|v| v == "true")
        .unwrap_or(false);

    let expires_at = match extract_argument(arguments, "expires_at_unix") {
        Some(s) => {
            let secs: i64 = s.parse().map_err(|_| {
                McpError::ToolExecution("expires_at_unix must be integer seconds".into())
            })?;
            Some(prost_types::Timestamp {
                seconds: secs,
                nanos: 0,
            })
        }
        None => None,
    };

    Ok(QuestionEnvelope {
        question_id: String::new(),
        mission_id,
        session_id,
        turn_index,
        text,
        kind,
        options,
        constraints,
        expiry_token,
        sensitive,
        expires_at,
    })
}

fn parse_question_kind(s: &str) -> Result<i32, McpError> {
    let k = match s.to_ascii_lowercase().as_str() {
        "single" => QuestionKind::Single,
        "multi" => QuestionKind::Multi,
        "free_text" => QuestionKind::FreeText,
        "mixed" => QuestionKind::Mixed,
        _ => {
            return Err(McpError::ToolExecution(format!(
                "unknown kind '{s}' (expected single|multi|free_text|mixed)"
            )));
        }
    };
    Ok(k as i32)
}

fn retrieve_skills_struct(response: &RetrieveSkillsResponse) -> Struct {
    struct_from_map([
        ("request_id", string_value(response.request_id.clone())),
        (
            "skills",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: response
                        .skills
                        .iter()
                        .map(|skill| Value {
                            kind: Some(Kind::StructValue(retrieved_skill_struct(skill))),
                        })
                        .collect(),
                })),
            },
        ),
        (
            "diagnostics",
            Value {
                kind: response.diagnostics.as_ref().map(|diagnostics| {
                    Kind::StructValue(retrieval_diagnostics_struct(diagnostics))
                }),
            },
        ),
    ])
}

fn retrieve_code_struct(response: &RetrieveCodeContextResponse) -> Struct {
    struct_from_map([
        ("request_id", string_value(response.request_id.clone())),
        (
            "documents",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: response
                        .documents
                        .iter()
                        .map(|document| Value {
                            kind: Some(Kind::StructValue(struct_from_map([
                                ("chunk_id", string_value(document.chunk_id.clone())),
                                ("file_path", string_value(document.file_path.clone())),
                                ("symbol_path", string_value(document.symbol_path.clone())),
                                ("content", string_value(document.content.clone())),
                                ("token_cost", number_value(document.token_cost as f64)),
                                ("dense_score", number_value(document.dense_score as f64)),
                                (
                                    "accept_posterior",
                                    number_value(document.accept_posterior as f64),
                                ),
                                ("cross_score", number_value(document.cross_score as f64)),
                                ("fusion_score", number_value(document.fusion_score as f64)),
                            ]))),
                        })
                        .collect(),
                })),
            },
        ),
        (
            "diagnostics",
            Value {
                kind: response.diagnostics.as_ref().map(|diagnostics| {
                    Kind::StructValue(retrieval_diagnostics_struct(diagnostics))
                }),
            },
        ),
    ])
}

fn retrieved_skill_struct(skill: &RetrievedSkill) -> Struct {
    struct_from_map([
        ("skill_id", string_value(skill.skill_id.clone())),
        ("title", string_value(skill.title.clone())),
        ("source_path", string_value(skill.source_path.clone())),
        ("content", string_value(skill.content.clone())),
        ("token_cost", number_value(skill.token_cost as f64)),
        ("fusion_score", number_value(skill.fusion_score as f64)),
        (
            "accept_posterior",
            number_value(skill.accept_posterior as f64),
        ),
        ("cross_score", number_value(skill.cross_score as f64)),
    ])
}

fn retrieval_diagnostics_struct(diagnostics: &RetrievalDiagnostics) -> Struct {
    struct_from_map([
        (
            "contract",
            Value {
                kind: diagnostics
                    .contract
                    .as_ref()
                    .map(|contract| Kind::StructValue(retrieval_contract_struct(contract))),
            },
        ),
        (
            "channel_stats",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: diagnostics
                        .channel_stats
                        .iter()
                        .map(|stat| Value {
                            kind: Some(Kind::StructValue(struct_from_map([
                                ("channel", string_value(stat.channel.clone())),
                                ("hits", number_value(stat.hits as f64)),
                                ("participated", bool_value(stat.participated)),
                            ]))),
                        })
                        .collect(),
                })),
            },
        ),
        (
            "fused_candidates",
            number_value(diagnostics.fused_candidates as f64),
        ),
        (
            "accept_count",
            number_value(diagnostics.accept_count as f64),
        ),
        (
            "reject_count",
            number_value(diagnostics.reject_count as f64),
        ),
        (
            "selected_count",
            number_value(diagnostics.selected_count as f64),
        ),
        ("used_tokens", number_value(diagnostics.used_tokens as f64)),
        ("memgas_converged", bool_value(diagnostics.memgas_converged)),
        (
            "memgas_degenerate",
            bool_value(diagnostics.memgas_degenerate),
        ),
        (
            "candidate_scores",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: diagnostics
                        .candidate_scores
                        .iter()
                        .map(|score| Value {
                            kind: Some(Kind::StructValue(candidate_score_struct(score))),
                        })
                        .collect(),
                })),
            },
        ),
        (
            "stage_stats",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: diagnostics
                        .stage_stats
                        .iter()
                        .map(|stage| Value {
                            kind: Some(Kind::StructValue(struct_from_map([
                                ("stage", string_value(stage.stage.clone())),
                                ("latency_ms", number_value(stage.latency_ms as f64)),
                                ("input_count", number_value(stage.input_count as f64)),
                                ("output_count", number_value(stage.output_count as f64)),
                                ("degraded", bool_value(stage.degraded)),
                            ]))),
                        })
                        .collect(),
                })),
            },
        ),
        ("degraded_mode", bool_value(diagnostics.degraded_mode)),
    ])
}

fn candidate_score_struct(score: &CandidateScore) -> Struct {
    struct_from_map([
        ("document_id", string_value(score.document_id.clone())),
        ("title", string_value(score.title.clone())),
        (
            "channel_scores",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: score
                        .channel_scores
                        .iter()
                        .map(|channel| Value {
                            kind: Some(Kind::StructValue(struct_from_map([
                                ("channel", string_value(channel.channel.clone())),
                                ("rank", number_value(channel.rank as f64)),
                                ("score", number_value(channel.score as f64)),
                            ]))),
                        })
                        .collect(),
                })),
            },
        ),
        ("fusion_score", number_value(score.fusion_score as f64)),
        (
            "accept_posterior",
            number_value(score.accept_posterior as f64),
        ),
        ("cross_score", number_value(score.cross_score as f64)),
        ("token_cost", number_value(score.token_cost as f64)),
        ("selected", bool_value(score.selected)),
    ])
}

fn retrieval_contract_struct(contract: &openakta_proto::mcp::v1::RetrievalContract) -> Struct {
    struct_from_map([
        (
            "contract_version",
            string_value(contract.contract_version.clone()),
        ),
        (
            "embedding_schema_version",
            string_value(contract.embedding_schema_version.clone()),
        ),
        (
            "chunk_schema_version",
            string_value(contract.chunk_schema_version.clone()),
        ),
        (
            "payload_schema_version",
            string_value(contract.payload_schema_version.clone()),
        ),
        (
            "candidate_channels",
            string_list_value(contract.candidate_channels.clone()),
        ),
        (
            "fusion_policy",
            string_value(contract.fusion_policy.clone()),
        ),
        (
            "rerank_policy",
            string_value(contract.rerank_policy.clone()),
        ),
        (
            "selection_policy",
            string_value(contract.selection_policy.clone()),
        ),
        (
            "diagnostics_schema_version",
            string_value(contract.diagnostics_schema_version.clone()),
        ),
    ])
}

fn code_response_from_result(
    request_id: String,
    result: &openakta_rag::CodeRetrievalResult,
    include_diagnostics: bool,
) -> RetrieveCodeContextResponse {
    RetrieveCodeContextResponse {
        request_id,
        documents: result
            .final_result
            .selection
            .selected_documents
            .iter()
            .map(|candidate| RetrievedCodeContext {
                chunk_id: candidate.accepted.candidate.document.chunk_id.clone(),
                file_path: candidate.accepted.candidate.document.file_path.clone(),
                symbol_path: candidate
                    .accepted
                    .candidate
                    .document
                    .symbol_path
                    .clone()
                    .unwrap_or_default(),
                content: candidate.accepted.candidate.document.body_markdown.clone(),
                token_cost: candidate.token_cost as u32,
                dense_score: candidate.accepted.candidate.dense_score.unwrap_or_default(),
                accept_posterior: candidate.accepted.accept_posterior,
                cross_score: candidate.cross_score,
                fusion_score: candidate.accepted.candidate.rrf_score,
            })
            .collect(),
        diagnostics: include_diagnostics.then(|| diagnostics_to_proto(&result.diagnostics)),
    }
}

fn diagnostics_to_proto(data: &RetrievalDiagnosticsData) -> RetrievalDiagnostics {
    RetrievalDiagnostics {
        contract: Some(openakta_proto::mcp::v1::RetrievalContract {
            contract_version: data.contract.contract_version.clone(),
            embedding_schema_version: data.contract.embedding_schema_version.clone(),
            chunk_schema_version: data.contract.chunk_schema_version.clone(),
            payload_schema_version: data.contract.payload_schema_version.clone(),
            candidate_channels: data.contract.candidate_channels.clone(),
            fusion_policy: data.contract.fusion_policy.clone(),
            rerank_policy: data.contract.rerank_policy.clone(),
            selection_policy: data.contract.selection_policy.clone(),
            diagnostics_schema_version: data.contract.diagnostics_schema_version.clone(),
        }),
        channel_stats: data
            .channel_stats
            .iter()
            .map(|stat| openakta_proto::mcp::v1::RetrievalChannelStat {
                channel: stat.channel.clone(),
                hits: stat.hits,
                participated: stat.participated,
            })
            .collect(),
        fused_candidates: data.fused_candidates as u32,
        accept_count: data.accept_count as u32,
        reject_count: data.reject_count as u32,
        selected_count: data.selected_count as u32,
        used_tokens: data.used_tokens as u32,
        memgas_converged: data.memgas_converged,
        memgas_degenerate: data.memgas_degenerate,
        candidate_scores: data
            .candidate_scores
            .iter()
            .map(|score| CandidateScore {
                document_id: score.document_id.clone(),
                title: score.title.clone(),
                channel_scores: score
                    .channel_scores
                    .iter()
                    .map(|channel| openakta_proto::mcp::v1::ChannelScore {
                        channel: channel.channel.clone(),
                        rank: channel.rank,
                        score: channel.score,
                    })
                    .collect(),
                fusion_score: score.fusion_score,
                accept_posterior: score.accept_posterior,
                cross_score: score.cross_score,
                token_cost: score.token_cost as u32,
                selected: score.selected,
            })
            .collect(),
        stage_stats: data
            .stage_stats
            .iter()
            .map(|stage| openakta_proto::mcp::v1::RetrievalStageStat {
                stage: stage.stage.clone(),
                latency_ms: stage.latency_ms,
                input_count: stage.input_count,
                output_count: stage.output_count,
                degraded: stage.degraded,
            })
            .collect(),
        degraded_mode: data.degraded_mode,
        generated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
    }
}

fn success_result(request: &ToolCallRequest, output: Option<Struct>) -> ToolCallResult {
    ToolCallResult {
        request_id: request.request_id.clone(),
        success: true,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
        output,
        audit_event: None,
    }
}

fn failure_result(request: &ToolCallRequest, stderr: String) -> ToolCallResult {
    ToolCallResult {
        request_id: request.request_id.clone(),
        success: false,
        stdout: String::new(),
        stderr,
        exit_code: 1,
        output: None,
        audit_event: None,
    }
}

fn timeout_result(request: &ToolCallRequest, program: &str, timeout_secs: u32) -> ToolCallResult {
    ToolCallResult {
        request_id: request.request_id.clone(),
        success: false,
        stdout: String::new(),
        stderr: format!("command '{}' timed out after {}s", program, timeout_secs),
        exit_code: -1,
        output: Some(struct_from_map([("timed_out", bool_value(true))])),
        audit_event: None,
    }
}

fn extract_scope(arguments: &Option<Struct>) -> Option<PathBuf> {
    extract_argument(arguments, "path").map(PathBuf::from)
}

#[derive(Debug)]
struct ParsedCommand {
    program: String,
    args: Vec<String>,
}

fn extract_argument(arguments: &Option<Struct>, key: &str) -> Option<String> {
    arguments
        .as_ref()
        .and_then(|args| args.fields.get(key))
        .and_then(|value| value.kind.as_ref())
        .and_then(|kind| match kind {
            Kind::StringValue(value) => Some(value.clone()),
            _ => None,
        })
}

fn integer_argument(arguments: &Option<Struct>, key: &str) -> Option<usize> {
    extract_argument(arguments, key).and_then(|value| value.parse::<usize>().ok())
}

fn list_argument(arguments: &Option<Struct>, key: &str) -> Vec<String> {
    arguments
        .as_ref()
        .and_then(|args| args.fields.get(key))
        .and_then(|value| value.kind.as_ref())
        .and_then(|kind| match kind {
            Kind::ListValue(list) => Some(
                list.values
                    .iter()
                    .filter_map(|item| item.kind.as_ref())
                    .filter_map(|kind| match kind {
                        Kind::StringValue(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn required_argument(arguments: &Option<Struct>, key: &str) -> Result<String, McpError> {
    extract_argument(arguments, key)
        .ok_or_else(|| McpError::ToolExecution(format!("missing required argument '{}'", key)))
}

fn required_list_argument(arguments: &Option<Struct>, key: &str) -> Result<Vec<String>, McpError> {
    let values = list_argument(arguments, key);
    if values.is_empty() {
        return Err(McpError::ToolExecution(format!(
            "missing required list argument '{}'",
            key
        )));
    }
    Ok(values)
}

fn extract_string_list_argument(arguments: &Option<Struct>, key: &str) -> Option<Vec<String>> {
    arguments
        .as_ref()
        .and_then(|args| args.fields.get(key))
        .and_then(|value| value.kind.as_ref())
        .and_then(|kind| match kind {
            Kind::ListValue(ListValue { values }) => Some(
                values
                    .iter()
                    .filter_map(|value| match value.kind.as_ref() {
                        Some(Kind::StringValue(value)) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
}

fn extract_command(arguments: &Option<Struct>) -> Result<ParsedCommand, McpError> {
    if let Some(program) = extract_argument(arguments, "program") {
        return Ok(ParsedCommand {
            program,
            args: extract_string_list_argument(arguments, "args").unwrap_or_default(),
        });
    }

    let command = extract_argument(arguments, "command").unwrap_or_default();
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Ok(ParsedCommand {
            program: String::new(),
            args: Vec::new(),
        });
    };

    Ok(ParsedCommand {
        program: program.to_string(),
        args: parts.map(str::to_string).collect(),
    })
}

fn resolve_scope(workspace_root: &Path, arguments: &Option<Struct>) -> Result<PathBuf, McpError> {
    let scope = extract_scope(arguments).unwrap_or_else(|| workspace_root.to_path_buf());
    let candidate = if scope.is_absolute() {
        scope
    } else {
        workspace_root.join(scope)
    };

    let resolved = if candidate.exists() {
        candidate.canonicalize().map_err(|err| {
            McpError::ToolExecution(format!("failed to canonicalize scope: {err}"))
        })?
    } else if let Some(parent) = candidate.parent() {
        let parent = parent.canonicalize().map_err(|err| {
            McpError::ToolExecution(format!("failed to canonicalize scope parent: {err}"))
        })?;
        parent.join(candidate.file_name().unwrap_or_default())
    } else {
        workspace_root.to_path_buf()
    };

    Ok(resolved)
}

fn resolve_target_path(workspace_root: &Path, raw_path: &str) -> Result<PathBuf, McpError> {
    let candidate = PathBuf::from(raw_path);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        workspace_root.join(candidate)
    };

    let resolved = if candidate.exists() {
        candidate
            .canonicalize()
            .map_err(|err| McpError::ToolExecution(err.to_string()))?
    } else {
        let parent = candidate.parent().ok_or_else(|| {
            McpError::ToolExecution(format!("invalid target path '{}'", raw_path))
        })?;
        let resolved_parent = parent
            .canonicalize()
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        resolved_parent.join(candidate.file_name().ok_or_else(|| {
            McpError::ToolExecution(format!("invalid target path '{}'", raw_path))
        })?)
    };

    if !resolved.starts_with(workspace_root) {
        return Err(McpError::CapabilityDenied(format!(
            "target '{}' escapes workspace '{}'",
            raw_path,
            workspace_root.display()
        )));
    }

    Ok(resolved)
}

fn detect_language(path: &Path) -> Result<Language, McpError> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "rs" => Ok(Language::Rust),
        "ts" | "tsx" => Ok(Language::TypeScript),
        "py" => Ok(Language::Python),
        extension => Err(McpError::ToolExecution(format!(
            "unsupported language extension '{}'",
            extension
        ))),
    }
}

fn relative_to_workspace(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn should_descend_tool_walk(entry: &walkdir::DirEntry) -> bool {
    !matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".next"
            | ".openakta"
            | ".turbo"
            | "build"
            | "coverage"
            | "dist"
            | "node_modules"
            | "target"
            | ".DS_Store"
    )
}

/// Named capability profiles (`architect`, `coder`, …) gate which embedded tools are exposed.
/// This is **not** multi-tenant RBAC — profiles are local machine/runtime labels only.
fn capability_profile_allows_tool(capability_profile: &str, tool_name: &str) -> bool {
    match capability_profile {
        // Deny-by-default: unknown profile falls back to read-only — V-008.
        "" => tool_name == "read_file",
        "architect" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "symbol_lookup"
                | "graph_retrieve_skills"
                | "graph_retrieve_code"
                | "ast_chunk"
                | "request_user_input"
        ),
        "coder" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "generate_diff"
                | "apply_patch"
                | "ast_chunk"
                | "symbol_lookup"
                | "graph_retrieve_skills"
                | "graph_retrieve_code"
                | "request_user_input"
        ),
        "refactorer" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "graph_retrieve_skills"
                | "graph_retrieve_code"
                | "request_user_input"
                | "mass_refactor"
        ),
        "tester" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "run_command"
                | "graph_retrieve_skills"
                | "graph_retrieve_code"
                | "request_user_input"
        ),
        "executor" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "run_command"
                | "apply_patch"
                | "generate_diff"
                | "request_user_input"
        ),
        "reviewer" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "generate_diff"
                | "graph_retrieve_skills"
                | "graph_retrieve_code"
                | "symbol_lookup"
                | "request_user_input"
        ),
        "worker" | "implementation" => matches!(
            tool_name,
            "read_file"
                | "list_dir"
                | "glob_paths"
                | "run_command"
                | "generate_diff"
                | "apply_patch"
                | "request_user_input"
        ),
        _ => tool_name == "read_file",
    }
}

fn struct_from_map<const N: usize>(entries: [(&str, Value); N]) -> Struct {
    Struct {
        fields: entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    }
}

fn string_value(value: impl Into<String>) -> Value {
    Value {
        kind: Some(Kind::StringValue(value.into())),
    }
}

fn bool_value(value: bool) -> Value {
    Value {
        kind: Some(Kind::BoolValue(value)),
    }
}

fn string_list_value(values: Vec<String>) -> Value {
    Value {
        kind: Some(Kind::ListValue(ListValue {
            values: values.into_iter().map(string_value).collect(),
        })),
    }
}

fn number_value(value: f64) -> Value {
    Value {
        kind: Some(Kind::NumberValue(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_memory::{
        FusedCandidate, SkillCatalog, SkillCorpusIngestor, SkillDocument, SkillIndexBackend,
        SkillRetrievalConfig, SkillRetrievalPipeline,
    };
    use openakta_proto::mcp::v1::graph_retrieval_service_client::GraphRetrievalServiceClient;
    use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
    use openakta_proto::mcp::v1::tool_service_server::ToolService;
    use openakta_rag::{CrossEncoderScorer, RerankDocument};
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tonic::transport::Server;

    fn test_mcp_config(root: PathBuf) -> McpServiceConfig {
        McpServiceConfig {
            workspace_root: root.clone(),
            allowed_commands: vec!["cargo".to_string()],
            default_max_execution_seconds: 5,
            execution_mode: SandboxedToolExecutionMode::Hybrid,
            container_executor: ContainerExecutorConfig::default(),
            wasi_executor: WasiExecutorConfig::default(),
            mass_refactor_executor: MassRefactorExecutorConfig::default(),
            dense_backend: VectorBackendKind::SqliteJson,
            dense_qdrant_url: "http://127.0.0.1:6334".to_string(),
            dense_store_path: root.join(".openakta/vectors.db"),
            code_collection: CollectionSpec::code_default(),
            code_embedding: CodeEmbeddingConfig::default(),
            code_bm25_dir: root.join(".openakta/code-bm25"),
            code_index_state_path: root.join(".openakta/code-index-state.json"),
            code_retrieval_budget_tokens: 128,
            skill_config: SkillRetrievalConfig {
                corpus_root: root.join("skills"),
                catalog_db_path: root.join(".openakta/skill-index/skill-catalog.db"),
                dense_backend: VectorBackendKind::SqliteJson,
                dense_store_path: root.join(".openakta/vectors.db"),
                qdrant_url: "http://127.0.0.1:6334".to_string(),
                dense_collection: CollectionSpec::skill_default(),
                embedding: openakta_embeddings::SkillEmbeddingConfig::default(),
                bm25_dir: root.join(".openakta/skill-bm25"),
                skill_token_budget: 1500,
                dense_limit: 64,
                bm25_limit: 64,
            },
            hitl_gate: None,
        }
    }

    fn policy(root: &str, actions: &[&str]) -> CapabilityPolicy {
        CapabilityPolicy {
            agent_id: "agent-1".to_string(),
            role: "coder".to_string(),
            allowed_actions: actions.iter().map(|action| (*action).to_string()).collect(),
            allowed_scope_patterns: vec![
                root.to_string(),
                "/Users/noasantos/Fluri/openakta".to_string(),
            ],
            denied_scope_patterns: vec!["/etc".to_string()],
            max_execution_seconds: 5,
        }
    }

    fn output_string_list(result: &ToolCallResult, key: &str) -> Vec<String> {
        let output = result.output.as_ref().expect("structured tool output");
        let value = output.fields.get(key).expect("missing output field");
        match value.kind.as_ref() {
            Some(Kind::ListValue(list)) => list
                .values
                .iter()
                .filter_map(|value| match value.kind.as_ref() {
                    Some(Kind::StringValue(value)) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            _ => panic!("expected list output for {key}"),
        }
    }

    #[tokio::test]
    async fn read_file_denies_outside_workspace() {
        let service = McpService::new();
        let req = ToolCallRequest {
            request_id: "req-1".to_string(),
            agent_id: "agent-1".to_string(),
            role: "coder".to_string(),
            tool_name: "read_file".to_string(),
            arguments: Some(struct_from_map([("path", string_value("/etc/passwd"))])),
            policy: Some(policy("/Users/noasantos/Fluri/openakta", &["read_file"])),
            workspace_root: "/Users/noasantos/Fluri/openakta".to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service.call_tool(Request::new(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn call_tool_rejects_role_forbidden_tool() {
        let service = McpService::new();
        let req = ToolCallRequest {
            request_id: "req-role".into(),
            agent_id: "a1".into(),
            role: "architect".into(),
            tool_name: "apply_patch".into(),
            arguments: Some(struct_from_map([
                ("path", string_value("x.rs")),
                ("patch", string_value("--- x.rs\n+++ x.rs\n")),
            ])),
            policy: Some(CapabilityPolicy {
                agent_id: "a1".into(),
                role: "architect".into(),
                allowed_actions: vec!["apply_patch".into()],
                allowed_scope_patterns: vec!["/tmp".into()],
                denied_scope_patterns: vec![],
                max_execution_seconds: 5,
            }),
            workspace_root: "/tmp".into(),
            mission_id: String::new(),
            ..Default::default()
        };
        let err = service
            .call_tool(Request::new(req))
            .await
            .expect_err("architect must not invoke apply_patch");
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
    }

    #[tokio::test]
    async fn empty_role_denies_tools_other_than_read_file() {
        let service = McpService::new();
        let req = ToolCallRequest {
            request_id: "req-empty-role".into(),
            agent_id: "a1".into(),
            role: String::new(),
            tool_name: "apply_patch".into(),
            arguments: Some(struct_from_map([
                ("path", string_value("x.rs")),
                ("patch", string_value("--- x.rs\n+++ x.rs\n")),
            ])),
            policy: Some(CapabilityPolicy {
                agent_id: "a1".into(),
                role: String::new(),
                allowed_actions: vec!["apply_patch".into()],
                allowed_scope_patterns: vec!["/tmp".into()],
                denied_scope_patterns: vec![],
                max_execution_seconds: 5,
            }),
            workspace_root: "/tmp".into(),
            mission_id: String::new(),
            ..Default::default()
        };
        let err = service
            .call_tool(Request::new(req))
            .await
            .expect_err("empty role must not invoke apply_patch");
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
    }

    #[tokio::test]
    async fn empty_role_list_tools_only_includes_read_file() {
        let service = McpService::new();
        let response = service
            .list_tools(Request::new(ListToolsRequest {
                agent_id: "agent-1".to_string(),
                role: String::new(),
            }))
            .await
            .unwrap()
            .into_inner();
        let names = response
            .tools
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file".to_string()]);
    }

    #[tokio::test]
    async fn mass_refactor_is_refactorer_only() {
        let temp_dir = TempDir::new().unwrap();
        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let req = ToolCallRequest {
            request_id: "req-mass-refactor-role".into(),
            agent_id: "a1".into(),
            role: "coder".into(),
            tool_name: "mass_refactor".into(),
            arguments: Some(Struct {
                fields: [
                    ("script".to_string(), string_value("print('hi')")),
                    (
                        "target_paths".to_string(),
                        Value {
                            kind: Some(Kind::ListValue(ListValue {
                                values: vec![string_value("src/lib.rs")],
                            })),
                        },
                    ),
                    (
                        "consent_mode".to_string(),
                        string_value("mass_script_approved"),
                    ),
                ]
                .into_iter()
                .collect(),
            }),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["mass_refactor"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let err = service
            .call_tool(Request::new(req))
            .await
            .expect_err("coder must not invoke mass_refactor");
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
    }

    #[tokio::test]
    async fn list_tools_returns_batteries_included_tooling() {
        let service = McpService::new();
        let response = service
            .list_tools(Request::new(ListToolsRequest {
                agent_id: "agent-1".to_string(),
                role: "coder".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();
        let names = response
            .tools
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"apply_patch".to_string()));
        assert!(names.contains(&"graph_retrieve_skills".to_string()));
        assert!(names.contains(&"graph_retrieve_code".to_string()));
    }

    #[tokio::test]
    async fn run_command_denies_unlisted_binary() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().display().to_string();
        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let req = ToolCallRequest {
            request_id: "req-2".to_string(),
            agent_id: "agent-1".to_string(),
            role: "executor".to_string(),
            tool_name: "run_command".to_string(),
            arguments: Some(struct_from_map([("program", string_value("git"))])),
            policy: Some(policy(&workspace_root, &["run_command"])),
            workspace_root,
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service
            .call_tool(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert!(!result.success);
        assert!(result.stderr.contains("allowlist"));
    }

    #[tokio::test]
    async fn apply_patch_updates_file_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("demo.rs");
        fs::write(&file_path, "fn before() {}\n").unwrap();

        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let patch = "--- demo.rs\n+++ demo.rs\n@@ -1,1 +1,1 @@\n-fn before() {}\n+fn after() {}\n";
        let req = ToolCallRequest {
            request_id: "req-3".to_string(),
            agent_id: "agent-1".to_string(),
            role: "coder".to_string(),
            tool_name: "apply_patch".to_string(),
            arguments: Some(struct_from_map([
                ("path", string_value("demo.rs")),
                ("patch", string_value(patch)),
            ])),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["apply_patch"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service
            .call_tool(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert!(result.success);
        assert_eq!(fs::read_to_string(file_path).unwrap(), "fn after() {}\n");
    }

    #[tokio::test]
    async fn apply_patch_denies_path_escape() {
        let temp_dir = TempDir::new().unwrap();
        let outside = temp_dir.path().join("..").join("escape.rs");
        fs::write(&outside, "fn before() {}\n").unwrap();

        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let patch = "--- ../escape.rs\n+++ ../escape.rs\n@@ -1,1 +1,1 @@\n-fn before() {}\n+fn after() {}\n";
        let req = ToolCallRequest {
            request_id: "req-escape".to_string(),
            agent_id: "agent-1".to_string(),
            role: "coder".to_string(),
            tool_name: "apply_patch".to_string(),
            arguments: Some(struct_from_map([
                ("path", string_value("../escape.rs")),
                ("patch", string_value(patch)),
            ])),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["apply_patch"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service.call_tool(Request::new(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_command_executes_only_in_direct_mode() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = test_mcp_config(temp_dir.path().to_path_buf());
        config.allowed_commands = vec!["rustc".to_string()];

        let service = McpService::with_config_insecure_direct_host_for_tests(config);
        let req = ToolCallRequest {
            request_id: "req-direct".to_string(),
            agent_id: "agent-1".to_string(),
            role: "executor".to_string(),
            tool_name: "run_command".to_string(),
            arguments: Some(struct_from_map([
                ("program", string_value("rustc")),
                (
                    "args",
                    Value {
                        kind: Some(Kind::ListValue(ListValue {
                            values: vec![string_value("--version")],
                        })),
                    },
                ),
            ])),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["run_command"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service
            .call_tool(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        assert!(result.success);
    }

    #[tokio::test]
    async fn list_dir_returns_sorted_entries() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("apps/auth")).unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        fs::write(
            temp_dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - apps/*\n",
        )
        .unwrap();

        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let req = ToolCallRequest {
            request_id: "req-list-dir".to_string(),
            agent_id: "agent-1".to_string(),
            role: "architect".to_string(),
            tool_name: "list_dir".to_string(),
            arguments: Some(struct_from_map([
                ("path", string_value(".")),
                ("max_entries", string_value("10")),
            ])),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["list_dir"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service
            .call_tool(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        let entries = output_string_list(&result, "entries");
        assert!(result.success);
        assert!(entries.contains(&"apps".to_string()));
        assert!(entries.contains(&"package.json".to_string()));
        assert!(entries.contains(&"pnpm-workspace.yaml".to_string()));
    }

    #[tokio::test]
    async fn glob_paths_skips_generated_directories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("apps/auth/.next/dev")).unwrap();
        fs::create_dir_all(temp_dir.path().join("apps/bnf")).unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("apps/auth/package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("apps/bnf/package.json"), "{}").unwrap();
        fs::write(
            temp_dir.path().join("apps/auth/.next/dev/package.json"),
            "{}",
        )
        .unwrap();

        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));
        let req = ToolCallRequest {
            request_id: "req-glob".to_string(),
            agent_id: "agent-1".to_string(),
            role: "architect".to_string(),
            tool_name: "glob_paths".to_string(),
            arguments: Some(struct_from_map([
                ("pattern", string_value("**/package.json")),
                ("max_results", string_value("20")),
            ])),
            policy: Some(policy(
                &temp_dir.path().display().to_string(),
                &["glob_paths"],
            )),
            workspace_root: temp_dir.path().display().to_string(),
            mission_id: String::new(),
            ..Default::default()
        };

        let result = service
            .call_tool(Request::new(req))
            .await
            .unwrap()
            .into_inner();
        let paths = output_string_list(&result, "paths");
        assert!(result.success);
        assert!(paths.contains(&"apps/auth/package.json".to_string()));
        assert!(paths.contains(&"apps/bnf/package.json".to_string()));
        assert!(paths.contains(&"package.json".to_string()));
        assert!(!paths
            .iter()
            .any(|path| path.contains(".next/dev/package.json")));
    }

    #[tokio::test]
    async fn retrieve_code_context_allows_unanchored_queries() {
        let temp_dir = TempDir::new().unwrap();
        let service = McpService::with_config(test_mcp_config(temp_dir.path().to_path_buf()));

        let response = GraphRetrievalService::retrieve_code_context(
            &service,
            Request::new(RetrieveCodeContextRequest {
                request_id: "req-anchor".to_string(),
                agent_id: "agent-1".to_string(),
                role: "coder".to_string(),
                task_id: "task-1".to_string(),
                workspace_root: temp_dir.path().display().to_string(),
                query: "find entrypoint".to_string(),
                focal_files: vec![],
                focal_symbols: vec![],
                token_budget: 64,
                dense_limit: 8,
                include_diagnostics: false,
                candidate_limit: 8,
            }),
        )
        .await
        .unwrap()
        .into_inner();

        assert_eq!(response.request_id, "req-anchor");
    }

    struct PipelineRetriever<I, R>
    where
        I: SkillIndexBackend,
        R: CrossEncoderScorer,
    {
        pipeline: SkillRetrievalPipeline<I, R>,
    }

    #[tonic::async_trait]
    impl<I, R> SkillRetrieverService for PipelineRetriever<I, R>
    where
        I: SkillIndexBackend,
        R: CrossEncoderScorer,
    {
        async fn retrieve_skills(
            &self,
            request: RetrieveSkillsRequest,
        ) -> Result<RetrieveSkillsResponse, McpError> {
            self.pipeline
                .retrieve(&request)
                .await
                .map_err(|err| McpError::ToolExecution(err.to_string()))
        }
    }

    struct StaticCodeRetriever;

    #[tonic::async_trait]
    impl CodeRetrieverService for StaticCodeRetriever {
        async fn retrieve_code(
            &self,
            request: RetrieveCodeContextRequest,
        ) -> Result<RetrieveCodeContextResponse, McpError> {
            Ok(RetrieveCodeContextResponse {
                request_id: request.request_id,
                documents: vec![RetrievedCodeContext {
                    chunk_id: "chunk-1".to_string(),
                    file_path: "src/lib.rs".to_string(),
                    symbol_path: "demo::run".to_string(),
                    content: "fn run() {}".to_string(),
                    token_cost: 12,
                    dense_score: 0.9,
                    accept_posterior: 1.0,
                    cross_score: 0.8,
                    fusion_score: 0.95,
                }],
                diagnostics: None,
            })
        }
    }

    #[derive(Default)]
    struct InMemoryHybridIndex {
        docs: Mutex<HashMap<String, SkillDocument>>,
    }

    #[tonic::async_trait]
    impl SkillIndexBackend for InMemoryHybridIndex {
        async fn upsert_document(
            &self,
            document: &SkillDocument,
        ) -> openakta_memory::procedural_store::Result<()> {
            self.docs
                .lock()
                .await
                .insert(document.skill_id.clone(), document.clone());
            Ok(())
        }

        async fn delete_document(
            &self,
            skill_id: &str,
        ) -> openakta_memory::procedural_store::Result<()> {
            self.docs.lock().await.remove(skill_id);
            Ok(())
        }

        async fn search(
            &self,
            catalog: &SkillCatalog,
            _query: &str,
            _dense_limit: usize,
            _bm25_limit: usize,
        ) -> openakta_memory::procedural_store::Result<Vec<FusedCandidate>> {
            let docs = catalog.list_documents()?;
            Ok(docs
                .into_iter()
                .filter_map(|skill| {
                    let profile = match skill.skill_id.as_str() {
                        "AUTH_DEBUG" => Some((0.80, Some(1), Some(0.95), Some(1), Some(9.4))),
                        "AUTH_ROLLOUT" => Some((0.42, Some(4), Some(0.58), Some(5), Some(1.3))),
                        "CSS_THEME" => Some((0.01, Some(28), Some(0.04), Some(30), Some(0.1))),
                        _ => None,
                    };
                    profile.map(
                        |(rrf_score, dense_rank, dense_score, bm25_rank, bm25_score)| {
                            FusedCandidate {
                                skill,
                                rrf_score,
                                dense_rank,
                                dense_score,
                                bm25_rank,
                                bm25_score,
                            }
                        },
                    )
                })
                .collect())
        }
    }

    #[derive(Clone)]
    struct StaticCrossEncoder;

    #[tonic::async_trait]
    impl CrossEncoderScorer for StaticCrossEncoder {
        async fn score_pairs(
            &self,
            _query: &str,
            docs: &[RerankDocument],
        ) -> openakta_rag::Result<Vec<f32>> {
            Ok(docs
                .iter()
                .map(|doc| if doc.id == "AUTH_DEBUG" { 0.95 } else { 0.05 })
                .collect())
        }
    }

    #[tokio::test]
    async fn retrieve_skills_grpc_enforces_budget_and_filters_noise() {
        let temp_dir = TempDir::new().unwrap();
        let skill_root = temp_dir.path().join("skills");
        fs::create_dir_all(skill_root.join("auth")).unwrap();
        fs::create_dir_all(skill_root.join("ops")).unwrap();
        fs::create_dir_all(skill_root.join("noise")).unwrap();
        fs::write(
            skill_root.join("auth").join("SKILL.md"),
            "---\nskill_id: AUTH_DEBUG\nname: Auth Debug\ndomain: security\ntags: [jwt, auth]\nsummary: Debug JWT auth failures\n---\n# Auth Debug\n\nInspect JWT headers and issuer claims before rotating keys.\n",
        )
        .unwrap();
        fs::write(
            skill_root.join("ops").join("SKILL.md"),
            format!(
                "---\nskill_id: AUTH_ROLLOUT\nname: Auth Rollout\ndomain: security\ntags: [auth, rollout]\nsummary: Roll out auth config changes\n---\n# Auth Rollout\n\n{}\n",
                "Validate canary auth settings before rollout. ".repeat(80)
            ),
        )
        .unwrap();
        fs::write(
            skill_root.join("noise").join("SKILL.md"),
            "---\nskill_id: CSS_THEME\nname: CSS Theme\ndomain: frontend\ntags: [css]\nsummary: Change button colors\n---\n# CSS Theme\n\nAdjust button colors and spacing.\n",
        )
        .unwrap();

        let catalog_db = temp_dir.path().join("skill-catalog.db");
        let catalog = SkillCatalog::new(&catalog_db).unwrap();
        let ingestor = SkillCorpusIngestor::new(&skill_root);
        let index = Arc::new(InMemoryHybridIndex::default());
        let pipeline = SkillRetrievalPipeline::with_components(
            SkillRetrievalConfig {
                corpus_root: skill_root.clone(),
                catalog_db_path: catalog_db.clone(),
                dense_backend: VectorBackendKind::SqliteJson,
                dense_store_path: temp_dir.path().join("vectors.db"),
                qdrant_url: "http://127.0.0.1:6334".to_string(),
                dense_collection: CollectionSpec::skill_default(),
                embedding: openakta_embeddings::SkillEmbeddingConfig::default(),
                bm25_dir: temp_dir.path().join("bm25"),
                skill_token_budget: 64,
                dense_limit: 64,
                bm25_limit: 64,
            },
            catalog,
            ingestor,
            index,
            StaticCrossEncoder,
        )
        .unwrap();

        let mut config = test_mcp_config(temp_dir.path().to_path_buf());
        config.skill_config.skill_token_budget = 64;
        let service = McpService::with_retrievers(
            config,
            EmbeddedToolRegistry::builtin(),
            Arc::new(PipelineRetriever { pipeline }),
            Arc::new(StaticCodeRetriever),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let server = tokio::spawn(async move {
            Server::builder()
                .add_service(GraphRetrievalServiceServer::new(service))
                .serve(addr)
                .await
                .unwrap();
        });

        let endpoint = format!("http://{}", addr);
        let mut client = loop {
            match GraphRetrievalServiceClient::connect(endpoint.clone()).await {
                Ok(client) => break client,
                Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
            }
        };
        let response = client
            .retrieve_skills(RetrieveSkillsRequest {
                request_id: "req-typed".to_string(),
                agent_id: "agent-1".to_string(),
                role: "coder".to_string(),
                task_id: "task-1".to_string(),
                workspace_root: temp_dir.path().display().to_string(),
                query: "debug jwt auth failures".to_string(),
                focal_files: vec![],
                focal_symbols: vec![],
                skill_token_budget: 64,
                dense_limit: 8,
                bm25_limit: 8,
                include_diagnostics: true,
            })
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.skills.len(), 1);
        assert_eq!(response.skills[0].skill_id, "AUTH_DEBUG");
        assert!(response.skills[0].token_cost <= 64);

        let diagnostics = response.diagnostics.unwrap();
        assert_eq!(diagnostics.selected_count, 1);
        assert_eq!(diagnostics.reject_count, 1);
        assert!(diagnostics.used_tokens <= 64);

        server.abort();
    }

    #[tokio::test]
    async fn retrieve_code_context_routes_to_code_pipeline() {
        let temp_dir = TempDir::new().unwrap();
        let service = McpService::with_retrievers(
            test_mcp_config(temp_dir.path().to_path_buf()),
            EmbeddedToolRegistry::builtin(),
            Arc::new(PipelineRetriever {
                pipeline: SkillRetrievalPipeline::with_components(
                    SkillRetrievalConfig {
                        corpus_root: temp_dir.path().join("skills"),
                        catalog_db_path: temp_dir.path().join("catalog.db"),
                        dense_backend: VectorBackendKind::SqliteJson,
                        dense_store_path: temp_dir.path().join("vectors.db"),
                        qdrant_url: "http://127.0.0.1:6334".to_string(),
                        dense_collection: CollectionSpec::skill_default(),
                        embedding: openakta_embeddings::SkillEmbeddingConfig::default(),
                        bm25_dir: temp_dir.path().join("bm25"),
                        skill_token_budget: 64,
                        dense_limit: 8,
                        bm25_limit: 8,
                    },
                    SkillCatalog::new(temp_dir.path().join("catalog.db")).unwrap(),
                    SkillCorpusIngestor::new(temp_dir.path().join("skills")),
                    Arc::new(InMemoryHybridIndex::default()),
                    StaticCrossEncoder,
                )
                .unwrap(),
            }),
            Arc::new(StaticCodeRetriever),
        );

        let response = RetrievalService::retrieve_code_context(
            &service,
            Request::new(RetrieveCodeContextRequest {
                request_id: "req-code".to_string(),
                agent_id: "agent-1".to_string(),
                role: "coder".to_string(),
                task_id: "task-1".to_string(),
                workspace_root: temp_dir.path().display().to_string(),
                query: "find the run entrypoint".to_string(),
                focal_files: vec!["src/lib.rs".to_string()],
                focal_symbols: vec!["demo::run".to_string()],
                token_budget: 64,
                dense_limit: 8,
                include_diagnostics: false,
                candidate_limit: 8,
            }),
        )
        .await
        .unwrap()
        .into_inner();

        assert_eq!(response.documents.len(), 1);
        assert_eq!(response.documents[0].chunk_id, "chunk-1");
    }
}
