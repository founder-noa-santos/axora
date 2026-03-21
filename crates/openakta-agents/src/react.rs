//! Dual-thread ReAct runtime with planner/actor separation.

use crate::aci_formatter::ACIFormatter;
use crate::agent::TaskResult;
use crate::blackboard_runtime::RuntimeBlackboard;
use crate::error::AgentError;
use crate::hitl::MissionHitlGate;
use crate::mcp_client::McpClient;
use crate::task::Task;
use crate::Result;
use openakta_memory::{EpisodicStore, EpisodicStoreConfig};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use tokio::sync::{broadcast, mpsc, watch, Mutex};
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

    /// Create failed observation with structured result payload.
    pub fn failure_with_result(error: &str, result: serde_json::Value) -> Self {
        Self {
            success: false,
            result,
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
        /// Cycle number targeted by the stop signal. `0` targets the active cycle.
        cycle_number: u32,
        /// Whether the entire agent should terminate.
        terminate_agent: bool,
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
    /// Planned cycle number.
    pub cycle_number: u32,
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
    /// Identity passed to MCP `ToolCallRequest` (not the model name).
    mcp_agent_id: String,
    mcp_role: String,
    /// Trusted mission id for HITL / policy (MCP metadata).
    pub mission_id: Option<String>,
    hitl_gate: Option<Arc<MissionHitlGate>>,
    active_mission_id: Option<String>,
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
            mcp_agent_id: "react-agent".to_string(),
            mcp_role: "worker".to_string(),
            mission_id: None,
            hitl_gate: None,
            active_mission_id: None,
        }
    }

    /// Create a tool set that routes non-local tools through MCP.
    pub fn with_mcp_endpoint(endpoint: impl Into<String>, llm_model: impl Into<String>) -> Self {
        let mut tools = Self::new();
        tools.llm_model = llm_model.into();
        tools.mcp_client = Some(McpClient::new(endpoint));
        tools
    }

    /// MCP identity + optional mission id for trusted `ToolCallRequest` metadata.
    pub fn with_mcp_runtime_context(
        mut self,
        agent_id: impl Into<String>,
        role: impl Into<String>,
        mission_id: Option<String>,
    ) -> Self {
        self.mcp_agent_id = agent_id.into();
        self.mcp_role = role.into();
        self.mission_id = mission_id;
        self
    }

    /// Override workspace root sent on every MCP `ToolCallRequest`.
    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = root.into();
        self
    }

    /// Attach HITL gate + mission id so destructive tools are refused while a question is pending.
    pub fn with_hitl(mut self, gate: Arc<MissionHitlGate>, mission_id: impl Into<String>) -> Self {
        self.hitl_gate = Some(gate);
        self.active_mission_id = Some(mission_id.into());
        self
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Execute a tool
    pub async fn execute(&self, action: &Action) -> Result<Observation> {
        if let Some(tool) = self.tools.get(&action.tool_name) {
            if self.should_block_destructive_tool(&action.tool_name, tool.is_destructive) {
                return Err(AgentError::BlockedPendingAnswer(format!(
                    "tool '{}' blocked until human answers pending question",
                    action.tool_name
                ))
                .into());
            }
            return tool.execute(&action.parameters).await;
        }

        if let Some(client) = &self.mcp_client {
            if self.should_block_destructive_tool(
                &action.tool_name,
                mcp_tool_is_destructive(&action.tool_name),
            ) {
                return Err(AgentError::BlockedPendingAnswer(format!(
                    "tool '{}' blocked until human answers pending question",
                    action.tool_name
                ))
                .into());
            }
            let args = value_to_struct(&action.parameters);
            let request_id = format!("react-{}", uuid::Uuid::new_v4());
            return client
                .call_tool(
                    &request_id,
                    &self.mcp_agent_id,
                    &self.mcp_role,
                    &action.tool_name,
                    &self.workspace_root,
                    args,
                    None,
                    self.mission_id.as_deref(),
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

    fn should_block_destructive_tool(&self, tool_name: &str, destructive: bool) -> bool {
        if tool_name == "request_user_input" {
            return false;
        }
        let Some(gate) = &self.hitl_gate else {
            return false;
        };
        let Some(mid) = &self.active_mission_id else {
            return false;
        };
        destructive && gate.should_block_destructive_tools(mid)
    }
}

fn mcp_tool_is_destructive(tool_name: &str) -> bool {
    matches!(tool_name, "apply_patch" | "run_command")
}

impl Default for ToolSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for a [`Tool`] implementation.
pub type ToolHandler = Arc<dyn Fn(&serde_json::Value) -> Result<Observation> + Send + Sync>;

/// Tool definition
#[derive(Clone)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Whether this tool mutates workspace / is irreversible (HITL guard).
    pub is_destructive: bool,
    /// Tool implementation
    pub handler: ToolHandler,
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
            is_destructive: false,
            handler: Arc::new(handler),
        }
    }

    /// Mark as destructive for HITL blocking (`pending_answer`).
    pub fn destructive(mut self) -> Self {
        self.is_destructive = true;
        self
    }

    /// Execute tool
    pub async fn execute(&self, params: &serde_json::Value) -> Result<Observation> {
        let handler = Arc::clone(&self.handler);
        let params = params.clone();
        tokio::task::spawn_blocking(move || (handler)(&params))
            .await
            .map_err(|err| AgentError::ExecutionFailed(format!("tool task failed: {}", err)))?
    }
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("is_destructive", &self.is_destructive)
            .finish()
    }
}

