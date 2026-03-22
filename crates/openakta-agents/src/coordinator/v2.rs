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
use crate::agent::ReviewerAgent;
use crate::blackboard_runtime::{BlackboardEntry, RuntimeBlackboard};
use crate::communication::CommunicationProtocol;
use crate::decomposer::MissionDecomposer;
use crate::diagnostics::WideEvent;
use crate::patch_protocol::{
    resolve_workspace_relative_path, AstSummary, ContextPack, ContextSpan,
    DeterministicPatchApplier, DiffOutputValidator, PatchApplyStatus, PatchEnvelope, PatchFormat,
    RetrievalHit, SymbolMap, ValidationFact,
};
use crate::prompt_assembly::PromptAssembly;
use crate::provider::{CacheRetention, ModelRequest, ModelResponse, ProviderUsage};
use crate::provider_registry::ProviderRegistry;
use crate::provider_transport::{
    default_local_transport, transport_for_instance, CloudModelRef, FallbackPolicy, LocalModelRef,
    LocalProviderConfig, LocalProviderKind, ModelRegistrySnapshot, ProviderInstanceId,
    ProviderRuntimeBundle, ProviderTransport, ProviderTransportError,
};
use crate::retrieval::{GraphRetrievalConfig, GraphRetrievalRequest, GraphRetriever};
use crate::routing::{route, RoutedTarget};
use crate::task::{Task, TaskStatus, TaskType};
use crate::token_budget::{derive_effective_budget, EffectiveTokenBudget};
use crate::transport::{
    InternalContextReference, InternalResultSubmission, InternalTaskAssignment, InternalTokenUsage,
};
use crate::worker_pool::{WorkerId, WorkerStatus};
use openakta_indexing::{InfluenceGraph, Language, ParserRegistry};
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
        }
    }
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
    metrics: CoordinatorMetrics,
    mission_id: Option<String>,
    mission_started_at: Option<Instant>,
    merged_outputs: Vec<String>,
    tasks_failed: usize,
}

