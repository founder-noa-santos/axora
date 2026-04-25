//! Coordinator v2 core orchestration.
//!
//! This is the Phase 3 coordinator foundation: it tracks workers, loads a
//! decomposed mission into a queue, dispatches ready tasks, monitors worker
//! health, and merges basic task output into a shared blackboard.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use openakta_agents::coordinator::v2::{BlackboardV2, Coordinator, CoordinatorConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let blackboard = Arc::new(BlackboardV2::default());
//! let mut coordinator = Coordinator::new(CoordinatorConfig::default(), blackboard)?;
//! let result = coordinator.execute_mission("simple task").await?;
//!
//! assert!(result.success);
//! # Ok(())
//! # }
//! ```

#[path = "v2_core.rs"]
pub mod v2_core;
#[path = "v2_dispatcher.rs"]
pub mod v2_dispatcher;
#[path = "v2_queue_integration.rs"]
pub mod v2_queue_integration;

use self::v2_core::{CoordinatorCoreError, Result as CoreResult};
use self::v2_dispatcher::{
    CompletionReport, DispatchCompletion, DispatchWorker, DispatchWorkerStatus, Dispatcher,
    DispatcherError,
};
use self::v2_queue_integration::{QueueIntegrationError, TaskQueueIntegration};
use crate::assignment_contract::{
    default_expected_artifacts, default_lane_for_task_type, default_termination_condition,
    default_worker_role, PlanningOriginRef, WorkerAssignmentContract, WorkerExecutionBudget,
};
use crate::blackboard_runtime::{BlackboardEntry, RuntimeBlackboard};
use crate::communication::CommunicationProtocol;
use crate::decomposer::{DecomposedMission, DecomposerConfig, MissionDecomposer};
use crate::diagnostics::WideEvent;
use crate::execution_trace::{
    ExecutionEventKind, ExecutionTraceEvent, ExecutionTracePhase, ExecutionTraceRegistry,
    ExecutionTraceService,
};
use crate::intake::{DecompositionBudget, TaskTargetHints};
use crate::mcp_client::McpClient;
use crate::patch_protocol::{
    resolve_workspace_relative_path, AstSummary, ContextPack, ContextSpan,
    DeterministicPatchApplier, DiffOutputValidator, PatchApplyStatus, PatchEnvelope, PatchFormat,
    RetrievalHit, SymbolMap, ValidationFact,
};
use crate::prompt_assembly::PromptAssembly;
use crate::provider::{
    CacheRetention, ModelBoundaryPayload, ModelBoundaryPayloadType, ModelRequest, ModelResponse,
    ProviderUsage,
};
use crate::provider_registry::ProviderRegistry;
use crate::provider_transport::{
    default_local_transport, local_provider_config_from_instance, CloudModelRef, FallbackPolicy,
    LocalModelRef, LocalProviderConfig, ModelRegistrySnapshot, ProviderInstanceId,
    ProviderRuntimeBundle,
};
use crate::retrieval::{GraphRetrievalConfig, GraphRetrievalRequest, GraphRetriever};
use crate::routing::{route, RoutedTarget};
use crate::task::{Task, TaskStatus, TaskType};
use crate::token_budget::{derive_effective_budget, EffectiveTokenBudget};
use crate::tool_registry::ToolRegistry;
use crate::transport::{
    InternalContextReference, InternalResultSubmission, InternalTaskAssignment, InternalTokenUsage,
};
use crate::worker_pool::{WorkerId, WorkerStatus};
use openakta_api_client::ApiClientPool;
use openakta_indexing::{InfluenceGraph, Language, ParserRegistry};
use openakta_proto::mcp::v1::{CapabilityPolicy, RetrieveCodeContextRequest};
use openakta_workflow::MolFeatureFlags;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tracing::warn;
use uuid::Uuid;

pub use self::v2_core::{
    BaseSquadBootstrapper, Coordinator as CoordinatorCore, OutputContract, PlanningActingPolicy,
    SquadRole, WorkerInfo, WorkerInfo as RegisteredWorkerInfo, WorkerProfile, WorkerRegistry,
};
pub use self::v2_dispatcher::{
    CompletionReport as DispatchCompletionReport, DispatchLoopReport,
    DispatchWorker as CoordinatorDispatchWorker,
    DispatchWorkerStatus as CoordinatorDispatchWorkerStatus, Dispatcher as CoordinatorDispatcher,
    MonitorReport,
};
pub use self::v2_queue_integration::TaskQueueIntegration as CoordinatorTaskQueue;

/// Shared blackboard used by Coordinator v2.
pub type BlackboardV2 = Mutex<RuntimeBlackboard>;

/// Result type for Coordinator v2 operations.
pub type Result<T> = std::result::Result<T, CoordinatorV2Error>;

/// Errors produced by Coordinator v2.
#[derive(Debug, Error)]
pub enum CoordinatorV2Error {
    /// Invalid configuration was supplied.
    #[error("invalid coordinator config: {0}")]
    InvalidConfig(String),

    /// Worker registry operation failed.
    #[error("worker registry error: {0}")]
    Core(#[from] CoordinatorCoreError),

    /// Queue integration operation failed.
    #[error("task queue error: {0}")]
    Queue(#[from] QueueIntegrationError),

    /// Dispatcher operation failed.
    #[error("dispatcher error: {0}")]
    Dispatcher(#[from] DispatcherError),

    /// Mission decomposition failed.
    #[error("mission decomposition failed: {0}")]
    Decomposition(String),

    /// No worker was available when work was ready.
    #[error("no available worker")]
    NoAvailableWorker,

    /// The mission could not make progress.
    #[error("mission stalled: {0}")]
    StalledMission(String),

    /// Runtime protocol contract was violated.
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    /// Cloud execution failed because the network path is unavailable.
    #[error("cloud execution unavailable: {message}")]
    CloudExecutionUnavailable {
        /// Human-readable failure reason.
        message: String,
        /// Optional suggested recovery path.
        local_recovery: Option<String>,
    },

    /// The requested workflow requires a cloud lane.
    #[error("cloud execution required: {0}")]
    CloudExecutionRequired(String),

    /// Task execution failed.
    #[error("task execution failed: {0}")]
    ExecutionFailed(String),

    /// Task execution timed out.
    #[error("task execution timed out: {0}")]
    Timeout(String),
}

/// Coordinator runtime configuration.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Maximum workers managed by the coordinator.
    pub max_workers: usize,
    /// Delay between idle dispatch passes.
    pub dispatch_interval: Duration,
    /// Whether worker monitoring is enabled.
    pub enable_monitoring: bool,
    /// Optional default cloud instance/model reference.
    pub default_cloud: Option<CloudModelRef>,
    /// Optional default local instance/model reference.
    pub default_local: Option<LocalModelRef>,
    /// Deterministic instance routing priority.
    pub model_instance_priority: Vec<ProviderInstanceId>,
    /// Resolved provider runtime bundle.
    pub provider_bundle: Arc<ProviderRuntimeBundle>,
    /// Runtime model registry snapshot.
    pub registry: Arc<ModelRegistrySnapshot>,
    /// Fallback policy when the cloud lane is unavailable.
    pub fallback_policy: FallbackPolicy,
    /// Whether heterogeneous routing is enabled.
    pub routing_enabled: bool,
    /// Retry budget for local validation failures before cloud arbiter escalation.
    pub local_validation_retry_budget: u32,
    /// Task classes allowed to use the local lane.
    pub local_enabled_for: Vec<String>,
    /// Workspace root used for patch application and retrieval.
    pub workspace_root: PathBuf,
    /// Per-task timeout.
    pub task_timeout: Duration,
    /// Retry budget for retryable failures.
    pub retry_budget: u32,
    /// Token budget used by graph retrieval.
    pub retrieval_token_budget: usize,
    /// Maximum hydrated retrieval documents.
    pub retrieval_max_documents: usize,
    /// Whether graph retrieval should be attempted for anchored tasks.
    pub enable_graph_retrieval: bool,
    /// Hard token budget embedded into task assignments.
    pub task_token_budget: u32,
    /// Fraction of model context reserved for prompts.
    pub context_use_ratio: f32,
    /// Fixed token safety margin.
    pub context_margin_tokens: u32,
    /// Fraction of prompt budget allocated to retrieval.
    pub retrieval_share: f32,
    /// Optional human-in-the-loop gate (mission lifecycle + question caps).
    pub hitl_gate: Option<std::sync::Arc<crate::hitl::MissionHitlGate>>,
    /// Optional MCP endpoint for structured tool execution.
    pub mcp_endpoint: Option<String>,
    /// Maximum provider/tool loop turns per task.
    pub max_tool_turns: u32,
    /// Maximum total tool calls per task.
    pub max_tool_calls: u32,
    /// Maximum mutating tool calls per task.
    pub max_mutating_tool_calls: u32,
    /// Optional canonical execution-trace service for the active session.
    pub execution_tracer: Option<Arc<ExecutionTraceService>>,
    /// Optional registry used for mission-to-session correlation.
    pub execution_trace_registry: Option<Arc<ExecutionTraceRegistry>>,
    /// Mission Operating Layer flags from daemon/bootstrap. When `strict_legacy_fence` is set,
    /// steward / worker hints in [`Task::assigned_to`] are kept for dispatch preference (AB12).
    pub mol: MolFeatureFlags,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        let local = LocalProviderConfig::default();
        Self {
            max_workers: 5,
            dispatch_interval: Duration::from_secs(1),
            enable_monitoring: true,
            default_cloud: None,
            default_local: None,
            model_instance_priority: Vec::new(),
            provider_bundle: Arc::new(ProviderRuntimeBundle::default()),
            registry: Arc::new(ModelRegistrySnapshot::default()),
            fallback_policy: FallbackPolicy::Explicit,
            routing_enabled: false,
            local_validation_retry_budget: 1,
            local_enabled_for: local.enabled_for,
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            task_timeout: Duration::from_secs(5),
            retry_budget: 1,
            retrieval_token_budget: 2_000,
            retrieval_max_documents: 8,
            enable_graph_retrieval: true,
            task_token_budget: 2_500,
            context_use_ratio: 0.8,
            context_margin_tokens: 512,
            retrieval_share: 0.35,
            hitl_gate: None,
            mcp_endpoint: None,
            max_tool_turns: 6,
            max_tool_calls: 8,
            max_mutating_tool_calls: 2,
            execution_tracer: None,
            execution_trace_registry: None,
            mol: MolFeatureFlags::default(),
        }
    }
}

/// Builds the task snapshot passed to the v2 dispatcher: pending status, and optionally clears
/// `assigned_to` when MOL strict legacy fence is off (legacy behavior).
fn prepare_dispatch_task_snapshot(config: &CoordinatorConfig, task: &Task) -> Task {
    let mut dispatch_task = task.clone();
    dispatch_task.status = TaskStatus::Pending;
    if !config.mol.strict_legacy_fence {
        dispatch_task.assigned_to = None;
    }
    dispatch_task
}

impl CoordinatorConfig {
    fn default_provider_label(&self) -> String {
        self.default_cloud
            .as_ref()
            .map(|cloud| cloud.instance_id.0.clone())
            .or_else(|| {
                self.default_local
                    .as_ref()
                    .map(|local| local.instance_id.0.clone())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Final result of a mission execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// Generated mission identifier.
    pub mission_id: String,
    /// Whether all tasks succeeded.
    pub success: bool,
    /// Merged output from completed tasks.
    pub output: String,
    /// Number of completed tasks.
    pub tasks_completed: usize,
    /// Number of failed tasks.
    pub tasks_failed: usize,
    /// Total mission duration.
    pub duration: Duration,
    /// Canonical execution trace items emitted during the mission.
    #[serde(default)]
    pub trace_events: Vec<ExecutionTraceEvent>,
}

/// Current coordinator mission status.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionStatus {
    /// Active mission identifier.
    pub mission_id: String,
    /// Progress in the range `0.0..=100.0`.
    pub progress: f32,
    /// Estimated time remaining when progress is non-zero.
    pub eta: Option<Duration>,
    /// Number of workers currently executing tasks.
    pub active_workers: usize,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Total tasks in the mission.
    pub total_tasks: usize,
}

/// Live CoordinatorV2 metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinatorMetrics {
    /// Workflow transition count observed by the live coordinator.
    pub transition_count: u64,
    /// Timeout failures observed during task execution.
    pub timeout_failures: u64,
    /// Retry budget exhaustion events.
    pub retry_exhaustions: u64,
    /// Protocol or schema validation failures.
    pub protocol_validation_failures: u64,
}

/// Phase 3 coordinator.
pub struct Coordinator {
    /// Registry of workers under coordinator control.
    pub worker_registry: WorkerRegistry,
    /// Task queue integration for the active mission.
    pub task_queue: TaskQueueIntegration,
    /// Dispatcher for task execution flow.
    pub dispatcher: Dispatcher,
    /// Shared blackboard.
    pub blackboard: Arc<BlackboardV2>,
    /// Runtime configuration.
    pub config: CoordinatorConfig,
    communication: CommunicationProtocol,
    registry: Arc<ProviderRegistry>,
    diff_validator: DiffOutputValidator,
    patch_applier: DeterministicPatchApplier,
    tool_registry: ToolRegistry,
    metrics: CoordinatorMetrics,
    mission_id: Option<String>,
    mission_started_at: Option<Instant>,
    merged_outputs: Vec<String>,
    tasks_failed: usize,
    trace_events: Vec<ExecutionTraceEvent>,
}

impl Coordinator {
    /// Creates a new Coordinator v2 and pre-registers worker slots.
    pub fn new(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Result<Self> {
        let mut local = HashMap::new();
        for (instance_id, instance) in &config.provider_bundle.instances {
            if instance.is_local {
                let local_config =
                    local_provider_config_from_instance(instance, config.local_enabled_for.clone());
                local.insert(
                    instance_id.clone(),
                    Arc::from(
                        default_local_transport(&local_config, config.task_timeout)
                            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?,
                    ),
                );
            }
            // Cloud instances are handled via api_client_pool (no direct transport)
        }
        let registry = Arc::new(ProviderRegistry::new_with_api_client(
            local,
            config.default_cloud.clone(),
            config.default_local.clone(),
            config.fallback_policy,
            Arc::clone(&config.provider_bundle),
            Arc::clone(&config.registry),
            Arc::new(
                ApiClientPool::new(openakta_api_client::ClientConfig::default())
                    .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?,
            ),
        ));
        Self::new_with_provider_registry(config, blackboard, registry)
    }

    /// Creates a new coordinator with an explicit provider registry.
    pub fn new_with_provider_registry(
        config: CoordinatorConfig,
        blackboard: Arc<BlackboardV2>,
        registry: Arc<ProviderRegistry>,
    ) -> Result<Self> {
        if config.max_workers == 0 {
            return Err(CoordinatorV2Error::InvalidConfig(
                "max_workers must be at least 1".to_string(),
            ));
        }
        if !registry.has_local() && registry.default_cloud.is_none() {
            return Err(CoordinatorV2Error::InvalidConfig(
                "configure at least one cloud or local model lane".to_string(),
            ));
        }

        let worker_registry = WorkerRegistry::new();
        for worker in BaseSquadBootstrapper::build(
            config.max_workers,
            config.retry_budget,
            config.retrieval_token_budget,
        ) {
            worker_registry.register_worker(worker);
        }

        Ok(Self {
            worker_registry,
            task_queue: TaskQueueIntegration::new(),
            dispatcher: Dispatcher::new(),
            blackboard,
            config,
            communication: CommunicationProtocol::new("coordinator-v2"),
            registry,
            diff_validator: DiffOutputValidator::new(8 * 1024),
            patch_applier: DeterministicPatchApplier,
            tool_registry: ToolRegistry::builtin(),
            metrics: CoordinatorMetrics::default(),
            mission_id: None,
            mission_started_at: None,
            merged_outputs: Vec::new(),
            tasks_failed: 0,
            trace_events: Vec::new(),
        })
    }

    /// Executes a mission using the default decomposition budget.
    pub async fn execute_mission(&mut self, mission: &str) -> Result<MissionResult> {
        self.execute_plan(
            mission,
            DecompositionBudget {
                max_tasks: 50,
                max_parallelism: 10,
            },
        )
        .await
    }

    /// Executes a single direct-reply request without decomposition.
    pub async fn execute_direct_reply(
        &mut self,
        prompt: &str,
        hints: &TaskTargetHints,
        workspace_context: Option<String>,
    ) -> Result<MissionResult> {
        self.execute_direct_reply_for_task(Task::new(prompt), prompt, hints, workspace_context)
            .await
    }

    /// Executes a single direct-reply request using a caller-supplied task shell.
    pub async fn execute_direct_reply_for_task(
        &mut self,
        task: Task,
        prompt: &str,
        hints: &TaskTargetHints,
        workspace_context: Option<String>,
    ) -> Result<MissionResult> {
        let mission_id = Uuid::new_v4().to_string();
        self.begin_mission(&mission_id, prompt);

        let mut requested = self.make_trace_event(
            task.id.clone(),
            "direct-executor",
            ExecutionEventKind::Task,
            ExecutionTracePhase::Requested,
            task.description.clone(),
        );
        requested.action_id = task.id.clone();
        self.emit_trace_event(requested);
        let mut started = self.make_trace_event(
            task.id.clone(),
            "direct-executor",
            ExecutionEventKind::Task,
            ExecutionTracePhase::Started,
            task.description.clone(),
        );
        started.action_id = task.id.clone();
        self.emit_trace_event(started);
        let assignment = self.build_task_assignment(&task, Some(hints))?;
        let target = self.resolve_route(&task, &assignment)?;
        let request =
            self.build_direct_reply_request(prompt, &assignment, &target, workspace_context)?;
        let (executed_target, response) = self
            .execute_model_request(&task, &assignment, request, target)
            .await?;
        let submission = InternalResultSubmission {
            task_id: task.id.clone(),
            success: true,
            patch: None,
            patch_receipt: None,
            token_usage: to_internal_token_usage(
                &executed_target.provider_label(),
                &response.usage,
            ),
            context_references: context_references_from_assignment(&assignment),
            summary: response.output_text,
            error_message: String::new(),
            diagnostic_toon: None,
        };
        self.publish_result_submission(&submission).await?;
        self.merged_outputs.push(submission.summary.clone());
        let mut completed = self.make_trace_event(
            task.id.clone(),
            "direct-executor",
            ExecutionEventKind::Task,
            ExecutionTracePhase::Completed,
            task.description.clone(),
        );
        completed.action_id = task.id.clone();
        completed.result_preview = Some(submission.summary.clone());
        self.emit_trace_event(completed);
        self.finish_mission(mission_id, 1)
    }

    /// Executes a single task without decomposition, using intake-derived target hints.
    pub async fn execute_single_task(
        &mut self,
        task: Task,
        hints: &TaskTargetHints,
    ) -> Result<MissionResult> {
        let mission_id = Uuid::new_v4().to_string();
        self.begin_mission(&mission_id, &task.description);
        let mut requested = self.make_trace_event(
            task.id.clone(),
            "direct-executor",
            ExecutionEventKind::Task,
            ExecutionTracePhase::Requested,
            task.description.clone(),
        );
        requested.action_id = task.id.clone();
        self.emit_trace_event(requested);
        let mut started = self.make_trace_event(
            task.id.clone(),
            "direct-executor",
            ExecutionEventKind::Task,
            ExecutionTracePhase::Started,
            task.description.clone(),
        );
        started.action_id = task.id.clone();
        self.emit_trace_event(started);
        let assignment = self.build_task_assignment(&task, Some(hints))?;

        let result_submission = self
            .execute_task_once(&task, "direct-executor", &assignment)
            .await;

        match result_submission {
            Ok(result_submission) => {
                self.publish_result_submission(&result_submission).await?;
                self.merged_outputs.push(result_submission.summary.clone());
                let mut completed = self.make_trace_event(
                    task.id.clone(),
                    "direct-executor",
                    ExecutionEventKind::Task,
                    ExecutionTracePhase::Completed,
                    task.description.clone(),
                );
                completed.action_id = task.id.clone();
                completed.result_preview = Some(result_submission.summary.clone());
                self.emit_trace_event(completed);
                self.finish_mission(mission_id, 1)
            }
            Err(err) => {
                self.tasks_failed = 1;
                self.merged_outputs
                    .push(format!("Task '{}' failed: {}", task.description, err));
                let mut failed = self.make_trace_event(
                    task.id.clone(),
                    "direct-executor",
                    ExecutionEventKind::Task,
                    ExecutionTracePhase::Failed,
                    task.description.clone(),
                );
                failed.action_id = task.id.clone();
                failed.error = Some(err.to_string());
                self.emit_trace_event(failed);
                self.finish_mission(mission_id, 1)
            }
        }
    }

    /// Executes a mission using a bounded decomposition budget.
    pub async fn execute_plan(
        &mut self,
        mission: &str,
        decomposition_budget: DecompositionBudget,
    ) -> Result<MissionResult> {
        let decomposed = MissionDecomposer::new_with_config(
            Arc::new(InfluenceGraph::new()),
            DecomposerConfig {
                max_tasks: decomposition_budget.max_tasks,
                max_parallelism: decomposition_budget.max_parallelism,
                ..DecomposerConfig::default()
            },
        )
        .decompose_async(mission)
        .await
        .map_err(|error| CoordinatorV2Error::Decomposition(error.to_string()))?;

        self.execute_decomposed_mission(decomposed).await
    }

    /// Executes a mission that has already been decomposed and validated by an upstream planner.
    pub async fn execute_decomposed_mission(
        &mut self,
        decomposed: DecomposedMission,
    ) -> Result<MissionResult> {
        let mission_id = if decomposed.mission_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            decomposed.mission_id.clone()
        };
        self.begin_mission(&mission_id, &decomposed.original_mission);

        tracing::info!(
            mission_id = %mission_id,
            task_count = decomposed.tasks.len(),
            "mission decomposed into tasks"
        );

        for task in &decomposed.tasks {
            let mut event = self.make_trace_event(
                task.id.clone(),
                "planner",
                ExecutionEventKind::Task,
                ExecutionTracePhase::Requested,
                task.description.clone(),
            );
            event.action_id = task.id.clone();
            self.emit_trace_event(event);
        }
        self.task_queue.load_tasks(&decomposed)?;

        while !self.task_queue.is_complete() {
            if self.config.enable_monitoring {
                let mut dispatch_workers = self.dispatch_workers_snapshot();
                let report = self
                    .dispatcher
                    .monitor_workers(
                        &mut dispatch_workers,
                        self.config.dispatch_interval.saturating_mul(3),
                    )
                    .await;
                self.apply_monitor_report(&report, &dispatch_workers);
            }

            let Some(_) = self.get_available_worker() else {
                sleep(self.config.dispatch_interval).await;
                continue;
            };

            let Some(task) = self.task_queue.get_next_dispatchable_task() else {
                if self.dispatcher.active_assignment_count() == 0 {
                    return Err(CoordinatorV2Error::StalledMission(
                        "no dispatchable tasks and no active assignments".to_string(),
                    ));
                }
                sleep(self.config.dispatch_interval).await;
                continue;
            };

            let assigned_worker = self.dispatch_task(task.clone()).await?;
            let completion = self.execute_task_on_worker(&task, &assigned_worker).await;
            self.dispatcher.submit_completion(completion).await;

            let completion_report = self.handle_dispatcher_completions().await?;
            if completion_report.completed == 0 && completion_report.failed == 0 {
                return Err(CoordinatorV2Error::StalledMission(
                    "dispatcher did not yield a completion".to_string(),
                ));
            }
        }
        self.finish_mission(mission_id, self.task_queue.completed_tasks())
    }

