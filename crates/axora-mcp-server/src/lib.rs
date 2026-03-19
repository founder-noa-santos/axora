//! MCP gRPC tool sandbox server for AXORA.

use axora_cache::{apply_patch, UnifiedDiff};
use axora_docs::{DocReconciler, DocReconcilerConfig, ReconcileDecision};
use axora_indexing::{Chunker, Language, ParserRegistry};
use axora_proto::mcp::v1::tool_service_server::ToolService;
use axora_proto::mcp::v1::{
    AuditEvent, CapabilityPolicy, ListToolsRequest, ListToolsResponse, StreamAuditRequest,
    ToolCallRequest, ToolCallResult, ToolDefinition,
};
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
            Arc::new(GraphContextTool),
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
        Self {
            registry,
            rbac: RbacEngine,
            audit: AuditLog::new(),
            config: McpServiceConfig {
                workspace_root: std::fs::canonicalize(&config.workspace_root)
                    .unwrap_or(config.workspace_root),
                ..config
            },
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

struct ReadFileTool;
struct GenerateDiffTool;
struct ApplyPatchTool;
struct AstChunkTool;
struct SymbolLookupTool;
struct RunCommandTool;
struct GraphContextTool;

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
impl EmbeddedTool for GraphContextTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "graph_context".to_string(),
            description: "Read SCIP graph context and LivingDocs matches from the workspace".to_string(),
            required_actions: vec!["graph_context".to_string()],
            allowed_scope_patterns: vec![".".to_string()],
            supports_streaming: false,
        }
    }

    async fn execute(&self, ctx: ToolExecutionContext<'_>) -> Result<ToolCallResult, McpError> {
        let query = required_argument(&ctx.request.arguments, "query")?;
        let docs_root = ctx.workspace_root.join("docs");
        let docs = collect_doc_matches(&docs_root, &query);

        let language = detect_language(&ctx.scope).unwrap_or(Language::Rust);
        let registry = ParserRegistry::new();
        let scip = registry
            .parse(language, ctx.workspace_root)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let symbols = scip
            .symbols
            .into_iter()
            .filter(|symbol| symbol.symbol.to_lowercase().contains(&query.to_lowercase()))
            .take(10)
            .map(|symbol| Value {
                kind: Some(Kind::StructValue(struct_from_map([
                    ("symbol", string_value(symbol.symbol)),
                    ("signature", string_value(symbol.signature)),
                ]))),
            })
            .collect::<Vec<_>>();

        let mut reconciler = DocReconciler::new(DocReconcilerConfig::new(ctx.workspace_root));
        let decision = if docs_root.is_dir() {
            let readme = docs_root.join("README.md");
            let content = std::fs::read_to_string(&readme).unwrap_or_default();
            let (decision, _) = reconciler.reconcile_change(Path::new("README.md"), "", &content);
            decision
        } else {
            ReconcileDecision::Noop
        };

        Ok(success_result(
            ctx.request,
            Some(struct_from_map([
                (
                    "docs",
                    Value {
                        kind: Some(Kind::ListValue(ListValue { values: docs })),
                    },
                ),
                (
                    "symbols",
                    Value {
                        kind: Some(Kind::ListValue(ListValue { values: symbols })),
                    },
                ),
                ("living_docs_decision", string_value(format!("{:?}", decision))),
            ])),
        ))
    }
}

fn collect_doc_matches(docs_root: &Path, query: &str) -> Vec<Value> {
    let mut matches = Vec::new();
    if !docs_root.is_dir() {
        return matches;
    }

    let needle = query.to_lowercase();
    let mut stack = vec![docs_root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if !content.to_lowercase().contains(&needle) {
                continue;
            }
            let snippet = content
                .lines()
                .find(|line| line.to_lowercase().contains(&needle))
                .unwrap_or_default()
                .to_string();
            matches.push(Value {
                kind: Some(Kind::StructValue(struct_from_map([
                    ("path", string_value(path.to_string_lossy().to_string())),
                    ("snippet", string_value(snippet)),
                ]))),
            });
            if matches.len() >= 10 {
                return matches;
            }
        }
    }
    matches
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
        "architect" => matches!(tool_name, "read_file" | "symbol_lookup" | "graph_context" | "ast_chunk"),
        "coder" => matches!(tool_name, "read_file" | "generate_diff" | "apply_patch" | "ast_chunk" | "symbol_lookup" | "graph_context"),
        "tester" => matches!(tool_name, "read_file" | "run_command" | "graph_context"),
        "executor" => matches!(tool_name, "read_file" | "run_command" | "apply_patch" | "generate_diff"),
        "reviewer" => matches!(tool_name, "read_file" | "generate_diff" | "graph_context" | "symbol_lookup"),
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
    use axora_proto::mcp::v1::tool_service_server::ToolService;
    use std::fs;
    use tempfile::TempDir;

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
        assert!(names.contains(&"graph_context".to_string()));
    }

    #[tokio::test]
    async fn run_command_denies_unlisted_binary() {
        let service = McpService::with_config(McpServiceConfig {
            workspace_root: PathBuf::from("/Users/noasantos/Fluri/axora"),
            allowed_commands: vec!["cargo".to_string()],
            default_max_execution_seconds: 5,
        });
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

        let service = McpService::with_config(McpServiceConfig {
            workspace_root: temp_dir.path().to_path_buf(),
            allowed_commands: vec!["cargo".to_string()],
            default_max_execution_seconds: 5,
        });
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
}