impl Coordinator {
    /// Creates a new Coordinator v2 and pre-registers worker slots.
    pub fn new(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Result<Self> {
        let mut cloud = HashMap::new();
        let mut local = HashMap::new();
        for (instance_id, instance) in &config.provider_bundle.instances {
            if instance.is_local {
                let local_config = LocalProviderConfig {
                    provider: LocalProviderKind::Ollama,
                    base_url: instance.base_url.clone(),
                    default_model: instance
                        .default_model
                        .clone()
                        .unwrap_or_else(|| "qwen2.5-coder:7b".to_string()),
                    enabled_for: config.local_enabled_for.clone(),
                };
                local.insert(
                    instance_id.clone(),
                    Arc::from(
                        default_local_transport(&local_config, config.task_timeout)
                            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?,
                    ),
                );
            } else {
                cloud.insert(
                    instance_id.clone(),
                    Arc::from(
                        transport_for_instance(instance, &config.provider_bundle.http)
                            .map_err(|err| CoordinatorV2Error::ExecutionFailed(err.to_string()))?,
                    ),
                );
            }
        }
        let registry = Arc::new(ProviderRegistry::new(
            cloud,
            local,
            config.default_cloud.clone(),
            config.default_local.clone(),
            config.fallback_policy,
            Arc::clone(&config.provider_bundle),
            Arc::clone(&config.registry),
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
        if !registry.has_cloud() && !registry.has_local() {
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
            metrics: CoordinatorMetrics::default(),
            mission_id: None,
            mission_started_at: None,
            merged_outputs: Vec::new(),
            tasks_failed: 0,
        })
    }

    /// Creates a new coordinator with a cloud-only provider transport.
    pub fn new_with_provider_transport(
        config: CoordinatorConfig,
        blackboard: Arc<BlackboardV2>,
        instance_id: ProviderInstanceId,
        provider_transport: Arc<dyn ProviderTransport>,
    ) -> Result<Self> {
        let mut cloud = HashMap::new();
        cloud.insert(instance_id, provider_transport);
        let registry = Arc::new(ProviderRegistry::new(
            cloud,
            HashMap::new(),
            config.default_cloud.clone(),
            config.default_local.clone(),
            config.fallback_policy,
            Arc::clone(&config.provider_bundle),
            Arc::clone(&config.registry),
        ));
        Self::new_with_provider_registry(config, blackboard, registry)
    }

    /// Executes a mission using decompose → dispatch → monitor → merge.
    pub async fn execute_mission(&mut self, mission: &str) -> Result<MissionResult> {
        let mission_id = Uuid::new_v4().to_string();
        self.mission_id = Some(mission_id.clone());
        self.mission_started_at = Some(Instant::now());
        self.merged_outputs.clear();
        self.tasks_failed = 0;
        if let Some(ref gate) = self.config.hitl_gate {
            let _ = gate.register_mission_start(&mission_id);
        }

        let decomposed = MissionDecomposer::new()
            .decompose_async(mission)
            .await
            .map_err(|error| CoordinatorV2Error::Decomposition(error.to_string()))?;
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

        let duration = self
            .mission_started_at
            .map(|started| started.elapsed())
            .unwrap_or(Duration::ZERO);
        let tasks_completed = self.task_queue.completed_tasks();

        let success = self.tasks_failed == 0;
        if let Some(ref gate) = self.config.hitl_gate {
            gate.register_mission_complete(&mission_id, success);
        }

        Ok(MissionResult {
            mission_id,
            success,
            output: self.merged_outputs.join("\n"),
            tasks_completed,
            tasks_failed: self.tasks_failed,
            duration,
        })
    }

    /// Returns the next idle worker, if one exists.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.worker_registry.get_available_worker()
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
        let mut dispatch_task = task.clone();
        dispatch_task.status = TaskStatus::Pending;
        dispatch_task.assigned_to = None;

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

        self.assign_task(worker_id.clone(), task)?;
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
                }
                TaskStatus::Failed => {
                    self.task_queue.mark_task_complete(&task.id)?;
                    self.tasks_failed += 1;
                    if let Some(result_submission) = completion.result_submission.as_ref() {
                        self.publish_result_submission(result_submission).await?;
                    } else {
                        let result_submission =
                            self.completion_result_submission(task, completion, false);
                        self.publish_result_submission(&result_submission).await?;
                    }
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
        let assignment = match self.build_task_assignment(task) {
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
        match &target {
            RoutedTarget::Cloud(cloud_target) => {
                let cloud = self
                    .registry
                    .cloud_transport(&cloud_target.instance_id)
                    .ok_or_else(|| {
                        CoordinatorV2Error::CloudExecutionRequired(
                            "cloud lane is not configured".to_string(),
                        )
                    })?;
                match cloud.execute(&model_request).await {
                    Ok(response) => Ok((target, response)),
                    Err(ProviderTransportError::CloudExecutionUnavailable(message)) => {
                        self.handle_cloud_unavailable(task, assignment, target, message)
                            .await
                    }
                    Err(err) => Err(CoordinatorV2Error::ExecutionFailed(err.to_string())),
                }
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
        }
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
        let max_output_tokens = self
            .config
            .registry
            .models
            .get(target.model_label())
            .map(|entry| entry.max_output_tokens)
            .ok_or_else(|| {
                CoordinatorV2Error::InvalidConfig(format!(
                    "model '{}' not found in registry - cannot determine token budget",
                    target.model_label()
                ))
            })?;
        Ok(
            PromptAssembly::for_worker_task(task, assignment, task.assigned_to.as_deref())
                .into_model_request(
                    target.request_provider(),
                    target.model_label().to_string(),
                    max_output_tokens,
                    Some(0.0),
                    false,
                    CacheRetention::Extended,
                ),
        )
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
        let Some(cloud_transport) = self.registry.cloud_transport(&cloud_ref.instance_id) else {
            return Err(CoordinatorV2Error::CloudExecutionRequired(
                "arbiter escalation requires a configured cloud lane".to_string(),
            ));
        };

        let reviewer = ReviewerAgent::with_cloud_transport(
            cloud_ref.wire_profile,
            cloud_ref.model.clone(),
            cloud_transport,
        );
        let review_task = Task::new(&format!(
            "Repair the failed local patch for task '{}'. Validation error: {}. Failed output:\n{}",
            task.description, validation_error, failed_output
        ))
        .with_task_type(TaskType::Review);
        let reviewed = reviewer
            .execute_review(review_task)
            .await
            .map_err(|err| match err {
                crate::OpenaktaAgentsError::Agent(
                    crate::error::AgentError::CloudExecutionUnavailable(message),
                ) => CoordinatorV2Error::CloudExecutionUnavailable {
                    message,
                    local_recovery: None,
                },
                crate::OpenaktaAgentsError::Agent(
                    crate::error::AgentError::CloudExecutionRequired(message),
                ) => CoordinatorV2Error::CloudExecutionRequired(message),
                other => CoordinatorV2Error::ExecutionFailed(other.to_string()),
            })?;
        let validated = self
            .diff_validator
            .validate(&reviewed.output)
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

    fn build_task_assignment(&self, task: &Task) -> Result<InternalTaskAssignment> {
        let (target_files, target_symbols) = extract_targets(&task.description);
        let budget = self
            .planned_target(task, &target_files, &target_symbols)
            .map(|target| self.effective_budget_for_target(&target))
            .unwrap_or_else(|| self.default_effective_budget());
        let context_pack =
            self.build_context_pack(task, &target_files, &target_symbols, budget.retrieval_cap)?;

        Ok(InternalTaskAssignment {
            task_id: task.id.clone(),
            title: task.description.clone(),
            description: task.description.clone(),
            task_type: task.task_type.clone(),
            target_files,
            target_symbols,
            token_budget: budget.task_cap,
            context_pack,
        })
    }

    fn build_context_pack(
        &self,
        task: &Task,
        target_files: &[String],
        target_symbols: &[String],
        retrieval_token_budget: usize,
    ) -> Result<Option<ContextPack>> {
        if target_files.is_empty() && target_symbols.is_empty() {
            return Ok(None);
        }

        let mut spans = Vec::new();
        let mut retrieval_hits = Vec::new();
        let mut ast_summaries = Vec::new();
        let mut symbol_maps = Vec::new();
        let mut validation_facts = Vec::new();

        for file in target_files {
            let path = resolve_workspace_relative_path(&self.config.workspace_root, file).map_err(
                |e| CoordinatorV2Error::ProtocolViolation(e.to_string()),
            )?;
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
                    content,
                },
                vec!["coordinator".to_string()],
            )
            .map_err(|err| CoordinatorV2Error::ProtocolViolation(err.to_string()))?;
        Ok(())
    }
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
    let path = resolve_workspace_relative_path(workspace_root, file_path).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
    })?;
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
    use super::{BlackboardV2, Coordinator, CoordinatorConfig, CoordinatorV2Error};
    use crate::patch_protocol::PatchApplyStatus;
    use crate::provider::{ModelRequest, ModelResponse, ProviderKind, ProviderUsage};
    use crate::provider_registry::ProviderRegistry;
    use crate::provider_transport::{
        CloudModelRef, FallbackPolicy, LocalModelRef, LocalProviderKind, LocalProviderTransport,
        ModelRegistryEntry, ModelRegistrySnapshot, ProviderInstanceId, ProviderProfileId,
        ProviderRuntimeBundle, ProviderRuntimeConfig, ProviderTransport, ProviderTransportError,
        ResolvedProviderInstance, SyntheticTransport,
    };
    use crate::task::{Task, TaskType};
    use crate::transport::{InternalResultSubmission, InternalTaskAssignment};
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;
    use tonic::async_trait;
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;
    use tracing_subscriber::{Layer, Registry};

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
                    profile: ProviderProfileId::AnthropicMessagesV1,
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key: None,
                    is_local: false,
                    default_model: Some("claude-sonnet-4-5".to_string()),
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
                wire_profile: crate::wire_profile::WireProfile::AnthropicMessagesV1,
                telemetry_kind: ProviderKind::Anthropic,
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
        let workspace_root = config.workspace_root.clone();
        Coordinator::new_with_provider_transport(
            config,
            Arc::new(BlackboardV2::default()),
            cloud_instance_id(),
            Arc::new(SyntheticTransport::new(workspace_root)),
        )
        .unwrap()
    }