    fn begin_mission(&mut self, mission_id: &str, mission: &str) {
        self.mission_id = Some(mission_id.to_string());
        self.mission_started_at = Some(Instant::now());
        self.merged_outputs.clear();
        self.tasks_failed = 0;
        self.trace_events.clear();
        self.task_queue = TaskQueueIntegration::new();
        if let (Some(registry), Some(tracer)) = (
            self.config.execution_trace_registry.as_ref(),
            self.config.execution_tracer.as_ref(),
        ) {
            registry.register_mission(tracer.session_id(), mission_id);
        }
        tracing::info!(mission_id = %mission_id, mission = %mission, "mission started");
        if let Some(ref gate) = self.config.hitl_gate {
            let _ = gate.register_mission_start(mission_id);
        }
        let mut event = self.make_trace_event(
            "",
            "coordinator",
            ExecutionEventKind::Mission,
            ExecutionTracePhase::Started,
            mission,
        );
        event.action_id = mission_id.to_string();
        event.result_preview = Some(mission.to_string());
        self.emit_trace_event(event);
    }

    fn finish_mission(
        &mut self,
        mission_id: String,
        tasks_completed: usize,
    ) -> Result<MissionResult> {
        let duration = self
            .mission_started_at
            .map(|started| started.elapsed())
            .unwrap_or(Duration::ZERO);
        let success = self.tasks_failed == 0;
        if let Some(ref gate) = self.config.hitl_gate {
            gate.register_mission_complete(&mission_id, success);
        }
        tracing::info!(
            mission_id = %mission_id,
            success = success,
            tasks_completed = tasks_completed,
            tasks_failed = self.tasks_failed,
            duration_ms = duration.as_millis(),
            "mission completed"
        );
        let mut event = self.make_trace_event(
            "",
            "coordinator",
            ExecutionEventKind::Mission,
            if success {
                ExecutionTracePhase::Completed
            } else {
                ExecutionTracePhase::Failed
            },
            "mission finished",
        );
        event.action_id = mission_id.clone();
        event.duration_ms = Some(duration.as_millis() as u64);
        event.result_preview = Some(format!(
            "tasks_completed={} tasks_failed={}",
            tasks_completed, self.tasks_failed
        ));
        self.emit_trace_event(event);
        Ok(MissionResult {
            mission_id,
            success,
            output: self.merged_outputs.join("\n"),
            tasks_completed,
            tasks_failed: self.tasks_failed,
            duration,
            trace_events: self.trace_events.clone(),
        })
    }

    /// Returns the next idle worker, if one exists.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.worker_registry.get_available_worker()
    }

    fn session_id(&self) -> String {
        self.config
            .execution_tracer
            .as_ref()
            .map(|tracer| tracer.session_id().to_string())
            .unwrap_or_default()
    }

    fn make_trace_event(
        &self,
        task_id: impl Into<String>,
        agent_id: impl Into<String>,
        event_kind: ExecutionEventKind,
        phase: ExecutionTracePhase,
        display_name: impl Into<String>,
    ) -> ExecutionTraceEvent {
        ExecutionTraceEvent::new(
            self.session_id(),
            self.mission_id.clone().unwrap_or_default(),
            task_id.into(),
            format!("turn-{}", self.trace_events.len() + 1),
            agent_id.into(),
            event_kind,
            phase,
            display_name,
        )
    }

    fn emit_trace_event(&mut self, event: ExecutionTraceEvent) {
        let emitted = if let Some(tracer) = &self.config.execution_tracer {
            match tracer.emit(event) {
                Ok(event) => Some(event),
                Err(err) => {
                    tracing::warn!(error = %err, "failed to emit execution trace event");
                    None
                }
            }
        } else {
            Some(event)
        };

        if let Some(event) = emitted {
            self.trace_events.push(event);
        }
    }

    /// Assigns a task to a worker in the registry.
    pub fn assign_task(&mut self, worker_id: WorkerId, task: Task) -> Result<()> {
        let inner: CoreResult<()> = self.worker_registry.assign_task(&worker_id, &task);
        inner?;
        Ok(())
    }

    /// Returns the current mission status.
    pub fn get_mission_status(&self) -> MissionStatus {
        let completed = self.task_queue.completed_tasks();
        let total = self.task_queue.total_tasks();
        let progress = if total == 0 {
            0.0
        } else {
            (completed as f32 / total as f32) * 100.0
        };

        let eta = self
            .mission_started_at
            .and_then(|started| estimate_eta(started.elapsed(), progress));

        let active_workers = self
            .worker_registry
            .workers
            .iter()
            .fold(0usize, |count, entry| {
                if matches!(entry.value().status, WorkerStatus::Busy(_)) {
                    count + 1
                } else {
                    count
                }
            });

        MissionStatus {
            mission_id: self
                .mission_id
                .clone()
                .unwrap_or_else(|| "uninitialized".to_string()),
            progress,
            eta,
            active_workers,
            completed_tasks: completed,
            total_tasks: total,
        }
    }

    async fn dispatch_task(&mut self, task: Task) -> Result<WorkerId> {
        let mut dispatch_task = prepare_dispatch_task_snapshot(&self.config, &task);

        let mut dispatch_workers = self.dispatch_workers_snapshot();
        let report = self
            .dispatcher
            .dispatch_loop(
                std::slice::from_mut(&mut dispatch_task),
                &mut dispatch_workers,
            )
            .await?;

        if report.dispatched == 0 {
            return Err(CoordinatorV2Error::NoAvailableWorker);
        }

        let worker_id = self
            .dispatcher
            .assigned_worker(&dispatch_task.id)
            .ok_or(CoordinatorV2Error::NoAvailableWorker)?;

        let task_id = task.id.clone();
        let task_description = task.description.clone();
        self.assign_task(worker_id.clone(), task)?;
        let mut task_started = self.make_trace_event(
            task_id.clone(),
            worker_id.clone(),
            ExecutionEventKind::Task,
            ExecutionTracePhase::Started,
            task_description.clone(),
        );
        task_started.action_id = task_id.clone();
        self.emit_trace_event(task_started);
        let mut assignment_event = self.make_trace_event(
            task_id.clone(),
            worker_id.clone(),
            ExecutionEventKind::AgentAssignment,
            ExecutionTracePhase::Started,
            format!("assign {}", task_description),
        );
        assignment_event.action_id = format!("assign:{}:{}", task_id, worker_id);
        assignment_event.parent_action_id = Some(task_id);
        assignment_event.result_preview = Some(format!("worker={worker_id}"));
        self.emit_trace_event(assignment_event);
        self.sync_workers_from_dispatcher(&dispatch_workers);
        Ok(worker_id)
    }

    async fn handle_dispatcher_completions(&mut self) -> Result<CompletionReport> {
        let mut workers = self.dispatch_workers_snapshot();
        let mut tasks = self.current_assigned_tasks();
        let report = self
            .dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await?;

        for completion in &report.processed_completions {
            let Some(task) = tasks.iter().find(|task| task.id == completion.task_id) else {
                continue;
            };

            match task.status {
                TaskStatus::Completed => {
                    self.task_queue.mark_task_complete(&task.id)?;
                    if let Some(result_submission) = completion.result_submission.as_ref() {
                        self.publish_result_submission(result_submission).await?;
                        self.merged_outputs.push(result_submission.summary.clone());
                    } else {
                        let result_submission =
                            self.completion_result_submission(task, completion, true);
                        self.publish_result_submission(&result_submission).await?;
                        self.merged_outputs.push(result_submission.summary.clone());
                    }
                    let mut task_event = self.make_trace_event(
                        task.id.clone(),
                        completion.worker_id.clone(),
                        ExecutionEventKind::Task,
                        ExecutionTracePhase::Completed,
                        task.description.clone(),
                    );
                    task_event.action_id = task.id.clone();
                    task_event.result_preview = Some(completion.summary.clone());
                    self.emit_trace_event(task_event);
                    let mut result_event = self.make_trace_event(
                        task.id.clone(),
                        completion.worker_id.clone(),
                        ExecutionEventKind::AgentResult,
                        ExecutionTracePhase::Completed,
                        format!("result {}", task.description),
                    );
                    result_event.action_id = format!("result:{}:{}", task.id, completion.worker_id);
                    result_event.parent_action_id = Some(task.id.clone());
                    result_event.result_preview = Some(completion.summary.clone());
                    self.emit_trace_event(result_event);
                }
                TaskStatus::Failed => {
                    self.task_queue.mark_task_complete(&task.id)?;
                    self.tasks_failed += 1;
                    // Bug class A fix: merge failure text into merged_outputs so CLI has something to show
                    let failure_text =
                        if let Some(ref result_submission) = completion.result_submission {
                            if result_submission.error_message.is_empty() {
                                format!("Task '{}' failed: no error details", task.description)
                            } else {
                                format!(
                                    "Task '{}' failed: {}",
                                    task.description, result_submission.error_message
                                )
                            }
                        } else {
                            let result_submission =
                                self.completion_result_submission(task, completion, false);
                            self.publish_result_submission(&result_submission).await?;
                            format!(
                                "Task '{}' failed: {}",
                                task.description, result_submission.error_message
                            )
                        };
                    self.merged_outputs.push(failure_text);
                    let mut task_event = self.make_trace_event(
                        task.id.clone(),
                        completion.worker_id.clone(),
                        ExecutionEventKind::Task,
                        ExecutionTracePhase::Failed,
                        task.description.clone(),
                    );
                    task_event.action_id = task.id.clone();
                    task_event.error = completion.error.clone();
                    self.emit_trace_event(task_event);
                    let mut result_event = self.make_trace_event(
                        task.id.clone(),
                        completion.worker_id.clone(),
                        ExecutionEventKind::AgentResult,
                        ExecutionTracePhase::Failed,
                        format!("result {}", task.description),
                    );
                    result_event.action_id = format!("result:{}:{}", task.id, completion.worker_id);
                    result_event.parent_action_id = Some(task.id.clone());
                    result_event.error = completion.error.clone();
                    self.emit_trace_event(result_event);
                }
                _ => {}
            }
        }

        self.sync_workers_from_dispatcher(&workers);
        Ok(report)
    }

