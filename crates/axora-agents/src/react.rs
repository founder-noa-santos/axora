//! Dual-thread ReAct runtime with planner/actor separation.

use crate::aci_formatter::ACIFormatter;
use crate::agent::TaskResult;
use crate::error::AgentError;
use crate::mcp_client::McpClient;
use crate::memory::SharedBlackboard;
use crate::task::Task;
use crate::Result;
use axora_memory::{EpisodicStore, EpisodicStoreConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::info;

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
        Self::new(tool_name, serde_json::json!({ key: value }))
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
    tools: HashMap<String, Tool>,
    /// LLM model name
    pub llm_model: String,
    mcp_client: Option<McpClient>,
    workspace_root: String,
}

impl ToolSet {
    /// Create new tool set
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            llm_model: "default".to_string(),
            mcp_client: None,
            workspace_root: std::env::current_dir()
                .unwrap_or_else(|_| ".".into())
                .display()
                .to_string(),
        }
    }

    /// Create a tool set that routes non-local tools through MCP.
    pub fn with_mcp_endpoint(endpoint: impl Into<String>, llm_model: impl Into<String>) -> Self {
        let mut tools = Self::new();
        tools.llm_model = llm_model.into();
        tools.mcp_client = Some(McpClient::new(endpoint));
        tools
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Execute a tool
    pub async fn execute(&self, action: &Action) -> Result<Observation> {
        if let Some(tool) = self.tools.get(&action.tool_name) {
            return tool.execute(&action.parameters).await;
        }

        if let Some(client) = &self.mcp_client {
            let args = value_to_struct(&action.parameters);
            let request_id = format!("react-{}", uuid::Uuid::new_v4());
            return client
                .call_tool(
                    &request_id,
                    "react-agent",
                    &self.llm_model,
                    &action.tool_name,
                    &self.workspace_root,
                    args,
                    None,
                )
                .await
                .map_err(|err| AgentError::ExecutionFailed(err.to_string()).into());
        }

        if matches!(
            action.tool_name.as_str(),
            "run_command" | "execute_shell" | "read_file" | "get_file_content"
        ) {
            return Err(AgentError::ExecutionFailed(format!(
                "tool '{}' requires MCP and cannot run locally",
                action.tool_name
            ))
            .into());
        }

        Err(AgentError::ExecutionFailed(format!(
            "tool '{}' is not registered and MCP is not configured",
            action.tool_name
        ))
        .into())
    }

    /// Get available tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    fn has_mcp(&self) -> bool {
        self.mcp_client.is_some()
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

#[derive(Debug)]
struct PlannerRequest {
    cycle_number: u32,
    response_tx: oneshot::Sender<Result<ActionProposal>>,
}

#[derive(Debug)]
struct ActorRequest {
    proposal: ActionProposal,
    response_tx: oneshot::Sender<Result<ActionExecution>>,
}

/// Dual-Thread ReAct Agent
pub struct DualThreadReactAgent {
    planning_tx: mpsc::Sender<PlannerRequest>,
    planning_handle: Option<JoinHandle<Result<()>>>,
    acting_tx: mpsc::Sender<ActorRequest>,
    acting_handle: Option<JoinHandle<Result<()>>>,
    interrupt_tx: broadcast::Sender<InterruptSignal>,
    current_cycle: u32,
    max_cycles: u32,
    task: Task,
    cycles: Vec<ReactCycle>,
    session_id: String,
    episodic_store_path: Option<String>,
    aci_formatter: ACIFormatter,
}

impl DualThreadReactAgent {
    /// Spawn dual-thread agent
    pub async fn spawn(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
    ) -> Result<Self> {
        Self::spawn_internal(task, blackboard, tools, ACIFormatter::new(), None).await
    }

    /// Spawn dual-thread agent with custom ACI config
    pub async fn spawn_with_config(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
        aci_config: crate::aci_formatter::ACIConfig,
    ) -> Result<Self> {
        Self::spawn_internal(
            task,
            blackboard,
            tools,
            ACIFormatter::with_config(aci_config),
            None,
        )
        .await
    }

    /// Spawn dual-thread agent with episodic logging.
    pub async fn spawn_with_memory(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
        episodic_store_path: String,
    ) -> Result<Self> {
        Self::spawn_internal(
            task,
            blackboard,
            tools,
            ACIFormatter::new(),
            Some(episodic_store_path),
        )
        .await
    }

    async fn spawn_internal(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
        aci_formatter: ACIFormatter,
        episodic_store_path: Option<String>,
    ) -> Result<Self> {
        let (planning_tx, planning_rx) = mpsc::channel(32);
        let (acting_tx, acting_rx) = mpsc::channel(32);
        let (interrupt_tx, _) = broadcast::channel(32);

        let planner_handle = tokio::spawn(planner_loop(
            task.clone(),
            Arc::clone(&blackboard),
            tools.clone(),
            planning_rx,
            interrupt_tx.subscribe(),
        ));
        let actor_handle = tokio::spawn(actor_loop(
            tools,
            acting_rx,
            interrupt_tx.subscribe(),
        ));

        Ok(Self {
            planning_tx,
            planning_handle: Some(planner_handle),
            acting_tx,
            acting_handle: Some(actor_handle),
            interrupt_tx,
            current_cycle: 0,
            max_cycles: 12,
            session_id: task.id.clone(),
            task,
            cycles: Vec::new(),
            episodic_store_path,
            aci_formatter,
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

        let (plan_tx, plan_rx) = oneshot::channel();
        self.planning_tx
            .send(PlannerRequest {
                cycle_number: self.current_cycle,
                response_tx: plan_tx,
            })
            .await
            .map_err(|_| AgentError::ExecutionFailed("planner unavailable".to_string()))?;
        let proposal = plan_rx
            .await
            .map_err(|_| AgentError::ExecutionFailed("planner dropped".to_string()))??;

        let mut cycle = ReactCycle::new(&proposal.thought, proposal.action.clone(), self.current_cycle);
        self.log_thought(&cycle.thought).await;

        let (actor_tx, actor_rx) = oneshot::channel();
        self.acting_tx
            .send(ActorRequest {
                proposal: proposal.clone(),
                response_tx: actor_tx,
            })
            .await
            .map_err(|_| AgentError::ExecutionFailed("actor unavailable".to_string()))?;
        let execution = actor_rx
            .await
            .map_err(|_| AgentError::ExecutionFailed("actor dropped".to_string()))??;
        cycle.observation = self.format_observation(&cycle.action, execution.observation);
        self.log_action(&cycle.action, &cycle.observation).await;

        self.cycles.push(cycle.clone());
        Ok(cycle)
    }

    fn format_observation(&self, action: &Action, observation: Observation) -> Observation {
        let formatted_result = match action.tool_name.as_str() {
            "run_command" | "execute_shell" => {
                extract_stdout(&observation.result)
                    .map(|stdout| serde_json::json!(self.aci_formatter.format_output(stdout)))
                    .unwrap_or_else(|| observation.result.clone())
            }
            "read_file" | "get_file_content" => {
                if let serde_json::Value::String(s) = &observation.result {
                    serde_json::json!(self.aci_formatter.format_file_dump(s))
                } else {
                    observation.result.clone()
                }
            }
            _ => observation.result.clone(),
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
            if cycle.observation.success {
                self.log_success(&cycle.observation.result.to_string()).await;
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

        self.log_failure("Max ReAct cycles exceeded").await;
        Ok(TaskResult {
            success: false,
            output: format!("Max cycles ({}) reached", self.max_cycles),
            error: Some("Max ReAct cycles exceeded".to_string()),
        })
    }

    /// Send interrupt signal
    pub async fn send_interrupt(&self, signal: InterruptSignal) -> Result<()> {
        let _ = self.interrupt_tx.send(signal);
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
            failed_cycles: self.current_cycle.saturating_sub(successful),
            max_cycles: self.max_cycles,
        }
    }

    async fn log_thought(&self, thought: &str) {
        if let Some(store) = &self.episodic_store_path {
            let session_id = self.session_id.clone();
            let cycle = self.current_cycle as i32;
            let thought = thought.to_string();
            let store = store.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("episodic thought runtime");
                runtime.block_on(async move {
                    let db = EpisodicStore::new(EpisodicStoreConfig::persistent(&store)).await?;
                    db.log_conversation(&session_id, cycle, &thought).await
                })
            })
            .await;
        }
    }

    async fn log_action(&self, action: &Action, observation: &Observation) {
        if let Some(store) = &self.episodic_store_path {
            let output = observation.result.to_string();
            let tool_name = action.tool_name.clone();
            let session_id = self.session_id.clone();
            let cycle = self.current_cycle as i32;
            let success = observation.success;
            let store = store.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("episodic action runtime");
                runtime.block_on(async move {
                    let db = EpisodicStore::new(EpisodicStoreConfig::persistent(&store)).await?;
                    db.log_action(&session_id, cycle, &tool_name, &output, success)
                        .await
                })
            })
            .await;
        }
    }

    async fn log_success(&self, content: &str) {
        if let Some(store) = &self.episodic_store_path {
            let session_id = self.session_id.clone();
            let cycle = self.current_cycle as i32;
            let content = content.to_string();
            let store = store.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("episodic success runtime");
                runtime.block_on(async move {
                    let db = EpisodicStore::new(EpisodicStoreConfig::persistent(&store)).await?;
                    db.log_success(&session_id, cycle, &content).await
                })
            })
            .await;
        }
    }

    async fn log_failure(&self, content: &str) {
        if let Some(store) = &self.episodic_store_path {
            let session_id = self.session_id.clone();
            let cycle = self.current_cycle as i32;
            let content = content.to_string();
            let store = store.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("episodic failure runtime");
                runtime.block_on(async move {
                    let db = EpisodicStore::new(EpisodicStoreConfig::persistent(&store)).await?;
                    db.log_failure(&session_id, cycle, &content).await
                })
            })
            .await;
        }
    }
}