    #[derive(Clone, Default)]
    struct TestCloudTransport {
        responses: Arc<Mutex<VecDeque<std::result::Result<ModelResponse, ProviderTransportError>>>>,
        calls: Arc<AtomicUsize>,
    }

    impl TestCloudTransport {
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
    impl ProviderTransport for TestCloudTransport {
        async fn execute(
            &self,
            _request: &ModelRequest,
        ) -> std::result::Result<ModelResponse, ProviderTransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(ProviderTransportError::Http(
                        "no queued cloud response".to_string(),
                    ))
                })
        }

        fn mode(&self) -> &'static str {
            "test-cloud"
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
            output_text: output_text.to_string(),
            usage: ProviderUsage::default(),
            stop_reason: None,
            raw: json!({ "output_text": output_text }),
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

    fn heterogeneous_registry(
        cloud: Option<Arc<dyn ProviderTransport>>,
        local: Option<Arc<dyn LocalProviderTransport>>,
        fallback_policy: FallbackPolicy,
    ) -> Arc<ProviderRegistry> {
        let mut cloud_map = HashMap::new();
        let mut local_map = HashMap::new();
        if let Some(cloud) = cloud {
            cloud_map.insert(cloud_instance_id(), cloud);
        }
        if let Some(local) = local {
            local_map.insert(local_instance_id(), local);
        }
        Arc::new(ProviderRegistry::new(
            cloud_map,
            local_map,
            Some(CloudModelRef {
                instance_id: cloud_instance_id(),
                model: "claude-sonnet-4-5".to_string(),
                wire_profile: crate::wire_profile::WireProfile::AnthropicMessagesV1,
                telemetry_kind: ProviderKind::Anthropic,
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
        ))
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
        let mut coordinator = test_coordinator(base_config());

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
        let coordinator = test_coordinator(CoordinatorConfig {
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
        let assignment = coordinator.build_task_assignment(&task).unwrap();
        let target = coordinator.resolve_route(&task, &assignment).unwrap();
        let request = coordinator
            .build_model_request(&task, &assignment, &target)
            .unwrap();

        assert_eq!(assignment.token_budget, 924);
        assert_eq!(request.max_output_tokens, 1_024);
    }

    #[tokio::test]
    async fn status_reaches_full_progress_after_execution() {
        let mut coordinator = test_coordinator(base_config());

        let result = coordinator.execute_mission("simple task").await.unwrap();
        let status = coordinator.get_mission_status();

        assert_eq!(status.mission_id, result.mission_id);
        assert_eq!(status.progress, 100.0);
        assert_eq!(status.completed_tasks, result.tasks_completed);
    }

    #[tokio::test]
    async fn code_edit_missions_publish_typed_patch_results() {
        let tempdir = tempdir().unwrap();
        let src_dir = tempdir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub fn example() {}\n").unwrap();

        let blackboard = Arc::new(BlackboardV2::default());
        let mut coordinator = Coordinator::new_with_provider_transport(
            CoordinatorConfig {
                workspace_root: tempdir.path().to_path_buf(),
                enable_graph_retrieval: false,
                ..base_config()
            },
            blackboard.clone(),
            cloud_instance_id(),
            Arc::new(SyntheticTransport::new(tempdir.path())),
        )
        .unwrap();

        let result = coordinator
            .execute_mission("update src/lib.rs")
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tasks_failed, 0);

        let blackboard = blackboard.lock().await;
        let published = blackboard.get_accessible("coordinator");
        let typed_results = published
            .iter()
            .filter_map(|entry| {
                serde_json::from_str::<InternalResultSubmission>(&entry.content).ok()
            })
            .collect::<Vec<_>>();

        assert!(!typed_results.is_empty());
        assert!(typed_results.iter().any(|result| {
            result.patch.is_some()
                && result
                    .patch_receipt
                    .as_ref()
                    .is_some_and(|receipt| receipt.status == PatchApplyStatus::Applied)
        }));
        assert!(typed_results
            .iter()
            .all(|result| result.success || !result.error_message.is_empty()));
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
        let cloud = TestCloudTransport::with_responses(vec![Ok(make_response(
            ProviderKind::Anthropic,
            &diff_replace("src/lib.rs", "fn old() {}", "fn new() {}"),
        ))]);

        let registry = heterogeneous_registry(
            Some(Arc::new(cloud.clone())),
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        );
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
        assert_eq!(cloud.calls(), 1);
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
        let cloud = TestCloudTransport::with_responses(vec![Ok(make_response(
            ProviderKind::Anthropic,
            &diff_replace("src/lib.rs", "fn old() {}", "fn new() {}"),
        ))]);

        let registry = heterogeneous_registry(
            Some(Arc::new(cloud.clone())),
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        );
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
        assert_eq!(cloud.calls(), 1);
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
        let registry = Arc::new(ProviderRegistry::new(
            HashMap::new(),
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
        let cloud = TestCloudTransport::with_responses(vec![Err(
            ProviderTransportError::CloudExecutionUnavailable("cloud offline".to_string()),
        )]);
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local should not run",
        ))]);
        let registry = heterogeneous_registry(
            Some(Arc::new(cloud.clone())),
            Some(Arc::new(local.clone())),
            FallbackPolicy::Never,
        );
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
        assert_eq!(cloud.calls(), 1);
        assert_eq!(local.calls(), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_explicit_returns_recovery_without_local_dispatch() {
        let cloud = TestCloudTransport::with_responses(vec![Err(
            ProviderTransportError::CloudExecutionUnavailable("cloud offline".to_string()),
        )]);
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local should not run",
        ))]);
        let registry = heterogeneous_registry(
            Some(Arc::new(cloud.clone())),
            Some(Arc::new(local.clone())),
            FallbackPolicy::Explicit,
        );
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
        assert_eq!(cloud.calls(), 1);
        assert_eq!(local.calls(), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_automatic_redispatches_to_local_and_emits_warning() {
        let cloud = TestCloudTransport::with_responses(vec![Err(
            ProviderTransportError::CloudExecutionUnavailable("cloud offline".to_string()),
        )]);
        let local = TestLocalTransport::with_responses(vec![Ok(make_response(
            ProviderKind::OpenAi,
            "local fallback response",
        ))]);
        let registry = heterogeneous_registry(
            Some(Arc::new(cloud.clone())),
            Some(Arc::new(local.clone())),
            FallbackPolicy::Automatic,
        );
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
        assert_eq!(cloud.calls(), 1);
        assert_eq!(local.calls(), 1);
        assert!(capture.count() >= 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fallback_policy_automatic_without_local_fails_cleanly() {
        let cloud = TestCloudTransport::with_responses(vec![Err(
            ProviderTransportError::CloudExecutionUnavailable("cloud offline".to_string()),
        )]);
        let mut cloud_map = HashMap::new();
        cloud_map.insert(
            cloud_instance_id(),
            Arc::new(cloud.clone()) as Arc<dyn ProviderTransport>,
        );
        let registry = Arc::new(ProviderRegistry::new(
            cloud_map,
            HashMap::new(),
            Some(CloudModelRef {
                instance_id: cloud_instance_id(),
                model: "claude-sonnet-4-5".to_string(),
                wire_profile: crate::wire_profile::WireProfile::AnthropicMessagesV1,
                telemetry_kind: ProviderKind::Anthropic,
            }),
            None,
            FallbackPolicy::Automatic,
            runtime_bundle(true, false),
            Arc::new(test_registry()),
        ));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                routing_enabled: false,
                enable_graph_retrieval: false,
                fallback_policy: FallbackPolicy::Automatic,
                default_local: None,
                provider_bundle: runtime_bundle(true, false),
                registry: Arc::new(test_registry()),
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
        assert_eq!(cloud.calls(), 1);
    }

    /// Same path rules as `DeterministicPatchApplier` / V-005 (`resolve_workspace_relative_path`).
    #[test]
    fn coordinator_context_reads_use_patch_protocol_path_boundary() {
        let root = tempdir().unwrap();
        assert!(
            crate::patch_protocol::resolve_workspace_relative_path(root.path(), "../../../etc/passwd")
                .is_err()
        );
        assert!(
            crate::patch_protocol::resolve_workspace_relative_path(root.path(), "src/../../../etc/passwd")
                .is_err()
        );
        assert!(
            crate::patch_protocol::resolve_workspace_relative_path(root.path(), "src/lib.rs").is_ok()
        );
    }
}