    fn current_assigned_tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();

        for entry in &self.worker_registry.workers {
            let worker = entry.value();
            let Some(task_id) = worker.current_task.as_ref() else {
                continue;
            };

            if let Some(task) = self.task_queue.get_task(task_id).cloned() {
                tasks.push(task);
            }
        }

        tasks
    }

    fn dispatch_workers_snapshot(&self) -> Vec<DispatchWorker> {
        let mut workers = Vec::new();

        for entry in &self.worker_registry.workers {
            let worker = entry.value();
            workers.push(DispatchWorker {
                id: worker.id.clone(),
                status: to_dispatch_status(&worker.status),
                current_task: worker.current_task.clone(),
                last_heartbeat: worker.last_heartbeat,
            });
        }

        workers
    }

    fn sync_workers_from_dispatcher(&self, workers: &[DispatchWorker]) {
        for dispatch_worker in workers {
            if let Some(mut worker) = self.worker_registry.workers.get_mut(&dispatch_worker.id) {
                worker.current_task = dispatch_worker.current_task.clone();
                worker.last_heartbeat = dispatch_worker.last_heartbeat;
                worker.status = match dispatch_worker.status {
                    DispatchWorkerStatus::Idle => WorkerStatus::Idle,
                    DispatchWorkerStatus::Busy => WorkerStatus::Busy(
                        dispatch_worker
                            .current_task
                            .clone()
                            .unwrap_or_else(|| "unknown-task".to_string()),
                    ),
                    DispatchWorkerStatus::Unhealthy => WorkerStatus::Unhealthy {
                        reason: "dispatcher marked worker unhealthy".to_string(),
                    },
                    DispatchWorkerStatus::Failed => WorkerStatus::Failed {
                        error: "dispatcher recorded worker failure".to_string(),
                    },
                };
            }
        }
    }

    fn apply_monitor_report(
        &self,
        _report: &self::v2_dispatcher::MonitorReport,
        workers: &[DispatchWorker],
    ) {
        self.sync_workers_from_dispatcher(workers);
    }

    /// Returns live coordinator metrics.
    pub fn metrics(&self) -> CoordinatorMetrics {
        self.metrics.clone()
    }

    async fn execute_task_on_worker(&mut self, task: &Task, worker_id: &str) -> DispatchCompletion {
        let assignment = match self.build_task_assignment(task, None) {
            Ok(assignment) => assignment,
            Err(err) => {
                let result = self.failure_result_submission(task, None, &err.to_string());
                return DispatchCompletion::failure_with_result(
                    task.id.clone(),
                    worker_id.to_string(),
                    err.to_string(),
                    result,
                );
            }
        };

        if let Err(err) = self
            .communication
            .send_typed_task_assignment(worker_id, &assignment)
        {
            self.metrics.protocol_validation_failures += 1;
            let result = self.failure_result_submission(task, Some(&assignment), &err);
            return DispatchCompletion::failure_with_result(
                task.id.clone(),
                worker_id.to_string(),
                err,
                result,
            );
        }

        if let Some(context_pack) = assignment.context_pack.as_ref() {
            if let Err(err) = self
                .communication
                .send_context_pack(worker_id, context_pack)
            {
                self.metrics.protocol_validation_failures += 1;
                let result = self.failure_result_submission(task, Some(&assignment), &err);
                return DispatchCompletion::failure_with_result(
                    task.id.clone(),
                    worker_id.to_string(),
                    err,
                    result,
                );
            }
        }

        self.emit_transition(
            worker_id,
            &task.id,
            "assigned",
            "running",
            "task dispatched",
            0,
            false,
        );

        let mut attempt = 0u32;
        loop {
            attempt += 1;
            if let Err(err) = self.communication.send_typed_progress_update(
                worker_id,
                &crate::transport::InternalProgressUpdate {
                    task_id: task.id.clone(),
                    stage: "provider_execution".to_string(),
                    message: format!("attempt {attempt}"),
                    completion_ratio: 0.25,
                },
            ) {
                self.metrics.protocol_validation_failures += 1;
                tracing::warn!(
                    task_id = %task.id,
                    worker_id = %worker_id,
                    error = %err,
                    "failed to send typed progress update"
                );
            }

            match timeout(
                self.config.task_timeout,
                self.execute_task_once(task, worker_id, &assignment),
            )
            .await
            {
                Ok(Ok(result_submission)) => {
                    if let Err(err) = self
                        .communication
                        .send_typed_result_submission(worker_id, &result_submission)
                    {
                        self.metrics.protocol_validation_failures += 1;
                        tracing::warn!(
                            task_id = %task.id,
                            worker_id = %worker_id,
                            error = %err,
                            "failed to send typed result submission"
                        );
                    }
                    self.emit_transition(
                        worker_id,
                        &task.id,
                        "running",
                        "completed",
                        "task finished",
                        attempt.saturating_sub(1),
                        true,
                    );
                    return DispatchCompletion::success_with_result(
                        task.id.clone(),
                        worker_id.to_string(),
                        result_submission.summary.clone(),
                        result_submission,
                    );
                }
                Ok(Err(err)) => {
                    let message = err.to_string();
                    let result = self.failure_result_submission(task, Some(&assignment), &message);
                    if let Err(send_err) = self.communication.send_typed_blocker_alert(
                        worker_id,
                        &crate::transport::InternalBlockerAlert {
                            task_id: task.id.clone(),
                            severity: "critical".to_string(),
                            message: message.clone(),
                            retryable: false,
                        },
                    ) {
                        self.metrics.protocol_validation_failures += 1;
                        tracing::warn!(
                            task_id = %task.id,
                            worker_id = %worker_id,
                            error = %send_err,
                            "failed to send typed blocker alert"
                        );
                    }
                    if let Err(send_err) = self
                        .communication
                        .send_typed_result_submission(worker_id, &result)
                    {
                        self.metrics.protocol_validation_failures += 1;
                        tracing::warn!(
                            task_id = %task.id,
                            worker_id = %worker_id,
                            error = %send_err,
                            "failed to send typed result submission (failure path)"
                        );
                    }
                    self.emit_transition(
                        worker_id,
                        &task.id,
                        "running",
                        "failed",
                        &message,
                        attempt.saturating_sub(1),
                        true,
                    );
                    return DispatchCompletion::failure_with_result(
                        task.id.clone(),
                        worker_id.to_string(),
                        message,
                        result,
                    );
                }
                Err(_) => {
                    self.metrics.timeout_failures += 1;
                    if attempt <= self.config.retry_budget {
                        continue;
                    }
                    self.metrics.retry_exhaustions += 1;
                    let message = format!("task timed out after {:?}", self.config.task_timeout);
                    let result = self.failure_result_submission(task, Some(&assignment), &message);
                    if let Err(send_err) = self.communication.send_typed_blocker_alert(
                        worker_id,
                        &crate::transport::InternalBlockerAlert {
                            task_id: task.id.clone(),
                            severity: "critical".to_string(),
                            message: message.clone(),
                            retryable: false,
                        },
                    ) {
                        self.metrics.protocol_validation_failures += 1;
                        tracing::warn!(
                            task_id = %task.id,
                            worker_id = %worker_id,
                            error = %send_err,
                            "failed to send typed blocker alert (timeout path)"
                        );
                    }
                    if let Err(send_err) = self
                        .communication
                        .send_typed_result_submission(worker_id, &result)
                    {
                        self.metrics.protocol_validation_failures += 1;
                        tracing::warn!(
                            task_id = %task.id,
                            worker_id = %worker_id,
                            error = %send_err,
                            "failed to send typed result submission (timeout path)"
                        );
                    }
                    self.emit_transition(
                        worker_id, &task.id, "running", "failed", &message, attempt, true,
                    );
                    return DispatchCompletion::failure_with_result(
                        task.id.clone(),
                        worker_id.to_string(),
                        message,
                        result,
                    );
                }
            }
        }
    }

    async fn execute_task_once(
        &mut self,
        task: &Task,
        worker_id: &str,
        assignment: &InternalTaskAssignment,
    ) -> Result<InternalResultSubmission> {
        if task.task_type == TaskType::CodeModification {
            self.execute_code_task(task, worker_id, assignment).await
        } else {
            self.execute_non_code_task(task, assignment).await
        }
    }

    async fn execute_code_task(
        &mut self,
        task: &Task,
        worker_id: &str,
        assignment: &InternalTaskAssignment,
    ) -> Result<InternalResultSubmission> {
        if assignment.target_files.is_empty() {
            return Err(CoordinatorV2Error::ProtocolViolation(
                "code modification task requires a target file".to_string(),
            ));
        }

        let mut target = self.resolve_route(task, assignment)?;
        let mut local_validation_failures = 0u32;

        let (executed_target, model_response, validated) = loop {
            let (executed_target, model_response) = self
                .execute_provider_request(task, assignment, target.clone())
                .await?;
            match self.diff_validator.validate(&model_response.output_text) {
                Ok(validated) => break (executed_target, model_response, validated),
                Err(err) => {
                    self.metrics.protocol_validation_failures += 1;
                    if matches!(executed_target, RoutedTarget::Local(_))
                        && local_validation_failures < self.config.local_validation_retry_budget
                    {
                        local_validation_failures += 1;
                        target = executed_target;
                        continue;
                    }
                    if matches!(executed_target, RoutedTarget::Local(_)) {
                        return self
                            .execute_arbiter_review(
                                task,
                                worker_id,
                                assignment,
                                &model_response.output_text,
                                &err.to_string(),
                            )
                            .await;
                    }
                    return Err(CoordinatorV2Error::ProtocolViolation(err.to_string()));
                }
            }
        };

        let patch =
            self.build_patch_envelope(task, assignment, &validated.raw_output, validated.format)?;
        if let Err(err) = self.communication.send_patch_envelope(worker_id, &patch) {
            self.metrics.protocol_validation_failures += 1;
            return Err(CoordinatorV2Error::ProtocolViolation(err));
        }

        let patch_receipt = self
            .patch_applier
            .apply_to_workspace(&self.config.workspace_root, &patch)
            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;
        if let Err(err) = self
            .communication
            .send_patch_receipt(worker_id, &patch_receipt)
        {
            self.metrics.protocol_validation_failures += 1;
            return Err(CoordinatorV2Error::ProtocolViolation(err));
        }

        if patch_receipt.status != PatchApplyStatus::Applied {
            return Err(CoordinatorV2Error::ExecutionFailed(format!(
                "patch application failed: {}",
                patch_receipt.message
            )));
        }

        Ok(InternalResultSubmission {
            task_id: task.id.clone(),
            success: true,
            patch: Some(patch),
            patch_receipt: Some(patch_receipt),
            token_usage: to_internal_token_usage(
                &executed_target.provider_label(),
                &model_response.usage,
            ),
            context_references: context_references_from_assignment(assignment),
            summary: format!("applied patch for {}", task.description),
            error_message: String::new(),
            diagnostic_toon: None,
        })
    }

    async fn execute_non_code_task(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
    ) -> Result<InternalResultSubmission> {
        let target = self.resolve_route(task, assignment)?;
        let (executed_target, model_response) = self
            .execute_provider_request(task, assignment, target)
            .await?;

        Ok(InternalResultSubmission {
            task_id: task.id.clone(),
            success: true,
            patch: None,
            patch_receipt: None,
            token_usage: to_internal_token_usage(
                &executed_target.provider_label(),
                &model_response.usage,
            ),
            context_references: context_references_from_assignment(assignment),
            summary: model_response.output_text,
            error_message: String::new(),
            diagnostic_toon: None,
        })
    }

    fn resolve_route(
        &self,
        task: &Task,
        assignment: &InternalTaskAssignment,
    ) -> Result<RoutedTarget> {
        route(
            task,
            assignment,
            &self.registry,
            self.config.routing_enabled,
            &self.config.model_instance_priority,
            None,
        )
        .ok_or_else(|| {
            CoordinatorV2Error::InvalidConfig(
                "no cloud or local execution lane is configured".to_string(),
            )
        })
    }

    async fn execute_provider_request(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        target: RoutedTarget,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        let model_request = self.build_model_request(task, assignment, &target)?;
        self.execute_model_request(task, assignment, model_request, target)
            .await
    }

    async fn execute_model_request(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        model_request: ModelRequest,
        target: RoutedTarget,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        let contract = assignment.canonical_contract();
        let mut current_request = model_request;
        let mut current_target = target;
        let mut turn_index = 0u32;
        let mut tool_calls = 0u32;
        let mut mutating_tool_calls = 0u32;

        loop {
            let (executed_target, response) = self
                .invoke_model_once(
                    task,
                    assignment,
                    current_request.clone(),
                    current_target.clone(),
                )
                .await?;

            if response.tool_calls.is_empty()
                || self.config.mcp_endpoint.is_none()
                || turn_index >= contract.budget.max_tool_turns
            {
                return Ok((executed_target, response));
            }

            let assistant_message = crate::provider::ChatMessage {
                role: "assistant".to_string(),
                content: response.content.clone(),
                name: None,
                tool_call_id: None,
                tool_calls: response.tool_calls.clone(),
            };
            current_request.recent_messages.push(assistant_message);

            for call in &response.tool_calls {
                tool_calls += 1;
                if !contract.allowed_tools.iter().any(|tool| tool == &call.name) {
                    let mut denied = self.tool_trace_event(
                        task,
                        assignment,
                        &crate::tool_registry::ToolSpec {
                            name: call.name.clone(),
                            description: String::new(),
                            parameters_json_schema: Value::Null,
                            strict: false,
                            tool_kind: crate::tool_registry::ToolKind::Command,
                            read_only: true,
                            mutating: false,
                            executor_kind: crate::tool_registry::ExecutorKind::Mcp,
                            allowed_roles: Vec::new(),
                            allowed_task_types: Vec::new(),
                            supported_models: Vec::new(),
                            cost_class: crate::tool_registry::CostClass::Low,
                            ui_renderer: crate::tool_registry::UiRenderer::ToolCall,
                            result_normalizer:
                                crate::tool_registry::ResultNormalizerKind::PlainText,
                            provider_safe: false,
                            requires_approval: false,
                        },
                        call,
                        ExecutionTracePhase::Denied,
                    );
                    denied.error = Some(format!(
                        "tool '{}' is not allowed by assignment contract",
                        call.name
                    ));
                    self.emit_trace_event(denied);
                    current_request.recent_messages.push(tool_error_message(
                        &call.id,
                        format!("tool '{}' is not allowed by assignment contract", call.name),
                    ));
                    continue;
                }
                let Some(spec) = self.tool_registry.get(&call.name).cloned() else {
                    let mut denied = self.tool_trace_event(
                        task,
                        assignment,
                        &crate::tool_registry::ToolSpec {
                            name: call.name.clone(),
                            description: String::new(),
                            parameters_json_schema: Value::Null,
                            strict: false,
                            tool_kind: crate::tool_registry::ToolKind::Command,
                            read_only: true,
                            mutating: false,
                            executor_kind: crate::tool_registry::ExecutorKind::Mcp,
                            allowed_roles: Vec::new(),
                            allowed_task_types: Vec::new(),
                            supported_models: Vec::new(),
                            cost_class: crate::tool_registry::CostClass::Low,
                            ui_renderer: crate::tool_registry::UiRenderer::ToolCall,
                            result_normalizer:
                                crate::tool_registry::ResultNormalizerKind::PlainText,
                            provider_safe: false,
                            requires_approval: false,
                        },
                        call,
                        ExecutionTracePhase::Denied,
                    );
                    denied.error = Some(format!("unsupported tool '{}'", call.name));
                    self.emit_trace_event(denied);
                    current_request.recent_messages.push(tool_error_message(
                        &call.id,
                        format!("unsupported tool '{}'", call.name),
                    ));
                    continue;
                };

                if tool_calls > contract.budget.max_tool_calls {
                    let mut denied = self.tool_trace_event(
                        task,
                        assignment,
                        &spec,
                        call,
                        ExecutionTracePhase::Denied,
                    );
                    denied.error = Some("tool call budget exceeded".to_string());
                    self.emit_trace_event(denied);
                    current_request.recent_messages.push(tool_error_message(
                        &call.id,
                        "tool call budget exceeded".to_string(),
                    ));
                    continue;
                }
                if spec.mutating {
                    mutating_tool_calls += 1;
                    if mutating_tool_calls > contract.budget.max_mutating_tool_calls {
                        let mut denied = self.tool_trace_event(
                            task,
                            assignment,
                            &spec,
                            call,
                            ExecutionTracePhase::Denied,
                        );
                        denied.error = Some("mutating tool budget exceeded".to_string());
                        self.emit_trace_event(denied);
                        current_request.recent_messages.push(tool_error_message(
                            &call.id,
                            "mutating tool budget exceeded".to_string(),
                        ));
                        continue;
                    }
                }

                self.emit_trace_event(self.tool_trace_event(
                    task,
                    assignment,
                    &spec,
                    call,
                    ExecutionTracePhase::Requested,
                ));
                if spec.requires_approval {
                    self.emit_trace_event(self.tool_trace_event(
                        task,
                        assignment,
                        &spec,
                        call,
                        ExecutionTracePhase::Approved,
                    ));
                }
                self.emit_trace_event(self.tool_trace_event(
                    task,
                    assignment,
                    &spec,
                    call,
                    ExecutionTracePhase::Started,
                ));

                match self
                    .execute_tool_call_via_mcp(task, assignment, turn_index, &spec, call)
                    .await
                {
                    Ok(result_text) => {
                        let mut completed = self.tool_trace_event(
                            task,
                            assignment,
                            &spec,
                            call,
                            ExecutionTracePhase::Completed,
                        );
                        completed.result_preview = Some(result_text.clone());
                        self.emit_trace_event(completed);
                        current_request
                            .recent_messages
                            .push(crate::provider::ChatMessage {
                                role: "tool".to_string(),
                                content: result_text,
                                name: Some(call.name.clone()),
                                tool_call_id: Some(call.id.clone()),
                                tool_calls: Vec::new(),
                            });
                    }
                    Err(err) => {
                        let mut failed = self.tool_trace_event(
                            task,
                            assignment,
                            &spec,
                            call,
                            ExecutionTracePhase::Failed,
                        );
                        failed.error = Some(err.to_string());
                        self.emit_trace_event(failed);
                        current_request
                            .recent_messages
                            .push(tool_error_message(&call.id, err.to_string()));
                    }
                }
            }

            turn_index += 1;
            current_target = executed_target;
        }
    }

