# Phase 2 Integration: Competitive Analysis — Open Source AI Agent Frameworks

**Date:** 2026-03-16  
**Source:** Research — "Competitive Analysis — Open Source AI Agent Frameworks"  
**Impact:** VALIDATES our architecture + Provides 5 NEW patterns to adopt  

---

## ✅ Executive Summary

**This research VALIDATES our Graph-Based Workflow pivot (R-10) and provides CRITICAL implementation patterns.**

**Key Confirmations:**
1. ✅ **AST-based token optimization** (Aider) — 90%+ reduction validated
2. ✅ **Graph-based deterministic routing** (LangGraph) — validates our pivot
3. ✅ **Reducer-based state resolution** (LangGraph, Paperclip) — validates our approach
4. ✅ **Coordinator/Manager pattern** (CrewAI) — validates our coordinator agent
5. ✅ **Conversational orchestration is anti-pattern** (AutoGen failure) — validates our pivot

**NEW Patterns to Adopt:**
1. **Sliding-Window Semaphores** (Dify) — concurrent throttling
2. **Atomic Checkout Semantics** (Paperclip) — task locking
3. **ACI Formatting** (SWE-Agent) — output truncation/pagination
4. **Repository Map + Graph Ranking** (Aider) — AST-based context compression
5. **AGENTS.md Living Document** (Industry standard) — architectural ledger

---

## 📊 Validation Matrix

| Our Decision | Research Validates | Source | Verdict |
|--------------|-------------------|--------|---------|
| Graph-Based > Conversational | ✅ AutoGen fails, LangGraph succeeds | Tier 1 & 2 | **VALIDATED** |
| AST-based Token Reduction | ✅ Aider achieves 90%+ | Aider | **VALIDATED** |
| Reducer-Based State | ✅ LangGraph, Paperclip use reducers | LangGraph, Paperclip | **VALIDATED** |
| Coordinator Pattern | ✅ CrewAI Manager Agent | CrewAI | **VALIDATED** |
| Local-First (Rust) | ✅ Python GIL bottleneck | All Python frameworks | **VALIDATED** |
| Sliding-Window Concurrency | ⚠️ NEW (not implemented) | Dify | **ADOPT** |
| Atomic Checkout | ⚠️ NEW (not implemented) | Paperclip | **ADOPT** |
| ACI Formatting | ⚠️ NEW (not implemented) | SWE-Agent | **ADOPT** |

**Conclusion:** Our R-10 pivot was CORRECT. Research provides 5 NEW implementation patterns.

---

## 🔄 Architecture Updates (NEW Patterns)

### 1. Sliding-Window Semaphore Concurrency (from Dify)

**Current:**
```rust
// Dispatch all tasks in parallel
for task in parallel_group {
    tokio::spawn(execute_task(task));
}
```

**Updated (with sliding-window semaphores):**
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ConcurrentExecutor {
    // Sliding window semaphore (limits concurrent tasks)
    semaphore: Arc<Semaphore>,
    
    // Pre-flight token calculator
    token_calculator: TokenCalculator,
}

