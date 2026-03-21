# R-18: CLI-First vs MCP Architecture Analysis

**Priority:** 🔴 CRITICAL (foundational architecture decision)  
**Status:** 📋 Research Required — Decision Pending  
**Date:** 2026-03-18  
**Source:** User requirement + Industry analysis (Aider, MCP standards)  

---

## 🎯 Problem Statement

**The Core Tension:**

OPENAKTA aims to be:
- ✅ **3-5x faster** than single-agent systems (requires parallel execution)
- ✅ **CLI-first** for developer experience (terminal-native, fast)
- ✅ **Secure** (sandboxed tool execution, no accidental file deletions)
- ✅ **Scalable** (concurrent agents, no blocking)

**The Conflict:**

| Approach | Benefits | Critical Flaws for OPENAKTA |
|----------|----------|-------------------------|
| **Pure CLI** (Aider model) | Fast, optimized, token-efficient | ❌ Single blocking loop, no parallelism, no sandboxing |
| **MCP (Model Context Protocol)** | Secure, concurrent, sandboxed | ⚠️ More complex, potential overhead |

**Decision Required:**
> Should OPENAKTA adopt a pure CLI model (like Aider) or use MCP over gRPC for multi-agent orchestration?

---

## 🔍 Current State Analysis

### What We Have Implemented

| Component | Status | Location | Architecture |
|-----------|--------|----------|--------------|
| **Blackboard v2** | ✅ Implemented | `crates/openakta-cache/src/blackboard/v2.rs` | Pub/Sub (MCP-compatible) |
| **Worker Agents** | ✅ Implemented | `crates/openakta-agents/src/worker_pool.rs` | Parallel execution |
| **Dual-Thread ReAct** | ✅ Designed | Research complete | Parallel planning/acting |
| **Tool Execution** | 📋 Planned | Not yet implemented | CLI or MCP? |
| **gRPC Services** | ✅ Scaffolded | `crates/openakta-proto/` | MCP-ready infrastructure |

### Current Architecture (As-Built)

```
┌─────────────────────────────────────────────────────────────────┐
│                    OPENAKTA Current Architecture                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐   │
│  │  Coordinator │────▶│  Blackboard  │◀────│ Worker Agent │   │
│  │  (State      │     │  (Shared     │     │  (Planning   │   │
│  │   Machine)   │     │   State)     │     │   Thread)    │   │
│  └──────────────┘     └──────────────┘     └──────────────┘   │
│                              │                                  │
│                              ▼                                  │
│                     ┌──────────────┐                           │
│                     │ Worker Agent │                           │
│                     │ (Acting      │                           │
│                     │  Thread)     │                           │
│                     └──────────────┘                           │
│                                                                  │
│  Current Tool Execution: Direct CLI calls (NOT sandboxed)       │
│  Current Communication: In-memory pub/sub (NOT gRPC/MCP)        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Gap Analysis:**
- ✅ Blackboard architecture (MCP-compatible)
- ✅ Worker pool (parallel execution ready)
- ❌ Tool execution (currently direct CLI, no sandboxing)
- ❌ Inter-agent communication (in-memory, not network-ready)
- ❌ RBAC (no role-based access control for tools)

---

## 📊 Deep Dive: CLI-First Model (Aider Approach)

### How Aider Works

```
User Terminal
     │
     ▼