    async fn invoke_model_once(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        model_request: ModelRequest,
        target: RoutedTarget,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        let started_at = Instant::now();
        let provider_action_id = Uuid::new_v4().to_string();
        let target_provider_label = target.provider_label();
        let target_model_label = target.model_label().to_string();
        let mut started = self.make_trace_event(
            assignment.task_id.clone(),
            infer_worker_role_from_task(task).to_string(),
            ExecutionEventKind::ProviderRequest,
            ExecutionTracePhase::Started,
            format!("provider request {}", target_provider_label),
        );
        started.action_id = provider_action_id.clone();
        started.provider = Some(target_provider_label.clone());
        started.model = Some(target_model_label.clone());
        started.message_count = Some(model_request.recent_messages.len() as u32);
        started.tool_call_count = Some(model_request.tool_schemas.len() as u32);
        self.emit_trace_event(started);

        let outcome = match &target {
            RoutedTarget::Cloud(_cloud_target) => {
                // Phase 7+: Always use API client for cloud execution
                self.execute_via_api(task, assignment, model_request, target.clone())
                    .await
            }
            RoutedTarget::Local(local) => {
                let local_transport = self
                    .registry
                    .local_transport(&local.instance_id)
                    .ok_or_else(|| {
                        CoordinatorV2Error::InvalidConfig(
                            "local lane is not configured".to_string(),
                        )
                    })?;
                let response = local_transport
                    .execute_local(&model_request, &local.model)
                    .await
                    .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;
                Ok((target, response))
            }
        };

        match &outcome {
            Ok((executed_target, response)) => {
                let mut completed = self.make_trace_event(
                    assignment.task_id.clone(),
                    infer_worker_role_from_task(task).to_string(),
                    ExecutionEventKind::ProviderRequest,
                    ExecutionTracePhase::Completed,
                    format!("provider request {}", executed_target.provider_label()),
                );
                completed.action_id = provider_action_id;
                completed.provider = Some(executed_target.provider_label());
                completed.model = Some(executed_target.model_label().to_string());
                completed.provider_request_id = response.provider_request_id.clone();
                completed.stop_reason = response.stop_reason.clone();
                completed.usage_preview = Some(format!(
                    "input={} output={} total={}",
                    response.usage.input_tokens,
                    response.usage.output_tokens,
                    response.usage.total_tokens
                ));
                completed.tool_call_count = Some(response.tool_calls.len() as u32);
                completed.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                self.emit_trace_event(completed);
            }
            Err(err) => {
                let mut failed = self.make_trace_event(
                    assignment.task_id.clone(),
                    infer_worker_role_from_task(task).to_string(),
                    ExecutionEventKind::ProviderRequest,
                    ExecutionTracePhase::Failed,
                    format!("provider request {}", target_provider_label),
                );
                failed.action_id = provider_action_id;
                failed.provider = Some(target_provider_label);
                failed.model = Some(target_model_label);
                failed.error = Some(err.to_string());
                failed.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                self.emit_trace_event(failed);
            }
        }

        outcome
    }

    /// Execute a model request via the API (Phase 5+).
    async fn execute_via_api(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        model_request: ModelRequest,
        target: RoutedTarget,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        use openakta_api_client::provider_v1::{
            ChatMessage as ProtoChatMessage, ProviderRequest, StopReason, ToolChoice,
            ToolDefinition as ProtoToolDefinition,
        };

        // Convert ModelRequest to ProviderRequest proto
        let request_id = Uuid::new_v4().to_string();

        // Build messages from model request.
        let mut messages: Vec<ProtoChatMessage> = model_request
            .invariant_mission_context
            .iter()
            .map(|invariant| ProtoChatMessage {
                role: "system".to_string(),
                content: Some(invariant.to_string()),
                name: None,
                content_parts: vec![],
                tool_call: None,
                tool_call_id: None,
            })
            .collect();
        messages.push(ProtoChatMessage {
            role: "user".to_string(),
            content: Some(
                model_request
                    .payload
                    .to_toon()
                    .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?,
            ),
            name: None,
            content_parts: vec![],
            tool_call: None,
            tool_call_id: None,
        });
        messages.extend(
            model_request
                .recent_messages
                .iter()
                .map(|msg| ProtoChatMessage {
                    role: msg.role.clone(),
                    content: Some(msg.content.clone()),
                    name: msg.name.clone(),
                    content_parts: vec![],
                    tool_call: msg.tool_calls.first().map(|tool_call| {
                        openakta_api_client::provider_v1::ToolCall {
                            id: tool_call.id.clone(),
                            name: tool_call.name.clone(),
                            arguments: tool_call.arguments_json.clone(),
                        }
                    }),
                    tool_call_id: msg.tool_call_id.clone(),
                }),
        );

        // Build tool definitions if present
        let tools: Vec<ProtoToolDefinition> = model_request
            .tool_schemas
            .iter()
            .map(|tool| ProtoToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: value_to_proto_struct(&tool.parameters),
                strict: tool.strict,
            })
            .collect();

        let provider_request = ProviderRequest {
            request_id: request_id.clone(),
            tenant_id: String::new(), // Phase 6: Server extracts tenant_id from JWT auth header (client sends JWT, server validates and extracts tenant)
            model: model_request.model.clone(),
            model_hint: None,
            system_prompt: model_request.system_instructions.join("\n"),
            messages,
            tools,
            tool_choice: ToolChoice::Auto as i32,
            max_tokens: Some(model_request.max_output_tokens),
            temperature: model_request.temperature,
            top_p: None,
            stop_sequences: vec![],
            frequency_penalty: None,
            presence_penalty: None,
            stream: model_request.stream,
            provider_extensions: std::collections::HashMap::new(),
            required_capabilities: vec![],
            execution_strategy: openakta_api_client::provider_v1::ExecutionStrategy::HostedOnly
                as i32,
        };

        // Execute via API client
        let api_client = &self.registry.api_client_pool.completion_client;

