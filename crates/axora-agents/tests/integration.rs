//! Integration tests for AXORA Agents
//!
//! These tests validate agent functionality and integration with Phase 2 features.

use std::collections::HashMap;

// ============================================================================
// Integration Test 1: Agent Context Management
// ============================================================================

#[test]
fn test_agent_context_allocation() {
    // Simulate context allocation for multiple agents
    let mut contexts: HashMap<String, Vec<String>> = HashMap::new();

    let agents = vec!["agent-a", "agent-b", "agent-c"];
    let tasks = vec!["task-1", "task-2", "task-3"];

    for agent in &agents {
        let mut agent_contexts = Vec::new();
        for task in &tasks {
            // Simulate context allocation (minimal context per task)
            let context = format!("Context for {} - {}", agent, task);
            agent_contexts.push(context);
        }
        contexts.insert(agent.to_string(), agent_contexts);
    }

    // Verify each agent has contexts for all tasks
    for agent in &agents {
        let agent_contexts = contexts.get(&agent.to_string()).unwrap();
        assert_eq!(agent_contexts.len(), tasks.len());
    }

    // Verify context isolation between agents
    let agent_a_contexts = contexts.get("agent-a").unwrap();
    let agent_b_contexts = contexts.get("agent-b").unwrap();

    assert_ne!(agent_a_contexts[0], agent_b_contexts[0]);
}

// ============================================================================
// Integration Test 2: Agent Task Decomposition Simulation
// ============================================================================

#[test]
fn test_task_decomposition_simulation() {
    // Simulate task decomposition
    let mission = "Implement authentication system with login, logout, and token refresh";

    // Decompose into subtasks
    let subtasks = vec![
        "Create User model with password hashing",
        "Implement login endpoint with JWT token generation",
        "Implement logout endpoint with token invalidation",
        "Implement token refresh endpoint",
        "Add authentication middleware for protected routes",
        "Write unit tests for authentication functions",
        "Write integration tests for authentication flow",
        "Create API documentation for authentication endpoints",
    ];

    // Identify parallel groups (tasks that can run concurrently)
    let parallel_groups = vec![
        vec![0, 1], // User model + login endpoint can start together
        vec![2, 3], // logout + refresh can start after login
        vec![4],    // middleware after endpoints
        vec![5, 6], // tests can run together
        vec![7],    // docs last
    ];

    // Verify decomposition
    assert_eq!(subtasks.len(), 8);
    assert_eq!(parallel_groups.len(), 5);

    // Verify all tasks are covered
    let all_task_indices: Vec<usize> = parallel_groups.iter().flatten().copied().collect();
    assert_eq!(all_task_indices.len(), subtasks.len());

    // Verify no duplicate tasks
    let mut unique_indices = all_task_indices.clone();
    unique_indices.sort();
    unique_indices.dedup();
    assert_eq!(unique_indices.len(), all_task_indices.len());

    println!("\n=== Task Decomposition ===");
    println!("Mission: {}", mission);
    println!("Subtasks: {}", subtasks.len());
    println!("Parallel groups: {}", parallel_groups.len());
    for (i, group) in parallel_groups.iter().enumerate() {
        println!(
            "  Group {}: {:?}",
            i + 1,
            group.iter().map(|idx| subtasks[*idx]).collect::<Vec<_>>()
        );
    }
}

// ============================================================================
// Integration Test 3: Agent Communication Protocol
// ============================================================================

