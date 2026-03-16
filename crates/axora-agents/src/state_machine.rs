//! Agent state machine for orchestration

use crate::agent::AgentState;
use crate::error::AgentError;
use crate::heartbeat::{Heartbeat, HeartbeatConfig};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Agent state in the state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    /// Agent ID
    pub agent_id: String,
    /// Current state
    pub state: AgentState,
    /// Current task (if any)
    pub current_task: Option<String>,
}

/// State transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// From state
    pub from: AgentState,
    /// To state
    pub to: AgentState,
    /// Condition for transition
    pub condition: TransitionCondition,
}

/// Transition condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Always transition
    Always,
    /// Transition on task success
    OnSuccess,
    /// Transition on task failure
    OnFailure,
    /// Transition after timeout
    Timeout(u64), // milliseconds
    /// Custom condition (by name)
    Custom(String),
}

/// State machine for orchestrating agents
pub struct StateMachine {
    /// Agent nodes
    agents: HashMap<String, AgentNode>,
    /// State transitions
    transitions: Vec<StateTransition>,
    /// Current global state
    global_state: GlobalState,
    /// Heartbeat system for lifecycle management
    heartbeat: Option<Arc<Mutex<Heartbeat>>>,
}

/// Global orchestration state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GlobalState {
    #[default]
    /// Idle, waiting for tasks
    Idle,
    /// Processing tasks
    Running,
    /// Waiting for review
    WaitingForReview,
    /// Completed all tasks
    Completed,
    /// Error state
    Error(String),
}

impl StateMachine {
    /// Create new state machine
    pub fn new() -> Result<Self> {
        let mut machine = Self {
            agents: HashMap::new(),
            transitions: Vec::new(),
            global_state: GlobalState::Idle,
            heartbeat: None,
        };

        // Define default transitions
        machine.define_default_transitions();

        Ok(machine)
    }

    /// Create new state machine with heartbeat support
    pub fn with_heartbeat(config: HeartbeatConfig) -> Self {
        let heartbeat = Heartbeat::new(config);
        Self {
            agents: HashMap::new(),
            transitions: Vec::new(),
            global_state: GlobalState::Idle,
            heartbeat: Some(Arc::new(Mutex::new(heartbeat))),
        }
    }

    /// Get heartbeat system
    pub fn get_heartbeat(&self) -> Option<Arc<Mutex<Heartbeat>>> {
        self.heartbeat.clone()
    }

    /// Define default state transitions
    fn define_default_transitions(&mut self) {
        // Idle → Executing (on task assignment)
        self.transitions.push(StateTransition {
            from: AgentState::Idle,
            to: AgentState::Executing,
            condition: TransitionCondition::Always,
        });

        // Executing → Completed (on success)
        self.transitions.push(StateTransition {
            from: AgentState::Executing,
            to: AgentState::Completed,
            condition: TransitionCondition::OnSuccess,
        });

        // Executing → Blocked (on failure)
        self.transitions.push(StateTransition {
            from: AgentState::Executing,
            to: AgentState::Blocked,
            condition: TransitionCondition::OnFailure,
        });

        // Blocked → Idle (after retry)
        self.transitions.push(StateTransition {
            from: AgentState::Blocked,
            to: AgentState::Idle,
            condition: TransitionCondition::Always,
        });

        // Completed → Idle (ready for next task)
        self.transitions.push(StateTransition {
            from: AgentState::Completed,
            to: AgentState::Idle,
            condition: TransitionCondition::Always,
        });
    }

    /// Register an agent
    pub fn register_agent(&mut self, agent_id: &str) {
        info!("Registering agent: {}", agent_id);

        let node = AgentNode {
            agent_id: agent_id.to_string(),
            state: AgentState::Idle,
            current_task: None,
        };

        self.agents.insert(agent_id.to_string(), node);

        // Register with heartbeat if available
        if let Some(hb) = &self.heartbeat {
            let hb = Arc::clone(hb);
            let agent_id = agent_id.to_string();
            tokio::spawn(async move {
                let hb = hb.lock().await;
                hb.register_agent(&agent_id).await;
            });
        }
    }

    /// Assign task to agent
    pub fn assign_task(&mut self, agent_id: &str, task_id: &str) -> Result<()> {
        debug!("Assigning task {} to agent {}", task_id, agent_id);

        // Check if agent exists and is idle
        let is_idle = self.agents.get(agent_id)
            .map(|a| a.state == AgentState::Idle)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        if !is_idle {
            return Err(AgentError::InvalidStateTransition(
                format!("Agent {} is not idle", agent_id)
            ).into());
        }

        // Transition to Executing
        if self.can_transition(agent_id, &AgentState::Executing) {
            let agent = self.agents.get_mut(agent_id).unwrap();
            agent.state = AgentState::Executing;
            agent.current_task = Some(task_id.to_string());
            info!("Agent {} assigned task {}, state: Executing", agent_id, task_id);
            Ok(())
        } else {
            Err(AgentError::InvalidStateTransition(
                "Cannot transition to Executing".to_string()
            ).into())
        }
    }