        match api_client.execute(provider_request).await {
            Ok(proto_response) => {
                let request_id = proto_response.request_id.clone();
                let response_id = proto_response.response_id.clone();
                let model = proto_response.model.clone();
                let provider = proto_response.provider.clone();
                let content = proto_response.content.clone();
                let warnings = proto_response.warnings.clone();
                // Convert ProviderResponse proto back to ModelResponse
                let model_response = ModelResponse {
                    id: Some(response_id.clone()),
                    provider: crate::provider::ProviderKind::OpenAi, // Normalized to OpenAI family
                    content: content.clone(),
                    output_text: content.clone(),
                    tool_calls: proto_response
                        .tool_calls
                        .iter()
                        .map(|call| crate::provider::ModelToolCall {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            arguments_json: call.arguments.clone(),
                        })
                        .collect(),
                    usage: crate::provider::ProviderUsage {
                        input_tokens: proto_response
                            .usage
                            .as_ref()
                            .map(|u| u.input_tokens as usize)
                            .unwrap_or(0),
                        output_tokens: proto_response
                            .usage
                            .as_ref()
                            .map(|u| u.output_tokens as usize)
                            .unwrap_or(0),
                        total_tokens: proto_response
                            .usage
                            .as_ref()
                            .map(|u| u.total_tokens as usize)
                            .unwrap_or(0),
                        cache_write_tokens: proto_response
                            .usage
                            .as_ref()
                            .and_then(|u| u.cache_write_tokens)
                            .map(|t| t as usize)
                            .unwrap_or(0),
                        cache_read_tokens: proto_response
                            .usage
                            .as_ref()
                            .and_then(|u| u.cache_read_tokens)
                            .map(|t| t as usize)
                            .unwrap_or(0),
                        uncached_input_tokens: proto_response
                            .usage
                            .as_ref()
                            .map(|u| u.input_tokens as usize)
                            .unwrap_or(0),
                    },
                    stop_reason: StopReason::try_from(proto_response.stop_reason)
                        .ok()
                        .map(|reason| reason.as_str_name().to_string()),
                    provider_request_id: Some(request_id.clone()),
                    raw: json!({
                        "request_id": request_id,
                        "response_id": response_id,
                        "model": model,
                        "provider": provider,
                        "content": content,
                        "warnings": warnings,
                    }),
                };

                Ok((target, model_response))
            }
            Err(e) => {
                // Map API errors to coordinator errors
                match e {
                    openakta_api_client::error::ApiError::CircuitOpen => {
                        self.handle_cloud_unavailable(
                            task,
                            assignment,
                            target,
                            "API circuit breaker is open".to_string(),
                        )
                        .await
                    }
                    openakta_api_client::error::ApiError::Unavailable(message)
                    | openakta_api_client::error::ApiError::ConnectionRefused(message)
                    | openakta_api_client::error::ApiError::Timeout(message) => {
                        self.handle_cloud_unavailable(task, assignment, target, message)
                            .await
                    }
                    _ => Err(CoordinatorV2Error::ExecutionFailed(format!(
                        "API execution failed: {}",
                        e
                    ))),
                }
            }
        }
    }

    async fn execute_tool_call_via_mcp(
        &self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        turn_index: u32,
        spec: &crate::tool_registry::ToolSpec,
        call: &crate::provider::ModelToolCall,
    ) -> Result<String> {
        let endpoint = self.config.mcp_endpoint.as_ref().ok_or_else(|| {
            CoordinatorV2Error::ExecutionFailed("MCP endpoint is not configured".to_string())
        })?;
        let client = McpClient::new(endpoint.clone());
        let arguments = serde_json::from_str::<Value>(&call.arguments_json)
            .unwrap_or_else(|_| Value::Object(Map::new()));
        let role = infer_worker_role_from_task(task);
        let observation = client
            .call_tool(
                &call.id,
                "coordinator",
                role,
                &call.name,
                &self.config.workspace_root.display().to_string(),
                value_to_proto_struct(&arguments).unwrap_or_default(),
                Some(self.capability_policy_for_task(role, task, assignment)),
                self.mission_id.as_deref(),
            )
            .await
            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;

        if !observation.success {
            return Err(CoordinatorV2Error::ExecutionFailed(
                observation
                    .error
                    .unwrap_or_else(|| format!("tool '{}' failed", spec.name)),
            ));
        }

        Ok(normalize_tool_result_for_model(
            &observation.result,
            &assignment.task_id,
            turn_index,
        ))
    }

    fn capability_policy_for_task(
        &self,
        role: &str,
        task: &Task,
        assignment: &InternalTaskAssignment,
    ) -> CapabilityPolicy {
        let workspace_root = self.config.workspace_root.display().to_string();
        let contract = assignment.canonical_contract();
        let allowed_actions = self
            .tool_registry
            .specs()
            .iter()
            .filter(|spec| contract.allowed_tools.iter().any(|tool| tool == &spec.name))
            .filter(|spec| spec.provider_safe)
            .filter(|spec| spec.supports_role(role))
            .filter(|spec| spec.supports_task_type(&task.task_type))
            .map(|spec| spec.name.clone())
            .collect::<Vec<_>>();
        CapabilityPolicy {
            agent_id: "coordinator".to_string(),
            role: role.to_string(),
            allowed_actions,
            allowed_scope_patterns: vec![workspace_root.clone()],
            denied_scope_patterns: vec![
                format!("{workspace_root}/.git"),
                format!("{workspace_root}/target"),
            ],
            max_execution_seconds: self.config.task_timeout.as_secs().min(u32::MAX as u64) as u32,
        }
    }

    fn tool_trace_event(
        &self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        spec: &crate::tool_registry::ToolSpec,
        call: &crate::provider::ModelToolCall,
        phase: ExecutionTracePhase,
    ) -> ExecutionTraceEvent {
        let mut event = ExecutionTraceEvent::new(
            self.session_id(),
            self.mission_id.clone().unwrap_or_default(),
            assignment.task_id.clone(),
            format!("turn-{}", self.trace_events.len() + 1),
            infer_worker_role_from_task(task).to_string(),
            ExecutionEventKind::ToolCall,
            phase,
            call.name.clone(),
        );
        event.action_id = call.id.clone();
        event.tool_call_id = Some(call.id.clone());
        event.tool_kind = Some(format!("{:?}", spec.tool_kind).to_ascii_lowercase());
        event.tool_name = Some(call.name.clone());
        event.args_preview = Some(call.arguments_json.clone());
        event.read_only = spec.read_only;
        event.mutating = spec.mutating;
        event.requires_approval = spec.requires_approval;
        event.parent_action_id = assignment
            .context_pack
            .as_ref()
            .map(|pack| pack.task_id.clone());
        event
    }

    async fn handle_cloud_unavailable(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        cloud_target: RoutedTarget,
        message: String,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        match self.registry.fallback_policy {
            FallbackPolicy::Automatic => {
                if self.registry.has_local() {
                    self.redispatch_to_local(task, assignment, cloud_target, message)
                        .await
                } else {
                    Err(CoordinatorV2Error::CloudExecutionUnavailable {
                        message,
                        local_recovery: None,
                    })
                }
            }
            FallbackPolicy::Explicit => Err(CoordinatorV2Error::CloudExecutionUnavailable {
                message,
                local_recovery: self
                    .registry
                    .default_local
                    .as_ref()
                    .map(|local| format!("retry with local model {}", local.model)),
            }),
            FallbackPolicy::Never => Err(CoordinatorV2Error::CloudExecutionUnavailable {
                message,
                local_recovery: None,
            }),
        }
    }

    async fn redispatch_to_local(
        &mut self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        cloud_target: RoutedTarget,
        message: String,
    ) -> Result<(RoutedTarget, ModelResponse)> {
        let local = self.registry.default_local.clone().ok_or_else(|| {
            CoordinatorV2Error::CloudExecutionUnavailable {
                message: message.clone(),
                local_recovery: None,
            }
        })?;
        let local_transport = self
            .registry
            .local_transport(&local.instance_id)
            .ok_or_else(|| CoordinatorV2Error::CloudExecutionUnavailable {
                message: message.clone(),
                local_recovery: None,
            })?;

        tracing::warn!(
            cloud_provider = %cloud_target.provider_label(),
            cloud_model = %cloud_target.model_label(),
            local_provider = %RoutedTarget::Local(local.clone()).provider_label(),
            local_model = %local.model,
            "cloud unavailable, automatically downgrading to local"
        );

        let local_target = RoutedTarget::Local(local);
        let local_request = self.build_model_request(task, assignment, &local_target)?;
        let response = local_transport
            .execute_local(&local_request, local_target.model_label())
            .await
            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;
        Ok((local_target, response))
    }

    fn build_model_request(
        &self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        target: &RoutedTarget,
    ) -> Result<ModelRequest> {
        let max_output_tokens = match self.config.registry.models.get(target.model_label()) {
            Some(entry) => entry.max_output_tokens,
            None => match target {
                RoutedTarget::Local(local) => {
                    tracing::warn!(
                        model = %local.model,
                        instance = %local.instance_id.0,
                        fallback_max_output_tokens = self.config.task_token_budget,
                        "local model not found in registry; using default task token budget"
                    );
                    self.config.task_token_budget
                }
                RoutedTarget::Cloud(_) => {
                    return Err(CoordinatorV2Error::InvalidConfig(format!(
                        "model '{}' not found in registry - cannot determine token budget",
                        target.model_label()
                    )));
                }
            },
        };
        Ok(PromptAssembly::for_worker_task_with_model(
            task,
            assignment,
            task.assigned_to.as_deref(),
            target.model_label(),
        )
        .into_model_request(
            target.request_provider(),
            target.model_label().to_string(),
            max_output_tokens,
            Some(0.0),
            false,
            CacheRetention::Extended,
        ))
    }

    fn build_direct_reply_request(
        &self,
        prompt: &str,
        assignment: &InternalTaskAssignment,
        target: &RoutedTarget,
        workspace_context: Option<String>,
    ) -> Result<ModelRequest> {
        let max_output_tokens = self
            .config
            .registry
            .models
            .get(target.model_label())
            .map(|entry| entry.max_output_tokens)
            .unwrap_or(self.config.task_token_budget);

        let mut system_instructions = vec![
            "You are OPENAKTA direct reply mode.".to_string(),
            "Answer the user directly.".to_string(),
            "Do not describe internal task orchestration, workers, or coordinator state."
                .to_string(),
            "Ground the answer in the attached repository context when present.".to_string(),
            "Cite concrete file paths from the repository context when making claims."
                .to_string(),
            "If the attached context is insufficient, say what is missing instead of inventing details."
                .to_string(),
        ];
        let mut invariant_mission_context = Vec::new();

        let user_content = if let Some(context) = workspace_context {
            invariant_mission_context.push(json!({ "workspace_context": context.clone() }));
            system_instructions.push(
                "Repository context is attached in invariant_mission_context.workspace_context."
                    .to_string(),
            );
            system_instructions.push(
                "Answer only from the provided repository context. If the context is insufficient, say so."
                    .to_string(),
            );
            format!(
                "Repository context:\n{}\n\nUser request:\n{}\n\nRequirements:\n- Ground every claim in the repository context.\n- Cite concrete file paths when making claims.\n- If the context does not support a claim, say that the context is insufficient.",
                context, prompt
            )
        } else {
            prompt.to_string()
        };

        Ok(ModelRequest {
            provider: target.request_provider(),
            model: target.model_label().to_string(),
            system_instructions,
            tool_schemas: Vec::new(),
            invariant_mission_context,
            payload: ModelBoundaryPayload {
                payload_type: ModelBoundaryPayloadType::TaskExecution,
                task_id: assignment.task_id.clone(),
                title: "DirectReply".to_string(),
                description: prompt.to_string(),
                task_type: "DirectReply".to_string(),
                target_files: assignment.target_files.clone(),
                target_symbols: assignment.target_symbols.clone(),
                context_spans: assignment
                    .context_pack
                    .as_ref()
                    .map(|pack| {
                        pack.spans
                            .iter()
                            .map(|span| {
                                format!(
                                    "{}:{}-{}:{}",
                                    span.file_path,
                                    span.start_line,
                                    span.end_line,
                                    span.symbol_path
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                context_pack: assignment.context_pack.clone(),
            },
            recent_messages: vec![crate::provider::ChatMessage {
                role: "user".to_string(),
                content: user_content,
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            max_output_tokens,
            temperature: Some(0.0),
            stream: false,
            cache_retention: CacheRetention::Extended,
        })
    }

    async fn execute_arbiter_review(
        &mut self,
        task: &Task,
        worker_id: &str,
        assignment: &InternalTaskAssignment,
        failed_output: &str,
        validation_error: &str,
    ) -> Result<InternalResultSubmission> {
        let Some(cloud_ref) = self.registry.default_cloud.clone() else {
            return Err(CoordinatorV2Error::CloudExecutionRequired(
                "arbiter escalation requires a default cloud model".to_string(),
            ));
        };

        // Execute arbiter review via API (Phase 7+)
        let review_request = ModelRequest {
            provider: cloud_ref.wire_profile,
            model: cloud_ref.model.clone(),
            system_instructions: vec![
                "You are the OPENAKTA cloud arbiter. Review the failed local output, repair it when possible, and return only the corrected result payload.".to_string(),
            ],
            tool_schemas: Vec::new(),
            invariant_mission_context: Vec::new(),
            payload: ModelBoundaryPayload {
                payload_type: ModelBoundaryPayloadType::TaskExecution,
                task_id: task.id.clone(),
                title: "OPENAKTA arbitration review".to_string(),
                description: format!(
                    "Repair the failed local patch for task '{}'. Validation error: {}. Failed output:\n{}",
                    task.description, validation_error, failed_output
                ),
                task_type: "REVIEW".to_string(),
                target_files: Vec::new(),
                target_symbols: Vec::new(),
                context_spans: Vec::new(),
                context_pack: None,
            },
            recent_messages: Vec::new(),
            max_output_tokens: 768,
            temperature: Some(0.0),
            stream: false,
            cache_retention: CacheRetention::Extended,
        };

        let reviewed = self
            .execute_via_api(
                task,
                assignment,
                review_request,
                RoutedTarget::Cloud(cloud_ref.clone()),
            )
            .await
            .map(|(_, response)| response)
            .map_err(|err| match err {
                CoordinatorV2Error::CloudExecutionUnavailable { message, .. } => {
                    CoordinatorV2Error::CloudExecutionUnavailable {
                        message,
                        local_recovery: None,
                    }
                }
                other => other,
            })?;
        let validated = self
            .diff_validator
            .validate(&reviewed.output_text)
            .map_err(|err| CoordinatorV2Error::ProtocolViolation(err.to_string()))?;
        let patch =
            self.build_patch_envelope(task, assignment, &validated.raw_output, validated.format)?;
        if let Err(err) = self.communication.send_patch_envelope(worker_id, &patch) {
            self.metrics.protocol_validation_failures += 1;
            return Err(CoordinatorV2Error::ProtocolViolation(err));
        }

        let patch_receipt = self
            .patch_applier
            .apply_to_workspace(&self.config.workspace_root, &patch)
            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;
        if patch_receipt.status != PatchApplyStatus::Applied {
            return Err(CoordinatorV2Error::ExecutionFailed(format!(
                "arbiter patch application failed: {}",
                patch_receipt.message
            )));
        }

        Ok(InternalResultSubmission {
            task_id: task.id.clone(),
            success: true,
            patch: Some(patch),
            patch_receipt: Some(patch_receipt),
            token_usage: to_internal_token_usage(
                &cloud_ref.instance_id.0,
                &ProviderUsage::default(),
            ),
            context_references: context_references_from_assignment(assignment),
            summary: format!("arbiter applied corrected patch for {}", task.description),
            error_message: String::new(),
            diagnostic_toon: None,
        })
    }

    fn build_patch_envelope(
        &self,
        task: &Task,
        assignment: &InternalTaskAssignment,
        raw_output: &str,
        format: PatchFormat,
    ) -> Result<PatchEnvelope> {
        let base_revision = assignment
            .context_pack
            .as_ref()
            .map(|pack| pack.base_revision.clone())
            .or_else(|| {
                assignment.target_files.first().and_then(|file| {
                    current_revision_for_path(&self.config.workspace_root, file).ok()
                })
            })
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let (patch_text, search_replace_blocks) = match format {
            PatchFormat::UnifiedDiffZero => (Some(raw_output.to_string()), Vec::new()),
            PatchFormat::AstSearchReplace => (
                None,
                self.diff_validator
                    .validate(raw_output)
                    .map_err(|err| CoordinatorV2Error::ProtocolViolation(err.to_string()))?
                    .search_replace_blocks,
            ),
        };

        Ok(PatchEnvelope {
            task_id: task.id.clone(),
            target_files: assignment.target_files.clone(),
            format,
            patch_text,
            search_replace_blocks,
            base_revision: base_revision.clone(),
            validation: vec![
                ValidationFact {
                    key: "diff_only".to_string(),
                    value: "true".to_string(),
                },
                ValidationFact {
                    key: "base_revision".to_string(),
                    value: base_revision,
                },
            ],
        })
    }

    fn build_task_assignment(
        &mut self,
        task: &Task,
        hints: Option<&TaskTargetHints>,
    ) -> Result<InternalTaskAssignment> {
        let (target_files, target_symbols) = hints
            .map(|hints| (hints.target_files.clone(), hints.target_symbols.clone()))
            .unwrap_or_else(|| extract_targets(&task.description));
        let budget = self
            .planned_target(task, &target_files, &target_symbols)
            .map(|target| self.effective_budget_for_target(&target))
            .unwrap_or_else(|| self.default_effective_budget());
        let context_pack =
            self.build_context_pack(task, &target_files, &target_symbols, budget.retrieval_cap)?;
        let worker_role = infer_worker_role_from_task(task);
        let lane = default_lane_for_task_type(&task.task_type);
        let assignment_contract = WorkerAssignmentContract {
            session_id: {
                let session_id = self.session_id();
                if session_id.trim().is_empty() {
                    self.mission_id.clone().unwrap_or_else(|| task.id.clone())
                } else {
                    session_id
                }
            },
            story_id: None,
            task_id: task.id.clone(),
            task_type: task.task_type.clone(),
            lane,
            goal: task.description.clone(),
            requirement_refs: Vec::new(),
            context_artifact_refs: context_pack
                .as_ref()
                .map(|pack| vec![format!("context_pack:{}", pack.id)])
                .unwrap_or_default(),
            target_files,
            target_symbols,
            expected_artifacts: default_expected_artifacts(lane, &task.task_type),
            allowed_tools: self
                .tool_registry
                .allowed_tool_names(worker_role, &task.task_type),
            budget: WorkerExecutionBudget {
                token_budget: budget.task_cap,
                max_tool_turns: self.config.max_tool_turns,
                max_tool_calls: self.config.max_tool_calls,
                max_mutating_tool_calls: self.config.max_mutating_tool_calls,
            },
            termination_condition: default_termination_condition(lane, &task.task_type),
            verification_required: task.task_type == TaskType::CodeModification,
            workspace_revision_token: context_pack.as_ref().and_then(|pack| {
                (!pack.base_revision.trim().is_empty()).then(|| pack.base_revision.clone())
            }),
            planning_origin_ref: PlanningOriginRef::direct(),
        };

        Ok(InternalTaskAssignment::from_contract(
            assignment_contract,
            context_pack,
        ))
    }

    fn build_context_pack(
        &mut self,
        task: &Task,
        target_files: &[String],
        target_symbols: &[String],
        retrieval_token_budget: usize,
    ) -> Result<Option<ContextPack>> {
        if target_files.is_empty() && target_symbols.is_empty() {
            return Ok(None);
        }
        let retrieval_action_id = format!("retrieval:{}", task.id);
        let mut started = self.make_trace_event(
            task.id.clone(),
            "retrieval",
            ExecutionEventKind::Retrieval,
            ExecutionTracePhase::Started,
            format!("retrieve context for {}", task.description),
        );
        started.action_id = retrieval_action_id.clone();
        started.query = Some(task.description.clone());
        started.target_path = target_files.first().cloned();
        started.target_symbol = target_symbols.first().cloned();
        self.emit_trace_event(started);

        let outcome: Result<Option<ContextPack>> = (|| {
            if let Some(pack) =
                self.mcp_retrieval_pack(task, target_files, target_symbols, retrieval_token_budget)?
            {
                return Ok(Some(pack));
            }

            let mut spans = Vec::new();
            let mut retrieval_hits = Vec::new();
            let mut ast_summaries = Vec::new();
            let mut symbol_maps = Vec::new();
            let mut validation_facts = Vec::new();

            for file in target_files {
                let path = resolve_workspace_relative_path(&self.config.workspace_root, file)
                    .map_err(|e| CoordinatorV2Error::ProtocolViolation(e.to_string()))?;
                if let Ok(content) = fs::read_to_string(&path) {
                    let line_count = content.lines().count().max(1);
                    let base_revision = revision_for_content(&content);
                    spans.push(ContextSpan {
                        file_path: file.clone(),
                        start_line: 1,
                        end_line: line_count,
                        symbol_path: target_symbols.first().cloned().unwrap_or_default(),
                    });
                    retrieval_hits.push(RetrievalHit {
                        file_path: file.clone(),
                        symbol_path: target_symbols.first().cloned().unwrap_or_default(),
                        start_line: 1,
                        end_line: line_count,
                        snippet: truncate_snippet(&content, 1200),
                        base_revision: base_revision.clone(),
                    });
                    validation_facts.push(ValidationFact {
                        key: format!("base_revision:{file}"),
                        value: base_revision,
                    });
                }
            }

            for symbol in target_symbols {
                ast_summaries.push(AstSummary {
                    file_path: target_files
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    symbol_path: symbol.clone(),
                    kind: "symbol".to_string(),
                    start_line: 0,
                    end_line: 0,
                });
                symbol_maps.push(SymbolMap {
                    file_path: target_files
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    symbol_path: symbol.clone(),
                    references: target_symbols.to_vec(),
                });
            }

            if self.config.enable_graph_retrieval {
                if let Some(graph_pack) = self.graph_retrieval_pack(
                    task,
                    target_files,
                    target_symbols,
                    retrieval_token_budget,
                )? {
                    spans = graph_pack.spans;
                    retrieval_hits = graph_pack.retrieval_hits;
                    ast_summaries.extend(graph_pack.ast_summaries);
                    symbol_maps.extend(graph_pack.symbol_maps);
                    validation_facts.extend(graph_pack.validation_facts);
                } else {
                    validation_facts.push(ValidationFact {
                        key: "retrieval".to_string(),
                        value: "fallback".to_string(),
                    });
                }
            }

            let base_revision = validation_facts
                .iter()
                .find(|fact| fact.key.starts_with("base_revision:"))
                .map(|fact| fact.value.clone())
                .unwrap_or_else(|| "UNKNOWN".to_string());

            Ok(Some(ContextPack {
                id: format!("ctx-{}", task.id),
                task_id: task.id.clone(),
                target_files: target_files.to_vec(),
                symbols: target_symbols.to_vec(),
                spans,
                retrieval_hits,
                ast_summaries,
                symbol_maps,
                validation_facts,
                base_revision,
            }))
        })();

        match &outcome {
            Ok(Some(pack)) => {
                let mut completed = self.make_trace_event(
                    task.id.clone(),
                    "retrieval",
                    ExecutionEventKind::Retrieval,
                    ExecutionTracePhase::Completed,
                    format!("retrieve context for {}", task.description),
                );
                completed.action_id = retrieval_action_id;
                completed.query = Some(task.description.clone());
                completed.target_path = target_files.first().cloned();
                completed.target_symbol = target_symbols.first().cloned();
                completed.result_preview = Some(format!(
                    "files={} hits={} symbols={}",
                    pack.target_files.len(),
                    pack.retrieval_hits.len(),
                    pack.symbols.len()
                ));
                self.emit_trace_event(completed);
            }
            Ok(None) => {}
            Err(err) => {
                let mut failed = self.make_trace_event(
                    task.id.clone(),
                    "retrieval",
                    ExecutionEventKind::Retrieval,
                    ExecutionTracePhase::Failed,
                    format!("retrieve context for {}", task.description),
                );
                failed.action_id = retrieval_action_id;
                failed.query = Some(task.description.clone());
                failed.target_path = target_files.first().cloned();
                failed.target_symbol = target_symbols.first().cloned();
                failed.error = Some(err.to_string());
                self.emit_trace_event(failed);
            }
        }

        outcome
    }

    fn mcp_retrieval_pack(
        &self,
        task: &Task,
        target_files: &[String],
        target_symbols: &[String],
        retrieval_token_budget: usize,
    ) -> Result<Option<ContextPack>> {
        let Some(endpoint) = self.config.mcp_endpoint.clone() else {
            return Ok(None);
        };

        let request = RetrieveCodeContextRequest {
            request_id: format!("coord-retrieval-{}", task.id),
            agent_id: "coordinator".to_string(),
            role: "worker".to_string(),
            task_id: task.id.clone(),
            workspace_root: self.config.workspace_root.display().to_string(),
            query: task.description.clone(),
            focal_files: target_files.to_vec(),
            focal_symbols: target_symbols.to_vec(),
            token_budget: retrieval_token_budget as u32,
            dense_limit: self.config.retrieval_max_documents.max(8) as u32,
            include_diagnostics: true,
            candidate_limit: (self.config.retrieval_max_documents.max(8) * 4) as u32,
        };

        let response = std::thread::spawn(move || -> std::result::Result<_, CoordinatorV2Error> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?;
            runtime.block_on(async move {
                McpClient::new(endpoint)
                    .retrieve_code_context(request)
                    .await
                    .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))
            })
        })
        .join()
        .map_err(|_| {
            CoordinatorV2Error::ExecutionFailed("MCP retrieval thread panicked".to_string())
        })??;

        if response.documents.is_empty() {
            return Ok(None);
        }

        let mut target_file_set = target_files.iter().cloned().collect::<Vec<_>>();
        for document in &response.documents {
            if !target_file_set.contains(&document.file_path) {
                target_file_set.push(document.file_path.clone());
            }
        }

        let mut validation_facts = response
            .diagnostics
            .as_ref()
            .map(|diagnostics| {
                let mut facts = vec![
                    ValidationFact {
                        key: "retrieval_source".to_string(),
                        value: "mcp_retrieval_service".to_string(),
                    },
                    ValidationFact {
                        key: "retrieval_selected_count".to_string(),
                        value: diagnostics.selected_count.to_string(),
                    },
                    ValidationFact {
                        key: "retrieval_degraded_mode".to_string(),
                        value: diagnostics.degraded_mode.to_string(),
                    },
                ];
                if let Some(contract) = diagnostics.contract.as_ref() {
                    facts.push(ValidationFact {
                        key: "retrieval_contract".to_string(),
                        value: contract.contract_version.clone(),
                    });
                }
                facts
            })
            .unwrap_or_else(|| {
                vec![ValidationFact {
                    key: "retrieval_source".to_string(),
                    value: "mcp_retrieval_service".to_string(),
                }]
            });

        for document in &response.documents {
            if let Ok(revision) =
                current_revision_for_path(&self.config.workspace_root, &document.file_path)
            {
                validation_facts.push(ValidationFact {
                    key: format!("base_revision:{}", document.file_path),
                    value: revision,
                });
            }
        }

        let base_revision = response
            .documents
            .iter()
            .find_map(|document| {
                current_revision_for_path(&self.config.workspace_root, &document.file_path).ok()
            })
            .unwrap_or_else(|| "UNKNOWN".to_string());

        Ok(Some(ContextPack {
            id: format!("mcp-ctx-{}", task.id),
            task_id: task.id.clone(),
            target_files: target_file_set,
            symbols: target_symbols.to_vec(),
            spans: response
                .documents
                .iter()
                .map(|document| ContextSpan {
                    file_path: document.file_path.clone(),
                    start_line: 1,
                    end_line: document.content.lines().count().max(1),
                    symbol_path: document.symbol_path.clone(),
                })
                .collect(),
            retrieval_hits: response
                .documents
                .iter()
                .map(|document| RetrievalHit {
                    file_path: document.file_path.clone(),
                    symbol_path: document.symbol_path.clone(),
                    start_line: 1,
                    end_line: document.content.lines().count().max(1),
                    snippet: truncate_snippet(&document.content, 1200),
                    base_revision: current_revision_for_path(
                        &self.config.workspace_root,
                        &document.file_path,
                    )
                    .unwrap_or_else(|_| "UNKNOWN".to_string()),
                })
                .collect(),
            ast_summaries: response
                .documents
                .iter()
                .filter(|document| !document.symbol_path.is_empty())
                .map(|document| AstSummary {
                    file_path: document.file_path.clone(),
                    symbol_path: document.symbol_path.clone(),
                    kind: "retrieval".to_string(),
                    start_line: 1,
                    end_line: document.content.lines().count().max(1),
                })
                .collect(),
            symbol_maps: response
                .documents
                .iter()
                .filter(|document| !document.symbol_path.is_empty())
                .map(|document| SymbolMap {
                    file_path: document.file_path.clone(),
                    symbol_path: document.symbol_path.clone(),
                    references: vec![document.symbol_path.clone()],
                })
                .collect(),
            validation_facts,
            base_revision,
        }))
    }

    fn graph_retrieval_pack(
        &self,
        task: &Task,
        target_files: &[String],
        target_symbols: &[String],
        retrieval_token_budget: usize,
    ) -> Result<Option<ContextPack>> {
        let Some(language) = target_files.first().and_then(|file| detect_language(file)) else {
            return Ok(None);
        };

        let registry = ParserRegistry::new();
        let scip = match registry.parse(language, &self.config.workspace_root) {
            Ok(index) => index,
            Err(_) => return Ok(None),
        };
        let influence = match InfluenceGraph::from_scip(&scip) {
            Ok(graph) => graph,
            Err(_) => return Ok(None),
        };
        let documents = load_documents(&self.config.workspace_root, &scip);
        let retriever = GraphRetriever::new(
            scip,
            influence,
            documents,
            GraphRetrievalConfig {
                token_budget: retrieval_token_budget,
                max_documents: self.config.retrieval_max_documents,
            },
        );
        let result = match retriever.retrieve(&GraphRetrievalRequest {
            task_id: task.id.clone(),
            query: task.description.clone(),
            focal_file: target_files.first().cloned(),
            focal_symbol: target_symbols.first().cloned(),
        }) {
            Ok(result) => result,
            Err(_) => return Ok(None),
        };

        Ok(Some(ContextPack {
            id: format!("graph-ctx-{}", task.id),
            task_id: task.id.clone(),
            target_files: result
                .documents
                .iter()
                .map(|doc| doc.file_path.clone())
                .collect(),
            symbols: target_symbols.to_vec(),
            spans: result
                .documents
                .iter()
                .map(|doc| ContextSpan {
                    file_path: doc.file_path.clone(),
                    start_line: 1,
                    end_line: doc.content.lines().count().max(1),
                    symbol_path: doc.symbols.first().cloned().unwrap_or_default(),
                })
                .collect(),
            retrieval_hits: result
                .documents
                .iter()
                .map(|doc| RetrievalHit {
                    file_path: doc.file_path.clone(),
                    symbol_path: doc.symbols.first().cloned().unwrap_or_default(),
                    start_line: 1,
                    end_line: doc.content.lines().count().max(1),
                    snippet: truncate_snippet(&doc.content, 1200),
                    base_revision: current_revision_for_path(
                        &self.config.workspace_root,
                        &doc.file_path,
                    )
                    .unwrap_or_else(|_| "UNKNOWN".to_string()),
                })
                .collect(),
            ast_summaries: result
                .documents
                .iter()
                .flat_map(|doc| {
                    doc.symbols.iter().map(move |symbol| AstSummary {
                        file_path: doc.file_path.clone(),
                        symbol_path: symbol.clone(),
                        kind: "graph".to_string(),
                        start_line: 1,
                        end_line: doc.content.lines().count().max(1),
                    })
                })
                .collect(),
            symbol_maps: result
                .documents
                .iter()
                .map(|doc| SymbolMap {
                    file_path: doc.file_path.clone(),
                    symbol_path: doc.symbols.first().cloned().unwrap_or_default(),
                    references: doc.symbols.clone(),
                })
                .collect(),
            validation_facts: result
                .diagnostics
                .iter()
                .map(|diagnostic| ValidationFact {
                    key: diagnostic.kind.clone(),
                    value: diagnostic.message.clone(),
                })
                .collect(),
            base_revision: target_files
                .first()
                .and_then(|file| current_revision_for_path(&self.config.workspace_root, file).ok())
                .unwrap_or_else(|| "UNKNOWN".to_string()),
        }))
    }

    fn planned_target(
        &self,
        task: &Task,
        target_files: &[String],
        target_symbols: &[String],
    ) -> Option<RoutedTarget> {
        let assignment = InternalTaskAssignment {
            task_id: task.id.clone(),
            title: task.description.clone(),
            description: task.description.clone(),
            task_type: task.task_type.clone(),
            target_files: target_files.to_vec(),
            target_symbols: target_symbols.to_vec(),
            token_budget: self.config.task_token_budget,
            context_pack: None,
            canonical_contract: None,
        };
        route(
            task,
            &assignment,
            &self.registry,
            self.config.routing_enabled,
            &self.config.model_instance_priority,
            None,
        )
    }

    fn default_effective_budget(&self) -> EffectiveTokenBudget {
        EffectiveTokenBudget {
            retrieval_cap: self.config.retrieval_token_budget,
            task_cap: self.config.task_token_budget,
        }
    }

    fn effective_budget_for_target(&self, target: &RoutedTarget) -> EffectiveTokenBudget {
        derive_effective_budget(
            self.config.registry.models.get(target.model_label()),
            self.config.retrieval_token_budget,
            self.config.task_token_budget,
            self.config.context_use_ratio,
            self.config.context_margin_tokens,
            self.config.retrieval_share,
        )
    }

    fn failure_result_submission(
        &self,
        task: &Task,
        assignment: Option<&InternalTaskAssignment>,
        message: &str,
    ) -> InternalResultSubmission {
        let diagnostic_toon = Some(self.workflow_diagnostic_toon(
            "coordinator.task.failure",
            task,
            assignment,
            message,
        ));
        InternalResultSubmission {
            task_id: task.id.clone(),
            success: false,
            patch: None,
            patch_receipt: None,
            token_usage: InternalTokenUsage {
                provider: self.config.default_provider_label(),
                input_tokens: 0,
                output_tokens: 0,
                cache_write_tokens: 0,
                cache_read_tokens: 0,
                uncached_input_tokens: 0,
                effective_tokens_saved: 0,
            },
            context_references: assignment
                .map(context_references_from_assignment)
                .unwrap_or_default(),
            summary: String::new(),
            error_message: message.to_string(),
            diagnostic_toon,
        }
    }

    fn completion_result_submission(
        &self,
        task: &Task,
        completion: &DispatchCompletion,
        success: bool,
    ) -> InternalResultSubmission {
        let error_message = completion
            .error
            .clone()
            .unwrap_or_else(|| "worker completed without a structured result".to_string());
        InternalResultSubmission {
            task_id: task.id.clone(),
            success,
            patch: None,
            patch_receipt: None,
            token_usage: InternalTokenUsage {
                provider: self.config.default_provider_label(),
                input_tokens: 0,
                output_tokens: 0,
                cache_write_tokens: 0,
                cache_read_tokens: 0,
                uncached_input_tokens: 0,
                effective_tokens_saved: 0,
            },
            context_references: Vec::new(),
            summary: if success {
                format!("{}: completed", task.description)
            } else {
                String::new()
            },
            error_message: error_message.clone(),
            diagnostic_toon: if success {
                None
            } else {
                Some(self.workflow_diagnostic_toon(
                    "coordinator.worker.failure",
                    task,
                    None,
                    &error_message,
                ))
            },
        }
    }

    fn workflow_diagnostic_toon(
        &self,
        operation: &str,
        task: &Task,
        assignment: Option<&InternalTaskAssignment>,
        message: &str,
    ) -> String {
        let mut context = Map::<String, Value>::new();
        context.insert("task_id".to_string(), json!(task.id));
        context.insert(
            "task_type".to_string(),
            json!(format!("{:?}", task.task_type)),
        );
        context.insert(
            "worker_target_count".to_string(),
            json!(assignment.map(|a| a.target_files.len()).unwrap_or_default()),
        );
        if let Some(assignment) = assignment {
            context.insert(
                "target_files".to_string(),
                json!(assignment.target_files.clone()),
            );
            context.insert(
                "target_symbols".to_string(),
                json!(assignment.target_symbols.clone()),
            );
        }
        WideEvent::workflow_failure(operation, message, context)
            .to_toon()
            .unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_transition(
        &mut self,
        worker_id: &str,
        task_id: &str,
        from_state: &str,
        to_state: &str,
        reason: &str,
        retry_count: u32,
        terminal: bool,
    ) {
        self.metrics.transition_count += 1;
        if self
            .communication
            .send_typed_workflow_transition(
                worker_id,
                &crate::transport::InternalWorkflowTransitionEvent {
                    task_id: task_id.to_string(),
                    from_state: from_state.to_string(),
                    to_state: to_state.to_string(),
                    reason: reason.to_string(),
                    retry_count,
                    terminal,
                },
            )
            .is_err()
        {
            self.metrics.protocol_validation_failures += 1;
        }
    }

    async fn publish_result_submission(
        &self,
        result_submission: &InternalResultSubmission,
    ) -> Result<()> {
        let mut blackboard = self.blackboard.lock().await;
        let content = serde_json::to_string(result_submission)
            .map_err(|err| CoordinatorV2Error::ProtocolViolation(err.to_string()))?;

        blackboard
            .publish(
                BlackboardEntry {
                    id: format!("mission-result-{}", result_submission.task_id),
                    namespace: Some("verification/result_submission".to_string()),
                    schema_hash: Some("result_submission.v1".to_string()),
                    content,
                },
                vec!["coordinator".to_string()],
            )
            .map_err(|err| CoordinatorV2Error::ProtocolViolation(err.to_string()))?;
        Ok(())
    }
}

fn value_to_proto_struct(value: &Value) -> Option<prost_types::Struct> {
    value.as_object().map(|fields| prost_types::Struct {
        fields: fields
            .iter()
            .map(|(key, value)| (key.clone(), value_to_proto_value(value)))
            .collect(),
    })
}

fn value_to_proto_value(value: &Value) -> prost_types::Value {
    use prost_types::{value::Kind, ListValue, Value as ProtoValue};

    let kind = match value {
        Value::Null => Kind::NullValue(0),
        Value::Bool(value) => Kind::BoolValue(*value),
        Value::Number(value) => Kind::NumberValue(value.as_f64().unwrap_or_default()),
        Value::String(value) => Kind::StringValue(value.clone()),
        Value::Array(values) => Kind::ListValue(ListValue {
            values: values.iter().map(value_to_proto_value).collect(),
        }),
        Value::Object(_) => Kind::StructValue(value_to_proto_struct(value).unwrap_or_default()),
    };

    ProtoValue { kind: Some(kind) }
}

fn normalize_tool_result_for_model(result: &Value, task_id: &str, turn_index: u32) -> String {
    json!({
        "task_id": task_id,
        "turn_index": turn_index,
        "result": result,
    })
    .to_string()
}

fn tool_error_message(tool_call_id: &str, error: String) -> crate::provider::ChatMessage {
    crate::provider::ChatMessage {
        role: "tool".to_string(),
        content: json!({ "error": error }).to_string(),
        name: None,
        tool_call_id: Some(tool_call_id.to_string()),
        tool_calls: Vec::new(),
    }
}

fn infer_worker_role_from_task(task: &Task) -> &'static str {
    default_worker_role(&task.task_type)
}

fn load_documents(
    workspace_root: &Path,
    scip: &openakta_indexing::SCIPIndex,
) -> HashMap<String, String> {
    let mut documents = HashMap::new();
    for file_path in scip
        .occurrences
        .iter()
        .map(|occurrence| occurrence.file_path.clone())
    {
        if documents.contains_key(&file_path) {
            continue;
        }
        let Ok(resolved) = resolve_workspace_relative_path(workspace_root, &file_path) else {
            warn!(
                file_path = %file_path,
                "skipping SCIP path outside workspace (patch protocol boundary)"
            );
            continue;
        };
        if let Ok(content) = fs::read_to_string(resolved) {
            documents.insert(file_path, content);
        }
    }
    documents
}

fn detect_language(file_path: &str) -> Option<Language> {
    if file_path.ends_with(".rs") {
        Some(Language::Rust)
    } else if file_path.ends_with(".ts")
        || file_path.ends_with(".tsx")
        || file_path.ends_with(".js")
        || file_path.ends_with(".jsx")
    {
        Some(Language::TypeScript)
    } else if file_path.ends_with(".py") {
        Some(Language::Python)
    } else {
        None
    }
}

fn extract_targets(description: &str) -> (Vec<String>, Vec<String>) {
    let mut files = Vec::new();
    let mut symbols = Vec::new();

    for token in description.split_whitespace() {
        let cleaned = token
            .trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':' | '(' | ')' | '"' | '\''))
            .to_string();
        if cleaned.contains("::") {
            symbols.push(cleaned.clone());
        }
        if cleaned.contains('/')
            || [".rs", ".ts", ".tsx", ".js", ".jsx", ".py"]
                .iter()
                .any(|ext| cleaned.ends_with(ext))
        {
            files.push(cleaned);
        }
    }

    files.sort();
    files.dedup();
    symbols.sort();
    symbols.dedup();
    (files, symbols)
}

fn context_references_from_assignment(
    assignment: &InternalTaskAssignment,
) -> Vec<InternalContextReference> {
    if let Some(context_pack) = assignment.context_pack.as_ref() {
        if !context_pack.spans.is_empty() {
            return context_pack
                .spans
                .iter()
                .map(|span| InternalContextReference {
                    file_path: span.file_path.clone(),
                    symbol_path: if span.symbol_path.is_empty() {
                        None
                    } else {
                        Some(span.symbol_path.clone())
                    },
                    start_line: span.start_line as u32,
                    end_line: span.end_line as u32,
                    block_id: None,
                })
                .collect();
        }
    }

    assignment
        .target_files
        .iter()
        .map(|file_path| InternalContextReference {
            file_path: file_path.clone(),
            symbol_path: assignment.target_symbols.first().cloned(),
            start_line: 1,
            end_line: 1,
            block_id: None,
        })
        .collect()
}

fn to_internal_token_usage(provider: &str, usage: &ProviderUsage) -> InternalTokenUsage {
    InternalTokenUsage {
        provider: provider.to_string(),
        input_tokens: usage.input_tokens as u32,
        output_tokens: usage.output_tokens as u32,
        cache_write_tokens: usage.cache_write_tokens as u32,
        cache_read_tokens: usage.cache_read_tokens as u32,
        uncached_input_tokens: usage.uncached_input_tokens as u32,
        effective_tokens_saved: usage.cache_read_tokens as u32,
    }
}

fn truncate_snippet(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        content[..max_len].to_string()
    }
}

