# AOP Validator Specification

**Date:** 2026-03-16
**Status:** ADOPTED
**Implements:** ACONIC Decomposition Framework
**Priority:** HIGH (blocks Agent C implementation)

---

## 📋 Overview

### Purpose

The **AOP Validator** ensures task decompositions are **mathematically sound** before execution.

**AOP = And-Or-Parallel** validation:
- **Solvability** — Can agents actually do these tasks?
- **Completeness** — Do tasks cover all requirements?
- **Non-redundancy** — Is there no overlapping work?

### Why AOP Validation?

**Without validation:**
- LLM may create impossible tasks (no capable agent)
- LLM may miss requirements (incomplete coverage)
- LLM may duplicate work (redundant tasks)

**With validation:**
- ✅ Every task has a capable agent
- ✅ All requirements covered
- ✅ No wasted effort on duplicates

---

## ✅ Solvability Check

### Rule 1: Capability Match

**Every task must match at least one agent's capabilities.**

**Formal:**
```
∀ task ∈ tasks:
    ∃ agent ∈ agents:
        agent.capabilities ⊇ task.required_capabilities
```

**Implementation:**
```rust
pub fn check_capability_match(
    tasks: &[Task],
    agents: &[Agent],
) -> ValidationResult {
    let mut failures = Vec::new();
    
    for task in tasks {
        let has_capable_agent = agents.iter().any(|agent| {
            agent.capabilities.contains_all(&task.required_capabilities)
        });
        
        if !has_capable_agent {
            failures.push(CapabilityFailure {
                task_id: task.id.clone(),
                required: task.required_capabilities.clone(),
                available: agents.iter()
                    .flat_map(|a| a.capabilities.iter())
                    .cloned()
                    .collect(),
            });
        }
    }
    
    if failures.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} tasks have no capable agent: {:?}",
                failures.len(),
                failures.iter().map(|f| &f.task_id).collect::<Vec<_>>()
            ),
            details: ValidationDetails::CapabilityMismatch(failures),
        }
    }
}
```

**Example Failure:**
```
Task: "Deploy to Kubernetes cluster"
Required capabilities: [Kubernetes, Docker, CloudDeployment]

Available agents:
- Agent A: [Rust, Testing, CodeReview]
- Agent B: [Python, Testing, Documentation]

Result: FAILED
Reason: No agent has Kubernetes, Docker, or CloudDeployment capabilities
```

---

### Rule 2: Tool Availability

**Every task's required tools must be available.**

**Formal:**
```
∀ task ∈ tasks:
    task.required_tools ⊆ available_tools
```

**Implementation:**
```rust
pub fn check_tool_availability(
    tasks: &[Task],
    available_tools: &[Tool],
) -> ValidationResult {
    let mut failures = Vec::new();
    
    for task in tasks {
        for required_tool in &task.required_tools {
            if !available_tools.contains(required_tool) {
                failures.push(ToolFailure {
                    task_id: task.id.clone(),
                    required_tool: required_tool.clone(),
                    available_tools: available_tools.to_vec(),
                });
            }
        }
    }
    
    if failures.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} tool requirements not met",
                failures.len()
            ),
            details: ValidationDetails::ToolUnavailable(failures),
        }
    }
}
```

**Example Failure:**
```
Task: "Run integration tests"
Required tools: [PostgreSQL, Redis, TestRunner]

Available tools: [TestRunner, MockDatabase]

Result: FAILED
Reason: PostgreSQL and Redis not available
```

---

### Rule 3: Complexity Threshold

**Every task's treewidth must be within LLM handling capacity.**

**Formal:**
```
∀ task ∈ tasks:
    treewidth(task.constraint_graph) ≤ LLM_OPTIMAL_LINEWIDTH
```

**Implementation:**
```rust
pub fn check_complexity_threshold(
    tasks: &[Task],
    threshold: usize,
) -> ValidationResult {
    let mut failures = Vec::new();
    
    for task in tasks {
        let treewidth = calculate_treewidth(&task.constraint_graph);
        
        if treewidth > threshold {
            failures.push(ComplexityFailure {
                task_id: task.id.clone(),
                treewidth,
                threshold,
            });
        }
    }
    
    if failures.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} tasks exceed complexity threshold (treewidth > {})",
                failures.len(),
                threshold
            ),
            details: ValidationDetails::ComplexityExceeded(failures),
        }
    }
}
```

**Example Failure:**
```
Task: "Implement full OAuth2 flow with refresh tokens"
Treewidth: 8
Threshold: 5

Result: FAILED
Reason: Task complexity (treewidth 8) exceeds threshold (5)
Recommendation: Decompose into smaller tasks
```

---

## 📔 Completeness Check

