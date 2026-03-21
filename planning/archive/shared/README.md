# OPENAKTA Development Plan

## Overview

This folder contains the comprehensive development plan for the OPENAKTA Multi-Agent Coding System.

**Vision:** "A primeira equipe de AI que trabalha pra você, do seu jeito."

**Target:** Individual developers who want a ready-to-use team of specialized AI agents.

**Key Differentiators:**
1. Multi-agent swarm (not a single assistant)
2. Specialized agents out-of-the-box
3. Configurable & flexible (BYOK or pay-us)
4. Token efficiency innovation (all layers)

---

## Project Status Summary

| Phase | Status | Completion | Notes |
|-------|--------|------------|-------|
| Phase 0: Business Alignment | ✅ COMPLETED | 100% | 16 questions answered |
| Phase R: Research & Architecture | ✅ COMPLETED | 100% | 8/8 areas, 42 ADRs |
| **Phase 1: Foundation Implementation** | **🔄 IN PROGRESS** | **0%** | **4 sprints, starting now** |
| Phase 2: Agent Framework | 📋 PLANNED | 0% | After Phase 1 |
| Phase 3: Token Optimization | 📋 PLANNED | 0% | After Phase 2 |
| Phase 4: Desktop App | 📋 PLANNED | 0% | After Phase 3 |
| Phase 5: Integration & Testing | 📋 PLANNED | 0% | After Phase 4 |

---

## Research Summary (100% Complete 🎉)

**8/8 research areas complete with 42 ADRs:**

| Area | ADRs | Key Decisions |
|------|------|---------------|
| R-01: Context Management | 6 | Modular RAG, Jina embeddings, Qdrant, AST chunking, Merkle sync |
| R-02: Inter-Agent Communication | 6 | NATS JetStream, Protobuf, State machine, MCP, Capability security |
| R-03: Token Efficiency | 7 | 90% cost reduction, Prefix caching, Diff comms, Minification, TOON |
| R-04: Local Indexing | 7 | Qdrant embedded, Tree-sitter, Hybrid retrieval, <100ms P95 |
| R-05: Model Optimization | 6 | Qwen 2.5 Coder 7B/32B, Ollama, Multi-model routing |
| R-06: Agent Architecture | 3 | Hierarchical state machine, Capability assignment |
| R-07: Memory & State | 2 | Multi-tier memory, Shared blackboard |
| R-08: Evaluation | 2 | Multi-dimensional eval, Production monitoring |

**See:** [`../research/README.md`](../research/README.md) for complete research summary

---

## Current Phase: Foundation Implementation

**Phase 1** implements the core RAG and indexing infrastructure:

### Sprint 1: Project Setup & Dependencies (Week 1)
- [ ] Create new crate structure
- [ ] Add all dependencies
- [ ] Set up basic module structure
- [ ] Verify build

### Sprint 2: Embeddings & Vector Search (Week 2)
- [ ] Implement Jina Code Embeddings v2
- [ ] Set up Qdrant embedded
- [ ] Create HNSW index
- [ ] Benchmark performance

### Sprint 3: Code Chunking & Indexing (Week 3)
- [ ] Implement Tree-sitter chunking
- [ ] Create Merkle tree sync
- [ ] Implement incremental indexing
- [ ] Benchmark indexing speed

### Sprint 4: RAG Pipeline & Integration (Week 4)
- [ ] Implement hybrid retrieval
- [ ] Implement cross-encoder re-ranking
- [ ] Implement context reordering
- [ ] End-to-end integration testing

**Success Criteria:**
- Query latency P95: <100ms
- Retrieval recall @10: >95%
- Indexing speed: >100 files/sec
- Incremental sync: <5s

**See:** [`PHASE-001-foundation-implementation.md`](./PHASE-001-foundation-implementation.md)

## Phase Documents

### [Phase 1: Daemon Build Fixes](./PHASE-001-daemon-fixes.md) ✅
**Completed:** March 16, 2026

Fixed all compilation issues to get the daemon building and running.

**Key fixes:**
- Updated Rust toolchain (1.75.0 → 1.94.0)
- Fixed protobuf serde derive issues
- Added missing dependencies
- Fixed timestamp conversion

**Result:** Daemon compiles and runs successfully.

---

### [Phase 2: Storage Implementation](./PHASE-002-storage-implementation.md) 🔄
**Priority:** HIGH  
**Estimate:** 3-5 days

Implement the SQLite storage layer with migrations and full CRUD operations.

**Tasks:**
- [ ] Create database migrations
- [ ] Implement AgentStore CRUD
- [ ] Implement TaskStore CRUD
- [ ] Implement MessageStore CRUD
- [ ] Add integration tests

---