/// Dual-Thread ReAct Agent
pub struct DualThreadReactAgent {
    planning_tx: mpsc::Sender<u32>,
    proposal_rx: mpsc::Receiver<Result<ActionProposal>>,
    proposal_cache: HashMap<u32, ActionProposal>,
    pending_plan_requests: HashSet<u32>,
    planning_handle: Option<JoinHandle<Result<()>>>,
    acting_tx: mpsc::Sender<ActionProposal>,
    execution_rx: mpsc::Receiver<Result<ActionExecution>>,
    acting_handle: Option<JoinHandle<Result<()>>>,
    blackboard_watch_handle: Option<JoinHandle<Result<()>>>,
    interrupt_tx: broadcast::Sender<InterruptSignal>,
    active_cycle: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
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
        blackboard: Arc<Mutex<RuntimeBlackboard>>,
        tools: ToolSet,
    ) -> Result<Self> {
        Self::spawn_internal(task, blackboard, tools, ACIFormatter::new(), None).await
    }

    /// Spawn dual-thread agent with custom ACI config
    pub async fn spawn_with_config(
        task: Task,
        blackboard: Arc<Mutex<RuntimeBlackboard>>,
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
        blackboard: Arc<Mutex<RuntimeBlackboard>>,
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
        blackboard: Arc<Mutex<RuntimeBlackboard>>,
        tools: ToolSet,
        aci_formatter: ACIFormatter,
        episodic_store_path: Option<String>,
    ) -> Result<Self> {
        let (planning_tx, planning_rx) = mpsc::channel(32);
        let (proposal_tx, proposal_rx) = mpsc::channel(32);
        let (acting_tx, acting_rx) = mpsc::channel(32);
        let (execution_tx, execution_rx) = mpsc::channel(32);
        let (interrupt_tx, _) = broadcast::channel(32);
        let version_rx = {
            let blackboard = blackboard.lock().await;
            blackboard.subscribe_version()
        };
        let active_cycle = Arc::new(AtomicU32::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let planner_handle = tokio::spawn(planner_loop(
            task.clone(),
            Arc::clone(&blackboard),
            tools.clone(),
            planning_rx,
            proposal_tx,
            interrupt_tx.subscribe(),
        ));
        let actor_handle = tokio::spawn(actor_loop(
            tools,
            acting_rx,
            execution_tx,
            interrupt_tx.subscribe(),
        ));
        let blackboard_watch_handle = tokio::spawn(planner_interrupt_loop(
            version_rx,
            interrupt_tx.clone(),
            Arc::clone(&active_cycle),
            Arc::clone(&shutdown),
        ));

        Ok(Self {
            planning_tx,
            proposal_rx,
            proposal_cache: HashMap::new(),
            pending_plan_requests: HashSet::new(),
            planning_handle: Some(planner_handle),
            acting_tx,
            execution_rx,
            acting_handle: Some(actor_handle),
            blackboard_watch_handle: Some(blackboard_watch_handle),
            interrupt_tx,
            active_cycle,
            shutdown,
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

        self.ensure_cycle_requested(self.current_cycle).await?;
        let proposal = self.recv_proposal(self.current_cycle).await?;

        let mut cycle = ReactCycle::new(
            &proposal.thought,
            proposal.action.clone(),
            self.current_cycle,
        );
        self.log_thought(&cycle.thought).await;

        if self.current_cycle < self.max_cycles {
            self.ensure_cycle_requested(self.current_cycle + 1).await?;
        }

        self.active_cycle
            .store(self.current_cycle, Ordering::SeqCst);
        if self.acting_tx.send(proposal.clone()).await.is_err() {
            self.active_cycle.store(0, Ordering::SeqCst);
            return Err(AgentError::ExecutionFailed("actor unavailable".to_string()).into());
        }
        let execution = self.recv_execution(self.current_cycle).await;
        self.active_cycle.store(0, Ordering::SeqCst);
        let execution = execution?;
        cycle.observation = self.format_observation(&cycle.action, execution.observation);
        self.log_action(&cycle.action, &cycle.observation).await;

        self.cycles.push(cycle.clone());
        Ok(cycle)
    }

    async fn ensure_cycle_requested(&mut self, cycle_number: u32) -> Result<()> {
        if cycle_number == 0 || cycle_number > self.max_cycles {
            return Ok(());
        }
        if self.proposal_cache.contains_key(&cycle_number)
            || !self.pending_plan_requests.insert(cycle_number)
        {
            return Ok(());
        }

        self.planning_tx
            .send(cycle_number)
            .await
            .map_err(|_| AgentError::ExecutionFailed("planner unavailable".to_string()))?;
        Ok(())
    }

    async fn recv_proposal(&mut self, cycle_number: u32) -> Result<ActionProposal> {
        if let Some(proposal) = self.proposal_cache.remove(&cycle_number) {
            self.pending_plan_requests.remove(&cycle_number);
            return Ok(proposal);
        }

        loop {
            let proposal = self
                .proposal_rx
                .recv()
                .await
                .ok_or_else(|| AgentError::ExecutionFailed("planner dropped".to_string()))??;
            self.pending_plan_requests.remove(&proposal.cycle_number);
            if proposal.cycle_number == cycle_number {
                return Ok(proposal);
            }
            self.proposal_cache.insert(proposal.cycle_number, proposal);
        }
    }

    async fn recv_execution(&mut self, cycle_number: u32) -> Result<ActionExecution> {
        loop {
            tokio::select! {
                maybe_execution = self.execution_rx.recv() => {
                    let execution = maybe_execution
                        .ok_or_else(|| AgentError::ExecutionFailed("actor dropped".to_string()))??;
                    if execution.proposal.cycle_number != cycle_number {
                        return Err(AgentError::ExecutionFailed(format!(
                            "actor returned cycle {} while waiting for {}",
                            execution.proposal.cycle_number, cycle_number
                        ))
                        .into());
                    }
                    return Ok(execution);
                }
                maybe_proposal = self.proposal_rx.recv() => {
                    let proposal = maybe_proposal
                        .ok_or_else(|| AgentError::ExecutionFailed("planner dropped".to_string()))??;
                    self.pending_plan_requests.remove(&proposal.cycle_number);
                    self.proposal_cache.insert(proposal.cycle_number, proposal);
                }
            }
        }
    }

    fn format_observation(&self, action: &Action, observation: Observation) -> Observation {
        if !observation.success {
            return observation;
        }

        let formatted_result = match action.tool_name.as_str() {
            "run_command" | "execute_shell" => extract_stdout(&observation.result)
                .map(|stdout| serde_json::json!(self.aci_formatter.format_output(stdout)))
                .unwrap_or_else(|| observation.result.clone()),
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
                self.log_success(&cycle.observation.result.to_string())
                    .await;
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
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = &self.planning_handle {
            handle.abort();
        }
        if let Some(handle) = &self.acting_handle {
            handle.abort();
        }
        if let Some(handle) = &self.blackboard_watch_handle {
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
    blackboard: Arc<Mutex<RuntimeBlackboard>>,
    tools: ToolSet,
    mut planning_rx: mpsc::Receiver<u32>,
    proposal_tx: mpsc::Sender<Result<ActionProposal>>,
    mut interrupt_rx: broadcast::Receiver<InterruptSignal>,
) -> Result<()> {
    loop {
        tokio::select! {
            maybe_request = planning_rx.recv() => {
                let Some(cycle_number) = maybe_request else {
                    break;
                };
                let (version, summary) = {
                    let blackboard = blackboard.lock().await;
                    (blackboard.version(), blackboard.snapshot_summary("all"))
                };
                let proposal = ActionProposal {
                    cycle_number,
                    thought: format!(
                        "Cycle {} planning for '{}' on snapshot v{}.\n{}",
                        cycle_number,
                        task.description,
                        version,
                        summary,
                    ),
                    action: choose_action(&task, &tools, cycle_number),
                    snapshot_version: version,
                };
                if proposal_tx.send(Ok(proposal)).await.is_err() {
                    break;
                }
            }
            interrupt = interrupt_rx.recv() => {
                match interrupt {
                    Ok(InterruptSignal::Stop { reason, terminate_agent, .. }) if terminate_agent => {
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
    mut acting_rx: mpsc::Receiver<ActionProposal>,
    execution_tx: mpsc::Sender<Result<ActionExecution>>,
    mut interrupt_rx: broadcast::Receiver<InterruptSignal>,
) -> Result<()> {
    'outer: loop {
        tokio::select! {
            maybe_request = acting_rx.recv() => {
                let Some(proposal) = maybe_request else {
                    break;
                };
                let action = proposal.action.clone();
                let execution = tools.execute(&action);
                tokio::pin!(execution);
                let observation = loop {
                    tokio::select! {
                        result = &mut execution => break result?,
                        interrupt = interrupt_rx.recv() => {
                            match interrupt {
                                Ok(InterruptSignal::Stop { reason, cycle_number, terminate_agent }) => {
                                    if terminate_agent {
                                        return Err(AgentError::ExecutionFailed(format!(
                                            "actor interrupted during tool execution: {}",
                                            reason
                                        )).into());
                                    }
                                    if cycle_number == 0 || cycle_number == proposal.cycle_number {
                                        break Observation::failure_with_result(
                                            &format!("action interrupted: {}", reason),
                                            serde_json::json!({
                                                "interrupted": true,
                                                "reason": reason,
                                                "cycle_number": proposal.cycle_number,
                                                "terminate_agent": false,
                                            }),
                                        );
                                    }
                                }
                                Ok(_) => {}
                                Err(broadcast::error::RecvError::Closed) => break 'outer,
                                Err(broadcast::error::RecvError::Lagged(_)) => {}
                            }
                        }
                    };
                };
                if execution_tx.send(Ok(ActionExecution { proposal, observation })).await.is_err() {
                    break;
                }
            }
            interrupt = interrupt_rx.recv() => {
                match interrupt {
                    Ok(InterruptSignal::Stop { reason, terminate_agent, .. }) if terminate_agent => {
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

async fn planner_interrupt_loop(
    mut version_rx: watch::Receiver<u64>,
    interrupt_tx: broadcast::Sender<InterruptSignal>,
    active_cycle: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    let mut last_seen = *version_rx.borrow();
    loop {
        if version_rx.changed().await.is_err() || shutdown.load(Ordering::SeqCst) {
            break;
        }

        let version = *version_rx.borrow_and_update();
        if version == last_seen {
            continue;
        }
        last_seen = version;

        let cycle_number = active_cycle.load(Ordering::SeqCst);
        if cycle_number == 0 {
            continue;
        }

        let _ = interrupt_tx.send(InterruptSignal::Stop {
            reason: format!("blackboard snapshot advanced to v{}", version),
            cycle_number,
            terminate_agent: false,
        });
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
                "args": [format!("openakta-react:{}:{}\\n", task.id, cycle_number)],
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
                        serde_json::Value::String(v) => {
                            prost_types::value::Kind::StringValue(v.clone())
                        }
                        serde_json::Value::Bool(v) => prost_types::value::Kind::BoolValue(*v),
                        serde_json::Value::Number(v) => {
                            prost_types::value::Kind::NumberValue(v.as_f64().unwrap_or_default())
                        }
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
    use crate::BlackboardEntry;
    use std::time::Duration;

    fn shared_entry(id: &str, content: &str) -> BlackboardEntry {
        BlackboardEntry {
            id: id.to_string(),
            content: content.to_string(),
        }
    }

    fn slow_tool(delay: Duration) -> Tool {
        Tool::new("slow_tool", "Slow test tool", move |_params| {
            std::thread::sleep(delay);
            Ok(Observation::success(serde_json::json!({"completed": true})))
        })
    }

    #[tokio::test]
    async fn test_dual_thread_spawn() {
        let task = Task::new("Test task for dual-thread spawn");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let tools = ToolSet::new();

        let agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        assert_eq!(agent.current_cycle, 0);
        assert_eq!(agent.max_cycles, 12);
    }

    #[tokio::test]
    async fn test_planner_prefetches_next_cycle() {
        let task = Task::new("Test planner prefetch");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let mut tools = ToolSet::new();
        tools.register_tool(Tool::new("test_tool", "Test tool", |_params| {
            Ok(Observation::failure("keep planning"))
        }));

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();

        let cycle = agent.execute_cycle().await.unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;

        assert_eq!(cycle.cycle_number, 1);
        assert!(agent.proposal_cache.contains_key(&2));
    }

    #[tokio::test]
    async fn test_acting_thread_tool_execution() {
        let task = Task::new("Test tool execution");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));

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
    async fn destructive_tool_blocked_while_question_pending() {
        use crate::hitl::{HitlConfig, MissionHitlGate};
        use crate::OpenaktaAgentsError;
        use openakta_proto::collective::v1::{QuestionEnvelope, QuestionKind, QuestionOption};

        let gate = Arc::new(MissionHitlGate::new(HitlConfig::default(), None));
        gate.register_mission_start("m1").unwrap();
        let env = QuestionEnvelope {
            question_id: String::new(),
            mission_id: "m1".into(),
            session_id: "s1".into(),
            turn_index: 1,
            text: "continue?".into(),
            kind: QuestionKind::Single as i32,
            options: vec![
                QuestionOption {
                    id: "yes".into(),
                    label: "Yes".into(),
                    description: String::new(),
                    is_default: true,
                },
                QuestionOption {
                    id: "no".into(),
                    label: "No".into(),
                    description: String::new(),
                    is_default: false,
                },
            ],
            constraints: Some(openakta_proto::collective::v1::QuestionConstraints {
                min_selections: 1,
                max_selections: 1,
                free_text_max_chars: None,
            }),
            expiry_token: None,
            sensitive: false,
            expires_at: None,
        };
        gate.raise_question(env, "m1").await.unwrap();

        let mut tools = ToolSet::new().with_hitl(gate, "m1");
        tools.register_tool(
            Tool::new("danger", "destructive test tool", |_p| {
                Ok(Observation::success(serde_json::json!({})))
            })
            .destructive(),
        );

        let err = tools
            .execute(&Action::new("danger", serde_json::json!({})))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            OpenaktaAgentsError::Agent(AgentError::BlockedPendingAnswer(_))
        ));
    }

    #[tokio::test]
    async fn test_nonterminal_interrupt_cancels_in_flight_action() {
        let task = Task::new("Test nonterminal interrupt");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let mut tools = ToolSet::new();
        tools.register_tool(slow_tool(Duration::from_millis(250)));

        let agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();
        let interrupt_tx = agent.interrupt_tx.clone();
        let execute = tokio::spawn(async move {
            let mut agent = agent;
            agent.execute_cycle().await
        });
        tokio::time::sleep(Duration::from_millis(40)).await;
        let _ = interrupt_tx.send(InterruptSignal::Stop {
            reason: "planner replanned".to_string(),
            cycle_number: 1,
            terminate_agent: false,
        });
        let cycle = execute.await.unwrap().unwrap();

        assert!(!cycle.observation.success);
        assert_eq!(
            cycle.observation.result["interrupted"],
            serde_json::Value::Bool(true)
        );
    }

    #[tokio::test]
    async fn test_max_cycles_prevention() {
        let task = Task::new("Test max cycles");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
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
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
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

    #[tokio::test]
    async fn test_blackboard_update_interrupts_actor() {
        let task = Task::new("Blackboard interrupt");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let mut tools = ToolSet::new();
        tools.register_tool(slow_tool(Duration::from_millis(250)));

        let agent = DualThreadReactAgent::spawn(task, Arc::clone(&blackboard), tools)
            .await
            .unwrap();
        let execute = tokio::spawn(async move {
            let mut agent = agent;
            agent.execute_cycle().await
        });
        tokio::time::sleep(Duration::from_millis(40)).await;
        {
            let mut board = blackboard.lock().await;
            board
                .publish(
                    shared_entry("bb-1", "updated context"),
                    vec!["react-agent".to_string()],
                )
                .unwrap();
        }
        let cycle = execute.await.unwrap().unwrap();

        assert!(!cycle.observation.success);
        assert_eq!(
            cycle.observation.result["interrupted"],
            serde_json::Value::Bool(true)
        );
    }

    #[tokio::test]
    async fn test_terminal_interrupt_stops_actor_loop() {
        let task = Task::new("Terminal interrupt");
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let mut tools = ToolSet::new();
        tools.register_tool(slow_tool(Duration::from_millis(250)));

        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools)
            .await
            .unwrap();
        let interrupt_tx = agent.interrupt_tx.clone();

        let execute = agent.execute_cycle();
        tokio::pin!(execute);
        tokio::time::sleep(Duration::from_millis(40)).await;
        let _ = interrupt_tx.send(InterruptSignal::Stop {
            reason: "coordinator shutdown".to_string(),
            cycle_number: 1,
            terminate_agent: true,
        });

        let result = execute.await;
        assert!(result.is_err());
    }
}
