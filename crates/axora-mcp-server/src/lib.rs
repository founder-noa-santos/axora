//! MCP gRPC tool sandbox server for AXORA.

use axora_cache::{apply_patch, UnifiedDiff};
use axora_embeddings::{CodeEmbeddingConfig, JinaCodeEmbedder};
use axora_indexing::{
    Chunker, CollectionSpec, DenseVectorCollection, Language, ParserRegistry, QdrantVectorCollection,
    SqliteVecCollection, VectorBackendKind,
};
use axora_memory::{SkillRetrievalConfig, SkillRetrievalPipeline};
use axora_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalService;
use axora_proto::mcp::v1::tool_service_server::ToolService;
use axora_proto::mcp::v1::{
    AuditEvent, CapabilityPolicy, CandidateScore, ListToolsRequest, ListToolsResponse,
    RetrievalDiagnostics, RetrieveCodeContextRequest, RetrieveCodeContextResponse,
    RetrieveSkillsRequest, RetrieveSkillsResponse, RetrievedCodeContext, RetrievedSkill,
    StreamAuditRequest, ToolCallRequest, ToolCallResult, ToolDefinition,
};
use axora_rag::{CandleCrossEncoder, CodeRetrievalPipeline, CodeRetrievalQuery};
use parking_lot::RwLock;
use prost_types::{value::Kind, ListValue, Struct, Timestamp, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::process::Command as TokioCommand;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{timeout, Duration};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// MCP errors.
#[derive(Debug, Error)]
pub enum McpError {
    /// Scope denied by RBAC.
    #[error("scope denied: {0}")]
    ScopeDenied(String),

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
    /// Default code retrieval budget.
    pub code_retrieval_budget_tokens: usize,
    /// Skill retrieval configuration.
    pub skill_config: SkillRetrievalConfig,
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
            dense_backend: VectorBackendKind::Qdrant,
            dense_qdrant_url: "http://127.0.0.1:6334".to_string(),
            dense_store_path: PathBuf::from(".axora/vectors.db"),
            code_collection: CollectionSpec::code_default(),
            code_embedding: CodeEmbeddingConfig::default(),
            code_retrieval_budget_tokens: 2_000,
            skill_config: SkillRetrievalConfig::default(),
        }
    }
}

/// Capability evaluator for tool requests.
#[derive(Debug, Default, Clone)]
pub struct RbacEngine;

impl RbacEngine {
    /// Validate access for a tool call.
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

        if !policy.allowed_actions.is_empty()
            && !policy.allowed_actions.iter().any(|action| action == tool_name)
        {
            return Err(McpError::ScopeDenied(format!(
                "tool '{tool_name}' not allowed for role '{}'",
                policy.role
            )));
        }