┌─────────────────────────────────────────┐
│  Aider CLI (Single Agent Loop)          │
│                                         │
│  1. Read user input                     │
│  2. Send to LLM (with AST-compressed    │
│     context: ~500-2K tokens)            │
│  3. Receive diff response               │
│  4. Apply diff to files                 │
│  5. Run tests (optional)                │
│  6. Repeat (BLOCKING - no parallelism)  │
│                                         │
└─────────────────────────────────────────┘
```

### CLI Advantages

| Benefit | Impact | Relevance to OPENAKTA |
|---------|--------|-------------------|
| **Speed** | Sub-second response times | ✅ Highly relevant |
| **Token Efficiency** | AST compression + diffs = 90%+ savings | ✅ Already implemented |
| **Simplicity** | Single process, no orchestration | ⚠️ Conflicts with multi-agent goal |
| **Developer Experience** | Terminal-native, familiar | ✅ Aligned with CLI-first vision |

### CLI Critical Flaws for OPENAKTA

| Flaw | Impact | Why It Matters |
|------|--------|----------------|
| **Single Blocking Loop** | No parallelism | ❌ Destroys 3-5x speed goal |
| **No Tool Sandboxing** | Any agent can delete any file | ❌ Security risk |
| **No RBAC** | All agents have full system access | ❌ Violation of zero-trust |
| **No Network Transparency** | Can't distribute agents across machines | ❌ Limits scalability |
| **File System Collisions** | Concurrent agents can overwrite each other | ❌ Race conditions |

### Aider's Token Optimization (Worth Stealing)

**What OPENAKTA Should Adopt:**
1. ✅ **AST-Based Repository Maps** — Tree-sitter for context compression
2. ✅ **Diff-Only Responses** — Git-style patches (already planned)
3. ✅ **Prefix Caching** — Static prompts cached (already implemented)
4. ✅ **Context Pruning** — Send only relevant files (already planned via Influence Graph)

**What OPENAKTA Should NOT Adopt:**
1. ❌ **Single-Agent Loop** — Blocks parallelism
2. ❌ **Direct CLI Tool Execution** — No sandboxing
3. ❌ **In-Memory Communication** — Not network-transparent

---

## 📊 Deep Dive: MCP (Model Context Protocol)

### What Is MCP?

**Model Context Protocol** is an industry standard for:
- **Secure tool execution** (sandboxed, RBAC-enforced)
- **Context retrieval** (pull-based, on-demand)
- **Network transparency** (agents can run locally or remotely)

### MCP Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    MCP Architecture                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐   │
│  │  Agent A     │     │  Agent B     │     │  Agent C     │   │
│  │  (Frontend)  │     │  (Backend)   │     │  (Testing)   │   │
│  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘   │
│         │                    │                    │            │
│         └────────────────────┼────────────────────┘            │
│                              │                                  │
│                     ┌────────▼────────┐                        │
│                     │  MCP Server     │                        │
│                     │  (gRPC-based)   │                        │
│                     └────────┬────────┘                        │
│                              │                                  │
│         ┌────────────────────┼────────────────────┐            │
│         │                    │                    │            │
│  ┌──────▼───────┐     ┌──────▼───────┐     ┌──────▼───────┐   │
│  │ File System  │     │   Terminal   │     │   Testing    │   │
│  │ (Sandboxed)  │     │ (Sandboxed)  │     │ (Sandboxed)  │   │
│  └──────────────┘     └──────────────┘     └──────────────┘   │
│                                                                  │
│  Security: RBAC enforced at MCP server layer                    │
│  Communication: gRPC (network-transparent)                       │
│  Concurrency: Multiple agents can execute in parallel           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### MCP Advantages for OPENAKTA

| Benefit | Impact | Relevance to OPENAKTA |
|---------|--------|-------------------|
| **Parallel Execution** | Multiple agents run concurrently | ✅ Enables 3-5x speed goal |
| **Tool Sandboxing** | Each tool is isolated, permissioned | ✅ Zero-trust security |
| **RBAC** | Role-based access control (Frontend agent can't delete backend files) | ✅ Prevents accidents |
| **Network Transparency** | Agents can run on different machines | ✅ Future scalability |
| **Pull-Based Context** | Agents query Blackboard on-demand (not pushed) | ✅ Reduces token usage |
| **Audit Trail** | All tool calls logged at MCP layer | ✅ Compliance, debugging |

### MCP Potential Drawbacks

| Drawback | Mitigation |
|----------|------------|
| **Complexity** | Use `tonic` (Rust gRPC library) — already in workspace |
| **Latency Overhead** | gRPC is fast (<1ms overhead for local calls) |
| **Setup Complexity** | Provide pre-configured MCP server binaries |

---

## 🔬 Comparative Analysis

### Performance Comparison

| Metric | Pure CLI | MCP over gRPC | OPENAKTA Target |
|--------|----------|---------------|--------------|
| **Single Tool Call Latency** | ~5ms | ~6ms (+1ms gRPC overhead) | <10ms ✅ |
| **Parallel Tool Calls** | 1 at a time (blocking) | N at a time (concurrent) | 3-5x speedup ✅ |
| **Token Efficiency** | 90%+ (AST + diffs) | 90%+ (AST + diffs) | 90%+ ✅ |
| **Security** | Low (full system access) | High (sandboxed, RBAC) | High ✅ |
| **Scalability** | Single machine | Distributed | Distributed ✅ |

### Security Comparison

| Security Feature | Pure CLI | MCP over gRPC | OPENAKTA Requirement |
|------------------|----------|---------------|-------------------|
| **Tool Sandboxing** | ❌ No | ✅ Yes | ✅ Required |
| **RBAC** | ❌ No | ✅ Yes | ✅ Required |
| **Audit Logging** | ❌ No | ✅ Yes | ✅ Required |
| **Permission Escalation Prevention** | ❌ No | ✅ Yes | ✅ Required |
| **File System Isolation** | ❌ No | ✅ Yes | ✅ Required |

### Developer Experience Comparison

| DX Feature | Pure CLI | MCP over gRPC | OPENAKTA Target |
|------------|----------|---------------|--------------|
| **Terminal-Native** | ✅ Yes | ✅ Yes (CLI wrapper) | ✅ Required |
| **Fast Response** | ✅ Yes | ✅ Yes (<1ms overhead) | ✅ Required |
| **Simple Setup** | ✅ Yes | ⚠️ Moderate (MCP server) | ⚠️ Acceptable |
| **Debugging** | ⚠️ Moderate | ✅ Yes (audit trail) | ✅ Required |

---

## 🏗️ Recommended Architecture: Hybrid Approach

### "CLI-First, MCP-Backed" Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│              OPENAKTA Hybrid Architecture                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  User Interface Layer (CLI-First)                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Terminal UI (ink, ratatui)                              │  │
│  │  • Fast, responsive                                      │  │
│  │  • AST-compressed context display                        │  │
│  │  • Diff-only output                                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  Orchestration Layer (MCP-Backed)                              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Coordinator (State Machine)                             │  │
│  │  • Deterministic workflow                                │  │
│  │  • Parallel task dispatch                                │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  MCP Server (gRPC)                                       │  │
│  │  • RBAC enforcement                                      │  │
│  │  • Tool sandboxing                                       │  │
│  │  • Audit logging                                         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│         ┌────────────────────┼────────────────────┐            │
│         │                    │                    │            │
│  ┌──────▼───────┐     ┌──────▼───────┐     ┌──────▼───────┐   │
│  │ File System  │     │   Terminal   │     │   Testing    │   │
│  │ (Sandboxed)  │     │ (Sandboxed)  │     │ (Sandboxed)  │   │
│  └──────────────┘     └──────────────┘     └──────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **CLI for User Experience** — Terminal-native, fast, familiar
2. **MCP for Tool Execution** — Sandboxed, secure, concurrent
3. **AST Compression** — Tree-sitter for context reduction (90%+ savings)
4. **Diff-Only Output** — Git-style patches (89-98% output savings)
5. **Pull-Based Context** — Agents query Blackboard on-demand (not pushed)

---

## 📋 Research Questions

### Critical Questions to Answer

#### 1. MCP Implementation Complexity

**Questions:**
- How much effort to implement MCP server in Rust?
- Should we use existing MCP implementations or build from scratch?
- What is the learning curve for the team?

**Research Needed:**
- [ ] Survey existing Rust MCP implementations
- [ ] Estimate implementation effort (days/weeks)
- [ ] Identify potential blockers

---

#### 2. Performance Overhead

**Questions:**
- What is the actual latency overhead of gRPC for local tool calls?
- Does MCP add significant memory overhead?
- Can we achieve sub-10ms tool call latency with MCP?

**Research Needed:**
- [ ] Benchmark gRPC vs direct CLI calls
- [ ] Measure memory overhead of MCP server
- [ ] Profile end-to-end latency (agent → MCP → tool)

---

#### 3. Security Model

**Questions:**
- What RBAC policies should OPENAKTA enforce by default?
- How to sandbox tool execution without breaking functionality?
- Should users be able to customize permissions per agent type?

**Research Needed:**
- [ ] Survey industry RBAC best practices
- [ ] Design OPENAKTA-specific permission model
- [ ] Identify sandboxing strategies (containers, namespaces, etc.)

---

#### 4. Migration Path

**Questions:**
- How to migrate from current in-memory Blackboard to MCP-backed?
- Can we implement MCP incrementally (one tool at a time)?
- What is the rollback strategy if MCP causes issues?

**Research Needed:**
- [ ] Design incremental migration plan
- [ ] Identify low-risk tools to start with (e.g., file reading)
- [ ] Create rollback procedure

---

## 📊 Implementation Options

### Option 1: Pure CLI (Aider Model) — REJECTED

**Description:** Single-agent loop, direct CLI tool execution

**Pros:**
- ✅ Simple implementation
- ✅ Fast (no orchestration overhead)
- ✅ Token-efficient (AST + diffs)

**Cons:**
- ❌ No parallelism (blocks 3-5x speed goal)
- ❌ No sandboxing (security risk)
- ❌ No RBAC (all agents have full access)
- ❌ Not scalable (single machine only)

**Verdict:** ❌ **REJECTED** — Conflicts with core OPENAKTA goals

---

### Option 2: Pure MCP — REJECTED

**Description:** Full MCP implementation, no CLI optimizations

**Pros:**
- ✅ Secure (sandboxed, RBAC)
- ✅ Concurrent (parallel agents)
- ✅ Scalable (network-transparent)

**Cons:**
- ❌ Complex implementation
- ❌ Potential latency overhead
- ❌ Poor developer experience (not terminal-native)
- ❌ Token-inefficient (no AST compression)

**Verdict:** ❌ **REJECTED** — Over-engineered, poor DX

---

### Option 3: Hybrid (CLI-First, MCP-Backed) — RECOMMENDED

**Description:** CLI for UX, MCP for tool execution, AST compression, diff-only output

**Pros:**
- ✅ Fast (CLI-native UX)
- ✅ Secure (MCP sandboxing)
- ✅ Concurrent (parallel agents via MCP)
- ✅ Token-efficient (AST + diffs)
- ✅ Scalable (network-transparent)
- ✅ Developer-friendly (terminal-native)

**Cons:**
- ⚠️ Moderate complexity (two layers to maintain)
- ⚠️ Implementation effort (MCP server + CLI wrapper)

**Verdict:** ✅ **RECOMMENDED** — Best of both worlds

---

## 🛠️ Implementation Plan (If Hybrid Approved)

### Phase 1: MCP Foundation (Week 1-2)

| Sprint | Title | Duration | Owner |
|--------|-------|----------|-------|
| **M1** | MCP Server Scaffold | 2 days | Agent C |
| **M2** | gRPC Service Definitions | 2 days | Agent C |
| **M3** | RBAC Policy Engine | 2 days | Agent C |
| **M4** | Tool Sandboxing (File System) | 3 days | Agent C |

**Deliverables:**
- `crates/openakta-mcp-server/` — MCP server implementation
- `crates/openakta-proto/mcp.proto` — gRPC service definitions
- `crates/openakta-mcp-server/src/rbac.rs` — RBAC engine

---

### Phase 2: CLI Wrapper (Week 2-3)

| Sprint | Title | Duration | Owner |
|--------|-------|----------|-------|
| **M5** | Terminal UI (ratatui) | 3 days | Agent A |
| **M6** | AST Compression Integration | 2 days | Agent B |
| **M7** | Diff-Only Output Enforcement | 2 days | Agent C |
| **M8** | CLI → MCP Bridge | 2 days | Agent C |

**Deliverables:**
- `apps/cli/` — Terminal UI
- `apps/cli/src/ast_compressor.rs` — Tree-sitter integration
- `apps/cli/src/mcp_bridge.rs` — CLI to MCP communication

---

### Phase 3: Migration & Testing (Week 3-4)

| Sprint | Title | Duration | Owner |
|--------|-------|----------|-------|
| **M9** | Incremental Tool Migration | 3 days | Agent C |
| **M10** | Performance Benchmarking | 2 days | Agent A |
| **M11** | Security Audit | 2 days | Agent B |
| **M12** | Documentation & Polish | 2 days | Agent A |

**Deliverables:**
- `docs/MCP-MIGRATION-GUIDE.md` — Migration documentation
- `benches/mcp_bench.rs` — Performance benchmarks
- `docs/SECURITY-AUDIT.md` — Security validation

---

## 📈 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Tool Call Latency** | <10ms P95 | End-to-end (CLI → MCP → tool) |
| **Parallel Tool Calls** | 3-5x speedup | vs single-agent baseline |
| **Token Efficiency** | 90%+ savings | AST compression + diffs |
| **Security Incidents** | 0 | Sandboxing effectiveness |
| **Developer Satisfaction** | >4.5/5 | User feedback |

---

## 🔗 Related Documents

- [`01_CORE_ARCHITECTURE.md`](../docs/active_architecture/01_CORE_ARCHITECTURE.md) — Blackboard, orchestration
- [`03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`](../docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md) — AST compression, diffs
- [R-17: Multi-Agent Optimization](../research/findings/multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md) — Agent communication

---

## ✅ Decision Required

**Question:** Should OPENAKTA adopt the **Hybrid (CLI-First, MCP-Backed)** architecture?

**If Approved:**
- Agent C starts Phase 1 (MCP Foundation)
- Agent A prepares CLI wrapper (Phase 2)
- Agent B integrates AST compression (Phase 2)

**If Rejected:**
- Stick with current in-memory Blackboard
- Implement CLI optimizations only (AST, diffs)
- Accept security and scalability limitations

---

**Research Status:** 📋 **Pending Decision**  
**Priority:** 🔴 **CRITICAL** (blocks tool execution architecture)  
**Estimated Research Time:** 2-3 days  
**Estimated Implementation Time:** 4 weeks (12 sprints)