### Rule 1: Requirement Coverage

**All explicit requirements from the mission must be covered by tasks.**

**Formal:**
```
all_requirements = extract_requirements(original_mission)
covered_requirements = ⋃{extract_requirements(task) | task ∈ tasks}
all_requirements == covered_requirements
```

**Implementation:**
```rust
pub fn check_requirement_coverage(
    mission: &Mission,
    tasks: &[Task],
) -> ValidationResult {
    // Extract requirements from mission
    let all_requirements = extract_requirements(mission);
    
    // Extract requirements covered by tasks
    let covered_requirements: HashSet<Requirement> = tasks
        .iter()
        .flat_map(|task| extract_requirements(task))
        .collect();
    
    // Find missing requirements
    let missing: Vec<Requirement> = all_requirements
        .difference(&covered_requirements)
        .cloned()
        .collect();
    
    if missing.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} requirements not covered: {:?}",
                missing.len(),
                missing
            ),
            details: ValidationDetails::MissingRequirements(missing),
        }
    }
}

fn extract_requirements(source: impl RequirementsSource) -> HashSet<Requirement> {
    let mut requirements = HashSet::new();
    
    // Parse explicit requirements
    for req in source.explicit_requirements() {
        requirements.insert(req);
    }
    
    // Parse implicit requirements
    for req in source.implicit_requirements() {
        requirements.insert(req);
    }
    
    requirements
}
```

**Example Failure:**
```
Mission: "Add user authentication with OAuth and rate limiting"

Extracted requirements:
- User authentication
- OAuth integration
- Rate limiting

Tasks:
- "Implement OAuth flow"
- "Add login endpoint"

Covered requirements:
- User authentication (via login endpoint)
- OAuth integration (via OAuth flow)

Missing requirements:
- Rate limiting

Result: FAILED
Reason: 1 requirement not covered: Rate limiting
```

---

### Rule 2: Implicit Requirements

**All inferred implicit requirements must be addressed.**

**Formal:**
```
∀ implicit ∈ infer_implicit_requirements(original_mission):
    ∃ task ∈ tasks:
        task.addresses(implicit)
```

**Implementation:**
```rust
pub fn check_implicit_requirements(
    mission: &Mission,
    tasks: &[Task],
) -> ValidationResult {
    let implicit_reqs = infer_implicit_requirements(mission);
    
    let mut unaddressed = Vec::new();
    
    for implicit in &implicit_reqs {
        let addressed = tasks.iter().any(|task| {
            task.addresses(implicit)
        });
        
        if !addressed {
            unaddressed.push(implicit.clone());
        }
    }
    
    if unaddressed.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} implicit requirements not addressed: {:?}",
                unaddressed.len(),
                unaddressed
            ),
            details: ValidationDetails::ImplicitNotAddressed(unaddressed),
        }
    }
}

fn infer_implicit_requirements(mission: &Mission) -> Vec<Requirement> {
    let mut implicit = Vec::new();
    
    // Security is always implicit for auth-related tasks
    if mission.contains_keywords(&["auth", "login", "oauth", "token"]) {
        implicit.push(Requirement::Security("Secure password handling".to_string()));
        implicit.push(Requirement::Security("Token expiration".to_string()));
        implicit.push(Requirement::Security("Rate limiting".to_string()));
    }
    
    // Error handling is always implicit
    implicit.push(Requirement::Quality("Error handling".to_string()));
    implicit.push(Requirement::Quality("Logging".to_string()));
    
    // Testing is always implicit
    implicit.push(Requirement::Quality("Unit tests".to_string()));
    
    implicit
}
```

**Example Failure:**
```
Mission: "Add user login with OAuth"

Inferred implicit requirements:
- Secure password handling
- Token expiration
- Rate limiting
- Error handling
- Logging
- Unit tests

Tasks:
- "Implement OAuth flow"
- "Add login endpoint"

Addressed implicit requirements:
- Secure password handling (via OAuth)
- Token expiration (via OAuth)

Unaddressed implicit requirements:
- Rate limiting
- Error handling
- Logging
- Unit tests

Result: FAILED
Reason: 4 implicit requirements not addressed
```

---

## 🚫 Non-redundancy Check

### Rule 1: Responsibility Overlap

**No two tasks should have overlapping responsibilities.**

**Formal:**
```
∀ (task_a, task_b) ∈ combinations(tasks, 2):
    task_a.responsibilities ∩ task_b.responsibilities = ∅
```