        if !scope.starts_with(workspace_root) {
            return Err(McpError::ScopeDenied(format!(
                "scope '{}' escapes workspace '{}'",
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
            return Err(McpError::ScopeDenied(format!(
                "scope '{}' denied by policy",
                scope.display()
            )));
        }

        if !policy.allowed_scope_patterns.is_empty()
            && !policy
                .allowed_scope_patterns
                .iter()
                .any(|pattern| scope_string.contains(pattern.trim_matches('*')))
        {
            return Err(McpError::ScopeDenied(format!(
                "scope '{}' not in allowlist",
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
        let mut tools: HashMap<String, Arc<dyn EmbeddedTool>> = HashMap::new();
        for tool in [
            Arc::new(ReadFileTool) as Arc<dyn EmbeddedTool>,
            Arc::new(GenerateDiffTool),
            Arc::new(ApplyPatchTool),
            Arc::new(AstChunkTool),
            Arc::new(SymbolLookupTool),
            Arc::new(RunCommandTool),
            Arc::new(GraphRetrieveSkillsTool),
            Arc::new(GraphRetrieveCodeTool),
        ] {
            tools.insert(tool.definition().name.clone(), tool);
        }
        Self {
            tools: Arc::new(tools),
        }
    }

    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddedTool>> {
        self.tools.get(name).cloned()
    }

    fn definitions_for_role(&self, role: &str) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|tool| role_allows_tool(role, &tool.definition().name))
            .map(|tool| tool.definition())
            .collect()
    }
}

/// MCP tool sandbox service.
#[derive(Clone)]
pub struct McpService {
    registry: EmbeddedToolRegistry,
    rbac: RbacEngine,
    audit: AuditLog,
    config: McpServiceConfig,
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

struct LazyPipelineCodeRetriever {
    config: McpServiceConfig,
    pipeline: tokio::sync::OnceCell<Arc<CodeRetrievalPipeline>>,
}

struct LazyPipelineSkillRetriever {
    config: SkillRetrievalConfig,
    pipeline: tokio::sync::OnceCell<Arc<SkillRetrievalPipeline>>,
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
                let collection: Arc<dyn DenseVectorCollection> = match self.config.dense_backend {
                    VectorBackendKind::Qdrant => Arc::new(
                        QdrantVectorCollection::new(
                            &self.config.dense_qdrant_url,
                            self.config.code_collection.clone(),
                        )
                        .await
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?,
                    ),
                    VectorBackendKind::SqliteVec => Arc::new(
                        SqliteVecCollection::new(
                            &self.config.dense_store_path,
                            self.config.code_collection.clone(),
                        )
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?,
                    ),
                };
                let embedder = Arc::new(
                    JinaCodeEmbedder::new(self.config.code_embedding.clone())
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?,
                );
                let reranker =
                    CandleCrossEncoder::new().map_err(|err| McpError::ToolExecution(err.to_string()))?;
                Ok(Arc::new(CodeRetrievalPipeline::new(collection, embedder, reranker)))
            })
            .await
            .map(Arc::clone)
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
        let result = pipeline
            .retrieve(&CodeRetrievalQuery {
                workspace_root: PathBuf::from(&request.workspace_root),
                query: request.query.clone(),
                focal_files: request.focal_files.clone(),
                focal_symbols: request.focal_symbols.clone(),
                dense_limit: request.dense_limit.max(1) as usize,
                token_budget: budget,
            })
            .await
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;

        Ok(RetrieveCodeContextResponse {
            request_id: request.request_id,
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
                })
                .collect(),
            diagnostics: if request.include_diagnostics {
                Some(RetrievalDiagnostics {
                    dense_hits: result.fused_candidates.len() as u32,
                    bm25_hits: 0,
                    fused_candidates: result.fused_candidates.len() as u32,
                    accept_count: result.final_result.memgas.accept_set.len() as u32,
                    reject_count: result.final_result.memgas.reject_set.len() as u32,
                    selected_count: result.final_result.selection.selected_documents.len() as u32,
                    used_tokens: result.final_result.selection.used_tokens as u32,
                    memgas_converged: result.final_result.memgas.converged,
                    memgas_degenerate: result.final_result.memgas.degenerate,
                    scores: Vec::new(),
                    generated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                })
            } else {
                None
            },
        })
    }
}

impl McpService {
    /// Create a new MCP service with built-in tools.
    pub fn new() -> Self {
        Self::with_config(McpServiceConfig::default())
    }

    /// Create a new MCP service with explicit configuration.
    pub fn with_config(config: McpServiceConfig) -> Self {
        Self::with_registry(config, EmbeddedToolRegistry::builtin())
    }

    /// Create a new MCP service backed by an explicit embedded registry.
    pub fn with_registry(config: McpServiceConfig, registry: EmbeddedToolRegistry) -> Self {
        let workspace_root = std::fs::canonicalize(&config.workspace_root)
            .unwrap_or_else(|_| config.workspace_root.clone());
        let mut config = config;
        config.workspace_root = workspace_root.clone();
        Self {
            registry,
            rbac: RbacEngine,
            audit: AuditLog::new(),
            retrieval_router: Arc::new(RetrievalRouter {
                skill: Arc::new(LazyPipelineSkillRetriever::new(config.skill_config.clone())),
                code: Arc::new(LazyPipelineCodeRetriever::new(config.clone())),
            }),
            config,
        }
    }

