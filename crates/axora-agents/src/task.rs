//! Task definition

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task created, waiting to be assigned
    Pending,
    /// Task assigned to agent
    Assigned,
    /// Task in progress
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task cancelled
    Cancelled,
}

/// Task for agent to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub id: String,
    /// Task description
    pub description: String,
    /// Task priority
    pub priority: Priority,
    /// Task status
    pub status: TaskStatus,
    /// Assigned agent ID
    pub assigned_to: Option<String>,
    /// Parent task (for subtasks)
    pub parent_task: Option<String>,
    /// High-level task type used for transport and result validation.
    pub task_type: TaskType,
}

/// Task priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// High-level task type used by the orchestration protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    /// Planning or coordination work.
    General,
    /// Code editing work that must emit unified diffs.
    CodeModification,
    /// Review or validation work.
    Review,
    /// Retrieval or indexing work.
    Retrieval,
}

impl Task {
    /// Create new task
    pub fn new(description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            priority: Priority::Normal,
            status: TaskStatus::Pending,
            assigned_to: None,
            parent_task: None,
            task_type: TaskType::General,
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set parent task
    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_task = Some(parent_id.to_string());
        self
    }

    /// Set task type.
    pub fn with_task_type(mut self, task_type: TaskType) -> Self {
        self.task_type = task_type;
        self
    }

    /// Assign to agent
    pub fn assign(&mut self, agent_id: &str) {
        self.assigned_to = Some(agent_id.to_string());
        self.status = TaskStatus::Assigned;
    }

    /// Mark as in progress
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
    }

    /// Mark as failed
    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
    }

    /// Returns true when the task must produce a patch-only result.
    pub fn is_code_modification(&self) -> bool {
        self.task_type == TaskType::CodeModification
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task");
        assert_eq!(task.description, "Test task");
        assert_eq!(task.priority, Priority::Normal);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.assigned_to.is_none());
    }

    #[test]
    fn test_task_priority() {
        let task = Task::new("Test task").with_priority(Priority::High);
        assert_eq!(task.priority, Priority::High);
    }

    #[test]
    fn test_task_assignment() {
        let mut task = Task::new("Test task");
        task.assign("agent1");

        assert_eq!(task.assigned_to, Some("agent1".to_string()));
        assert_eq!(task.status, TaskStatus::Assigned);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("Test task");
        task.assign("agent1");
        task.start();
        task.complete();

        assert_eq!(task.status, TaskStatus::Completed);
    }
}
