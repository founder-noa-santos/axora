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
pub mod react;
pub mod state_machine;
pub mod task;

pub use aci_formatter::{
    ACIConfig, ACIFormatter, TokenSavings,
};
pub use agent::{
    Agent, AgentState, ArchitectAgent, BaseAgent, CoderAgent, DebuggerAgent,
    ReviewerAgent, TaskResult as AgentTaskResult, TesterAgent,
};
pub use capabilities::{AgentCapabilities, CapabilityRegistry, TaskRequirements};
pub use communication::{
    AgentMessage, CommunicationProtocol, Envelope, MessageBus, MessageType,
};
pub use conflict::{
    Conflict, ConflictResolution, ConflictStatus, ConflictType, ConflictResolver,
    Decision, Resolution, Vote,
};
pub use coordinator::{
    Coordinator, CoordinatorStats, DAG, MissionResult, TaskResult,
};
pub use decomposer::{
    DecomposedMission, DecompositionRule, Dependency, DependencyType, MissionDecomposer,
    MissionTemplate, TaskTemplate, TaskId,
};
pub use error::AgentError;
pub use executor::{ConcurrentExecutor, ExecutorConfig, ExecutorStats, MissionResult as ExecutorMissionResult};
pub use graph::{
    Edge, ExecutionMode, ExecutionState, GraphStats, Node, NodeId, NodeRole, ParallelismDetector,
    TransitionCondition as GraphTransitionCondition, WorkflowGraph,
};
pub use heartbeat::{
    AgentSleepState, Heartbeat, HeartbeatConfig, HeartbeatEvent, HeartbeatMessage,
};
pub use memory::{MemoryEntry, MemoryStore, MemoryType, SharedBlackboard};
pub use react::{
    Action, ActionProposal, ActionExecution, DualThreadReactAgent, InterruptSignal,
    Observation, ReactCycle, ReactStats, Tool, ToolSet,
};
pub use state_machine::{GlobalState, StateMachine, StateTransition, TransitionCondition};
pub use task::{Priority, Task, TaskStatus};

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
