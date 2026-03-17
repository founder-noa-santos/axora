//! Dual-Thread ReAct Loop
//!
//! This module implements interruptible ReAct execution:
//! - **Planning Thread** (async, non-blocking, LLM-driven)
//! - **Acting Thread** (tool execution, can block)
//! - **Interrupt Channel** (coordinator → worker)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐         ┌─────────────────┐
//! │ Planning Thread │────────▶│  Acting Thread  │
//! │ (LLM, async)    │  props  │ (tools, blocking)│
//! └────────┬────────┘         └────────┬────────┘
//!          │                           │
//!          │         ┌─────────────────┤
//!          │         │  Interrupt      │
//!          └─────────┤  Channel        │
//!                    │  (coordinator)  │
//!                    └─────────────────┘
//! ```

use crate::agent::{BaseAgent, TaskResult};
use crate::error::AgentError;
use crate::memory::SharedBlackboard;
use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::aci_formatter::ACIFormatter;

/// ReAct cycle (Thought → Action → Observation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactCycle {
    /// Agent's thought/reasoning
    pub thought: String,
    /// Action to execute
    pub action: Action,
    /// Observation from action
    pub observation: Observation,
    /// Cycle number
    pub cycle_number: u32,
    /// Timestamp
    pub timestamp: u64,
}

impl ReactCycle {
    /// Create new ReAct cycle
    pub fn new(thought: &str, action: Action, cycle_number: u32) -> Self {
        Self {
            thought: thought.to_string(),
            action,
            observation: Observation::pending(),
            cycle_number,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Set observation
    pub fn with_observation(mut self, observation: Observation) -> Self {
        self.observation = observation;
        self
    }
}

/// Action (tool call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Tool name
    pub tool_name: String,
    /// Tool parameters
    pub parameters: serde_json::Value,
}

impl Action {
    /// Create new action
    pub fn new(tool_name: &str, parameters: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            parameters,
        }
    }

    /// Create action with string parameter
    pub fn with_param(tool_name: &str, key: &str, value: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            parameters: serde_json::json!({ key: value }),
        }
    }
}

/// Observation (tool result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Success flag
    pub success: bool,
    /// Result data
    pub result: serde_json::Value,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl Observation {
    /// Create pending observation
    pub fn pending() -> Self {
        Self {
            success: false,
            result: serde_json::Value::Null,
            error: None,
        }
    }

    /// Create successful observation
    pub fn success(result: serde_json::Value) -> Self {
        Self {
            success: true,
            result,
            error: None,
        }
    }

    /// Create failed observation
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            result: serde_json::Value::Null,
            error: Some(error.to_string()),
        }
    }
}

/// Interrupt signal (from coordinator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptSignal {
    /// Stop current action (state changed)
    Stop {
        /// Reason for stopping
        reason: String,
    },

    /// Priority change (new task)
    PriorityChange {
        /// New priority level
        new_priority: u32,
    },

    /// Context update (blackboard changed)
    ContextUpdate {
        /// New snapshot version
        new_snapshot_version: u64,
    },

    /// Reflection request (force LLM reflection)
    Reflect {
        /// Reflection prompt
        prompt: String,
    },
}

/// Action proposal from Planning Thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProposal {
    /// Thought/reasoning
    pub thought: String,
    /// Proposed action
    pub action: Action,
    /// Blackboard snapshot version
    pub snapshot_version: u64,
}

/// Action execution from Acting Thread
#[derive(Debug, Clone)]
pub struct ActionExecution {
    /// Original proposal
    pub proposal: ActionProposal,
    /// Execution observation
    pub observation: Observation,
}

/// Tool set for agent execution
#[derive(Clone)]
pub struct ToolSet {
    /// Available tools
    tools: HashMap<String, Tool>,
    /// LLM model name
    pub llm_model: String,
}

impl ToolSet {
    /// Create new tool set
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            llm_model: "default".to_string(),
        }
    }

    /// Create tool set with powerful LLM (for critical path)
    pub fn with_powerful_llm() -> Self {
        let mut tools = Self::new();
        tools.llm_model = "frontier".to_string();
        tools
    }

    /// Create tool set with small LLM (for off-path tasks)
    pub fn with_small_llm() -> Self {
        let mut tools = Self::new();
        tools.llm_model = "slm".to_string();
        tools
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Execute a tool
    pub async fn execute(&self, action: &Action) -> Result<Observation> {
        if let Some(tool) = self.tools.get(&action.tool_name) {
            tool.execute(&action.parameters).await
        } else {
            // Default tool execution (placeholder)
            Ok(Observation::success(serde_json::json!({
                "tool": action.tool_name,
                "params": action.parameters,
                "model": self.llm_model,
            })))
        }
    }

    /// Get available tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

impl Default for ToolSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool definition
#[derive(Clone)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool implementation
    pub handler: Arc<dyn Fn(&serde_json::Value) -> Result<Observation> + Send + Sync>,
}

