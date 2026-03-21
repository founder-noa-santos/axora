# Phase 5: Integration & Testing

**Status:** 📋 PLANNED  
**Priority:** MEDIUM  
**Estimated Effort:** 5-7 days

## Summary

Integrate all components and implement comprehensive testing to ensure system reliability.

---

## Integration Points

### 1. Desktop ↔ Daemon Communication

```
┌──────────────┐      gRPC       ┌──────────────┐
│   Desktop    │ ◄─────────────► │    Daemon    │
│   (Tauri)    │   Port 50051    │   (Rust)     │
└──────────────┘                 └──────────────┘
       │                                │
       │                                │
       ▼                                ▼
┌──────────────┐                 ┌──────────────┐
│   React UI   │                 │   SQLite     │
│   + Query    │                 │   Database   │
└──────────────┘                 └──────────────┘
```

### 2. Data Flow

```
User Action → Desktop UI → gRPC Call → Daemon Handler
    │                                          │
    │                                          ▼
    │                                   Database Write
    │                                          │
    │                                          ▼
    ◄─────────────────────────────────── Response
    │
    ▼
UI Update (React Query invalidates)
```

---

## Testing Strategy

### Level 1: Unit Tests

**Coverage Target:** 80%

```rust
// crates/openakta-storage/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_store_create() {
        let db = create_test_db();
        let store = AgentStore::new(&db.conn);
        
        let agent = store.create("test", "coder").unwrap();
        assert_eq!(agent.name, "test");
        assert_eq!(agent.role, "coder");
    }
    
    #[test]
    fn test_task_store_assignment() {
        // Test task assignment logic
    }
    
    #[test]
    fn test_message_store_retrieval() {
        // Test message retrieval
    }
}
```

### Level 2: Integration Tests

```rust
// crates/openakta-core/tests/integration.rs

#[tokio::test]
async fn test_agent_registration_flow() {
    // Start test daemon
    let daemon = TestDaemon::start().await;
    
    // Create gRPC client
    let client = daemon.create_client().await;
    
    // Register agent
    let response = client.register_agent(RegisterAgentRequest {
        name: "test-agent".to_string(),
        role: "coder".to_string(),
        metadata: HashMap::new(),
    }).await.unwrap();
    
    // Verify agent was created
    assert!(response.agent.is_some());
    
    // List agents
    let agents = client.list_agents(ListAgentsRequest {}).await.unwrap();
    assert_eq!(agents.agents.len(), 1);
}

#[tokio::test]
async fn test_task_lifecycle() {
    // Create task
    // Assign to agent
    // Update status
    // Complete task
    // Verify final state
}
```

### Level 3: End-to-End Tests

```typescript
// apps/desktop/tests/e2e/agent-flow.spec.ts

import { test, expect } from '@playwright/test';

test('can register and view agent', async ({ page }) => {
  // Start daemon
  await daemon.start();
  
  // Open desktop app
  await page.goto('http://localhost:3000');
  
  // Register agent
  await page.click('[data-testid="register-agent"]');
  await page.fill('[name="agentName"]', 'TestBot');
  await page.select('[name="role"]', 'coder');
  await page.click('[type="submit"]');
  
  // Verify agent appears in list
  await expect(page.locator('[data-testid="agent-card"]'))
    .toContainText('TestBot');
});
```

---

## Test Commands

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run specific test
cargo test test_agent_registration

# Run integration tests
cargo test --test integration

# Desktop tests
cd apps/desktop
pnpm test

# E2E tests
pnpm test:e2e
```

---

## CI/CD Pipeline

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Install protobuf
        run: sudo apt-get install -y protobuf-compiler
      
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Build
        run: cargo build --verbose
      
      - name: Run tests
        run: cargo test --verbose
      
      - name: Run clippy
        run: cargo clippy -- -D warnings

  desktop:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      
      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8
      
      - name: Install dependencies
        run: pnpm install
      
      - name: Type check
        run: pnpm typecheck
      
      - name: Lint
        run: pnpm lint
      
      - name: Build desktop
        run: pnpm build
```

---

## Performance Benchmarks

```rust
// crates/openakta-core/benches/frame_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use openakta_core::FrameExecutor;

fn bench_frame_execution(c: &mut Criterion) {
    let executor = FrameExecutor::new();
    
    c.bench_function("frame_execute_100_ops", |b| {
        b.iter(|| {
            for _ in 0..100 {
                executor.process_operation(black_box(Operation::Noop));
            }
        })
    });
}

fn bench_agent_registration(c: &mut Criterion) {
    c.bench_function("register_agent", |b| {
        b.iter(|| {
            // Benchmark agent registration
        })
    });
}

criterion_group!(benches, bench_frame_execution, bench_agent_registration);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
```

---

## Error Handling

### Error Types

```rust
// crates/openakta-core/src/error.rs

#[derive(Error, Debug)]
pub enum OpenaktaError {
    #[error("Database error: {0}")]
    Database(#[from] StorageError),
    
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
}

pub type Result<T> = std::result::Result<T, OpenaktaError>;
```

### Error Responses

```rust
// Map internal errors to gRPC status codes
fn map_error(err: OpenaktaError) -> tonic::Status {
    match err {
        OpenaktaError::AgentNotFound(_) => tonic::Status::not_found(err.to_string()),
        OpenaktaError::TaskNotFound(_) => tonic::Status::not_found(err.to_string()),
        OpenaktaError::Database(_) => tonic::Status::internal(err.to_string()),
        OpenaktaError::Connection(_) => tonic::Status::unavailable(err.to_string()),
        _ => tonic::Status::unknown(err.to_string()),
    }
}
```

---

## Logging & Debugging

### Structured Logging

```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(self), fields(agent_id = %agent.id))]
async fn register_agent(&self, agent: Agent) -> Result<()> {
    debug!("Registering agent");
    
    // ... registration logic
    
    info!(agent_id = %agent.id, "Agent registered successfully");
    Ok(())
}
```

### Log Configuration

```rust
// Initialize with environment-based filtering
tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::from_default_env()
            .add_directive("openakta=debug".parse().unwrap())
            .add_directive("tokio=info".parse().unwrap())
    )
    .with_target(true)
    .with_thread_ids(true)
    .init();
```

Run with custom log level:
```bash
RUST_LOG=openakta=debug,tonic=info cargo run -p openakta-daemon
```

---

## Acceptance Criteria

- [ ] All unit tests pass (>80% coverage)
- [ ] Integration tests pass
- [ ] E2E tests pass
- [ ] CI/CD pipeline works
- [ ] No memory leaks (verified with valgrind/sanitizers)
- [ ] Performance benchmarks meet targets
- [ ] Error handling is comprehensive
- [ ] Logging provides useful debug info

---

## Related Phases

- Phase 1: ✅ Daemon Build Fixes
- Phase 2: 🔄 Storage Implementation
- Phase 3: 📋 Desktop App
- Phase 4: 📋 Agent System
- Phase 6: 📋 Production Readiness