impl ConcurrentExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            token_calculator: TokenCalculator::new(),
        }
    }
    
    /// Execute with sliding-window concurrency
    pub async fn execute_with_throttle(
        &self,
        tasks: Vec<Task>,
        max_tokens: usize,
    ) -> Result<Vec<TaskResult>> {
        let mut handles = Vec::new();
        
        for task in tasks {
            // Pre-flight token check (prevent mid-flight overflow)
            let estimated_tokens = self.token_calculator.estimate(&task)?;
            if estimated_tokens > max_tokens {
                return Err(Error::TokenBudgetExceeded);
            }
            
            // Acquire semaphore permit (throttles concurrency)
            let permit = self.semaphore.clone().acquire_owned().await?;
            
            // Spawn task (releases permit when complete)
            let handle = tokio::spawn(async move {
                let result = execute_task(task).await;
                drop(permit); // Release semaphore
                result
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks (sliding window ensures throughput)
        futures::future::try_join_all(handles).await
    }
}
```

**Why:** Prevents resource starvation (local LLM or API rate limits) while maximizing throughput.

---

### 2. Atomic Checkout Semantics (from Paperclip)

**NEW: Task Locking**
```rust
use sqlx::{PgPool, Transaction};

pub struct TaskQueue {
    db: PgPool,
}

impl TaskQueue {
    /// Atomic checkout (prevents duplicate execution)
    pub async fn checkout_task(
        &self,
        agent_id: &str,
    ) -> Result<Option<Task>> {
        let mut tx = self.db.begin().await?;
        
        // SELECT FOR UPDATE (locks row)
        let task = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks 
             WHERE status = 'pending' 
             ORDER BY priority DESC, created_at ASC 
             LIMIT 1 
             FOR UPDATE SKIP LOCKED"
        )
        .fetch_optional(&mut tx)
        .await?;
        
        if let Some(mut task) = task {
            // Atomic update (single-assignee model)
            task.status = TaskStatus::InProgress;
            task.assignee_id = Some(agent_id.to_string());
            task.checked_out_at = Some(chrono::Utc::now());
            
            sqlx::query("UPDATE tasks SET status = $1, assignee_id = $2, checked_out_at = $3 WHERE id = $4")
                .bind(&task.status)
                .bind(&task.assignee_id)
                .bind(&task.checked_out_at)
                .bind(&task.id)
                .execute(&mut tx)
                .await?;
            
            tx.commit().await?;
            Ok(Some(task))
        } else {
            Ok(None) // No pending tasks
        }
    }
}
```

**Why:** Prevents race conditions where multiple agents attempt same task.

---

### 3. ACI Formatting (from SWE-Agent)

**NEW: Output Truncation/Pagination**
```rust
pub struct ACIFormatter {
    max_output_lines: usize,
    max_stack_trace_lines: usize,
}

impl ACIFormatter {
    /// Format terminal output (truncate/paginate)
    pub fn format_output(&self, output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        
        if lines.len() > self.max_output_lines {
            // Truncate with summary
            let summary = format!(
                "[Output truncated: {} lines total. Showing first {} and last {} lines]",
                lines.len(),
                self.max_output_lines / 2,
                self.max_output_lines / 2
            );
            
            let first_half = lines[..self.max_output_lines / 2].join("\n");
            let last_half = lines[lines.len() - self.max_output_lines / 2..].join("\n");
            
            format!("{}\n{}\n{}", first_half, summary, last_half)
        } else {
            output.to_string()
        }
    }
    
    /// Format stack trace (truncate deep traces)
    pub fn format_stack_trace(&self, trace: &str) -> String {
        let lines: Vec<&str> = trace.lines().collect();
        
        if lines.len() > self.max_stack_trace_lines {
            // Keep first N lines (root cause) + last N lines (actual error)
            let summary = format!("[{} frames omitted]", lines.len() - self.max_stack_trace_lines);
            
            let first_lines = lines[..10].join("\n");
            let last_lines = lines[lines.len() - 10..].join("\n");
            
            format!("{}\n{}\n{}", first_lines, summary, last_lines)
        } else {
            trace.to_string()
        }
    }
}
```

**Why:** Defends context window from arbitrary system bloat (infinite loops, massive stack traces).

---

### 4. Repository Map + Graph Ranking (from Aider)

**NEW: AST-based Context Compression**
```rust
use tree_sitter::{Parser, Tree};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::page_rank;

pub struct RepositoryMapper {
    parser: Parser,
    graph: DiGraph<Symbol, f32>,
}

impl RepositoryMapper {
    /// Build repository map (AST + graph ranking)
    pub fn build_map(&mut self, codebase_path: &Path) -> Result<RepositoryMap> {
        // Parse all files with tree-sitter
        for file in walkdir::WalkDir::new(codebase_path) {
            let file = file?;
            if is_code_file(file.path()) {
                self.parse_file(file.path())?;
            }
        }
        
        // Calculate PageRank (identify most referenced symbols)
        let ranks = page_rank(&self.graph, 0.85);
        
        // Build compressed map (top N symbols by rank)
        let mut symbols: Vec<_> = self.graph.node_indices()
            .map(|idx| (idx, ranks[idx.index()]))
            .collect();
        
        symbols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let top_symbols = symbols.into_iter()
            .take(100) // Top 100 symbols (fits in ~1000 tokens)
            .map(|(idx, _)| self.graph[idx].clone())
            .collect();
        
        Ok(RepositoryMap {
            symbols: top_symbols,
            token_count: self.estimate_tokens(&top_symbols),
        })
    }
}

pub struct RepositoryMap {
    pub symbols: Vec<Symbol>,
    pub token_count: usize,
}

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind, // Function, Class, Variable
    pub file_path: PathBuf,
    pub line_range: (usize, usize),
    pub references: usize, // Number of incoming references
}
```

**Why:** Achieves 90%+ token reduction (Aider validates this approach).

---

### 5. AGENTS.md Living Document (Industry Standard)

**NEW: Architectural Ledger**
```markdown
# OPENAKTA Architecture Ledger