    /// Complete task for agent
    pub fn complete_task(&mut self, agent_id: &str, success: bool) -> Result<()> {
        debug!("Completing task for agent {} (success: {})", agent_id, success);

        let target_state = if success {
            AgentState::Completed
        } else {
            AgentState::Blocked
        };

        // Check if transition is valid first
        if !self.can_transition(agent_id, &target_state) {
            return Err(AgentError::InvalidStateTransition(
                format!("Cannot transition to {:?}", target_state)
            ).into());
        }

        // Now mutate
        let agent = self.agents.get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        agent.state = target_state.clone();
        agent.current_task = None;
        info!("Agent {} completed task, state: {:?}", agent_id, target_state);

        // Schedule heartbeat wake for idle transition
        if success {
            if let Some(hb) = &self.heartbeat {
                let hb = Arc::clone(hb);
                let agent_id = agent_id.to_string();
                tokio::spawn(async move {
                    let hb = hb.lock().await;
                    // Schedule wake for next task after brief delay
                    hb.schedule_wake(&agent_id, Duration::from_secs(5)).await;
                });
            }
        }

        Ok(())
    }

    /// Check if transition is valid
    pub fn can_transition(&self, agent_id: &str, target_state: &AgentState) -> bool {
        let agent = match self.agents.get(agent_id) {
            Some(a) => a,
            None => return false,
        };

        // Check if there's a valid transition
        self.transitions.iter().any(|t| {
            t.from == agent.state && t.to == *target_state
        })
    }

    /// Get agent state
    pub fn get_agent_state(&self, agent_id: &str) -> Option<AgentState> {
        self.agents.get(agent_id).map(|a| a.state.clone())
    }

    /// Get all agent states
    pub fn get_all_states(&self) -> Vec<(String, AgentState)> {
        self.agents
            .iter()
            .map(|(id, node)| (id.clone(), node.state.clone()))
            .collect()
    }

    /// Get global state
    pub fn get_global_state(&self) -> &GlobalState {
        &self.global_state
    }

    /// Set global state
    pub fn set_global_state(&mut self, state: GlobalState) {
        info!("Global state: {:?}", state);
        self.global_state = state;
    }

    /// Get idle agents
    pub fn get_idle_agents(&self) -> Vec<String> {
        self.agents
            .iter()
            .filter(|(_, node)| node.state == AgentState::Idle)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get busy agents
    pub fn get_busy_agents(&self) -> Vec<String> {
        self.agents
            .iter()
            .filter(|(_, node)| node.state == AgentState::Executing)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get agent count
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Reset agent to idle
    pub fn reset_agent(&mut self, agent_id: &str) -> Result<()> {
        let agent = self.agents.get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        agent.state = AgentState::Idle;
        agent.current_task = None;
        Ok(())
    }

    /// Reset all agents
    pub fn reset_all(&mut self) {
        for agent in self.agents.values_mut() {
            agent.state = AgentState::Idle;
            agent.current_task = None;
        }
        self.global_state = GlobalState::Idle;
    }

    /// Transition agent to idle with heartbeat scheduling
    pub fn transition_to_idle(&mut self, agent_id: &str) -> Result<()> {
        let agent = self.agents.get_mut(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(agent_id.to_string()))?;

        agent.state = AgentState::Idle;
        agent.current_task = None;

        // Schedule heartbeat wake
        if let Some(hb) = &self.heartbeat {
            let hb = Arc::clone(hb);
            let agent_id = agent_id.to_string();
            tokio::spawn(async move {
                let hb = hb.lock().await;
                hb.schedule_wake(&agent_id, Duration::from_secs(30)).await;
            });
        }

        info!("Agent {} transitioned to idle with heartbeat", agent_id);
        Ok(())
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_creation() {
        let machine = StateMachine::new().unwrap();
        assert_eq!(machine.agent_count(), 0);
        assert!(matches!(machine.get_global_state(), GlobalState::Idle));
    }

    #[test]
    fn test_agent_registration() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");
        machine.register_agent("agent2");

        assert_eq!(machine.agent_count(), 2);
        assert_eq!(machine.get_idle_agents().len(), 2);
    }

    #[test]
    fn test_task_assignment() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");

        // Assign task
        let result = machine.assign_task("agent1", "task1");
        assert!(result.is_ok());

        // Check state
        assert_eq!(machine.get_agent_state("agent1"), Some(AgentState::Executing));
        assert_eq!(machine.get_idle_agents().len(), 0);
        assert_eq!(machine.get_busy_agents().len(), 1);
    }

    #[test]
    fn test_task_completion() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");
        machine.assign_task("agent1", "task1").unwrap();

        // Complete successfully
        let result = machine.complete_task("agent1", true);
        assert!(result.is_ok());

        assert_eq!(machine.get_agent_state("agent1"), Some(AgentState::Completed));
    }

    #[test]
    fn test_task_failure() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");
        machine.assign_task("agent1", "task1").unwrap();

        // Complete with failure
        let result = machine.complete_task("agent1", false);
        assert!(result.is_ok());

        assert_eq!(machine.get_agent_state("agent1"), Some(AgentState::Blocked));
    }

    #[test]
    fn test_invalid_transition() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");

        // Try to complete without assigning task
        let result = machine.complete_task("agent1", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");
        machine.assign_task("agent1", "task1").unwrap();

        // Reset agent
        machine.reset_agent("agent1").unwrap();

        assert_eq!(machine.get_agent_state("agent1"), Some(AgentState::Idle));
        assert_eq!(machine.get_idle_agents().len(), 1);
    }

    #[test]
    fn test_reset_all() {
        let mut machine = StateMachine::new().unwrap();
        machine.register_agent("agent1");
        machine.register_agent("agent2");
        machine.assign_task("agent1", "task1").unwrap();

        // Reset all
        machine.reset_all();

        assert_eq!(machine.get_idle_agents().len(), 2);
        assert!(matches!(machine.get_global_state(), GlobalState::Idle));
    }
}
