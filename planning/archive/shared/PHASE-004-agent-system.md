# Phase 4: Agent System Implementation

**Status:** 📋 PLANNED  
**Priority:** HIGH  
**Estimated Effort:** 5-7 days

## Summary

Implement the core agent lifecycle management system including registration, status tracking, and task assignment.

## Current State

### ✅ What Exists
- gRPC service definition in `proto/collective/v1/core.proto`
- Server implementation in `crates/openakta-core/src/server.rs`
- In-memory agent storage in `CollectiveServer` struct
- Basic CRUD operations (register, unregister, list)

### ❌ What's Missing
1. **Persistent storage** - Agents not saved to database
2. **Agent status updates** - No heartbeat mechanism
3. **Task assignment logic** - No intelligent assignment
4. **Agent capabilities** - No skill/role matching
5. **Session management** - No tracking of active sessions

---

## Implementation Plan

### Step 1: Integrate Storage with Server

Update `crates/openakta-core/src/server.rs`:

```rust
pub struct CollectiveServer {
    config: CoreConfig,
    db: Arc<Database>,  // Add database connection
    message_tx: mpsc::Sender<Message>,
    message_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
}

impl CollectiveServer {
    pub fn new(config: CoreConfig, db: Arc<Database>) -> Self {
        Self {
            config,
            db,
            message_tx,
            message_rx,
        }
    }
}
```

### Step 2: Implement Persistent Agent Registration

```rust
async fn register_agent(
    &self,
    request: Request<RegisterAgentRequest>,
) -> Result<Response<RegisterAgentResponse>, Status> {
    let req = request.into_inner();
    debug!("Registering agent: {}", req.name);

    let conn = self.db.connect()
        .map_err(|e| Status::internal(e.to_string()))?;
    let store = AgentStore::new(&conn);

    let agent_proto = store.create(&req.name, &req.role)
        .map_err(|e| Status::internal(e.to_string()))?;

    info!("Agent registered: {} ({})", agent_proto.name, agent_proto.id);

    Ok(Response::new(RegisterAgentResponse {
        agent: Some(agent_proto),
    }))
}
```

### Step 3: Agent Heartbeat System

Create `crates/openakta-core/src/agent/heartbeat.rs`:

```rust
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub struct HeartbeatManager {
    agents: Arc<DashMap<String, AgentInfo>>,
    timeout_duration: Duration,
}

impl HeartbeatManager {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            timeout_duration: Duration::from_secs(timeout_secs),
        }
    }

    pub async fn record_heartbeat(&self, agent_id: &str) {
        self.agents.insert(
            agent_id.to_string(),
            AgentInfo {
                last_heartbeat: Instant::now(),
                status: AgentStatus::Idle,
            },
        );
    }

    pub async fn check_timeouts(&self) -> Vec<String> {
        let mut timed_out = Vec::new();
        let now = Instant::now();

        for mut entry in self.agents.iter_mut() {
            if now.duration_since(entry.value().last_heartbeat) > self.timeout_duration {
                entry.value_mut().status = AgentStatus::Offline;
                timed_out.push(entry.key().clone());
            }
        }

        timed_out
    }

    pub fn start_monitoring(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let timed_out = self.check_timeouts().await;
                for agent_id in timed_out {
                    warn!("Agent {} timed out", agent_id);
                    // Update status in database
                }
            }
        })
    }
}
```

### Step 4: Task Assignment Logic

Create `crates/openakta-core/src/agent/assignment.rs`:

```rust
pub struct TaskAssigner {
    db: Arc<Database>,
}

impl TaskAssigner {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Find the best agent for a task based on role and availability
    pub async fn find_best_agent(&self, required_role: &str) -> Result<Option<Agent>> {
        let conn = self.db.connect()?;
        let agent_store = AgentStore::new(&conn);
        let agents = agent_store.list()?;

        // Filter by role and status
        let available = agents
            .into_iter()
            .filter(|a| {
                a.role == required_role && 
                a.status == AgentStatus::Idle as i32
            })
            .collect::<Vec<_>>();

        // Simple round-robin: pick first available
        Ok(available.first().cloned())
    }

    /// Assign task to agent
    pub async fn assign_task(
        &self,
        task_id: &str,
        agent_id: &str,
    ) -> Result<()> {
        let conn = self.db.connect()?;
        let task_store = TaskStore::new(&conn);
        let agent_store = AgentStore::new(&conn);

        // Update task
        task_store.assign(task_id, agent_id)?;

        // Update agent status
        agent_store.update_status(agent_id, AgentStatus::Busy)?;

        Ok(())
    }
}
```

