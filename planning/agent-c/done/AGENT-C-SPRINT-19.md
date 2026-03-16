# Agent C — Sprint 19: Bidirectional Traceability

**Phase:** 2  
**Sprint:** 19 (Implementation)  
**File:** `crates/axora-indexing/src/traceability.rs`  
**Priority:** MEDIUM (depends on Agent A Sprint 18)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Bidirectional Traceability** between code and business rules.

### Context

Research provides CRITICAL implementation details:
- **@req Annotations** — Code → Rule links (in docstrings)
- **YAML applies_to** — Rule → Code links (in frontmatter)
- **AST Parsing** — Extract @req tags (no LLM needed)
- **ESLint Plugin** — Enforce annotations (TypeScript)

**Your job:** Implement bidirectional traceability (depends on Agent A's business rule format).

---

## 📋 Deliverables

### 1. Create traceability.rs

**File:** `crates/axora-indexing/src/traceability.rs`

**Core Structure:**
```rust
//! Bidirectional Traceability
//!
//! This module implements bidirectional links between code and business rules:
//! - Code → Rules: @req annotations (parsed from docstrings)
//! - Rules → Code: applies_to (parsed from YAML frontmatter)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use regex::Regex;

/// Traceability link (code ↔ business rule)
#[derive(Debug, Clone)]
pub struct TraceabilityLink {
    pub code_file: PathBuf,
    pub code_symbol: Option<String>, // function, class, etc.
    pub rule_id: String,
    pub link_type: LinkType,
}

/// Link type (direction)
#[derive(Debug, Clone, Copy)]
pub enum LinkType {
    /// Code → Rule (@req annotation)
    CodeToRule,
    
    /// Rule → Code (applies_to in YAML)
    RuleToCode,
}

/// Traceability matrix (automated RTM)
pub struct TraceabilityMatrix {
    // Code file → Rules
    code_to_rules: HashMap<PathBuf, Vec<TraceabilityLink>>,
    
    // Rule → Code files
    rule_to_code: HashMap<String, Vec<TraceabilityLink>>,
    
    // Validation errors (orphaned links)
    errors: Vec<TraceabilityError>,
}

impl TraceabilityMatrix {
    /// Build traceability matrix from codebase
    pub fn build(codebase_path: &Path, rules_path: &Path) -> Result<Self> {
        let mut matrix = Self {
            code_to_rules: HashMap::new(),
            rule_to_code: HashMap::new(),
            errors: Vec::new(),
        };
        
        // Parse code files (extract @req annotations)
        matrix.parse_code_files(codebase_path)?;
        
        // Parse business rules (extract applies_to)
        matrix.parse_business_rules(rules_path)?;
        
        // Validate bidirectional links
        matrix.validate()?;
        
        Ok(matrix)
    }
    
    /// Get rules for code file
    pub fn get_rules_for_code(&self, code_file: &Path) -> Vec<&TraceabilityLink> {
        self.code_to_rules.get(code_file)
            .map(|links| links.iter().collect())
            .unwrap_or_default()
    }
    
    /// Get code files for rule
    pub fn get_code_for_rule(&self, rule_id: &str) -> Vec<&TraceabilityLink> {
        self.rule_to_code.get(rule_id)
            .map(|links| links.iter().collect())
            .unwrap_or_default()
    }
}
```

---

### 2. Implement @req Annotation Parser

**File:** `crates/axora-indexing/src/traceability.rs` (add to existing)

```rust
impl TraceabilityMatrix {
    /// Parse code files (extract @req annotations)
    fn parse_code_files(&mut self, codebase_path: &Path) -> Result<()> {
        // Regex for @req annotation
        let req_regex = Regex::new(r"@req\s+([A-Z]{3,4}-\d{3})")?;
        
        // Walk codebase
        for entry in walkdir::WalkDir::new(codebase_path) {
            let entry = entry?;
            let path = entry.path();
            
            // Skip non-code files
            if !is_code_file(path) {
                continue;
            }
            
            // Parse file
            let content = std::fs::read_to_string(path)?;
            
            // Extract @req annotations
            for cap in req_regex.captures_iter(&content) {
                let rule_id = cap[1].to_string();
                
                // Extract symbol (function, class, etc.)
                let symbol = extract_symbol_at_position(&content, cap.get(0).unwrap().start())?;
                
                // Create link
                let link = TraceabilityLink {
                    code_file: path.to_path_buf(),
                    code_symbol: symbol,
                    rule_id: rule_id.clone(),
                    link_type: LinkType::CodeToRule,
                };
                
                // Add to matrix
                self.code_to_rules.entry(path.to_path_buf()).or_default().push(link);
                self.rule_to_code.entry(rule_id).or_default().push(link);
            }
        }
        
        Ok(())
    }
}

/// Extract symbol (function, class, etc.) at position
fn extract_symbol_at_position(content: &str, position: usize) -> Result<Option<String>> {
    // Find nearest function/class definition before position
    // Use language-specific parser (tree-sitter)
    
    // For simplicity, use regex (production should use tree-sitter)
    let func_regex = Regex::new(r"(?:pub\s+)?fn\s+(\w+)")?;
    
    // Search backwards from position
    let before = &content[..position];
    
    if let Some(cap) = func_regex.captures(before).and_then(|c| c.get(1)) {
        return Ok(Some(cap.as_str().to_string()));
    }
    
    // Try class/struct
    let struct_regex = Regex::new(r"(?:pub\s+)?struct\s+(\w+)")?;
    if let Some(cap) = struct_regex.captures(before).and_then(|c| c.get(1)) {
        return Ok(Some(cap.as_str().to_string()));
    }
    
    Ok(None)
}
```

---

### 3. Implement YAML applies_to Parser

**File:** `crates/axora-indexing/src/traceability.rs` (add to existing)

```rust
impl TraceabilityMatrix {
    /// Parse business rules (extract applies_to from YAML frontmatter)
    fn parse_business_rules(&mut self, rules_path: &Path) -> Result<()> {
        // Walk business rules directory
        for entry in walkdir::WalkDir::new(rules_path) {
            let entry = entry?;
            let path = entry.path();
            
            // Skip non-Markdown files
            if !path.extension().map_or(false, |ext| ext == "md") {
                continue;
            }
            
            // Parse file
            let content = std::fs::read_to_string(path)?;
            
            // Extract YAML frontmatter
            let yaml = extract_yaml_frontmatter(&content)?;
            
            // Extract rule_id
            let rule_id = yaml.get("rule_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::MissingRuleId(path.to_path_buf()))?;
            
            // Extract applies_to
            let applies_to = yaml.get("applies_to")
                .and_then(|v| v.as_sequence())
                .ok_or_else(|| Error::MissingAppliesTo(path.to_path_buf()))?;
            
            // Create links (Rule → Code)
            for file_path in applies_to {
                if let Some(file_path_str) = file_path.as_str() {
                    let link = TraceabilityLink {
                        code_file: PathBuf::from(file_path_str),
                        code_symbol: None,
                        rule_id: rule_id.to_string(),
                        link_type: LinkType::RuleToCode,
                    };
                    
                    // Add to matrix
                    self.rule_to_code.entry(rule_id.to_string()).or_default().push(link);
                }
            }
        }
        
        Ok(())
    }
}

/// Extract YAML frontmatter from Markdown
fn extract_yaml_frontmatter(content: &str) -> Result<serde_yaml::Value> {
    // Check for frontmatter delimiters
    if !content.starts_with("---") {
        return Err(Error::MissingFrontmatter);
    }
    
    // Find end of frontmatter
    let end = content.find("\n---\n")
        .ok_or_else(|| Error::MissingFrontmatterEnd)?;
    
    // Parse YAML
    let yaml_str = &content[4..end]; // Skip first "---\n"
    let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str)?;
    
    Ok(yaml)
}
```

---

### 4. Implement Validation (Orphaned Links Detection)

**File:** `crates/axora-indexing/src/traceability.rs` (add to existing)

```rust
/// Traceability error (orphaned link)
#[derive(Debug, Clone)]
pub enum TraceabilityError {
    /// Code has @req but rule doesn't exist
    OrphanedCodeLink {
        code_file: PathBuf,
        rule_id: String,
    },
    
    /// Rule has applies_to but code file doesn't exist
    OrphanedRuleLink {
        rule_id: String,
        code_file: PathBuf,
    },
    
    /// Code has @req but rule doesn't have applies_to back
    MissingBacklink {
        code_file: PathBuf,
        rule_id: String,
    },
}

impl TraceabilityMatrix {
    /// Validate bidirectional links
    fn validate(&mut self) -> Result<()> {
        // Check for orphaned code links (@req without existing rule)
        for (code_file, links) in &self.code_to_rules {
            for link in links {
                if !self.rule_to_code.contains_key(&link.rule_id) {
                    self.errors.push(TraceabilityError::OrphanedCodeLink {
                        code_file: code_file.clone(),
                        rule_id: link.rule_id.clone(),
                    });
                }
            }
        }
        
        // Check for orphaned rule links (applies_to without existing code)
        for (rule_id, links) in &self.rule_to_code {
            for link in links {
                if !link.code_file.exists() {
                    self.errors.push(TraceabilityError::OrphanedRuleLink {
                        rule_id: rule_id.clone(),
                        code_file: link.code_file.clone(),
                    });
                }
            }
        }
        
        // Check for missing backlinks (@req without applies_to back)
        for (code_file, links) in &self.code_to_rules {
            for link in links {
                // Check if rule has applies_to back to this code file
                let has_backlink = self.rule_to_code.get(&link.rule_id)
                    .map_or(false, |rule_links| {
                        rule_links.iter().any(|rl| {
                            rl.link_type == LinkType::RuleToCode &&
                            rl.code_file == *code_file
                        })
                    });
                
                if !has_backlink {
                    self.errors.push(TraceabilityError::MissingBacklink {
                        code_file: code_file.clone(),
                        rule_id: link.rule_id.clone(),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Get validation errors
    pub fn get_errors(&self) -> &[TraceabilityError] {
        &self.errors
    }
    
    /// Check if matrix is valid (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}
```

---

### 5. Integrate with Influence Graph

**File:** `crates/axora-indexing/src/influence.rs` (UPDATE from Sprint 17)

```rust
impl InfluenceVector {
    /// Business rule count (from traceability links)
    pub fn business_rule_count: usize,
}

impl InfluenceGraph {
    /// Link business rules to influence vectors
    pub fn link_business_rules(&mut self, matrix: &TraceabilityMatrix) -> Result<()> {
        for (file_id, vector) in &mut self.vectors {
            // Get rules for this file
            let rules = matrix.get_rules_for_code(&file_id.to_path());
            vector.business_rule_count = rules.len();
        }
        
        Ok(())
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-indexing/src/traceability.rs` (NEW)

**Update:**
- `crates/axora-indexing/src/lib.rs` (add module export)
- `crates/axora-indexing/src/influence.rs` (link business rules)

**DO NOT Edit:**
- `crates/axora-cache/` (Agent B's domain)
- `crates/axora-docs/` (Agent A's domain)

**Dependencies:**
- Agent A Sprint 18 (Business Rule Documentation) — must complete first

---

## 🧪 Tests Required

```rust
#[test]
fn test_req_annotation_parsing() { }

#[test]
fn test_yaml_applies_to_parsing() { }

#[test]
fn test_bidirectional_link_validation() { }

#[test]
fn test_orphaned_code_link_detection() { }

#[test]
fn test_orphaned_rule_link_detection() { }

#[test]
fn test_missing_backlink_detection() { }

#[test]
fn test_traceability_matrix_build() { }

#[test]
fn test_influence_graph_business_rule_linking() { }
```

---

## ✅ Success Criteria

- [ ] `traceability.rs` created (bidirectional links)
- [ ] @req annotation parsing works
- [ ] YAML applies_to parsing works
- [ ] Validation detects orphaned links
- [ ] Validation detects missing backlinks
- [ ] Influence graph links business rules
- [ ] 8+ tests passing
- [ ] Works with all 10+ business rules from Agent A

---

## 🔗 References

- [`AGENT-A-SPRINT-18.md`](../agent-a/AGENT-A-SPRINT-18.md) — Business Rule Documentation (dependency)
- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](../shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Integration doc
- Research document — Bidirectional traceability spec

---

**⚠️ EXCEPTIONAL DEPENDENCY: This sprint is BLOCKED by Agent A Sprint 18.**

**Start AFTER Agent A completes Sprint 18 (Business Rule Documentation).**

**Priority: MEDIUM — needed for full traceability.**

**Dependencies:**
- Agent A Sprint 18 (Business Rule Documentation) — must complete first

**Blocks:**
- None (final step in traceability chain)