### [Phase 3: Desktop App](./PHASE-003-desktop-app.md) 📋
**Priority:** HIGH  
**Estimate:** 5-7 days

Research and implement the desktop application.

**Key decisions:**
- ✅ **Stay with Tauri v2** - Best Rust integration, good performance
- ❌ Electron - Too heavy
- ❌ Wails - Would require Go
- ⚠️ Consider Iced/Dioxus for future

**Tasks:**
- [ ] Set up gRPC client
- [ ] Implement React Query
- [ ] Create agent management UI
- [ ] Create task dashboard
- [ ] Implement message streaming

---

### [Phase 4: Agent System](./PHASE-004-agent-system.md) 📋
**Priority:** HIGH  
**Estimate:** 5-7 days

Implement core agent lifecycle management.

**Features:**
- Agent registration with persistence
- Heartbeat system for health monitoring
- Task assignment logic
- Agent capabilities matching
- Session management

**Tasks:**
- [ ] Integrate storage with server
- [ ] Implement heartbeat system
- [ ] Create task assigner
- [ ] Add capabilities matching
- [ ] Implement session manager

---

### [Phase 5: Integration & Testing](./PHASE-005-integration-testing.md) 📋
**Priority:** MEDIUM  
**Estimate:** 5-7 days

Integrate all components and implement comprehensive testing.

**Testing levels:**
1. Unit tests (80% coverage target)
2. Integration tests
3. End-to-end tests
4. Performance benchmarks

**Tasks:**
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Set up E2E testing
- [ ] Configure CI/CD pipeline
- [ ] Add performance benchmarks

---

### Phase 6: Production Readiness 📋
**Priority:** MEDIUM  
**Estimate:** 3-5 days

Prepare for production deployment.

**Tasks:**
- [ ] Security audit
- [ ] Performance optimization
- [ ] Documentation
- [ ] Release process
- [ ] Monitoring setup

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Desktop Application                       │
│                    (Tauri v2 + React + TypeScript)              │
└───────────────────────────────┬─────────────────────────────────┘
                                │ gRPC (Port 50051)
┌───────────────────────────────▼─────────────────────────────────┐
│                         OPENAKTA Daemon                             │
│                    (Tokio + Tonic gRPC Server)                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Config    │  │    Frame    │  │   Collective Server     │  │
│  │   Module    │  │   Executor  │  │   (gRPC Service)        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Protocol Buffer Definitions                 │    │
│  │           (Agent, Task, Message schemas)                │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              SQLite Storage Layer                        │    │
│  │         (Agents, Tasks, Messages, Sessions)             │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Backend Runtime | Rust 1.94.0 | ✅ |
| Async Runtime | Tokio | ✅ |
| gRPC Framework | Tonic | ✅ |
| Database | SQLite + rusqlite | ✅ |
| Migrations | Refinery | 🔄 |
| Desktop Framework | Tauri v2 | 🔄 |
| Frontend | React + TypeScript | 🔄 |
| State Management | React Query | 📋 |
| Protocol Buffers | Prost | ✅ |

---

## Quick Start

### Prerequisites
```bash
# Rust 1.94+
rustup install 1.94.0

# Protocol Buffers
brew install protobuf  # macOS
# or
apt-get install protobuf-compiler  # Linux

# Node.js 20+ and pnpm
nvm install 20
npm install -g pnpm
```

### Build Daemon
```bash
cd openakta
cargo build -p openakta-daemon
```

### Run Daemon
```bash
cargo run -p openakta-daemon -- --debug
```

### Build Desktop (when ready)
```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

---

## Current Issues & Blockers

### Resolved ✅
- Rust toolchain version mismatch
- Missing protobuf compiler
- prost-types serde compatibility
- Missing dependencies in Cargo.toml files
- Timestamp conversion errors

### Known Warnings (Non-blocking)
- 72 missing documentation warnings in generated proto code
- Some unused imports in storage and core crates

---

## Next Steps

1. **Immediate:** Complete Phase 2 (Storage Implementation)
   - Create database migrations
   - Implement CRUD operations
   - Add tests

2. **Short-term:** Start Phase 3 (Desktop App)
   - Set up gRPC client
   - Build basic UI components

3. **Medium-term:** Phase 4 (Agent System)
   - Implement heartbeat system
   - Add task assignment logic

---

## Contributing

When working on a phase:
1. Create a branch: `git checkout -b phase-2-storage`
2. Update the phase document with progress
3. Mark tasks as complete
4. Submit PR when phase is complete

---

## Contact & Resources

- **Repository:** https://github.com/openakta/aktacode
- **Architecture Docs:** `/docs/architecture.md`
- **Implementation Plan:** `/docs/implementation-plan.md`