    #[cfg(test)]
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

        Self {
            registry,
            rbac: RbacEngine,
            audit: AuditLog::new(),
            config,
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
            tools: self.registry.definitions_for_role(&req.role),
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

        self.rbac
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
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
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

struct ReadFileTool;
struct GenerateDiffTool;
struct ApplyPatchTool;
struct AstChunkTool;
struct SymbolLookupTool;
struct RunCommandTool;
struct GraphRetrieveSkillsTool;
struct GraphRetrieveCodeTool;

#[tonic::async_trait]
impl EmbeddedTool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a UTF-8 file inside the workspace".to_string(),
            required_actions: vec!["read_file".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let content = std::fs::read_to_string(&ctx.scope)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([(
                "content",
                string_value(content),
            )])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GenerateDiffTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "generate_diff".to_string(),
            description: "Generate a unified diff from current and updated file content".to_string(),
            required_actions: vec!["generate_diff".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
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
        ToolDefinition {
            name: "apply_patch".to_string(),
            description: "Apply a unified diff patch to a file inside the workspace".to_string(),
            required_actions: vec!["apply_patch".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let patch = required_argument(&ctx.request.arguments, "patch")?;
        let current = std::fs::read_to_string(&ctx.scope).unwrap_or_default();
        let result = apply_patch(&current, &patch);
        if !result.success {
            return Ok(failure_result(
                ctx.request,
                result.error.unwrap_or_else(|| "patch failed".to_string()),
            ));
        }

        std::fs::write(&ctx.scope, &result.content)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                ("path", string_value(relative_to_workspace(ctx.workspace_root, &ctx.scope))),
                ("applied", bool_value(true)),
            ])),
        ))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for AstChunkTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ast_chunk".to_string(),
            description: "Chunk a source file using the native Tree-sitter chunker".to_string(),
            required_actions: vec!["ast_chunk".to_string()],
            allowed_scope_patterns: vec!["src".to_string(), ".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let content = std::fs::read_to_string(&ctx.scope)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let language = Chunker::detect_language(&ctx.scope).unwrap_or_else(|| "unknown".to_string());
        let mut chunker =
            Chunker::new().map_err(|err| McpError::ToolExecution(err.to_string()))?;
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
        ToolDefinition {
            name: "symbol_lookup".to_string(),
            description: "Lookup symbols through the native SCIP parser registry".to_string(),
            required_actions: vec!["symbol_lookup".to_string()],
            allowed_scope_patterns: vec!["src".to_string(), ".".to_string()],
            supports_streaming: false,
        }
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
        ToolDefinition {
            name: "run_command".to_string(),
            description: "Run a bounded command in the workspace".to_string(),
            required_actions: vec!["run_command".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
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

        let mut process = TokioCommand::new(&command.program);
        process
            .args(&command.args)
            .current_dir(ctx.workspace_root)
            .kill_on_drop(true);
        let output = match timeout(Duration::from_secs(timeout_secs as u64), process.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(err)) => return Err(McpError::ToolExecution(err.to_string())),
            Err(_) => return Ok(timeout_result(ctx.request, &command.program, timeout_secs)),
        };

        Ok(ToolCallResult {
            request_id: ctx.request.request_id.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            output: Some(struct_from_map([
                ("program", string_value(command.program)),
                (
                    "args",
                    Value {
                        kind: Some(Kind::ListValue(ListValue {
                            values: command
                                .args
                                .into_iter()
                                .map(|arg| string_value(arg))
                                .collect(),
                        })),
                    },
                ),
            ])),
            audit_event: None,
        })
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GraphRetrieveSkillsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "graph_retrieve_skills".to_string(),
            description: "Retrieve statistically relevant SKILL.md payloads for the active task".to_string(),
            required_actions: vec!["graph_retrieve_skills".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let query = required_argument(&ctx.request.arguments, "query")?;
        let role = extract_argument(&ctx.request.arguments, "role").unwrap_or_else(|| ctx.request.role.clone());
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

        let service = McpService::with_config(McpServiceConfig {
            workspace_root: ctx.workspace_root.to_path_buf(),
            allowed_commands: ctx.config.allowed_commands.clone(),
            default_max_execution_seconds: ctx.config.default_max_execution_seconds,
            dense_backend: ctx.config.dense_backend,
            dense_qdrant_url: ctx.config.dense_qdrant_url.clone(),
            dense_store_path: ctx.config.dense_store_path.clone(),
            code_collection: ctx.config.code_collection.clone(),
            code_embedding: ctx.config.code_embedding.clone(),
            code_retrieval_budget_tokens: ctx.config.code_retrieval_budget_tokens,
            skill_config: ctx.config.skill_config.clone(),
        });
        let response = service
            .retrieve_skills_internal(RetrieveSkillsRequest {
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

        Ok(success_result(ctx.request, Some(retrieve_skills_struct(&response))))
    }
}

#[tonic::async_trait]
impl EmbeddedTool for GraphRetrieveCodeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "graph_retrieve_code".to_string(),
            description: "Retrieve dense code context for the active task".to_string(),
            required_actions: vec!["graph_retrieve_code".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let query = required_argument(&ctx.request.arguments, "query")?;
        let role = extract_argument(&ctx.request.arguments, "role").unwrap_or_else(|| ctx.request.role.clone());
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

        let service = McpService::with_config(ctx.config.clone());
        let response = service
            .retrieve_code_internal(RetrieveCodeContextRequest {
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
            })
            .await?;

        Ok(success_result(ctx.request, Some(retrieve_code_struct(&response))))
    }
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
                kind: response
                    .diagnostics
                    .as_ref()
                    .map(|diagnostics| Kind::StructValue(retrieval_diagnostics_struct(diagnostics))),
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
                                ("accept_posterior", number_value(document.accept_posterior as f64)),
                                ("cross_score", number_value(document.cross_score as f64)),
                            ]))),
                        })
                        .collect(),
                })),
            },
        ),
        (
            "diagnostics",
            Value {
                kind: response
                    .diagnostics
                    .as_ref()
                    .map(|diagnostics| Kind::StructValue(retrieval_diagnostics_struct(diagnostics))),
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
        ("rrf_score", number_value(skill.rrf_score as f64)),
        ("accept_posterior", number_value(skill.accept_posterior as f64)),
        ("cross_score", number_value(skill.cross_score as f64)),
    ])
}