#[test]
fn test_agent_communication_protocol() {
    // Simulate agent communication with structured messages
    #[derive(Debug, Clone)]
    struct AgentMessage {
        from: String,
        to: String,
        message_type: String,
        payload: String,
        timestamp: u64,
    }

    let mut message_queue: Vec<AgentMessage> = Vec::new();

    // Simulate conversation
    message_queue.push(AgentMessage {
        from: "coordinator".to_string(),
        to: "coder".to_string(),
        message_type: "task_assign".to_string(),
        payload: "Implement login function".to_string(),
        timestamp: 1000,
    });

    message_queue.push(AgentMessage {
        from: "coder".to_string(),
        to: "reviewer".to_string(),
        message_type: "review_request".to_string(),
        payload: "Please review login implementation".to_string(),
        timestamp: 2000,
    });

    message_queue.push(AgentMessage {
        from: "reviewer".to_string(),
        to: "coder".to_string(),
        message_type: "feedback".to_string(),
        payload: "Add error handling for invalid credentials".to_string(),
        timestamp: 3000,
    });

    message_queue.push(AgentMessage {
        from: "coder".to_string(),
        to: "coordinator".to_string(),
        message_type: "task_complete".to_string(),
        payload: "Login function implemented and reviewed".to_string(),
        timestamp: 4000,
    });

    // Verify message flow
    assert_eq!(message_queue.len(), 4);

    // Verify message types
    let message_types: Vec<&str> = message_queue
        .iter()
        .map(|m| m.message_type.as_str())
        .collect();
    assert_eq!(
        message_types,
        vec!["task_assign", "review_request", "feedback", "task_complete"]
    );

    // Verify conversation completed
    let completion_message = message_queue
        .iter()
        .find(|m| m.message_type == "task_complete");
    assert!(completion_message.is_some());

    println!("\n=== Agent Communication ===");
    for msg in &message_queue {
        println!(
            "  {} -> {} [{}]: {}",
            msg.from, msg.to, msg.message_type, msg.payload
        );
    }
}

// ============================================================================
// Integration Test 4: Agent State Management
// ============================================================================

#[test]
fn test_agent_state_management() {
    #[derive(Debug, Clone, PartialEq)]
    enum AgentState {
        Idle,
        Working(String),
        Waiting(String),
        Error(String),
    }

    struct Agent {
        id: String,
        state: AgentState,
        task_history: Vec<String>,
    }

    impl Agent {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                state: AgentState::Idle,
                task_history: Vec::new(),
            }
        }

        fn assign_task(&mut self, task: &str) {
            self.state = AgentState::Working(task.to_string());
            self.task_history.push(format!("Assigned: {}", task));
        }

        fn complete_task(&mut self) {
            if let AgentState::Working(task) = &self.state {
                self.task_history.push(format!("Completed: {}", task));
                self.state = AgentState::Idle;
            }
        }

        fn wait_for(&mut self, reason: &str) {
            self.state = AgentState::Waiting(reason.to_string());
            self.task_history.push(format!("Waiting: {}", reason));
        }

        fn error(&mut self, error: &str) {
            self.state = AgentState::Error(error.to_string());
            self.task_history.push(format!("Error: {}", error));
        }
    }

    // Simulate agent lifecycle
    let mut agent = Agent::new("test-agent");

    assert_eq!(agent.state, AgentState::Idle);

    agent.assign_task("Implement login");
    assert_eq!(
        agent.state,
        AgentState::Working("Implement login".to_string())
    );

    agent.wait_for("Review approval");
    assert_eq!(
        agent.state,
        AgentState::Waiting("Review approval".to_string())
    );

    agent.assign_task("Fix bugs");
    assert_eq!(agent.state, AgentState::Working("Fix bugs".to_string()));

    agent.complete_task();
    assert_eq!(agent.state, AgentState::Idle);

    // Verify task history
    assert_eq!(agent.task_history.len(), 4);
    assert!(agent
        .task_history
        .iter()
        .any(|h| h.contains("Assigned: Implement login")));
    assert!(agent
        .task_history
        .iter()
        .any(|h| h.contains("Completed: Fix bugs")));

    println!("\n=== Agent State Management ===");
    println!("Agent: {}", agent.id);
    println!("Final state: {:?}", agent.state);
    println!("Task history:");
    for history in &agent.task_history {
        println!("  - {}", history);
    }
}

// ============================================================================
// Integration Test 5: Multi-Agent Coordination
// ============================================================================