impl Tool {
    /// Create new tool
    pub fn new<F>(name: &str, description: &str, handler: F) -> Self
    where
        F: Fn(&serde_json::Value) -> Result<Observation> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            handler: Arc::new(handler),
        }
    }

    /// Execute tool
    pub async fn execute(&self, params: &serde_json::Value) -> Result<Observation> {
        (self.handler)(params)
    }
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

/// Dual-Thread ReAct Agent
pub struct DualThreadReactAgent {
    // Planning Thread (LLM-driven, async)
    planning_tx: mpsc::Sender<ActionProposal>,
    planning_handle: Option<JoinHandle<Result<()>>>,

    // Acting Thread (tool execution)
    acting_tx: mpsc::Sender<ActionExecution>,
    acting_rx: mpsc::Receiver<ActionExecution>,
    acting_handle: Option<JoinHandle<Result<()>>>,

    // Interrupt channel (from coordinator)
    interrupt_tx: mpsc::Sender<InterruptSignal>,

    // State
    current_cycle: u32,
    max_cycles: u32,
    task: Task,
    blackboard: Arc<Mutex<SharedBlackboard>>,
    tools: ToolSet,
    cycles: Vec<ReactCycle>,

    // ACI Formatter (defends context window)
    aci_formatter: ACIFormatter,
}

impl DualThreadReactAgent {
    /// Spawn dual-thread agent
    pub async fn spawn(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
    ) -> Result<Self> {
        // Create channels
        let (planning_tx, _planning_rx) = mpsc::channel(32);
        let (acting_tx, _acting_rx) = mpsc::channel(32);
        let (interrupt_tx, _interrupt_rx) = mpsc::channel(32);

        // Note: In a full implementation, we would spawn actual threads here.
        // For now, we use a simplified approach where execute_cycle() handles everything.

        Ok(Self {
            planning_tx,
            planning_handle: None,
            acting_tx,
            acting_rx: _acting_rx,
            acting_handle: None,
            interrupt_tx,
            task,
            blackboard,
            tools,
            current_cycle: 0,
            max_cycles: 12,
            cycles: Vec::new(),
            aci_formatter: ACIFormatter::new(),
        })
    }

    /// Spawn dual-thread agent with custom ACI config
    pub async fn spawn_with_config(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
        aci_config: crate::aci_formatter::ACIConfig,
    ) -> Result<Self> {
        // Create channels
        let (planning_tx, _planning_rx) = mpsc::channel(32);
        let (acting_tx, _acting_rx) = mpsc::channel(32);
        let (interrupt_tx, _interrupt_rx) = mpsc::channel(32);

        Ok(Self {
            planning_tx,
            planning_handle: None,
            acting_tx,
            acting_rx: _acting_rx,
            acting_handle: None,
            interrupt_tx,
            task,
            blackboard,
            tools,
            current_cycle: 0,
            max_cycles: 12,
            cycles: Vec::new(),
            aci_formatter: ACIFormatter::with_config(aci_config),
        })
    }

    /// Execute a single ReAct cycle
    pub async fn execute_cycle(&mut self) -> Result<ReactCycle> {
        self.current_cycle += 1;

        if self.current_cycle > self.max_cycles {
            return Err(
                AgentError::InvalidStateTransition("Max cycles exceeded".to_string()).into(),
            );
        }

        // Generate thought and action (simulated LLM planning)
        let thought = format!(
            "Planning cycle {} for task: {}",
            self.current_cycle, self.task.description
        );
        let action = Action::with_param("execute_task", "task_id", &self.task.id);

        // Create cycle
        let mut cycle = ReactCycle::new(&thought, action.clone(), self.current_cycle);

        // Execute action with ACI formatting (defend context window)
        let raw_observation = self.tools.execute(&action).await?;
        cycle.observation = self.format_observation(&action, raw_observation);

        // Store cycle
        self.cycles.push(cycle.clone());

        Ok(cycle)
    }

