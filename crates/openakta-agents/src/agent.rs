//! Agent trait and implementations

use crate::error::AgentError;
use crate::provider::{
    CacheRetention, ModelBoundaryPayload, ModelBoundaryPayloadType, ModelRequest,
};
use crate::provider_transport::{ProviderTransport, ProviderTransportError};
use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
pub struct ReviewerAgent {
    base: BaseAgent,
    cloud_transport: Option<Arc<dyn ProviderTransport>>,
    wire_profile: Option<crate::wire_profile::WireProfile>,
    model: Option<String>,
}

impl ReviewerAgent {
    pub fn new() -> Self {
        Self {
            base: BaseAgent::new("Reviewer", "Code Reviewer"),
            cloud_transport: None,
            wire_profile: None,
            model: None,
        }
    }

    /// Create a reviewer agent bound to a cloud transport.
    pub fn with_cloud_transport(
        wire_profile: crate::wire_profile::WireProfile,
        model: impl Into<String>,
        transport: Arc<dyn ProviderTransport>,
    ) -> Self {
        Self {
            base: BaseAgent::new("Reviewer", "Code Reviewer"),
            cloud_transport: Some(transport),
            wire_profile: Some(wire_profile),
            model: Some(model.into()),
        }
    }

    async fn execute_async(&self, task: Task) -> Result<TaskResult> {
        let wire_profile = self.wire_profile.ok_or_else(|| {
            AgentError::ExecutionFailed("reviewer cloud provider is not configured".to_string())
        })?;
        let model = self.model.clone().ok_or_else(|| {
            AgentError::ExecutionFailed("reviewer cloud model is not configured".to_string())
        })?;
        let transport = self.cloud_transport.as_ref().ok_or_else(|| {
            AgentError::ExecutionFailed("reviewer cloud transport is not configured".to_string())
        })?;

        let request = ModelRequest {
            provider: wire_profile,
            model,
            system_instructions: vec![
                "You are the OPENAKTA cloud arbiter. Review the failed local output, repair it when possible, and return only the corrected result payload.".to_string(),
            ],
            tool_schemas: Vec::new(),
            invariant_mission_context: Vec::new(),
            payload: ModelBoundaryPayload {
                payload_type: ModelBoundaryPayloadType::TaskExecution,
                task_id: task.id.clone(),
                title: "OPENAKTA arbitration review".to_string(),
                description: task.description.clone(),
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

        let response = transport.execute(&request).await.map_err(|err| match err {
            ProviderTransportError::CloudExecutionUnavailable(message) => {
                AgentError::CloudExecutionUnavailable(message)
            }
            ProviderTransportError::CloudExecutionRequired(message) => {
                AgentError::CloudExecutionRequired(message)
            }
            other => AgentError::ExecutionFailed(other.to_string()),
        })?;

        Ok(TaskResult {
            success: true,
            output: response.output_text,
            error: None,
        })
    }

    /// Execute a cloud-backed review through the async mainline.
    pub async fn execute_review(&self, task: Task) -> Result<TaskResult> {
        self.execute_async(task).await
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
        if self.cloud_transport.is_none() {
            return Ok(TaskResult {
                success: true,
                output: format!("Code reviewed for: {}", task.description),
                error: None,
            });
        }

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| handle.block_on(self.execute_async(task)))
        } else {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|err| AgentError::ExecutionFailed(err.to_string()))?
                .block_on(self.execute_async(task))
        }
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
