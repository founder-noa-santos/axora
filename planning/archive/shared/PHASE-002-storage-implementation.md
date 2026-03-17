# Phase 2: Storage Layer Implementation

**Status:** 🔄 IN PROGRESS  
**Priority:** HIGH  
**Estimated Effort:** 3-5 days

## Summary

The storage layer has the structure in place but lacks actual database migrations and full CRUD implementations.

## Current State

### ✅ What Exists
- Database connection manager (`crates/axora-storage/src/db.rs`)
- Store structs for Agent, Task, Message operations
- Basic error handling with `StorageError`

### ❌ What's Missing
1. **Database migrations** - No SQL schema exists
2. **CRUD implementations** - Most methods return placeholders
3. **Integration tests** - No test coverage

## Implementation Plan

### Step 1: Create Database Migrations

Create `crates/axora-storage/migrations/0001_init.sql`:

```sql
-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'idle',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    assignee_id TEXT REFERENCES agents(id),
    result TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    sender_id TEXT NOT NULL REFERENCES agents(id),
    recipient_id TEXT NOT NULL REFERENCES agents(id),
    message_type INTEGER NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    read BOOLEAN NOT NULL DEFAULT FALSE
);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    task_id TEXT REFERENCES tasks(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'active'
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee_id);
CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_id);
CREATE INDEX IF NOT EXISTS idx_messages_recipient ON messages(recipient_id);
CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_id);
```

### Step 2: Implement Database Migrations

Update `crates/axora-storage/src/db.rs`:

```rust
use refinery::{embed_migrations, Runner};

embed_migrations!("migrations");

pub fn migrate(&self, conn: &mut Connection) -> Result<()> {
    let mut runner = Runner::new();
    runner.set_abort_divergent(true);
    runner.run(conn)?;
    info!("Database migrations completed");
    Ok(())
}
```

Add to `axora-storage/Cargo.toml`:
```toml
refinery = { version = "0.8", features = ["rusqlite"] }
```

### Step 3: Implement AgentStore CRUD

```rust
/// Get an agent by ID
pub fn get(&self, id: &str) -> Result<Agent> {
    let mut stmt = self.conn.prepare(
        "SELECT id, name, role, status, metadata, created_at, updated_at 
         FROM agents WHERE id = ?1"
    )?;
    
    let agent = stmt.query_row(params![id], |row| {
        let status_str: String = row.get(3)?;
        let status = match status_str.as_str() {
            "idle" => AgentStatus::Idle,
            "busy" => AgentStatus::Busy,
            _ => AgentStatus::Offline,
        };
        
        Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            role: row.get(2)?,
            status: status as i32,
            created_at: Some(parse_timestamp(&row.get::<_, String>(5)?)),
            updated_at: Some(parse_timestamp(&row.get::<_, String>(6)?)),
            metadata: serde_json::from_str(&row.get::<_, String>(4)?)?,
        })
    })?;
    
    Ok(agent)
}

/// List all agents
pub fn list(&self) -> Result<Vec<Agent>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, name, role, status, metadata, created_at, updated_at 
         FROM agents ORDER BY created_at DESC"
    )?;
    
    let agents = stmt.query_map([], |row| {
        // Similar parsing as get()
    })?;
    
    let mut result = Vec::new();
    for agent in agents {
        result.push(agent?);
    }
    Ok(result)
}
```

### Step 4: Implement TaskStore CRUD

Similar pattern to AgentStore, with additional methods for:
- `assign(task_id, agent_id)` - Assign task to agent
- `complete(task_id, result)` - Mark task as completed
- `update_status(task_id, status)` - Update task status

### Step 5: Implement MessageStore

- `store(&self, message)` - Already exists
- `get_for_agent(&self, agent_id, limit)` - Get messages for agent
- `mark_read(&self, message_id)` - Mark message as read
- `get_unread_count(&self, agent_id)` - Count unread messages

### Step 6: Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    fn create_test_db() -> (Connection, Database) {
        let temp = NamedTempFile::new().unwrap();
        let config = DatabaseConfig {
            path: temp.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        let db = Database::new(config);
        let conn = db.init().unwrap();
        (conn, db)
    }
    
    #[test]
    fn test_agent_create_and_get() {
        let (conn, _db) = create_test_db();
        let store = AgentStore::new(&conn);
        
        let agent = store.create("test-agent", "coder").unwrap();
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.role, "coder");
        
        let retrieved = store.get(&agent.id).unwrap();
        assert_eq!(retrieved.id, agent.id);
    }
}
```

## Dependencies to Add

```toml
# axora-storage/Cargo.toml
[dependencies]
refinery = { version = "0.8", features = ["rusqlite"] }

[dev-dependencies]
tempfile = "3.10"
```

## Acceptance Criteria

- [ ] Database migrations run on startup
- [ ] All CRUD operations implemented and tested
- [ ] Integration tests pass
- [ ] No compiler warnings
- [ ] Documentation for public APIs

## Risks

1. **Migration failures** - Need proper error handling
2. **Schema changes** - Design for extensibility
3. **Performance** - Add indexes as needed

## Related Issues

- Phase 1: Daemon Build Fixes (completed)
- Phase 3: Desktop App Implementation (pending)
- Phase 4: Agent System (pending)