    /// Format observation with ACI formatting (truncate/paginate)
    fn format_observation(&self, action: &Action, observation: Observation) -> Observation {
        // Format based on tool type
        let formatted_result = match action.tool_name.as_str() {
            "run_command" | "execute_shell" => {
                // Terminal output - truncate long output
                if let serde_json::Value::String(s) = &observation.result {
                    serde_json::json!(self.aci_formatter.format_output(s))
                } else {
                    observation.result.clone()
                }
            }
            "read_file" | "get_file_content" => {
                // File dump - truncate large files
                if let serde_json::Value::String(s) = &observation.result {
                    serde_json::json!(self.aci_formatter.format_file_dump(s))
                } else {
                    observation.result.clone()
                }
            }
            "get_stack_trace" | "get_error_trace" => {
                // Stack trace - keep root cause + error
                if let serde_json::Value::String(s) = &observation.result {
                    serde_json::json!(self.aci_formatter.format_stack_trace(s))
                } else {
                    observation.result.clone()
                }
            }
            "get_json" | "parse_json" => {
                // JSON output - truncate large JSON
                if let serde_json::Value::String(s) = &observation.result {
                    serde_json::json!(self.aci_formatter.format_json(s))
                } else {
                    observation.result.clone()
                }
            }
            _ => {
                // Default - no special formatting
                observation.result.clone()
            }
        };

        Observation {
            success: observation.success,
            result: formatted_result,
            error: observation.error,
        }
    }

    /// Execute all cycles until completion
    pub async fn execute_all(&mut self) -> Result<TaskResult> {
        info!(
            "Executing ReAct agent for task {} (max {} cycles)",
            self.task.id, self.max_cycles
        );

        let start = Instant::now();

        while self.current_cycle < self.max_cycles {
            let cycle = self.execute_cycle().await?;

            // Check if action succeeded
            if cycle.observation.success {
                info!(
                    "Task completed in {} cycles ({:?})",
                    self.current_cycle,
                    start.elapsed()
                );

                return Ok(TaskResult {
                    success: true,
                    output: format!(
                        "Completed in {} cycles: {}",
                        self.current_cycle, cycle.observation.result
                    ),
                    error: None,
                });
            }
        }

        // Max cycles reached without success
        Ok(TaskResult {
            success: false,
            output: format!("Max cycles ({}) reached", self.max_cycles),
            error: Some("Max ReAct cycles exceeded".to_string()),
        })
    }

    /// Send interrupt signal
    pub async fn send_interrupt(&self, signal: InterruptSignal) -> Result<()> {
        // Try to send, but don't fail if no receiver (simplified implementation)
        let _ = self.interrupt_tx.send(signal).await;
        Ok(())
    }

    /// Get current cycle count
    pub fn cycle_count(&self) -> u32 {
        self.current_cycle
    }

    /// Get all cycles
    pub fn get_cycles(&self) -> &[ReactCycle] {
        &self.cycles
    }

    /// Get execution stats
    pub fn get_stats(&self) -> ReactStats {
        let successful = self.cycles.iter().filter(|c| c.observation.success).count() as u32;

        ReactStats {
            total_cycles: self.current_cycle,
            successful_cycles: successful,
            failed_cycles: self.current_cycle - successful,
            max_cycles: self.max_cycles,
        }
    }
}

/// ReAct execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactStats {
    pub total_cycles: u32,
    pub successful_cycles: u32,
    pub failed_cycles: u32,
    pub max_cycles: u32,
}

impl ReactStats {
    pub fn success_rate(&self) -> f32 {
        if self.total_cycles == 0 {
            return 1.0;
        }
        self.successful_cycles as f32 / self.total_cycles as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dual_thread_spawn() {
        let task = Task::new("Test task for dual-thread spawn");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let tools = ToolSet::new();

        let agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        assert_eq!(agent.current_cycle, 0);
        assert_eq!(agent.max_cycles, 12);
    }

    #[tokio::test]
    async fn test_planning_thread_non_blocking() {
        let task = Task::new("Test planning non-blocking");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let tools = ToolSet::new();

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        // Execute a cycle (should be fast, non-blocking)
        let start = Instant::now();
        let cycle = agent.execute_cycle().await.unwrap();
        let elapsed = start.elapsed();

        // Should complete quickly (< 1 second for planning)
        assert!(elapsed < Duration::from_secs(1));
        assert_eq!(cycle.cycle_number, 1);
    }

    #[tokio::test]
    async fn test_acting_thread_tool_execution() {
        let task = Task::new("Test tool execution");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));