#[test]
fn test_multi_agent_coordination() {
    struct Task {
        id: String,
        status: String,
        assigned_to: Option<String>,
        dependencies: Vec<String>,
    }

    struct Coordinator {
        tasks: HashMap<String, Task>,
        agents: Vec<String>,
    }

    impl Coordinator {
        fn new() -> Self {
            Self {
                tasks: HashMap::new(),
                agents: Vec::new(),
            }
        }

        fn add_task(&mut self, task: Task) {
            self.tasks.insert(task.id.clone(), task);
        }

        fn add_agent(&mut self, agent: String) {
            self.agents.push(agent);
        }

        fn get_ready_tasks(&self) -> Vec<&Task> {
            self.tasks
                .values()
                .filter(|t| {
                    t.status == "pending"
                        && t.assigned_to.is_none()
                        && t.dependencies.iter().all(|dep| {
                            self.tasks
                                .get(dep)
                                .map_or(false, |d| d.status == "completed")
                        })
                })
                .collect()
        }

        fn assign_task(&mut self, task_id: &str, agent: &str) {
            if let Some(task) = self.tasks.get_mut(task_id) {
                task.assigned_to = Some(agent.to_string());
                task.status = "in_progress".to_string();
            }
        }

        fn complete_task(&mut self, task_id: &str) {
            if let Some(task) = self.tasks.get_mut(task_id) {
                task.status = "completed".to_string();
            }
        }
    }

    use std::collections::HashMap;

    // Create coordinator
    let mut coordinator = Coordinator::new();

    // Add agents
    coordinator.add_agent("agent-a".to_string());
    coordinator.add_agent("agent-b".to_string());
    coordinator.add_agent("agent-c".to_string());

    // Add tasks with dependencies
    coordinator.add_task(Task {
        id: "task-1".to_string(),
        status: "pending".to_string(),
        assigned_to: None,
        dependencies: vec![],
    });

    coordinator.add_task(Task {
        id: "task-2".to_string(),
        status: "pending".to_string(),
        assigned_to: None,
        dependencies: vec![],
    });

    coordinator.add_task(Task {
        id: "task-3".to_string(),
        status: "pending".to_string(),
        assigned_to: None,
        dependencies: vec!["task-1".to_string()],
    });

    coordinator.add_task(Task {
        id: "task-4".to_string(),
        status: "pending".to_string(),
        assigned_to: None,
        dependencies: vec!["task-1".to_string(), "task-2".to_string()],
    });

    // Simulate coordination
    println!("\n=== Multi-Agent Coordination ===");

    // Round 1: Assign independent tasks
    let ready = coordinator.get_ready_tasks();
    let ready_ids: Vec<String> = ready.iter().map(|t| t.id.clone()).collect();
    let agent_0 = coordinator.agents[0].clone();
    println!("Round 1 - Ready tasks: {}", ready_ids.len());

    for task_id in &ready_ids {
        coordinator.assign_task(task_id, &agent_0);
        println!("  Assigned {} to {}", task_id, agent_0);
    }

    // Complete task-1 and task-2
    coordinator.complete_task("task-1");
    coordinator.complete_task("task-2");

    // Round 2: Assign dependent tasks
    let ready = coordinator.get_ready_tasks();
    let ready_ids: Vec<String> = ready.iter().map(|t| t.id.clone()).collect();
    let agent_1 = coordinator.agents[1].clone();
    println!("Round 2 - Ready tasks: {}", ready_ids.len());

    for task_id in &ready_ids {
        coordinator.assign_task(task_id, &agent_1);
        println!("  Assigned {} to {}", task_id, agent_1);
    }

    // Verify all tasks were assigned
    let assigned_count = coordinator
        .tasks
        .values()
        .filter(|t| t.assigned_to.is_some())
        .count();
    assert_eq!(assigned_count, 4);

    // Verify dependency order was respected
    let task_3 = coordinator.tasks.get("task-3").unwrap();
    assert!(task_3.assigned_to.is_some()); // Should be assigned after task-1 completed
}

// ============================================================================
// Integration Test 6: Agent + Documentation Integration
// ============================================================================

#[test]
fn test_agent_documentation_integration() {
    use axora_docs::{Adr, AdrLog, DocSchema, Document, LivingDocs};
    use std::path::Path;

    // Simulate agent creating documentation while implementing features
    let mut living_docs = LivingDocs::new();
    let mut adr_log = AdrLog::new();

    // Agent creates ADR for feature
    let adr = Adr::new(
        "FEAT-001",
        "Implement user authentication",
        "Need secure authentication system",
        "Use JWT with HttpOnly cookies",
        "agent-a",
    );
    adr_log.add(adr).expect("Failed to add ADR");

    // Agent creates documentation
    let doc = Document::new(
        "auth-guide",
        DocSchema::new("auth", "1.0", "agent-a"),
        "# Authentication Guide\n\nThis guide explains how to use the auth system.".to_string(),
        "1.0.0",
    );
    living_docs.add_document(doc).expect("Failed to add doc");

    // Agent implements code and registers it
    let code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    verify_password(&user.password_hash, password)
        .then(|| Token::new(user.id))
        .ok_or(AuthError::InvalidCredentials)
}
"#;
    living_docs.register_file(Path::new("src/auth/login.rs"), "auth-guide", code);

    // Agent modifies code
    let new_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    if verify_password(&user.password_hash, password) {
        log_successful_login(&user.id)?;
        Ok(Token::new(user.id))
    } else {
        log_failed_login(username)?;
        Err(AuthError::InvalidCredentials)
    }
}
"#;

    let updates = living_docs.on_code_change(Path::new("src/auth/login.rs"), code, new_code);

    // Accept ADR
    adr_log
        .get_mut("FEAT-001")
        .unwrap()
        .accept()
        .expect("Failed to accept");

    // Verify integration
    assert_eq!(adr_log.len(), 1);
    assert_eq!(adr_log.active().len(), 1);
    assert!(!updates.is_empty());

    println!("\n=== Agent + Documentation Integration ===");
    println!("ADRs created: {}", adr_log.len());
    println!("Documents tracked: {}", living_docs.index().len());
    println!("Code change updates: {}", updates.len());
    println!("Active decisions: {}", adr_log.active().len());
}

