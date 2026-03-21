# Agent C — Sprint 23: ACI Formatting

**Phase:** 2  
**Sprint:** 23 (Implementation)  
**File:** `crates/openakta-agents/src/aci_formatter.rs`  
**Priority:** MEDIUM (defends context window)  
**Estimated Tokens:** ~80K output  

---

## 🎯 Task

Implement **ACI (Agent-Computer Interface) Formatting** (from SWE-Agent pattern) for output truncation/pagination.

### Context

Competitive analysis provides CRITICAL implementation details:
- **ACI Formatting** — Truncate/paginate system outputs
- **Stack Trace Truncation** — Keep root cause + error, omit middle frames
- **SWE-Agent Pattern** — Production-validated (prevents context overflow)

**Your job:** Implement ACI formatter (defends context window from bloat).

---

## 📋 Deliverables

### 1. Create aci_formatter.rs

**File:** `crates/openakta-agents/src/aci_formatter.rs`

**Core Structure:**
```rust
//! ACI (Agent-Computer Interface) Formatting
//!
//! This module implements production-grade output formatting:
//! - Truncation/pagination (defends context window)
//! - Stack trace truncation (keep root cause + error)
//! - SWE-Agent pattern (validated in production)

use std::fmt;

/// ACI Formatter (formats system outputs for LLM)
pub struct ACIFormatter {
    max_output_lines: usize,
    max_stack_trace_lines: usize,
    max_file_dump_lines: usize,
}

impl ACIFormatter {
    /// Create new formatter with defaults
    pub fn new() -> Self {
        Self {
            max_output_lines: 100,
            max_stack_trace_lines: 20,
            max_file_dump_lines: 50,
        }
    }
    
    /// Format terminal output (truncate/paginate)
    pub fn format_output(&self, output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        
        if lines.len() > self.max_output_lines {
            // Truncate with summary
            let summary = format!(
                "\n[Output truncated: {} lines total. Showing first {} and last {} lines]\n",
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
            // Keep first 10 lines (root cause) + last 10 lines (actual error)
            let omitted = lines.len() - self.max_stack_trace_lines;
            let summary = format!("\n[{} frames omitted]\n", omitted);
            
            let first_lines = lines[..10].join("\n");
            let last_lines = lines[lines.len() - 10..].join("\n");
            
            format!("{}\n{}\n{}", first_lines, summary, last_lines)
        } else {
            trace.to_string()
        }
    }
    
    /// Format file dump (truncate large files)
    pub fn format_file_dump(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.len() > self.max_file_dump_lines {
            let summary = format!(
                "\n[File truncated: {} lines total. Showing first {} lines]\n",
                lines.len(),
                self.max_file_dump_lines
            );
            
            let preview = lines[..self.max_file_dump_lines].join("\n");
            format!("{}\n{}", preview, summary)
        } else {
            content.to_string()
        }
    }
}

impl Default for ACIFormatter {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 2. Integrate with ReAct Loops

**File:** `crates/openakta-agents/src/react.rs` (UPDATE)

```rust
// Add to existing DualThreadReactAgent
impl DualThreadReactAgent {
    /// Execute tool with ACI formatting
    async fn execute_tool_with_formatting(
        &self,
        action: &Action,
    ) -> Result<Observation> {
        // Execute tool
        let raw_output = self.tools.execute(action).await?;
        
        // Format output (defend context window)
        let formatted_output = match action.tool_name.as_str() {
            "run_command" => self.aci_formatter.format_output(&raw_output),
            "read_file" => self.aci_formatter.format_file_dump(&raw_output),
            "get_stack_trace" => self.aci_formatter.format_stack_trace(&raw_output),
            _ => raw_output,
        };
        
        Ok(Observation {
            success: true,
            result: formatted_output,
            error: None,
        })
    }
}
```

---

### 3. Add Configuration

**File:** `crates/openakta-agents/src/aci_formatter.rs` (add to existing)

```rust
/// ACI configuration
#[derive(Debug, Clone)]
pub struct ACIConfig {
    /// Max output lines (before truncation)
    pub max_output_lines: usize,
    
    /// Max stack trace lines (before truncation)
    pub max_stack_trace_lines: usize,
    
    /// Max file dump lines (before truncation)
    pub max_file_dump_lines: usize,
}

impl Default for ACIConfig {
    fn default() -> Self {
        Self {
            max_output_lines: 100,
            max_stack_trace_lines: 20,
            max_file_dump_lines: 50,
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-agents/src/aci_formatter.rs` (NEW)

**Update:**
- `crates/openakta-agents/src/lib.rs` (add module export)
- `crates/openakta-agents/src/react.rs` (integrate formatting)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-indexing/` (Agent B's domain)
- `crates/openakta-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_output_truncation() { }

#[test]
fn test_stack_trace_truncation() { }

#[test]
fn test_file_dump_truncation() { }

#[test]
fn test_no_truncation_small_output() { }

#[test]
fn test_truncation_summary_format() { }

#[test]
fn test_react_loop_integration() { }

#[test]
fn test_context_window_defense() { }

#[test]
fn test_configuration_override() { }
```

---

## ✅ Success Criteria

- [ ] `aci_formatter.rs` created (ACI formatting)
- [ ] Output truncation works
- [ ] Stack trace truncation works
- [ ] File dump truncation works
- [ ] ReAct loop integration works
- [ ] Context window defense works
- [ ] 8+ tests passing
- [ ] Configuration override works

---

## 🔗 References

- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](../shared/PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- Research document — SWE-Agent pattern spec

---

**Start AFTER Sprint 9 (Dual-Thread ReAct) is complete.**

**Priority: MEDIUM — defends context window from bloat.**

**Dependencies:**
- Sprint 9 (Dual-Thread ReAct) — must complete first

**Blocks:**
- None (defensive improvement)