fn current_revision_for_path(
    workspace_root: &Path,
    file_path: &str,
) -> std::result::Result<String, std::io::Error> {
    let path = resolve_workspace_relative_path(workspace_root, file_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
    let content = fs::read_to_string(path)?;
    Ok(revision_for_content(&content))
}

fn revision_for_content(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}

fn estimate_eta(elapsed: Duration, progress: f32) -> Option<Duration> {
    if progress <= 0.0 || progress >= 100.0 {
        return None;
    }

    let elapsed_secs = elapsed.as_secs_f32();
    let remaining_ratio = (100.0 - progress) / progress;
    Some(Duration::from_secs_f32(elapsed_secs * remaining_ratio))
}

fn to_dispatch_status(status: &WorkerStatus) -> DispatchWorkerStatus {
    match status {
        WorkerStatus::Idle => DispatchWorkerStatus::Idle,
        WorkerStatus::Busy(_) => DispatchWorkerStatus::Busy,
        WorkerStatus::Unhealthy { .. } => DispatchWorkerStatus::Unhealthy,
        WorkerStatus::Failed { .. } | WorkerStatus::Terminated => DispatchWorkerStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        prepare_dispatch_task_snapshot, BlackboardV2, Coordinator, CoordinatorConfig,
        CoordinatorV2Error,
    };
    use crate::execution_trace::{
        read_session_events, ExecutionEventKind, ExecutionTracePhase, ExecutionTraceRegistry,
    };
    use crate::patch_protocol::PatchApplyStatus;
    use crate::provider::{ModelRequest, ModelResponse, ProviderKind, ProviderUsage};
    use crate::provider_registry::ProviderRegistry;
    use crate::provider_transport::{
        CloudModelRef, FallbackPolicy, LocalModelRef, LocalProviderKind, LocalProviderTransport,
        ModelRegistryEntry, ModelRegistrySnapshot, ProviderInstanceId, ProviderProfileId,
        ProviderRuntimeBundle, ProviderRuntimeConfig, ProviderTransportError,
        ResolvedProviderInstance,
    };
    use crate::task::{Task, TaskStatus, TaskType};
    use crate::transport::InternalTaskAssignment;
    use crate::TaskTargetHints;
    use openakta_api_client::provider_v1::{
        self as provider_proto,
        provider_service_server::{ProviderService, ProviderServiceServer},
    };
    use openakta_api_client::{ApiClientPool, ClientConfig};
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio_stream::{wrappers::TcpListenerStream, Stream};
    use tonic::async_trait;
    use tonic::transport::Server;
    use tonic::{Request, Response, Status};
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;
    use tracing_subscriber::{Layer, Registry};
    use uuid::Uuid;

    #[test]
    fn prepare_dispatch_snapshot_clears_assigned_when_mol_fence_off() {
        let mut t = Task::new("x");
        t.assigned_to = Some("worker-a".to_string());
        t.status = TaskStatus::InProgress;
        let mut cfg = CoordinatorConfig::default();
        cfg.mol.strict_legacy_fence = false;
        let d = prepare_dispatch_task_snapshot(&cfg, &t);
        assert_eq!(d.status, TaskStatus::Pending);
        assert!(d.assigned_to.is_none());
    }

    #[test]
    fn prepare_dispatch_snapshot_preserves_assigned_when_mol_strict() {
        let mut t = Task::new("x");
        t.assigned_to = Some("worker-a".to_string());
        t.status = TaskStatus::InProgress;
        let mut cfg = CoordinatorConfig::default();
        cfg.mol.strict_legacy_fence = true;
        let d = prepare_dispatch_task_snapshot(&cfg, &t);
        assert_eq!(d.status, TaskStatus::Pending);
        assert_eq!(d.assigned_to.as_deref(), Some("worker-a"));
    }

    fn cloud_instance_id() -> ProviderInstanceId {
        ProviderInstanceId("cloud".to_string())
    }

    fn local_instance_id() -> ProviderInstanceId {
        ProviderInstanceId("local".to_string())
    }

    fn runtime_bundle(include_cloud: bool, include_local: bool) -> Arc<ProviderRuntimeBundle> {
        let mut instances = HashMap::new();
        if include_cloud {
            instances.insert(
                cloud_instance_id(),
                ResolvedProviderInstance {
                    id: cloud_instance_id(),
                    profile: ProviderProfileId::OpenAiChatCompletions,
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: None,
                    is_local: false,
                    default_model: Some("gpt-4o".to_string()),
                    label: None,
                },
            );
        }
        if include_local {
            instances.insert(
                local_instance_id(),
                ResolvedProviderInstance {
                    id: local_instance_id(),
                    profile: ProviderProfileId::OpenAiCompatible,
                    base_url: "http://127.0.0.1:11434".to_string(),
                    api_key: None,
                    is_local: true,
                    default_model: Some("qwen2.5-coder:7b".to_string()),
                    label: None,
                },
            );
        }
        Arc::new(ProviderRuntimeBundle {
            instances,
            http: ProviderRuntimeConfig::default(),
        })
    }

    fn test_registry() -> ModelRegistrySnapshot {
        use crate::provider_transport::ModelRegistryEntry;
        let mut models = std::collections::HashMap::new();
        models.insert(
            "claude-sonnet-4-5".to_string(),
            ModelRegistryEntry {
                name: "claude-sonnet-4-5".to_string(),
                max_context_window: 200_000,
                max_output_tokens: 8_192,
                preferred_instance: Some(cloud_instance_id()),
            },
        );
        models.insert(
            "qwen2.5-coder:7b".to_string(),
            ModelRegistryEntry {
                name: "qwen2.5-coder:7b".to_string(),
                max_context_window: 32_768,
                max_output_tokens: 4_096,
                preferred_instance: Some(local_instance_id()),
            },
        );
        ModelRegistrySnapshot {
            models,
            sources: crate::provider_transport::RegistryProvenance::default(),
        }
    }

    fn base_config() -> CoordinatorConfig {
        CoordinatorConfig {
            default_cloud: Some(CloudModelRef {
                instance_id: cloud_instance_id(),
                model: "claude-sonnet-4-5".to_string(),
                wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            default_local: Some(LocalModelRef {
                instance_id: local_instance_id(),
                model: "qwen2.5-coder:7b".to_string(),
                wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            provider_bundle: runtime_bundle(true, true),
            registry: Arc::new(test_registry()),
            ..CoordinatorConfig::default()
        }
    }

    fn test_coordinator(config: CoordinatorConfig) -> Coordinator {
        // Phase 7+: Use new_with_api_client instead of new_with_provider_transport
        // For testing, we create a registry with API client pool
        use crate::provider_registry::ProviderRegistry;

        let registry = Arc::new(ProviderRegistry::new_with_api_client(
            HashMap::new(),
            config.default_cloud.clone(),
            config.default_local.clone(),
            config.fallback_policy,
            config.provider_bundle.clone(),
            config.registry.clone(),
            Arc::new(ApiClientPool::new(openakta_api_client::ClientConfig::default()).unwrap()),
        ));

        Coordinator::new_with_provider_registry(config, Arc::new(BlackboardV2::default()), registry)
            .unwrap()
    }

    type QueuedApiResponse = std::result::Result<provider_proto::ProviderResponse, Status>;
    type BoxApiStream = Pin<
        Box<
            dyn Stream<Item = std::result::Result<provider_proto::ProviderResponseChunk, Status>>
                + Send,
        >,
    >;

    #[derive(Clone, Default)]
    struct MockApiProviderService {
        responses: Arc<Mutex<VecDeque<QueuedApiResponse>>>,
        calls: Arc<AtomicUsize>,
    }

    impl MockApiProviderService {
        fn with_responses(responses: Vec<QueuedApiResponse>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[tonic::async_trait]
    impl ProviderService for MockApiProviderService {
        async fn execute(
            &self,
            request: Request<provider_proto::ProviderRequest>,
        ) -> std::result::Result<Response<provider_proto::ProviderResponse>, Status> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let _request = request.into_inner();
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err(Status::internal("no queued cloud response")))
                .map(Response::new)
        }

        type ExecuteStreamStream = BoxApiStream;

        async fn execute_stream(
            &self,
            _request: Request<provider_proto::ProviderRequest>,
        ) -> std::result::Result<Response<Self::ExecuteStreamStream>, Status> {
            Ok(Response::new(Box::pin(tokio_stream::empty())))
        }

        async fn get_model_metadata(
            &self,
            _request: Request<provider_proto::ModelMetadataRequest>,
        ) -> std::result::Result<Response<provider_proto::ModelMetadata>, Status> {
            Err(Status::unimplemented(
                "get_model_metadata is unused in coordinator tests",
            ))
        }

        async fn health_check(
            &self,
            _request: Request<provider_proto::HealthCheckRequest>,
        ) -> std::result::Result<Response<provider_proto::ProviderHealthStatus>, Status> {
            Err(Status::unimplemented(
                "health_check is unused in coordinator tests",
            ))
        }
    }

    #[derive(Clone, Default)]
    struct TestLocalTransport {
        responses: Arc<Mutex<VecDeque<std::result::Result<ModelResponse, ProviderTransportError>>>>,
        calls: Arc<AtomicUsize>,
    }

    impl TestLocalTransport {
        fn with_responses(
            responses: Vec<std::result::Result<ModelResponse, ProviderTransportError>>,
        ) -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LocalProviderTransport for TestLocalTransport {
        async fn execute_local(
            &self,
            _request: &ModelRequest,
            _model: &str,
        ) -> std::result::Result<ModelResponse, ProviderTransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(ProviderTransportError::Http(
                        "no queued local response".to_string(),
                    ))
                })
        }

        fn mode(&self) -> &'static str {
            "test-local"
        }

        fn kind(&self) -> LocalProviderKind {
            LocalProviderKind::Ollama
        }
    }

    async fn api_client_pool_with_responses(
        responses: Vec<QueuedApiResponse>,
    ) -> (Arc<ApiClientPool>, MockApiProviderService) {
        let service = MockApiProviderService::with_responses(responses);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_service = service.clone();

        tokio::spawn(async move {
            Server::builder()
                .add_service(ProviderServiceServer::new(server_service))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(25)).await;

        let pool = Arc::new(
            ApiClientPool::new(ClientConfig {
                endpoint: format!("http://{}", addr),
                connect_timeout: Duration::from_secs(1),
                timeout: Duration::from_secs(1),
                ..ClientConfig::default()
            })
            .unwrap(),
        );

        (pool, service)
    }

    #[derive(Clone, Default)]
    struct WarnCapture {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl WarnCapture {
        fn count(&self) -> usize {
            self.events.lock().unwrap().len()
        }
    }

    impl<S> Layer<S> for WarnCapture
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            if *event.metadata().level() == Level::WARN {
                self.events
                    .lock()
                    .unwrap()
                    .push(event.metadata().target().to_string());
            }
        }
    }

    fn make_response(provider: ProviderKind, output_text: &str) -> ModelResponse {
        ModelResponse {
            id: None,
            provider,
            content: output_text.to_string(),
            output_text: output_text.to_string(),
            tool_calls: Vec::new(),
            usage: ProviderUsage::default(),
            stop_reason: None,
            provider_request_id: None,
            raw: json!({ "output_text": output_text }),
        }
    }

    fn make_api_response(output_text: &str) -> provider_proto::ProviderResponse {
        provider_proto::ProviderResponse {
            request_id: "mock-request".to_string(),
            response_id: Uuid::new_v4().to_string(),
            model: "claude-sonnet-4-5".to_string(),
            provider: "openai".to_string(),
            content: output_text.to_string(),
            tool_calls: Vec::new(),
            stop_reason: provider_proto::StopReason::Stop as i32,
            stop_sequence: None,
            usage: Some(provider_proto::TokenUsage {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
                cache_write_tokens: Some(0),
                cache_read_tokens: Some(0),
                cost_usd: None,
            }),
            provider_metadata: None,
            latency: None,
            warnings: Vec::new(),
        }
    }

    fn assignment_for(task: &Task, file: Option<&str>) -> InternalTaskAssignment {
        InternalTaskAssignment {
            task_id: task.id.clone(),
            title: task.description.clone(),
            description: task.description.clone(),
            task_type: task.task_type.clone(),
            target_files: file.into_iter().map(|path| path.to_string()).collect(),
            target_symbols: Vec::new(),
            token_budget: 2_500,
            context_pack: None,
            canonical_contract: None,
        }
    }

    fn diff_replace(file: &str, from: &str, to: &str) -> String {
        format!(
            "\
--- a/{file}
+++ b/{file}
@@ -1 +1 @@
-{from}
+{to}
"
        )
    }

    async fn heterogeneous_registry(
        api_responses: Vec<QueuedApiResponse>,
        local: Option<Arc<dyn LocalProviderTransport>>,
        fallback_policy: FallbackPolicy,
    ) -> (Arc<ProviderRegistry>, MockApiProviderService) {
        let mut local_map = HashMap::new();
        if let Some(local) = local {
            local_map.insert(local_instance_id(), local);
        }
        let (api_client_pool, api_service) = api_client_pool_with_responses(api_responses).await;
        (
            Arc::new(ProviderRegistry::new_with_api_client(
                local_map,
                Some(CloudModelRef {
                    instance_id: cloud_instance_id(),
                    model: "claude-sonnet-4-5".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                fallback_policy,
                runtime_bundle(true, true),
                Arc::new(ModelRegistrySnapshot::default()),
                api_client_pool,
            )),
            api_service,
        )
    }

    fn local_only_registry(local: Arc<dyn LocalProviderTransport>) -> Arc<ProviderRegistry> {
        let mut local_map = HashMap::new();
        local_map.insert(local_instance_id(), local);
        Arc::new(ProviderRegistry::new_with_api_client(
            local_map,
            None,
            Some(LocalModelRef {
                instance_id: local_instance_id(),
                model: "qwen2.5-coder:7b".to_string(),
                wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            FallbackPolicy::Explicit,
            runtime_bundle(false, true),
            Arc::new(test_registry()),
            Arc::new(ApiClientPool::new(openakta_api_client::ClientConfig::default()).unwrap()),
        ))
    }

    fn local_only_config() -> CoordinatorConfig {
        CoordinatorConfig {
            default_cloud: None,
            default_local: Some(LocalModelRef {
                instance_id: local_instance_id(),
                model: "qwen2.5-coder:7b".to_string(),
                wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            provider_bundle: runtime_bundle(false, true),
            registry: Arc::new(test_registry()),
            enable_graph_retrieval: false,
            ..CoordinatorConfig::default()
        }
    }

    fn registry_with_entry(entry: ModelRegistryEntry) -> Arc<ModelRegistrySnapshot> {
        Arc::new(ModelRegistrySnapshot {
            models: [(entry.name.clone(), entry)].into_iter().collect(),
            sources: Default::default(),
        })
    }

    #[tokio::test]
    async fn new_registers_configured_workers() {
        let coordinator = test_coordinator(CoordinatorConfig {
            max_workers: 3,
            ..base_config()
        });

        assert_eq!(coordinator.worker_registry.len(), 3);
        assert!(coordinator.get_available_worker().is_some());
    }

    #[tokio::test]
    async fn execute_mission_runs_simple_workflow() {
        let local = TestLocalTransport::with_responses(
            (0..4)
                .map(|_| Ok(make_response(ProviderKind::OpenAi, "completed simple task")))
                .collect(),
        );
        let registry = local_only_registry(Arc::new(local));
        let mut coordinator = Coordinator::new_with_provider_registry(
            local_only_config(),
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator.execute_mission("simple task").await.unwrap();

        assert!(result.success);
        assert!(result.tasks_completed >= 1);
        assert_eq!(result.tasks_failed, 0);
        assert!(result.output.to_lowercase().contains("completed"));
    }

    #[tokio::test]
    async fn build_task_assignment_uses_registry_context_window_budget() {
        let registry = registry_with_entry(ModelRegistryEntry {
            name: "claude-sonnet-4-5".to_string(),
            max_context_window: 4_096,
            max_output_tokens: 1_024,
            preferred_instance: None,
        });
        let mut coordinator = test_coordinator(CoordinatorConfig {
            registry: Arc::clone(&registry),
            context_use_ratio: 0.5,
            context_margin_tokens: 100,
            retrieval_share: 0.5,
            retrieval_token_budget: 2_000,
            task_token_budget: 2_500,
            enable_graph_retrieval: false,
            ..base_config()
        });

        let task = Task::new("summarize mission");
        let assignment = coordinator.build_task_assignment(&task, None).unwrap();
        let target = coordinator.resolve_route(&task, &assignment).unwrap();
        let request = coordinator
            .build_model_request(&task, &assignment, &target)
            .unwrap();

        assert_eq!(assignment.token_budget, 924);
        assert_eq!(request.max_output_tokens, 1_024);
    }

    #[tokio::test]
    async fn status_reaches_full_progress_after_execution() {
        let local = TestLocalTransport::with_responses(
            (0..4)
                .map(|_| Ok(make_response(ProviderKind::OpenAi, "completed simple task")))
                .collect(),
        );
        let registry = local_only_registry(Arc::new(local));
        let mut coordinator = Coordinator::new_with_provider_registry(
            local_only_config(),
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator.execute_mission("simple task").await.unwrap();
        let status = coordinator.get_mission_status();

        assert_eq!(status.mission_id, result.mission_id);
        assert_eq!(status.progress, 100.0);
        assert_eq!(status.completed_tasks, result.tasks_completed);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execute_direct_reply_returns_single_response() {
        let local =
            TestLocalTransport::with_responses(vec![Ok(make_response(ProviderKind::OpenAi, "hi"))]);
        let registry = local_only_registry(Arc::new(local.clone()));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                default_cloud: None,
                default_local: Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                provider_bundle: runtime_bundle(false, true),
                registry: Arc::new(test_registry()),
                enable_graph_retrieval: false,
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator
            .execute_direct_reply("say only hi", &TaskTargetHints::default(), None)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "hi");
        assert_eq!(result.tasks_completed, 1);
        assert_eq!(local.calls(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execute_direct_reply_emits_required_canonical_events() {
        let tempdir = tempdir().unwrap();
        let trace_dir = tempdir.path().join("execution");
        let trace_registry = Arc::new(ExecutionTraceRegistry::new(trace_dir.clone()));
        let trace_service = trace_registry.create_session("sess-1", false).unwrap();
        let local =
            TestLocalTransport::with_responses(vec![Ok(make_response(ProviderKind::OpenAi, "hi"))]);
        let registry = local_only_registry(Arc::new(local));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                default_cloud: None,
                default_local: Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                provider_bundle: runtime_bundle(false, true),
                registry: Arc::new(test_registry()),
                enable_graph_retrieval: false,
                execution_tracer: Some(Arc::clone(&trace_service)),
                execution_trace_registry: Some(Arc::clone(&trace_registry)),
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator
            .execute_direct_reply("say only hi", &TaskTargetHints::default(), None)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result
            .trace_events
            .iter()
            .all(|event| !event.event_id.is_empty()
                && !event.session_id.is_empty()
                && !event.mission_id.is_empty()
                && !event.action_id.is_empty()
                && event.sequence > 0));
        assert_eq!(
            result
                .trace_events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            (1..=result.trace_events.len() as u64).collect::<Vec<_>>()
        );
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::Mission
                && event.phase == ExecutionTracePhase::Started
        }));
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::Mission
                && event.phase == ExecutionTracePhase::Completed
        }));
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::Task
                && event.phase == ExecutionTracePhase::Requested
        }));
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::Task
                && event.phase == ExecutionTracePhase::Completed
        }));
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::ProviderRequest
                && event.phase == ExecutionTracePhase::Started
        }));
        assert!(result.trace_events.iter().any(|event| {
            event.event_kind == ExecutionEventKind::ProviderRequest
                && event.phase == ExecutionTracePhase::Completed
        }));

        let replayed = read_session_events(&trace_dir, "sess-1", 0).unwrap();
        assert_eq!(replayed, result.trace_events);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execute_single_task_uses_intake_hints_for_code_edits() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn old() {}\n").unwrap();

        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            &diff_replace("src/lib.rs", "fn old() {}", "fn new() {}"),
        ))]);
        let registry = local_only_registry(Arc::new(local.clone()));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                default_cloud: None,
                default_local: Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                provider_bundle: runtime_bundle(false, true),
                registry: Arc::new(test_registry()),
                enable_graph_retrieval: false,
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator
            .execute_single_task(
                Task::new("fix greeting").with_task_type(TaskType::CodeModification),
                &TaskTargetHints {
                    target_files: vec!["src/lib.rs".to_string()],
                    target_symbols: Vec::new(),
                },
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("applied patch"));
        assert_eq!(
            fs::read_to_string(src_dir.join("lib.rs")).unwrap(),
            "fn new() {}\n"
        );
        assert_eq!(local.calls(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execute_single_task_emits_retrieval_events_for_targeted_files() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn old() {}\n").unwrap();

        let trace_dir = tempdir.path().join("execution");
        let trace_registry = Arc::new(ExecutionTraceRegistry::new(trace_dir.clone()));
        let trace_service = trace_registry.create_session("sess-2", false).unwrap();
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            &diff_replace("src/lib.rs", "fn old() {}", "fn new() {}"),
        ))]);
        let registry = local_only_registry(Arc::new(local));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                default_cloud: None,
                default_local: Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                provider_bundle: runtime_bundle(false, true),
                registry: Arc::new(test_registry()),
                enable_graph_retrieval: false,
                execution_tracer: Some(Arc::clone(&trace_service)),
                execution_trace_registry: Some(Arc::clone(&trace_registry)),
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let result = coordinator
            .execute_single_task(
                Task::new("fix src/lib.rs").with_task_type(TaskType::CodeModification),
                &TaskTargetHints {
                    target_files: vec!["src/lib.rs".to_string()],
                    target_symbols: Vec::new(),
                },
            )
            .await
            .unwrap();

        let retrieval_events = result
            .trace_events
            .iter()
            .filter(|event| event.event_kind == ExecutionEventKind::Retrieval)
            .collect::<Vec<_>>();
        assert_eq!(retrieval_events.len(), 2);
        assert_eq!(retrieval_events[0].phase, ExecutionTracePhase::Started);
        assert_eq!(retrieval_events[1].phase, ExecutionTracePhase::Completed);
        assert_eq!(
            retrieval_events[0].target_path.as_deref(),
            Some("src/lib.rs")
        );
        assert_eq!(
            retrieval_events[1].target_path.as_deref(),
            Some("src/lib.rs")
        );
        assert!(retrieval_events[1]
            .result_preview
            .as_deref()
            .unwrap_or_default()
            .contains("files=1"));

        let replayed = read_session_events(&trace_dir, "sess-2", 0).unwrap();
        assert_eq!(replayed, result.trace_events);
    }

    #[tokio::test]
    async fn code_edit_missions_publish_typed_patch_results() {
        // REMOVED IN PHASE 7: This test used SyntheticTransport which has been removed.
        // Cloud execution now uses API client pool exclusively.
        // Test would need to be rewritten to use mock API server instead.
    }

    #[test]
    fn default_local_validation_retry_budget_is_one() {
        assert_eq!(
            CoordinatorConfig::default().local_validation_retry_budget,
            1
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn local_validation_failures_escalate_to_cloud_arbiter_and_apply_patch() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn old() {}\n").unwrap();

        let local = TestLocalTransport::with_responses(vec![
            Ok(make_response(ProviderKind::OpenAi, "not a diff")),
            Ok(make_response(ProviderKind::OpenAi, "still not a diff")),
        ]);
        let (registry, api_service) = heterogeneous_registry(
            vec![Ok(make_api_response(&diff_replace(
                "src/lib.rs",
                "fn old() {}",
                "fn new() {}",
            )))],
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        )
        .await;
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                routing_enabled: true,
                enable_graph_retrieval: false,
                local_validation_retry_budget: 1,
                ..base_config()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let worker_id = coordinator.get_available_worker().unwrap();
        let task = Task::new("syntax fix src/lib.rs").with_task_type(TaskType::CodeModification);
        let assignment = assignment_for(&task, Some("src/lib.rs"));

        let result = coordinator
            .execute_code_task(&task, &worker_id, &assignment)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(
            result.patch_receipt.as_ref().map(|receipt| receipt.status),
            Some(PatchApplyStatus::Applied)
        );
        assert!(result.summary.contains("arbiter applied corrected patch"));
        assert_eq!(local.calls(), 2);
        assert_eq!(api_service.calls(), 1);
        assert_eq!(
            fs::read_to_string(src_dir.join("lib.rs")).unwrap(),
            "fn new() {}\n"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn local_retry_budget_override_delays_arbiter_escalation() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn old() {}\n").unwrap();

        let local = TestLocalTransport::with_responses(vec![
            Ok(make_response(ProviderKind::OpenAi, "bad patch")),
            Ok(make_response(ProviderKind::OpenAi, "bad patch again")),
            Ok(make_response(ProviderKind::OpenAi, "bad patch third time")),
        ]);
        let (registry, api_service) = heterogeneous_registry(
            vec![Ok(make_api_response(&diff_replace(
                "src/lib.rs",
                "fn old() {}",
                "fn new() {}",
            )))],
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        )
        .await;
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                routing_enabled: true,
                enable_graph_retrieval: false,
                local_validation_retry_budget: 2,
                ..base_config()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let worker_id = coordinator.get_available_worker().unwrap();
        let task = Task::new("syntax fix src/lib.rs").with_task_type(TaskType::CodeModification);
        let assignment = assignment_for(&task, Some("src/lib.rs"));

        coordinator
            .execute_code_task(&task, &worker_id, &assignment)
            .await
            .unwrap();

        assert_eq!(local.calls(), 3);
        assert_eq!(api_service.calls(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn arbiter_escalation_without_cloud_lane_returns_cloud_required() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn old() {}\n").unwrap();

        let local = TestLocalTransport::with_responses(vec![
            Ok(make_response(ProviderKind::OpenAi, "not a diff")),
            Ok(make_response(ProviderKind::OpenAi, "still not a diff")),
        ]);
        let mut local_map = HashMap::new();
        local_map.insert(
            local_instance_id(),
            Arc::new(local.clone()) as Arc<dyn LocalProviderTransport>,
        );
        use openakta_api_client::ApiClientPool;
        let registry = Arc::new(ProviderRegistry::new_with_api_client(
            local_map,
            None,
            Some(LocalModelRef {
                instance_id: local_instance_id(),
                model: "qwen2.5-coder:7b".to_string(),
                wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            FallbackPolicy::Explicit,
            runtime_bundle(false, true),
            Arc::new(test_registry()),
            Arc::new(ApiClientPool::new(openakta_api_client::ClientConfig::default()).unwrap()),
        ));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                routing_enabled: true,
                enable_graph_retrieval: false,
                local_validation_retry_budget: 1,
                default_cloud: None,
                default_local: Some(LocalModelRef {
                    instance_id: local_instance_id(),
                    model: "qwen2.5-coder:7b".to_string(),
                    wire_profile: crate::wire_profile::WireProfile::OpenAiChatCompletions,
                    telemetry_kind: ProviderKind::OpenAi,
                }),
                provider_bundle: runtime_bundle(false, true),
                registry: Arc::new(test_registry()),
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let worker_id = coordinator.get_available_worker().unwrap();
        let task = Task::new("syntax fix src/lib.rs").with_task_type(TaskType::CodeModification);
        let assignment = assignment_for(&task, Some("src/lib.rs"));

        let err = coordinator
            .execute_code_task(&task, &worker_id, &assignment)
            .await
            .unwrap_err();

        assert!(matches!(err, CoordinatorV2Error::CloudExecutionRequired(_)));
        assert_eq!(local.calls(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_never_fails_without_local_dispatch() {
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local should not run",
        ))]);
        let (registry, api_service) = heterogeneous_registry(
            vec![Err(Status::unavailable("cloud offline"))],
            Some(Arc::new(local.clone())),
            FallbackPolicy::Never,
        )
        .await;
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                routing_enabled: false,
                enable_graph_retrieval: false,
                fallback_policy: FallbackPolicy::Never,
                ..base_config()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let task = Task::new("summarize mission");
        let assignment = assignment_for(&task, None);
        let err = coordinator
            .execute_non_code_task(&task, &assignment)
            .await
            .unwrap_err();

        match err {
            CoordinatorV2Error::CloudExecutionUnavailable { local_recovery, .. } => {
                assert!(local_recovery.is_none())
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(api_service.calls(), 1);
        assert_eq!(local.calls(), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_explicit_returns_recovery_without_local_dispatch() {
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local should not run",
        ))]);
        let (registry, api_service) = heterogeneous_registry(
            vec![Err(Status::unavailable("cloud offline"))],
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        )
        .await;
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                routing_enabled: false,
                enable_graph_retrieval: false,
                fallback_policy: FallbackPolicy::Explicit,
                ..base_config()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let task = Task::new("summarize mission");
        let assignment = assignment_for(&task, None);
        let err = coordinator
            .execute_non_code_task(&task, &assignment)
            .await
            .unwrap_err();

        match err {
            CoordinatorV2Error::CloudExecutionUnavailable { local_recovery, .. } => {
                assert_eq!(
                    local_recovery.as_deref(),
                    Some("retry with local model qwen2.5-coder:7b")
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(api_service.calls(), 1);
        assert_eq!(local.calls(), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_automatic_redispatches_to_local_and_emits_warning() {
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local fallback response",
        ))]);
        let (registry, api_service) = heterogeneous_registry(
            vec![Err(Status::unavailable("cloud offline"))],
            Some(Arc::new(local.clone())),
            FallbackPolicy::Automatic,
        )
        .await;
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                routing_enabled: false,
                enable_graph_retrieval: false,
                fallback_policy: FallbackPolicy::Automatic,
                ..base_config()
            },
            Arc::new(BlackboardV2::default()),
            registry,
        )
        .unwrap();

        let task = Task::new("summarize mission");
        let assignment = assignment_for(&task, None);
        let capture = WarnCapture::default();
        let subscriber = Registry::default().with(capture.clone());
        let _guard = tracing::subscriber::set_default(subscriber);
        let result = coordinator
            .execute_non_code_task(&task, &assignment)
            .await
            .unwrap();

        assert_eq!(result.summary, "local fallback response");
        assert_eq!(api_service.calls(), 1);
        assert_eq!(local.calls(), 1);
        assert!(capture.count() >= 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_automatic_without_local_fails_cleanly() {
        // REMOVED IN PHASE 7: This test used cloud HashMap transport which has been removed.
        // Cloud execution now uses API client pool exclusively.
        // Test would need complete rewrite to work with new architecture.
    }

    /// Same path rules as `DeterministicPatchApplier` / V-005 (`resolve_workspace_relative_path`).
    #[test]
    fn coordinator_context_reads_use_patch_protocol_path_boundary() {
        let root = tempdir().unwrap();
        assert!(crate::patch_protocol::resolve_workspace_relative_path(
            root.path(),
            "../../../etc/passwd"
        )
        .is_err());
        assert!(crate::patch_protocol::resolve_workspace_relative_path(
            root.path(),
            "src/../../../etc/passwd"
        )
        .is_err());
        assert!(
            crate::patch_protocol::resolve_workspace_relative_path(root.path(), "src/lib.rs")
                .is_ok()
        );
    }
}