**Implementation:**
```rust
pub fn check_responsibility_overlap(
    tasks: &[Task],
) -> ValidationResult {
    let mut overlaps = Vec::new();
    
    for (i, task_a) in tasks.iter().enumerate() {
        for task_b in tasks.iter().skip(i + 1) {
            let overlap: Vec<Responsibility> = task_a.responsibilities
                .intersection(&task_b.responsibilities)
                .cloned()
                .collect();
            
            if !overlap.is_empty() {
                overlaps.push(OverlapFailure {
                    task_a_id: task_a.id.clone(),
                    task_b_id: task_b.id.clone(),
                    overlapping_responsibilities: overlap,
                });
            }
        }
    }
    
    if overlaps.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Failed {
            reason: format!(
                "{} task pairs have overlapping responsibilities",
                overlaps.len()
            ),
            details: ValidationDetails::ResponsibilityOverlap(overlaps),
        }
    }
}
```

**Example Failure:**
```
Task A: "Implement OAuth flow"
Responsibilities: [OAuth integration, Token handling, User authentication]

Task B: "Add login endpoint"
Responsibilities: [User authentication, Session management, Password validation]

Overlap: [User authentication]

Result: FAILED
Reason: 1 task pair has overlapping responsibilities
Recommendation: Merge tasks or clarify responsibility boundaries
```

---

### Rule 2: Duplicate Tool Calls

**No two tasks should make identical tool calls (unless legitimately needed).**

**Formal:**
```
∀ (task_a, task_b) ∈ combinations(tasks, 2):
    ¬(task_a.tool_calls == task_b.tool_calls ∧ task_a.tool_calls ≠ ∅)
```

**Implementation:**
```rust
pub fn check_duplicate_tool_calls(
    tasks: &[Task],
) -> ValidationResult {
    let mut duplicates = Vec::new();
    
    for (i, task_a) in tasks.iter().enumerate() {
        for task_b in tasks.iter().skip(i + 1) {
            // Skip if both tasks have no tool calls
            if task_a.tool_calls.is_empty() {
                continue;
            }
            
            // Check for identical tool calls
            if task_a.tool_calls == task_b.tool_calls {
                duplicates.push(DuplicateToolFailure {
                    task_a_id: task_a.id.clone(),
                    task_b_id: task_b.id.clone(),
                    tool_calls: task_a.tool_calls.clone(),
                });
            }
        }
    }
    
    if duplicates.is_empty() {
        ValidationResult::Passed
    } else {
        ValidationResult::Warning {
            reason: format!(
                "{} task pairs make identical tool calls",
                duplicates.len()
            ),
            details: ValidationDetails::DuplicateToolCalls(duplicates),
        }
    }
}
```

**Example Warning:**
```
Task A: "Run unit tests"
Tool calls: [cargo test]

Task B: "Run integration tests"
Tool calls: [cargo test]

Result: WARNING
Reason: 1 task pair makes identical tool calls
Note: This may be legitimate if both tasks need the same tool
```

---

## 🔧 Implementation

### AOPValidator Structure

```rust
pub struct AOPValidator {
    agent_registry: AgentRegistry,
    tool_registry: ToolRegistry,
    llm: LlmClient,
}

impl AOPValidator {
    pub fn new(
        agent_registry: AgentRegistry,
        tool_registry: ToolRegistry,
        llm: LlmClient,
    ) -> Self {
        Self {
            agent_registry,
            tool_registry,
            llm,
        }
    }
    
    /// Validate a task decomposition
    pub fn validate(&self, mission: &str, tasks: &[Task]) -> Result<AOPReport> {
        // Solvability checks
        let solvability = self.check_solvability(tasks)?;
        
        // Completeness checks
        let completeness = self.check_completeness(mission, tasks)?;
        
        // Non-redundancy checks
        let non_redundancy = self.check_non_redundancy(tasks)?;
        
        // Overall pass/fail
        let passed = solvability.passed 
            && completeness.passed 
            && non_redundancy.passed;
        
        Ok(AOPReport {
            solvability,
            completeness,
            non_redundancy,
            passed,
            timestamp: Utc::now(),
        })
    }
    
    /// Check solvability (capability + tools + complexity)
    fn check_solvability(&self, tasks: &[Task]) -> Result<SolvabilityReport> {
        let agents = self.agent_registry.get_all_agents()?;
        let tools = self.tool_registry.get_available_tools()?;
        
        let capability_result = check_capability_match(tasks, &agents);
        let tool_result = check_tool_availability(tasks, &tools);
        let complexity_result = check_complexity_threshold(tasks, LLM_OPTIMAL_LINEWIDTH);
        
        Ok(SolvabilityReport {
            capability_match: capability_result.passed(),
            tool_availability: tool_result.passed(),
            complexity_threshold: complexity_result.passed(),
            passed: capability_result.passed() 
                && tool_result.passed() 
                && complexity_result.passed(),
            failures: vec![capability_result, tool_result, complexity_result],
        })
    }
    
    /// Check completeness (requirement coverage + implicit requirements)
    fn check_completeness(&self, mission: &str, tasks: &[Task]) -> Result<CompletenessReport> {
        let mission_obj = self.parse_mission(mission)?;
        
        let coverage_result = check_requirement_coverage(&mission_obj, tasks);
        let implicit_result = check_implicit_requirements(&mission_obj, tasks);
        
        Ok(CompletenessReport {
            requirement_coverage: coverage_result.passed(),
            implicit_addressed: implicit_result.passed(),
            passed: coverage_result.passed() && implicit_result.passed(),
            failures: vec![coverage_result, implicit_result],
        })
    }
    
    /// Check non-redundancy (responsibility overlap + duplicate tools)
    fn check_non_redundancy(&self, tasks: &[Task]) -> Result<NonRedundancyReport> {
        let overlap_result = check_responsibility_overlap(tasks);
        let duplicate_result = check_duplicate_tool_calls(tasks);
        
        Ok(NonRedundancyReport {
            no_responsibility_overlap: overlap_result.passed(),
            no_duplicate_tools: duplicate_result.passed(),
            passed: overlap_result.passed() && duplicate_result.passed(),
            failures: vec![overlap_result, duplicate_result],
        })
    }
    
    fn parse_mission(&self, mission: &str) -> Result<Mission> {
        // Use LLM to parse mission into structured format
        self.llm.parse_mission(mission).await
    }
}
```