fn retrieval_diagnostics_struct(diagnostics: &RetrievalDiagnostics) -> Struct {
    struct_from_map([
        ("dense_hits", number_value(diagnostics.dense_hits as f64)),
        ("bm25_hits", number_value(diagnostics.bm25_hits as f64)),
        ("fused_candidates", number_value(diagnostics.fused_candidates as f64)),
        ("accept_count", number_value(diagnostics.accept_count as f64)),
        ("reject_count", number_value(diagnostics.reject_count as f64)),
        ("selected_count", number_value(diagnostics.selected_count as f64)),
        ("used_tokens", number_value(diagnostics.used_tokens as f64)),
        ("memgas_converged", bool_value(diagnostics.memgas_converged)),
        ("memgas_degenerate", bool_value(diagnostics.memgas_degenerate)),
        (
            "scores",
            Value {
                kind: Some(Kind::ListValue(ListValue {
                    values: diagnostics
                        .scores
                        .iter()
                        .map(|score| Value {
                            kind: Some(Kind::StructValue(candidate_score_struct(score))),
                        })
                        .collect(),
                })),
            },
        ),
    ])
}

fn candidate_score_struct(score: &CandidateScore) -> Struct {
    struct_from_map([
        ("skill_id", string_value(score.skill_id.clone())),
        ("dense_rank", number_value(score.dense_rank as f64)),
        ("dense_score", number_value(score.dense_score as f64)),
        ("bm25_rank", number_value(score.bm25_rank as f64)),
        ("bm25_score", number_value(score.bm25_score as f64)),
        ("rrf_score", number_value(score.rrf_score as f64)),
        (
            "accept_posterior",
            number_value(score.accept_posterior as f64),
        ),
        ("cross_score", number_value(score.cross_score as f64)),
        ("token_cost", number_value(score.token_cost as f64)),
        ("selected", bool_value(score.selected)),
    ])
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
        candidate
            .canonicalize()
            .map_err(|err| McpError::ToolExecution(format!("failed to canonicalize scope: {err}")))?
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

fn detect_language(path: &Path) -> Result<Language, McpError> {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
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

fn role_allows_tool(role: &str, tool_name: &str) -> bool {
    match role {
        "" => true,
        "architect" => matches!(tool_name, "read_file" | "symbol_lookup" | "graph_retrieve_skills" | "graph_retrieve_code" | "ast_chunk"),
        "coder" => matches!(tool_name, "read_file" | "generate_diff" | "apply_patch" | "ast_chunk" | "symbol_lookup" | "graph_retrieve_skills" | "graph_retrieve_code"),
        "tester" => matches!(tool_name, "read_file" | "run_command" | "graph_retrieve_skills" | "graph_retrieve_code"),
        "executor" => matches!(tool_name, "read_file" | "run_command" | "apply_patch" | "generate_diff"),
        "reviewer" => matches!(tool_name, "read_file" | "generate_diff" | "graph_retrieve_skills" | "graph_retrieve_code" | "symbol_lookup"),
        "worker" | "implementation" => matches!(tool_name, "read_file" | "run_command" | "generate_diff" | "apply_patch"),
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

fn number_value(value: f64) -> Value {
    Value {
        kind: Some(Kind::NumberValue(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axora_memory::{
        FusedCandidate, SkillCatalog, SkillCorpusIngestor, SkillDocument, SkillIndexBackend,
        SkillRetrievalConfig, SkillRetrievalPipeline,
    };
    use axora_proto::mcp::v1::graph_retrieval_service_client::GraphRetrievalServiceClient;
    use axora_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
    use axora_proto::mcp::v1::tool_service_server::ToolService;
    use axora_rag::{CrossEncoderScorer, RerankDocument};
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tonic::transport::Server;

    fn test_mcp_config(root: PathBuf) -> McpServiceConfig {
        McpServiceConfig {
            workspace_root: root.clone(),
            allowed_commands: vec!["cargo".to_string()],
            default_max_execution_seconds: 5,
            dense_backend: VectorBackendKind::SqliteVec,
            dense_qdrant_url: "http://127.0.0.1:6334".to_string(),
            dense_store_path: root.join(".axora/vectors.db"),
            code_collection: CollectionSpec::code_default(),
            code_embedding: CodeEmbeddingConfig::default(),
            code_retrieval_budget_tokens: 128,
            skill_config: SkillRetrievalConfig {
                corpus_root: root.join("skills"),
                catalog_db_path: root.join(".axora/skill-index/skill-catalog.db"),
                dense_backend: VectorBackendKind::SqliteVec,
                dense_store_path: root.join(".axora/vectors.db"),
                qdrant_url: "http://127.0.0.1:6334".to_string(),
                dense_collection: CollectionSpec::skill_default(),
                embedding: axora_embeddings::SkillEmbeddingConfig::default(),
                bm25_dir: root.join(".axora/skill-bm25"),
                skill_token_budget: 1500,
                dense_limit: 64,
                bm25_limit: 64,
            },
        }
    }

    fn policy(root: &str, actions: &[&str]) -> CapabilityPolicy {
        CapabilityPolicy {
            agent_id: "agent-1".to_string(),
            role: "coder".to_string(),
            allowed_actions: actions.iter().map(|action| (*action).to_string()).collect(),
            allowed_scope_patterns: vec![root.to_string(), "/Users/noasantos/Fluri/axora".to_string()],
            denied_scope_patterns: vec!["/etc".to_string()],
            max_execution_seconds: 5,
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
            policy: Some(policy("/Users/noasantos/Fluri/axora", &["read_file"])),
            workspace_root: "/Users/noasantos/Fluri/axora".to_string(),
        };

        let result = service.call_tool(Request::new(req)).await;
        assert!(result.is_err());
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
        let names = response.tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>();
        assert!(names.contains(&"apply_patch".to_string()));
        assert!(names.contains(&"graph_retrieve_skills".to_string()));
        assert!(names.contains(&"graph_retrieve_code".to_string()));
    }

    #[tokio::test]
    async fn run_command_denies_unlisted_binary() {
        let service = McpService::with_config(test_mcp_config(PathBuf::from("/Users/noasantos/Fluri/axora")));
        let req = ToolCallRequest {
            request_id: "req-2".to_string(),
            agent_id: "agent-1".to_string(),
            role: "executor".to_string(),
            tool_name: "run_command".to_string(),
            arguments: Some(struct_from_map([("program", string_value("git"))])),
            policy: Some(policy("/Users/noasantos/Fluri/axora", &["run_command"])),
            workspace_root: "/tmp/ignore-me".to_string(),
        };

        let result = service.call_tool(Request::new(req)).await.unwrap().into_inner();
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
            policy: Some(policy(&temp_dir.path().display().to_string(), &["apply_patch"])),
            workspace_root: temp_dir.path().display().to_string(),
        };

        let result = service.call_tool(Request::new(req)).await.unwrap().into_inner();
        assert!(result.success);
        assert_eq!(fs::read_to_string(file_path).unwrap(), "fn after() {}\n");
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
        async fn upsert_document(&self, document: &SkillDocument) -> axora_memory::procedural_store::Result<()> {
            self.docs
                .lock()
                .await
                .insert(document.skill_id.clone(), document.clone());
            Ok(())
        }

        async fn delete_document(&self, skill_id: &str) -> axora_memory::procedural_store::Result<()> {
            self.docs.lock().await.remove(skill_id);
            Ok(())
        }

        async fn search(
            &self,
            catalog: &SkillCatalog,
            _query: &str,
            _dense_limit: usize,
            _bm25_limit: usize,
        ) -> axora_memory::procedural_store::Result<Vec<FusedCandidate>> {
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
                    profile.map(|(rrf_score, dense_rank, dense_score, bm25_rank, bm25_score)| FusedCandidate {
                        skill,
                        rrf_score,
                        dense_rank,
                        dense_score,
                        bm25_rank,
                        bm25_score,
                    })
                })
                .collect())
        }
    }

    #[derive(Clone)]
    struct StaticCrossEncoder;

    #[tonic::async_trait]
    impl CrossEncoderScorer for StaticCrossEncoder {
        async fn score_pairs(&self, _query: &str, docs: &[RerankDocument]) -> axora_rag::Result<Vec<f32>> {
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
                dense_backend: VectorBackendKind::SqliteVec,
                dense_store_path: temp_dir.path().join("vectors.db"),
                qdrant_url: "http://127.0.0.1:6334".to_string(),
                dense_collection: CollectionSpec::skill_default(),
                embedding: axora_embeddings::SkillEmbeddingConfig::default(),
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
                        dense_backend: VectorBackendKind::SqliteVec,
                        dense_store_path: temp_dir.path().join("vectors.db"),
                        qdrant_url: "http://127.0.0.1:6334".to_string(),
                        dense_collection: CollectionSpec::skill_default(),
                        embedding: axora_embeddings::SkillEmbeddingConfig::default(),
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

        let response = service
            .retrieve_code_context(Request::new(RetrieveCodeContextRequest {
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
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.documents.len(), 1);
        assert_eq!(response.documents[0].chunk_id, "chunk-1");
    }
}