**Last Updated:** 2026-03-16  
**Maintained By:** Architect Agent

## Current Architecture

### Coordinator Agent
- **Role:** Decompose missions, dispatch tasks, validate outputs
- **State:** `planning/coordinator/COORDINATION-BOARD.md`
- **Constraints:** Max 10 concurrent tasks, 50K token budget

### Worker Agents
- **Agent A:** Documentation Specialist
- **Agent B:** Storage + Context Specialist
- **Agent C:** Implementation Specialist

## Active Constraints

### Token Budget
- **Per-Task Limit:** 2,500 tokens (pruned context)
- **Per-Agent Limit:** 50,000 tokens (daily budget)
- **Enforcement:** Hard stop (circuit breaker)

### Concurrency Limits
- **Max Parallel Tasks:** 10 (sliding-window semaphore)
- **Rate Limit:** 100 requests/minute (API throttle)

## Execution Graph

```
Sprint 18 (A) → Sprint 19 (C)
Sprint 16 (B) → Sprint 17 (B) → Sprint 20 (B)
```

## Recent Changes

### 2026-03-16
- Adopted sliding-window semaphores (Dify pattern)
- Adopted atomic checkout semantics (Paperclip pattern)
- Adopted ACI formatting (SWE-Agent pattern)

### 2026-03-15
- Graph-Based Workflow pivot (R-10 validation)
- Implemented dual-thread ReAct loops
```

**Why:** Provides architectural visibility, prevents opaque delegation.

---

## 📋 NEW Implementation Sprints

Based on this research, we need to add:

### Sprint 21: Sliding-Window Semaphores (Agent B)
- Implement semaphore-based concurrency throttling
- Add pre-flight token calculator
- Integrate with existing ContextManager

### Sprint 22: Atomic Checkout (Agent B)
- Implement task queue with PostgreSQL (or SQLite for local-first)
- Add `checkout_task()` with `FOR UPDATE SKIP LOCKED`
- Integrate with Coordinator

### Sprint 23: ACI Formatting (Agent C)
- Implement output truncation/pagination
- Add stack trace formatter
- Integrate with ReAct loops (observation formatting)

### Sprint 24: Repository Map (Agent B)
- Implement tree-sitter parsing
- Add PageRank algorithm
- Build compressed repository map (top 100 symbols)

### Sprint 25: AGENTS.md Ledger (Agent A)
- Create architectural ledger template
- Implement auto-update on sprint completion
- Add constraint enforcement (token budgets, concurrency limits)

---

## ✅ Validation Metrics (from Research)

| Metric | Target | Industry Benchmark |
|--------|--------|-------------------|
| Token Reduction | 90%+ | Aider: 90%+ (AST-based) |
| Concurrency Speedup | 3-5x | CrewAI: Linear scaling |
| State Resolution | 0 race conditions | LangGraph: Reducer-based |
| Task Duplication | 0% | Paperclip: Atomic checkout |
| Context Overflow | 0 mid-flight errors | Dify: Pre-flight token check |

---

## 🔗 Updated References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Graph pivot (VALIDATED)
- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](./PHASE-2-INTEGRATION-REACT-PATTERNS.md) — ReAct patterns
- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](./PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Influence graph
- Research document — Competitive analysis (10 frameworks)

---

**This research CONFIRMS our R-10 pivot and provides 5 NEW implementation patterns.**

**No major pivots needed — just refinements and NEW pattern adoption.**
