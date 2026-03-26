//! OPENAKTA Agents
//!
//! Agent framework with state machine orchestration.

pub mod aci_formatter;
pub mod agent;
pub mod blackboard_runtime;
pub mod capabilities;
pub mod catalog_registry;
pub mod communication;
pub mod conflict;
pub mod coordinator;
pub mod decomposer;
pub mod diagnostics;
pub mod error;
pub mod execution_trace;
pub mod executor;
pub mod graph;
pub mod heartbeat;
pub mod hitl;
pub mod intake;
pub mod mcp_client;
pub mod merger;
pub mod model_registry;
pub mod monitor;
pub mod openai_family;
pub mod patch_protocol;
pub mod prompt_assembly;
pub mod provider;
pub mod provider_registry;
pub mod provider_transport;
pub mod react;
pub mod result_contract;
pub mod retrieval;
pub mod routing;
pub mod state_machine;
pub mod task;
pub mod task_queue;
pub mod token_budget;
pub mod tool_registry;
pub mod transport;
pub mod wire_profile;
pub mod worker_pool;

pub use aci_formatter::{ACIConfig, ACIFormatter, TokenSavings};
pub use agent::{
    Agent, AgentState, ArchitectAgent, BaseAgent, CoderAgent, DebuggerAgent, RefactorerAgent,
    ReviewerAgent, TaskResult as AgentTaskResult, TesterAgent,
};
pub use blackboard_runtime::{BlackboardEntry, RuntimeBlackboard};
pub use capabilities::{AgentCapabilities, CapabilityRegistry, TaskRequirements};
pub use catalog_registry::{
    AdapterHint, ApiSurface, CatalogRegistry, CompatibilityFamily, EffectiveCapabilities, Modality,
    Model, ModelCapabilities, ModelStatus, Provider, ProviderCapabilities, ProviderStatus,
    RegistryConfig, RegistryDiagnostics, RegistryError, RegistrySnapshot,
};
pub use communication::{AgentMessage, CommunicationProtocol, Envelope, MessageBus, MessageType};
pub use conflict::{
    Conflict, ConflictResolution, ConflictResolver, ConflictStatus, ConflictType, Decision,
    Resolution, Vote,
};
pub use coordinator::{
    BaseSquadBootstrapper, BlackboardV2, Coordinator, CoordinatorConfig, CoordinatorMetrics,
    CoordinatorTaskQueue, CoordinatorV2Error, MissionResult, MissionStatus, OutputContract,
    PlanningActingPolicy, RegisteredWorkerInfo, SquadRole, WorkerProfile, WorkerRegistry,
};
pub use decomposer::{
    DecomposedMission, DecomposerConfig, DecompositionRule, Dependency, DependencyType,
    DeterministicLLMBackend, GraphBuilder, LLMBackend, LLMDecomposer, MissionDecomposer,
    MissionTemplate, ParallelGroup, ParallelGroupIdentifier, RawTask, TaskDAG, TaskId,
    TaskTemplate,
};
pub use diagnostics::{WideEvent, WideEventError, WideEventMeta};
pub use error::AgentError;
pub use execution_trace::{
    read_events_from_path, read_session_events, ExecutionEventKind, ExecutionSummaryRenderer,
    ExecutionTraceEvent, ExecutionTracePhase, ExecutionTraceRegistry, ExecutionTraceService,
};
pub use executor::{
    ConcurrentExecutor, ExecutorConfig, ExecutorStats, MissionResult as ExecutorMissionResult,
};
pub use graph::{
    Edge, ExecutionMode, ExecutionState, GraphStats, Node, NodeId, NodeRole, ParallelismDetector,
    TransitionCondition as GraphTransitionCondition, WorkflowGraph,
};
pub use heartbeat::{
    AgentSleepState, Heartbeat, HeartbeatConfig, HeartbeatEvent, HeartbeatMessage,
};
pub use hitl::{
    redact_answer_for_logs, HitlConfig, HitlError, HitlMetrics, HitlSubmitAnswerOutcome,
    MissionHitlGate,
};
pub use intake::{
    DecompositionBudget, DelegationBudget, MessageExecutionMode, MessageSurface, MissionDecision,
    MissionGate, MissionGateRequest, ResponsePreference, RetrievalPlan, RiskLevel, TaskTargetHints,
};
pub use mcp_client::McpClient;
pub use merger::{
    ChangeRegion, FileChange, FileContentType, MergeConflict as ResultMergeConflict,
    MergeConflictResolution, MergeConflictType, MergeState, MergedFile, MergedResult, MergerConfig,
    ResultMerger, State as MergerState, TaskResult as MergerTaskResult, WorkerId as MergerWorkerId,
};
pub use model_registry::{DynamicModelMetadata, DynamicModelRegistry};
pub use monitor::{
    BlockerDetector, BlockerInfo, ProgressMonitor, ProgressTracker, StatusReport, StatusReporter,
    TaskCounts, TaskProgress,
};
pub use patch_protocol::{
    AstSummary, ContextPack, ContextSpan, DeterministicPatchApplier, DiffOutputValidator,
    MetaGlyphCommand, MetaGlyphOpcode, PatchApplyStatus, PatchEnvelope, PatchFormat, PatchReceipt,
    RetrievalHit, SearchReplaceBlock, SymbolMap, ValidatedAgentOutput, ValidationFact,
    ValidationResult,
};
pub use prompt_assembly::PromptAssembly;
pub use provider::{
    CacheMetrics, CacheRetention, ModelBoundaryPayload, ModelBoundaryPayloadType, ModelRequest,
    ModelResponse, ModelResponseChunk, ModelToolCall, ModelToolSchema, OpenAiProvider,
    PreparedProviderRequest, PromptCacheScope, PromptSegment, ProviderClient, ProviderKind,
    ProviderUsage,
};
pub use provider_registry::ProviderRegistry;
pub use provider_transport::{
    default_local_transport, local_provider_config_from_instance, CloudModelRef, FallbackPolicy,
    LocalModelRef, LocalProviderConfig, LocalProviderKind, LocalProviderTransport,
    ModelRegistryEntry, ModelRegistrySnapshot, ModelRoutingHint, OllamaTransport,
    ProviderExecutionTelemetry, ProviderInstanceConfig, ProviderInstanceId,
    ProviderInstancesConfig, ProviderProfileId, ProviderRuntimeBundle, ProviderRuntimeConfig,
    ProviderTransport, ProviderTransportError, RemoteRegistryConfig, ResolvedProviderInstance,
    RoutingReason, RoutingResolution, SecretRef, TomlModelRegistryEntry,
};
pub use react::{
    Action, ActionExecution, ActionProposal, DualThreadReactAgent, InterruptSignal, Observation,
    ReactCycle, ReactStats, Tool, ToolSet, DEFAULT_INTERRUPT_LAG_STREAK_LIMIT,
};
pub use result_contract::{
    DiffValidationDecision, PublicationPayload, PublicationPayloadType, ResultPublicationGuard,
};
pub use routing::{route as route_task, ExecutionDescriptor, RoutedTarget};
pub use state_machine::{GlobalState, StateMachine, StateTransition, TransitionCondition};
pub use task::{Priority, Task, TaskStatus, TaskType};
pub use task_queue::{
    DependencyTracker, DependencyTrackerError, LoadBalancer, LoadBalancingTask, PriorityScheduler,
    PrioritySchedulerError, QueueStats, QueueTaskId, QueuedTask, TaskQueue, TaskQueueConfig,
    TaskQueueError, TaskQueueStatus, WorkerAssignment,
};
pub use token_budget::{derive_effective_budget, EffectiveTokenBudget};
pub use tool_registry::{
    CostClass, ExecutorKind, ResultNormalizerKind, ToolKind, ToolRegistry, ToolSpec, UiRenderer,
};
pub use transport::{
    InternalBlockerAlert, InternalContextReference, InternalProgressUpdate,
    InternalResultSubmission, InternalTaskAssignment, InternalTokenUsage,
    InternalWorkflowTransitionEvent, ProtoTransport,
};
pub use wire_profile::WireProfile;
pub use worker_pool::{
    DispatchRecord, HealthMonitor, HealthState, PoolStats, ScaleAction, TaskDispatcher, Worker,
    WorkerFactory, WorkerId, WorkerLifecycle, WorkerLifecycleManager, WorkerPool, WorkerPoolConfig,
    WorkerPoolError, WorkerSpawner, WorkerStatus,
};

use thiserror::Error;

/// Agent-related errors
#[derive(Error, Debug)]
pub enum OpenaktaAgentsError {
    /// Agent error
    #[error("agent error: {0}")]
    Agent(#[from] AgentError),
    /// RAG error
    #[error("rag error: {0}")]
    Rag(#[from] openakta_rag::OpenaktaRagError),
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, OpenaktaAgentsError>;
