//! AXORA Agents
//!
//! Agent framework with state machine orchestration.

#![warn(missing_docs)]

pub mod aci_formatter;
pub mod agent;
pub mod capabilities;
pub mod communication;
pub mod conflict;
pub mod coordinator;
pub mod decomposer;
pub mod error;
pub mod executor;
pub mod graph;
pub mod heartbeat;
pub mod memory;
pub mod mcp_client;
pub mod merger;
pub mod monitor;
pub mod patch_protocol;
pub mod prompt_assembly;
pub mod provider;
pub mod provider_transport;
pub mod retrieval;
pub mod react;
pub mod result_contract;
pub mod state_machine;
pub mod task;
pub mod task_queue;
pub mod transport;
pub mod worker_pool;

pub use aci_formatter::{ACIConfig, ACIFormatter, TokenSavings};
pub use agent::{
    Agent, AgentState, ArchitectAgent, BaseAgent, CoderAgent, DebuggerAgent, ReviewerAgent,
    TaskResult as AgentTaskResult, TesterAgent,
};
pub use capabilities::{AgentCapabilities, CapabilityRegistry, TaskRequirements};
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
pub use error::AgentError;
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
pub use memory::{MemoryEntry, MemoryStore, MemoryType, SharedBlackboard};
pub use mcp_client::McpClient;
pub use merger::{
    ChangeRegion, FileChange, FileContentType, MergeConflict as ResultMergeConflict,
    MergeConflictResolution, MergeConflictType, MergeState, MergedFile, MergedResult, MergerConfig,
    ResultMerger, State as MergerState, TaskResult as MergerTaskResult, WorkerId as MergerWorkerId,
};
pub use monitor::{
    BlockerDetector, BlockerInfo, ProgressMonitor, ProgressTracker, StatusReport, StatusReporter,
    TaskCounts, TaskProgress,
};
pub use patch_protocol::{
    AstSummary, ContextPack, ContextSpan, DeterministicPatchApplier, DiffOutputValidator,
    MetaGlyphCommand, MetaGlyphOpcode, PatchApplyStatus, PatchEnvelope, PatchFormat, PatchReceipt,
    RetrievalHit, SearchReplaceBlock, SymbolMap, ValidationFact, ValidationResult,
    ValidatedAgentOutput,
};
pub use prompt_assembly::PromptAssembly;
pub use provider::{
    AnthropicProvider, CacheMetrics, CacheRetention, ModelBoundaryPayload, ModelBoundaryPayloadType,
    ModelRequest, ModelResponse, ModelResponseChunk, OpenAiProvider, PreparedProviderRequest,
    PromptCacheScope, PromptSegment, ProviderClient, ProviderKind, ProviderUsage,
};
pub use provider_transport::{
    default_transport as default_provider_transport, LiveHttpTransport, ProviderExecutionTelemetry,
    ProviderRuntimeConfig, ProviderTransport, ProviderTransportError, SyntheticTransport,
};
pub use react::{
    Action, ActionExecution, ActionProposal, DualThreadReactAgent, InterruptSignal, Observation,
    ReactCycle, ReactStats, Tool, ToolSet,
};
pub use result_contract::{
    DiffValidationDecision, PublicationPayload, PublicationPayloadType, ResultPublicationGuard,
};
pub use state_machine::{GlobalState, StateMachine, StateTransition, TransitionCondition};
pub use task::{Priority, Task, TaskStatus, TaskType};
pub use task_queue::{
    DependencyTracker, DependencyTrackerError, LoadBalancer, LoadBalancingTask, PriorityScheduler,
    PrioritySchedulerError, QueueStats, QueueTaskId, QueuedTask, TaskQueue, TaskQueueConfig,
    TaskQueueError, TaskQueueStatus, WorkerAssignment,
};
pub use transport::{
    InternalBlockerAlert, InternalContextReference, InternalProgressUpdate, InternalResultSubmission,
    InternalTaskAssignment, InternalTokenUsage, InternalWorkflowTransitionEvent, ProtoTransport,
};
pub use worker_pool::{
    DispatchRecord, HealthMonitor, HealthState, PoolStats, ScaleAction, TaskDispatcher, Worker,
    WorkerFactory, WorkerId, WorkerLifecycle, WorkerLifecycleManager, WorkerPool, WorkerPoolConfig,
    WorkerPoolError, WorkerSpawner, WorkerStatus,
};

use thiserror::Error;

/// Agent-related errors
#[derive(Error, Debug)]
pub enum AxoraAgentsError {
    /// Agent error
    #[error("agent error: {0}")]
    Agent(#[from] AgentError),
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, AxoraAgentsError>;
