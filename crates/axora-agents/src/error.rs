//! Agent errors

use thiserror::Error;

/// Agent operation errors
#[derive(Error, Debug)]
pub enum AgentError {
    /// Agent not found
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    /// Task execution failed
    #[error("task execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid state transition
    #[error("invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// Timeout
    #[error("operation timed out")]
    Timeout,

    /// Task not found
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// Invalid mission decomposition
    #[error("invalid decomposition: {0}")]
    InvalidDecomposition(String),

    /// Dependency graph validation failed
    #[error("graph validation failed: {0}")]
    GraphValidation(String),

    /// Serialization or parsing failed
    #[error("serialization failed: {0}")]
    Serialization(String),
}
