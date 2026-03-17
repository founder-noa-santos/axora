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
pub mod merger;
pub mod monitor;
pub mod react;
pub mod state_machine;
pub mod task;
pub mod task_queue;
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
pub use coordinator::v2::{
    BlackboardV2, Coordinator as CoordinatorV2, CoordinatorConfig, CoordinatorTaskQueue,
    CoordinatorV2Error, MissionResult as CoordinatorV2MissionResult, MissionStatus,
    RegisteredWorkerInfo, WorkerRegistry,
};
pub use coordinator::{Coordinator, CoordinatorStats, MissionResult, TaskResult, DAG};
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
pub use merger::{
    ChangeRegion, FileChange, FileContentType, MergeConflict as ResultMergeConflict,
    MergeConflictResolution, MergeConflictType, MergeState, MergedFile, MergedResult, MergerConfig,
    ResultMerger, State as MergerState, TaskResult as MergerTaskResult, WorkerId as MergerWorkerId,
};
pub use monitor::{
    BlockerDetector, BlockerInfo, ProgressMonitor, ProgressTracker, StatusReport, StatusReporter,
    TaskCounts, TaskProgress,
};
pub use react::{
    Action, ActionExecution, ActionProposal, DualThreadReactAgent, InterruptSignal, Observation,
    ReactCycle, ReactStats, Tool, ToolSet,
};
pub use state_machine::{GlobalState, StateMachine, StateTransition, TransitionCondition};
pub use task::{Priority, Task, TaskStatus};
pub use task_queue::{
    DependencyTracker, DependencyTrackerError, LoadBalancer, LoadBalancingTask, PriorityScheduler,
    PrioritySchedulerError, QueueStats, QueueTaskId, QueuedTask, TaskQueue, TaskQueueConfig,
    TaskQueueError, TaskQueueStatus, WorkerAssignment,
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
