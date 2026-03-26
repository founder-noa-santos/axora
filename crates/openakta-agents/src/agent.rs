//! Agent trait and implementations

use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    /// Idle, waiting for task
    Idle,
    /// Thinking/planning
    Thinking,
    /// Executing task
    Executing,
    /// Waiting for review
    WaitingForReview,
    /// Blocked
    Blocked,
    /// Task completed
    Completed,
}

/// Agent trait
pub trait Agent: Send + Sync {
    /// Get agent ID
    fn id(&self) -> &str;

    /// Get agent name
    fn name(&self) -> &str;

    /// Get agent role
    fn role(&self) -> &str;

    /// Execute task
    fn execute(&mut self, task: Task) -> Result<TaskResult>;
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Success or failure
    pub success: bool,
    /// Output content
    pub output: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Base agent implementation
pub struct BaseAgent {
    id: String,
    name: String,
    role: String,
}

impl BaseAgent {
    /// Create new base agent
    pub fn new(name: &str, role: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            role: role.to_string(),
        }
    }
}

impl Agent for BaseAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn role(&self) -> &str {
        &self.role
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // Placeholder implementation
        Ok(TaskResult {
            success: true,
            output: format!("Task {} completed by {}", task.id, self.name),
            error: None,
        })
    }
}

/// Architect agent - responsible for design and structure
pub struct ArchitectAgent {
    base: BaseAgent,
}

impl ArchitectAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Arquiteto", "Architect"),
        }
    }
}

impl Default for ArchitectAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for ArchitectAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // TODO: Implement architect logic
        Ok(TaskResult {
            success: true,
            output: format!("Architecture designed for: {}", task.description),
            error: None,
        })
    }
}

/// Coder agent - responsible for implementation
pub struct CoderAgent {
    base: BaseAgent,
}

impl CoderAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Coder", "Developer"),
        }
    }
}

impl Default for CoderAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CoderAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // TODO: Implement coding logic
        Ok(TaskResult {
            success: true,
            output: format!("Code implemented for: {}", task.description),
            error: None,
        })
    }
}

/// Refactorer agent - responsible for staged, scripted codebase refactors.
pub struct RefactorerAgent {
    base: BaseAgent,
}

impl RefactorerAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Refactorer", "Refactor Specialist"),
        }
    }
}

impl Default for RefactorerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for RefactorerAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        Ok(TaskResult {
            success: true,
            output: format!("Sandboxed refactor prepared for: {}", task.description),
            error: None,
        })
    }
}

/// Reviewer agent - responsible for code review
///
/// ## Phase 7 Note
///
/// This agent previously used direct cloud transports. In Phase 7+, cloud execution
/// uses the API client pool. The reviewer functionality is now handled directly by
/// the coordinator's `execute_arbiter_review` method using the API client.
pub struct ReviewerAgent {
    base: BaseAgent,
}

impl ReviewerAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Reviewer", "Code Reviewer"),
        }
    }
}

impl Default for ReviewerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for ReviewerAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // Phase 7+: ReviewerAgent no longer executes reviews directly.
        // Use coordinator's execute_arbiter_review() with API client instead.
        Ok(TaskResult {
            success: true,
            output: format!("Code reviewed for: {}", task.description),
            error: None,
        })
    }
}

/// Tester agent - responsible for test generation
pub struct TesterAgent {
    base: BaseAgent,
}

impl TesterAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Tester", "QA Engineer"),
        }
    }
}

impl Default for TesterAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for TesterAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // TODO: Implement testing logic
        Ok(TaskResult {
            success: true,
            output: format!("Tests generated for: {}", task.description),
            error: None,
        })
    }
}

/// Debugger agent - responsible for bug fixing
pub struct DebuggerAgent {
    base: BaseAgent,
}

impl DebuggerAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Debugger", "Debug Specialist"),
        }
    }
}

impl Default for DebuggerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for DebuggerAgent {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn role(&self) -> &str {
        self.base.role()
    }

    fn execute(&mut self, task: Task) -> Result<TaskResult> {
        // TODO: Implement debugging logic
        Ok(TaskResult {
            success: true,
            output: format!("Bug fixed for: {}", task.description),
            error: None,
        })
    }
}