### Step 5: Agent Capabilities System

Create `crates/openakta-core/src/agent/capabilities.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub languages: Vec<String>,      // e.g., ["rust", "typescript", "python"]
    pub frameworks: Vec<String>,     // e.g., ["react", "tonic", "tauri"]
    pub tools: Vec<String>,          // e.g., ["git", "docker", "cargo"]
    pub max_concurrent_tasks: usize,
}

impl AgentCapabilities {
    pub fn matches_requirement(&self, required: &AgentCapabilities) -> bool {
        // Check if agent has all required languages
        required.languages.iter()
            .all(|lang| self.languages.contains(lang))
    }
}

// Update Agent registration to include capabilities
pub struct RegisterAgentRequest {
    pub name: String,
    pub role: String,
    pub capabilities: Option<AgentCapabilities>,
    pub metadata: HashMap<String, String>,
}
```

### Step 6: Session Management

Create `crates/openakta-core/src/agent/session.rs`:

```rust
pub struct SessionManager {
    db: Arc<Database>,
    sessions: DashMap<String, Session>,
}

impl SessionManager {
    pub fn start_session(
        &self,
        agent_id: &str,
        task_id: &str,
    ) -> Result<Session> {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            task_id: task_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
            status: SessionStatus::Active,
        };

        // Save to database
        let conn = self.db.connect()?;
        // ... insert session

        self.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    pub fn end_session(&self, session_id: &str) -> Result<()> {
        let mut session = self.sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound)?;

        session.ended_at = Some(Utc::now());
        session.status = SessionStatus::Completed;

        // Update database
        Ok(())
    }
}
```

---

## Updated Proto Definition

Add to `proto/collective/v1/core.proto`:

```protobuf
// Add to Agent message
message Agent {
  // ... existing fields ...
  AgentCapabilities capabilities = 8;
  int32 current_task_count = 9;
}

message AgentCapabilities {
  repeated string languages = 1;
  repeated string frameworks = 2;
  repeated string tools = 3;
  int32 max_concurrent_tasks = 4;
}

// New service for heartbeats
service AgentService {
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc GetAgentStatus(GetAgentStatusRequest) returns (GetAgentStatusResponse);
}

message HeartbeatRequest {
  string agent_id = 1;
  AgentStatus status = 2;
  optional string current_task_id = 3;
}

message HeartbeatResponse {
  bool acknowledged = 1;
  int64 server_time = 2;
}
```

---

## Agent Lifecycle Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent Lifecycle                          │
└─────────────────────────────────────────────────────────────────┘

1. Registration
   Agent → register_agent() → Daemon → Store in DB → Return Agent ID

2. Heartbeat (every 30 seconds)
   Agent → heartbeat(agent_id, status) → Daemon → Update last_seen

3. Task Assignment
   Task Created → Find Best Agent → Assign → Update Status → Notify

4. Timeout Detection
   Daemon checks every 10s → No heartbeat for 60s → Mark Offline

5. Unregistration
   Agent → unregister_agent() → Daemon → Remove from active → Keep in DB
```

---

## Testing Plan

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_agent_registration() { }
    
    #[test]
    fn test_heartbeat_timeout() { }
    
    #[test]
    fn test_task_assignment() { }
    
    #[test]
    fn test_capability_matching() { }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_full_agent_lifecycle() {
    // Start daemon
    // Register agent
    // Send heartbeats
    // Assign task
    // Verify state
}
```

---

## Acceptance Criteria

- [ ] Agents persist to database
- [ ] Heartbeat system detects offline agents
- [ ] Task assignment finds available agents
- [ ] Capabilities matching works
- [ ] Session tracking implemented
- [ ] All tests pass

---

## Related Phases

- Phase 1: ✅ Daemon Build Fixes
- Phase 2: 🔄 Storage Implementation
- Phase 3: 📋 Desktop App
- Phase 5: 📋 Integration & Testing
