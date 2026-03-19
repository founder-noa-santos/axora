# CLI-First vs MCP Research

**Mission:** Determine optimal architecture for AXORA tool execution  
**Status:** 📋 Research Required — Decision Pending  
**Priority:** 🔴 CRITICAL  

---

## 🎯 Goal

Decide whether AXORA should:
1. **Pure CLI** (Aider model) — Fast but single-agent, no sandboxing
2. **Pure MCP** — Secure but complex, potential overhead
3. **Hybrid** (CLI-First, MCP-Backed) — Best of both worlds (RECOMMENDED)

---

## 📁 Research Structure

```
cli-vs-mcp/
├── README.md                        ← This file (overview)
├── R-18-CLI-VS-MCP-ANALYSIS.md      ← Full analysis (10 pages)
└── IMPLEMENTATION-PLAN.md           ← If Hybrid approved (TBD)
```

---

## 🚀 Quick Start

### For Decision Makers

1. **Read Analysis:** [`R-18-CLI-VS-MCP-ANALYSIS.md`](./R-18-CLI-VS-MCP-ANALYSIS.md)
2. **Review Options:**
   - Option 1: Pure CLI (REJECTED)
   - Option 2: Pure MCP (REJECTED)
   - Option 3: Hybrid (RECOMMENDED)
3. **Make Decision:** Approve or reject Hybrid approach

### For Implementers (If Approved)

1. **Wait for:** Decision confirmation
2. **Then read:** `IMPLEMENTATION-PLAN.md` (to be created)
3. **Start:** Phase 1 (MCP Foundation)

---

## 📊 Current State

### What We Have

| Component | Status | Location | Architecture |
|-----------|--------|----------|--------------|
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` | Pub/Sub (MCP-compatible) |
| Worker Agents | ✅ Implemented | `crates/axora-agents/src/worker_pool.rs` | Parallel execution |
| gRPC Scaffold | ✅ Exists | `crates/axora-proto/` | MCP-ready |
| Tool Execution | ❌ Not implemented | — | CLI or MCP? |

### What We Need to Decide

| Question | Impact | Urgency |
|----------|--------|---------|
| CLI vs MCP? | Foundational architecture | 🔴 CRITICAL |
| Sandboxing strategy? | Security model | 🔴 CRITICAL |
| RBAC policies? | Access control | 🟡 HIGH |
| Migration path? | Implementation approach | 🟡 HIGH |

---

## 🔍 Key Findings (Preliminary)

### CLI Advantages (Aider Model)

- ✅ **Speed:** Sub-second response times
- ✅ **Token Efficiency:** AST compression + diffs = 90%+ savings
- ✅ **Simplicity:** Single process, no orchestration
- ✅ **Developer Experience:** Terminal-native, familiar

### CLI Critical Flaws for AXORA

- ❌ **Single Blocking Loop:** No parallelism (destroys 3-5x speed goal)
- ❌ **No Tool Sandboxing:** Any agent can delete any file
- ❌ **No RBAC:** All agents have full system access
- ❌ **No Network Transparency:** Can't distribute agents

### MCP Advantages

- ✅ **Parallel Execution:** Multiple agents run concurrently
- ✅ **Tool Sandboxing:** Each tool is isolated, permissioned
- ✅ **RBAC:** Role-based access control
- ✅ **Network Transparency:** Agents can run locally or remotely
- ✅ **Pull-Based Context:** Agents query Blackboard on-demand

### MCP Potential Drawbacks

- ⚠️ **Complexity:** More moving parts
- ⚠️ **Latency Overhead:** ~1ms per gRPC call (acceptable)
- ⚠️ **Setup Complexity:** Need MCP server binaries

---

## 🏗️ Recommended Architecture: Hybrid

### "CLI-First, MCP-Backed"

```
User Terminal (CLI)
     │
     ▼
┌─────────────────────────┐
│  CLI Wrapper            │
│  • Fast, responsive     │
│  • AST-compressed       │
│  • Diff-only output     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  MCP Server (gRPC)      │
│  • RBAC enforcement     │
│  • Tool sandboxing      │
│  • Audit logging        │
└───────────┬─────────────┘
            │
     ┌──────┴──────┬──────┐
     │             │      │
┌────▼────┐  ┌────▼────┐ ┌▼──────┐
│  Files  │  │Terminal │ │Tests  │
│(Sandbox)│  │(Sandbox)│ │(Sandbox)│
└─────────┘  └─────────┘ └───────┘
```

**Why Hybrid?**
- ✅ CLI for user experience (fast, terminal-native)
- ✅ MCP for tool execution (secure, concurrent)
- ✅ AST compression (90%+ token savings)
- ✅ Diff-only output (89-98% output savings)

---

## 📋 Decision Required

**Question:** Should AXORA adopt the **Hybrid (CLI-First, MCP-Backed)** architecture?

**If Approved:**
- Agent C starts Phase 1 (MCP Foundation) — 4 weeks
- Agent A prepares CLI wrapper (Phase 2) — 2 weeks
- Agent B integrates AST compression (Phase 2) — 1 week

**If Rejected:**
- Stick with current in-memory Blackboard
- Implement CLI optimizations only
- Accept security and scalability limitations

---

## 📚 Reference Files

- **Full Analysis:** `R-18-CLI-VS-MCP-ANALYSIS.md` (10 pages)
- **Core Architecture:** `docs/active_architecture/01_CORE_ARCHITECTURE.md`
- **Token Optimization:** `docs/active_architecture/03_CONTEXT_AND_TOKEN_OPTIMIZATION.md`

---

## ✅ Next Steps

1. **Review:** `R-18-CLI-VS-MCP-ANALYSIS.md`
2. **Decide:** Approve or reject Hybrid approach
3. **If Approved:** Create `IMPLEMENTATION-PLAN.md`
4. **If Rejected:** Document alternative approach

---

**Research Status:** 📋 **Pending Decision**  
**Priority:** 🔴 **CRITICAL** (blocks tool execution architecture)  
**Last Updated:** 2026-03-18