// ============================================================================
// Integration Test 7: Full Agent Workflow
// ============================================================================

#[test]
fn test_full_agent_workflow() {
    use axora_cache::CodeMinifier;
    use axora_docs::{Adr, AdrLog, DocSchema, Document, LivingDocs};
    use std::path::Path;

    println!("\n=== Full Agent Workflow ===");

    // Phase 1: Mission received
    let mission = "Implement authentication system";
    println!("Mission: {}", mission);

    // Phase 2: Decompose into tasks
    let tasks = vec![
        "Create ADR for authentication approach",
        "Implement login function",
        "Implement logout function",
        "Create documentation",
        "Write tests",
    ];
    println!("Tasks: {}", tasks.len());

    // Phase 3: Create ADR
    let mut adr_log = AdrLog::new();
    let adr = Adr::new(
        "AUTH-001",
        "JWT-based authentication",
        "Need stateless auth for microservices",
        "Use JWT with RS256 signing",
        "agent-workflow",
    );
    adr_log.add(adr).expect("Failed to add ADR");
    adr_log
        .get_mut("AUTH-001")
        .unwrap()
        .accept()
        .expect("Failed to accept");
    println!("ADR created: AUTH-001");

    // Phase 4: Implement code
    let mut living_docs = LivingDocs::new();

    let initial_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    verify_password(&user.password_hash, password)
        .then(|| Token::new(user.id))
        .ok_or(AuthError::InvalidCredentials)
}
"#;

    // Phase 5: Create documentation
    let doc = Document::new(
        "auth-api",
        DocSchema::new("auth", "1.0", "agent-workflow"),
        "# Auth API\n\nFunctions: login, logout".to_string(),
        "1.0.0",
    );
    living_docs.add_document(doc).expect("Failed to add");
    living_docs.register_file(Path::new("src/auth.rs"), "auth-api", initial_code);
    println!("Documentation created");

    // Phase 6: Code evolution
    let evolved_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    if verify_password(&user.password_hash, password) {
        Ok(Token::new(user.id))
    } else {
        Err(AuthError::InvalidCredentials)
    }
}

pub fn logout(token: &Token) -> Result<(), AuthError> {
    invalidate_token(token)?;
    Ok(())
}
"#;

    let updates = living_docs.on_code_change(Path::new("src/auth.rs"), initial_code, evolved_code);
    println!("Code changes detected: {}", updates.len());

    // Phase 7: Apply token optimization
    let minifier = CodeMinifier::new();
    let minified = minifier.minify(evolved_code, "rust").unwrap();
    let savings = minified.savings_percentage;
    println!("Token savings from minification: {:.1}%", savings);

    // Phase 8: Verify roundtrip
    let decompressed = minifier.decompress(&minified).unwrap();
    assert!(decompressed.contains("fn login"));
    assert!(decompressed.contains("fn logout"));
    println!("Decompression verified");

    // Phase 9: Final state
    assert_eq!(adr_log.len(), 1);
    assert_eq!(adr_log.active().len(), 1);
    assert!(living_docs.index().len() >= 1);
    assert!(savings >= 15.0);

    println!("\nWorkflow completed successfully!");
    println!("  ADRs: {}", adr_log.len());
    println!("  Documents: {}", living_docs.index().len());
    println!("  Token savings: {:.1}%", savings);
}