impl Drop for DualThreadReactAgent {
    fn drop(&mut self) {
        if let Some(handle) = &self.planning_handle {
            handle.abort();
        }
        if let Some(handle) = &self.acting_handle {
            handle.abort();
        }
    }
}

/// ReAct execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactStats {
    /// Total cycle count observed.
    pub total_cycles: u32,
    /// Successful cycles.
    pub successful_cycles: u32,
    /// Failed cycles.
    pub failed_cycles: u32,
    /// Configured maximum cycles.
    pub max_cycles: u32,
}

impl ReactStats {
    /// Ratio of successful cycles.
    pub fn success_rate(&self) -> f32 {
        if self.total_cycles == 0 {
            1.0
        } else {
            self.successful_cycles as f32 / self.total_cycles as f32
        }
    }
}

async fn planner_loop(
    task: Task,
    blackboard: Arc<Mutex<SharedBlackboard>>,
    tools: ToolSet,
    mut planning_rx: mpsc::Receiver<PlannerRequest>,
    mut interrupt_rx: broadcast::Receiver<InterruptSignal>,
) -> Result<()> {
    loop {
        tokio::select! {
            maybe_request = planning_rx.recv() => {
                let Some(request) = maybe_request else {
                    break;
                };
                let (version, summary) = {
                    let blackboard = blackboard.lock().await;
                    (blackboard.version(), blackboard.snapshot_summary("all"))
                };
                let proposal = ActionProposal {
                    thought: format!(
                        "Cycle {} planning for '{}' on snapshot v{}.\n{}",
                        request.cycle_number,
                        task.description,
                        version,
                        summary,
                    ),
                    action: choose_action(&task, &tools, request.cycle_number),
                    snapshot_version: version,
                };
                let _ = request.response_tx.send(Ok(proposal));
            }
            interrupt = interrupt_rx.recv() => {
                match interrupt {
                    Ok(InterruptSignal::Stop { reason }) => {
                        return Err(AgentError::ExecutionFailed(format!("planner interrupted: {}", reason)).into());
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        }
    }
    Ok(())
}

async fn actor_loop(
    tools: ToolSet,
    mut acting_rx: mpsc::Receiver<ActorRequest>,
    mut interrupt_rx: broadcast::Receiver<InterruptSignal>,
) -> Result<()> {
    loop {
        tokio::select! {
            maybe_request = acting_rx.recv() => {
                let Some(request) = maybe_request else {
                    break;
                };
                let ActorRequest { proposal, response_tx } = request;
                let action = proposal.action.clone();
                let execution = tools.execute(&action);
                tokio::pin!(execution);
                let observation = tokio::select! {
                    result = &mut execution => result?,
                    interrupt = interrupt_rx.recv() => {
                        match interrupt {
                            Ok(InterruptSignal::Stop { reason }) => {
                                return Err(AgentError::ExecutionFailed(format!("actor interrupted during tool execution: {}", reason)).into());
                            }
                            Ok(_) => {
                                return Err(AgentError::ExecutionFailed("actor received unsupported interrupt".to_string()).into());
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                return Err(AgentError::ExecutionFailed("actor interrupt channel lagged".to_string()).into());
                            }
                        }
                    }
                };
                let _ = response_tx.send(Ok(ActionExecution {
                    proposal,
                    observation,
                }));
            }
            interrupt = interrupt_rx.recv() => {
                match interrupt {
                    Ok(InterruptSignal::Stop { reason }) => {
                        return Err(AgentError::ExecutionFailed(format!("actor interrupted: {}", reason)).into());
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        }
    }
    Ok(())
}

fn choose_action(task: &Task, tools: &ToolSet, cycle_number: u32) -> Action {
    if let Some(tool_name) = tools.tool_names().into_iter().next() {
        return Action::new(
            &tool_name,
            serde_json::json!({
                "task_id": task.id,
                "cycle_number": cycle_number,
                "description": task.description,
            }),
        );
    }

    if tools.has_mcp() {
        return Action::new(
            "run_command",
            serde_json::json!({
                "program": "printf",
                "args": [format!("axora-react:{}:{}\\n", task.id, cycle_number)],
            }),
        );
    }

    Action::new(
        "tool_unavailable",
        serde_json::json!({
            "task_id": task.id,
            "reason": "no MCP endpoint or registered tool available",
        }),
    )
}

fn value_to_struct(value: &serde_json::Value) -> prost_types::Struct {
    let fields = value
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(key, value)| {
                    let kind = match value {
                        serde_json::Value::String(v) => prost_types::value::Kind::StringValue(v.clone()),
                        serde_json::Value::Bool(v) => prost_types::value::Kind::BoolValue(*v),
                        serde_json::Value::Number(v) => prost_types::value::Kind::NumberValue(v.as_f64().unwrap_or_default()),
                        _ => prost_types::value::Kind::StringValue(value.to_string()),
                    };
                    (key.clone(), prost_types::Value { kind: Some(kind) })
                })
                .collect()
        })
        .unwrap_or_default();

    prost_types::Struct { fields }
}

fn extract_stdout(value: &serde_json::Value) -> Option<&str> {
    value
        .as_object()
        .and_then(|object| object.get("stdout"))
        .and_then(serde_json::Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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

        let start = Instant::now();
        let cycle = agent.execute_cycle().await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed < Duration::from_secs(1));
        assert_eq!(cycle.cycle_number, 1);
    }

    #[tokio::test]
    async fn test_acting_thread_tool_execution() {
        let task = Task::new("Test tool execution");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));

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

        let signal = InterruptSignal::Stop {
            reason: "Test stop".to_string(),
        };

        let result = agent.send_interrupt(signal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_cycles_prevention() {
        let task = Task::new("Test max cycles");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let mut tools = ToolSet::new();
        tools.register_tool(Tool::new("execute_task", "Keep cycling", |_params| {
            Ok(Observation::failure("not done yet"))
        }));

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();
        agent.max_cycles = 3;

        for _ in 0..3 {
            let _ = agent.execute_cycle().await.unwrap();
        }

        let result = agent.execute_cycle().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_full_execution() {
        let task = Task::new("Full execution test");
        let blackboard = Arc::new(Mutex::new(SharedBlackboard::new()));
        let mut tools = ToolSet::new();
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
}