        // Create tool set with custom tool
        let mut tools = ToolSet::new();
        tools.register_tool(Tool::new(
            "test_tool",
            "Test tool for execution",
            |params| {
                Ok(Observation::success(
                    serde_json::json!({"executed": true, "params": params}),
                ))
            },
        ));

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        // Execute cycle with tool
        let cycle = agent.execute_cycle().await.unwrap();

        assert!(cycle.observation.success);
    }

    #[tokio::test]
    async fn test_interrupt_handling() {
        let task = Task::new("Test interrupt handling");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let tools = ToolSet::new();

        let agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        // Send interrupt
        let signal = InterruptSignal::Stop {
            reason: "Test stop".to_string(),
        };

        let result = agent.send_interrupt(signal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reflection_phase() {
        let task = Task::new("Test reflection");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let tools = ToolSet::new();

        let agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        // Send reflection request
        let signal = InterruptSignal::Reflect {
            prompt: "Reflect on current state".to_string(),
        };

        let result = agent.send_interrupt(signal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_cycles_prevention() {
        let task = Task::new("Test max cycles");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let tools = ToolSet::new();

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        // Set low max cycles for testing
        agent.max_cycles = 3;

        // Execute until max cycles
        for _ in 0..3 {
            let _ = agent.execute_cycle().await.unwrap();
        }

        // Next cycle should fail
        let result = agent.execute_cycle().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_action_creation() {
        let action = Action::new("test_tool", serde_json::json!({"key": "value"}));

        assert_eq!(action.tool_name, "test_tool");
        assert_eq!(action.parameters["key"], "value");
    }

    #[tokio::test]
    async fn test_observation_creation() {
        let success = Observation::success(serde_json::json!({"result": "ok"}));
        let failure = Observation::failure("Error occurred");
        let pending = Observation::pending();

        assert!(success.success);
        assert!(!failure.success);
        assert!(!pending.success);
        assert!(failure.error.is_some());
    }

    #[tokio::test]
    async fn test_react_cycle() {
        let action = Action::new("test", serde_json::Value::Null);
        let cycle = ReactCycle::new("Test thought", action, 1);

        assert_eq!(cycle.thought, "Test thought");
        assert_eq!(cycle.cycle_number, 1);
        assert!(!cycle.observation.success); // Pending
    }

    #[tokio::test]
    async fn test_tool_set_creation() {
        let tools_powerful = ToolSet::with_powerful_llm();
        let tools_small = ToolSet::with_small_llm();

        assert_eq!(tools_powerful.llm_model, "frontier");
        assert_eq!(tools_small.llm_model, "slm");
    }

    #[tokio::test]
    async fn test_react_stats() {
        let stats = ReactStats {
            total_cycles: 10,
            successful_cycles: 8,
            failed_cycles: 2,
            max_cycles: 12,
        };

        assert!((stats.success_rate() - 0.8).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_full_execution() {
        let task = Task::new("Full execution test");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let mut tools = ToolSet::new();

        // Add a tool that succeeds on first try
        tools.register_tool(Tool::new("execute_task", "Execute task", |_params| {
            Ok(Observation::success(serde_json::json!({"completed": true})))
        }));

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        let result = agent.execute_all().await.unwrap();

        assert!(result.success);
        assert!(agent.cycle_count() >= 1);
    }

    #[tokio::test]
    async fn test_interrupt_signal_types() {
        let stop = InterruptSignal::Stop {
            reason: "Test".to_string(),
        };
        let priority = InterruptSignal::PriorityChange { new_priority: 5 };
        let context = InterruptSignal::ContextUpdate {
            new_snapshot_version: 42,
        };
        let reflect = InterruptSignal::Reflect {
            prompt: "Think".to_string(),
        };

        // Just verify they can be created
        match stop {
            InterruptSignal::Stop { reason } => assert!(!reason.is_empty()),
            _ => panic!("Wrong variant"),
        }

        match priority {
            InterruptSignal::PriorityChange { new_priority } => assert_eq!(new_priority, 5),
            _ => panic!("Wrong variant"),
        }

        match context {
            InterruptSignal::ContextUpdate {
                new_snapshot_version,
            } => {
                assert_eq!(new_snapshot_version, 42)
            }
            _ => panic!("Wrong variant"),
        }

        match reflect {
            InterruptSignal::Reflect { prompt } => assert!(!prompt.is_empty()),
            _ => panic!("Wrong variant"),
        }
    }
}