### Validation Report

```rust
pub struct AOPReport {
    pub solvability: SolvabilityReport,
    pub completeness: CompletenessReport,
    pub non_redundancy: NonRedundancyReport,
    pub passed: bool,
    pub timestamp: DateTime<Utc>,
}

impl AOPReport {
    /// Get all failures
    pub fn all_failures(&self) -> Vec<&ValidationResult> {
        let mut failures = Vec::new();
        
        failures.extend(&self.solvability.failures);
        failures.extend(&self.completeness.failures);
        failures.extend(&self.non_redundancy.failures);
        
        failures.iter()
            .filter(|r| !r.passed())
            .collect()
    }
    
    /// Get summary string
    pub fn summary(&self) -> String {
        if self.passed {
            "AOP validation PASSED".to_string()
        } else {
            let failures = self.all_failures();
            format!(
                "AOP validation FAILED: {} issues found",
                failures.len()
            )
        }
    }
}

pub struct SolvabilityReport {
    pub capability_match: bool,
    pub tool_availability: bool,
    pub complexity_threshold: bool,
    pub passed: bool,
    pub failures: Vec<ValidationResult>,
}

pub struct CompletenessReport {
    pub requirement_coverage: bool,
    pub implicit_addressed: bool,
    pub passed: bool,
    pub failures: Vec<ValidationResult>,
}

pub struct NonRedundancyReport {
    pub no_responsibility_overlap: bool,
    pub no_duplicate_tools: bool,
    pub passed: bool,
    pub failures: Vec<ValidationResult>,
}

pub enum ValidationResult {
    Passed,
    Failed {
        reason: String,
        details: ValidationDetails,
    },
    Warning {
        reason: String,
        details: ValidationDetails,
    },
}

impl ValidationResult {
    pub fn passed(&self) -> bool {
        matches!(self, ValidationResult::Passed)
    }
}

pub enum ValidationDetails {
    CapabilityMismatch(Vec<CapabilityFailure>),
    ToolUnavailable(Vec<ToolFailure>),
    ComplexityExceeded(Vec<ComplexityFailure>),
    MissingRequirements(Vec<Requirement>),
    ImplicitNotAddressed(Vec<Requirement>),
    ResponsibilityOverlap(Vec<OverlapFailure>),
    DuplicateToolCalls(Vec<DuplicateToolFailure>),
}
```

---

## 📊 Validation Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **AOP Pass Rate** | 100% | % of decompositions passing all checks |
| **False Positive Rate** | <5% | Invalid decompositions incorrectly accepted |
| **False Negative Rate** | <10% | Valid decompositions incorrectly rejected |
| **Validation Time** | <1s | Time to validate decomposition |
| **Actionable Feedback** | >90% | % of failures with clear remediation |

---

## 🔗 Related Documents

- [`ACONIC-DECOMPOSITION-DESIGN.md`](./ACONIC-DECOMPOSITION-DESIGN.md) — Main design spec
- [`GRAPH-WORKFLOW-DESIGN.md`](./GRAPH-WORKFLOW-DESIGN.md) — Graph-based workflow

---

**This specification provides IMPLEMENTATION-READY details for Agent C.**
