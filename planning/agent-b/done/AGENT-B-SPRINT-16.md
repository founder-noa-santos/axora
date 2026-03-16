# Agent B — Sprint 16: SCIP Indexing Implementation

**Phase:** 2  
**Sprint:** 16 (Implementation)  
**File:** `crates/axora-indexing/src/scip.rs`  
**Priority:** HIGH (foundation for Influence Graph)  
**Estimated Tokens:** ~120K output  

---

## 🎯 Task

Implement **SCIP (Source Code Intelligence Protocol)** indexing for language-agnostic codebase indexing.

### Context

Research validates our Influence Graph approach and provides CRITICAL implementation details:
- **SCIP Protocol** — Protobuf format (not JSON), human-readable identifiers
- **Language-Specific Parsers** — rust-analyzer, ts-morph, pyan3, scip-typescript, scip-python
- **Package Ownership** — Cross-repository navigation (manager, name, version, symbol)

**Your job:** Implement SCIP indexing so we can build Influence Graph.

---

## 📋 Deliverables

### 1. Create scip.rs (SCIP Index)

**File:** `crates/axora-indexing/src/scip.rs`

**Core Structure:**
```rust
//! SCIP (Source Code Intelligence Protocol) Index
//!
//! This module implements SCIP protocol for language-agnostic code indexing:
//! - Protocol Buffers format (not JSON)
//! - Human-readable string identifiers (not opaque numeric IDs)
//! - Package ownership (manager, name, version, symbol)

use prost::Message; // Protocol Buffers
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// SCIP Index (language-agnostic code metadata)
#[derive(Debug, Clone, Message)]
pub struct SCIPIndex {
    /// Protocol version
    #[prost(string, tag = "1")]
    pub version: String,
    
    /// Package info (manager, name, version)
    #[prost(message, required, tag = "2")]
    pub package: PackageInfo,
    
    /// Symbols (functions, classes, interfaces, etc.)
    #[prost(message, repeated, tag = "3")]
    pub symbols: Vec<Symbol>,
    
    /// Occurrences (where symbols appear in code)
    #[prost(message, repeated, tag = "4")]
    pub occurrences: Vec<Occurrence>,
}

/// Package info (cross-repository navigation)
#[derive(Debug, Clone, Message)]
pub struct PackageInfo {
    /// Package manager (cargo, npm, pip, go)
    #[prost(string, tag = "1")]
    pub manager: String,
    
    /// Package name
    #[prost(string, tag = "2")]
    pub name: String,
    
    /// Package version
    #[prost(string, tag = "3")]
    pub version: String,
}

/// Symbol metadata (function, class, interface, etc.)
#[derive(Debug, Clone, Message)]
pub struct Symbol {
    /// Fully qualified symbol name (human-readable string)
    #[prost(string, tag = "1")]
    pub symbol: String,
    
    /// Symbol kind (function, class, interface, variable, etc.)
    #[prost(enumeration = "SymbolKind", tag = "2")]
    pub kind: i32,
    
    /// Symbol signature (for display)
    #[prost(string, tag = "3")]
    pub signature: String,
    
    /// Documentation (from docstrings)
    #[prost(string, tag = "4")]
    pub documentation: String,
}

/// Occurrence (where symbol appears in code)
#[derive(Debug, Clone, Message)]
pub struct Occurrence {
    /// File path (relative to package root)
    #[prost(string, tag = "1")]
    pub file_path: String,
    
    /// Line number (0-indexed)
    #[prost(int32, tag = "2")]
    pub line: i32,
    
    /// Column number (0-indexed)
    #[prost(int32, tag = "3")]
    pub column: i32,
    
    /// Symbol reference (fully qualified name)
    #[prost(string, tag = "4")]
    pub symbol: String,
    
    /// Is this a definition or reference?
    #[prost(bool, tag = "5")]
    pub is_definition: bool,
}

/// Symbol kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SymbolKind {
    Unspecified = 0,
    Class = 1,
    Interface = 2,
    Method = 3,
    Function = 4,
    Variable = 5,
    Macro = 6,
    Parameter = 7,
    Type = 8,
}
```

---

### 2. Implement Language-Specific Parsers

**File:** `crates/axora-indexing/src/scip.rs` (add to existing)

```rust
/// Language-specific parser trait
pub trait CodeParser: Send + Sync {
    /// Generate SCIP index from codebase
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex>;
    
    /// Get package info (from Cargo.toml, package.json, etc.)
    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo>;
}

/// Parser registry (manages language-specific parsers)
pub struct ParserRegistry {
    parsers: HashMap<Language, Box<dyn CodeParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };
        
        // Register language-specific parsers
        registry.register(Language::Rust, Box::new(RustParser::new()));
        registry.register(Language::TypeScript, Box::new(TypeScriptParser::new()));
        registry.register(Language::Python, Box::new(PythonParser::new()));
        registry.register(Language::Go, Box::new(GoParser::new()));
        
        registry
    }
    
    pub fn register(&mut self, language: Language, parser: Box<dyn CodeParser>) {
        self.parsers.insert(language, parser);
    }
    
    pub fn parse(&self, language: Language, codebase: &Path) -> Result<SCIPIndex> {
        let parser = self.parsers.get(&language)
            .ok_or_else(|| Error::UnsupportedLanguage(language))?;
        
        parser.generate_scip(codebase)
    }
}
```

---

### 3. Implement Rust Parser (rust-analyzer)

**File:** `crates/axora-indexing/src/scip.rs` (add to existing)

```rust
/// Rust parser (uses rust-analyzer SCIP)
pub struct RustParser;

impl RustParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for RustParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Run rust-analyzer scip command
        // rust-analyzer scip > index.scip
        let output = std::process::Command::new("rust-analyzer")
            .arg("scip")
            .current_dir(codebase)
            .output()?;
        
        if !output.status.success() {
            return Err(Error::ScipGenerationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Parse Protobuf output
        let scip_bytes = output.stdout;
        let scip_index = SCIPIndex::decode(&scip_bytes[..])?;
        
        Ok(scip_index)
    }
    
    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse Cargo.toml
        let cargo_toml = codebase.join("Cargo.toml");
        let content = std::fs::read_to_string(cargo_toml)?;
        
        // Extract package info (simple parsing, or use toml crate)
        let package_info = PackageInfo {
            manager: "cargo".to_string(),
            name: extract_package_name(&content)?,
            version: extract_package_version(&content)?,
        };
        
        Ok(package_info)
    }
}
```

---

### 4. Implement TypeScript Parser (ts-morph + scip-typescript)

**File:** `crates/axora-indexing/src/scip.rs` (add to existing)

```rust
/// TypeScript parser (uses ts-morph + scip-typescript)
pub struct TypeScriptParser;

impl TypeScriptParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for TypeScriptParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Option 1: Use scip-typescript CLI
        // npx scip-typescript > index.scip
        let output = std::process::Command::new("npx")
            .arg("scip-typescript")
            .current_dir(codebase)
            .output()?;
        
        if !output.status.success() {
            return Err(Error::ScipGenerationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Parse Protobuf output
        let scip_bytes = output.stdout;
        let scip_index = SCIPIndex::decode(&scip_bytes[..])?;
        
        Ok(scip_index)
    }
    
    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse package.json
        let package_json = codebase.join("package.json");
        let content = std::fs::read_to_string(package_json)?;
        
        let package: serde_json::Value = serde_json::from_str(&content)?;
        
        let package_info = PackageInfo {
            manager: "npm".to_string(),
            name: package["name"].as_str().unwrap_or("unknown").to_string(),
            version: package["version"].as_str().unwrap_or("0.0.0").to_string(),
        };
        
        Ok(package_info)
    }
}
```

---

### 5. Implement Python Parser (pyan3 + scip-python)

**File:** `crates/axora-indexing/src/scip.rs` (add to existing)

```rust
/// Python parser (uses pyan3 + scip-python)
pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for PythonParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Option 1: Use scip-python CLI
        // scip-python > index.scip
        let output = std::process::Command::new("scip-python")
            .current_dir(codebase)
            .output()?;
        
        if !output.status.success() {
            return Err(Error::ScipGenerationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Parse Protobuf output
        let scip_bytes = output.stdout;
        let scip_index = SCIPIndex::decode(&scip_bytes[..])?;
        
        Ok(scip_index)
    }
    
    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse setup.py or pyproject.toml
        // For simplicity, use pyproject.toml
        let pyproject_toml = codebase.join("pyproject.toml");
        
        if pyproject_toml.exists() {
            let content = std::fs::read_to_string(pyproject_toml)?;
            // Parse TOML (use toml crate)
            let package_info = PackageInfo {
                manager: "pip".to_string(),
                name: extract_project_name(&content)?,
                version: extract_project_version(&content)?,
            };
            Ok(package_info)
        } else {
            // Fallback to default
            Ok(PackageInfo {
                manager: "pip".to_string(),
                name: "unknown".to_string(),
                version: "0.0.0".to_string(),
            })
        }
    }
}
```

---

### 6. Implement Go Parser (go/ast + godepgraph)

**File:** `crates/axora-indexing/src/scip.rs` (add to existing)

```rust
/// Go parser (uses go/ast + godepgraph)
pub struct GoParser;

impl GoParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for GoParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Option 1: Use custom Go tool to generate SCIP
        // go run scip-go/main.go > index.scip
        let output = std::process::Command::new("go")
            .arg("run")
            .arg("scip-go/main.go")
            .current_dir(codebase)
            .output()?;
        
        if !output.status.success() {
            return Err(Error::ScipGenerationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        
        // Parse Protobuf output
        let scip_bytes = output.stdout;
        let scip_index = SCIPIndex::decode(&scip_bytes[..])?;
        
        Ok(scip_index)
    }
    
    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse go.mod
        let go_mod = codebase.join("go.mod");
        let content = std::fs::read_to_string(go_mod)?;
        
        // Extract module name (first line: module example.com/pkg)
        let module_name = extract_module_name(&content)?;
        
        let package_info = PackageInfo {
            manager: "go".to_string(),
            name: module_name,
            version: "0.0.0".to_string(), // Go modules don't have versions in go.mod
        };
        
        Ok(package_info)
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-indexing/src/scip.rs` (NEW)

**Update:**
- `crates/axora-indexing/src/lib.rs` (add module export)
- `crates/axora-indexing/Cargo.toml` (add prost, toml, serde_json deps)

**DO NOT Edit:**
- `crates/axora-agents/` (Agent C's domain)
- `crates/axora-cache/` (Agent B's other work)
- `crates/axora-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_rust_scip_generation() { }

#[test]
fn test_typescript_scip_generation() { }

#[test]
fn test_python_scip_generation() { }

#[test]
fn test_go_scip_generation() { }

#[test]
fn test_package_info_parsing() { }

#[test]
fn test_symbol_extraction() { }

#[test]
fn test_occurrence_extraction() { }

#[test]
fn test_cross_repository_navigation() { }
```

---

## ✅ Success Criteria

- [ ] `scip.rs` created (SCIP Index implementation)
- [ ] Parser registry implemented (4 languages: Rust, TypeScript, Python, Go)
- [ ] SCIP generation works for all 4 languages
- [ ] Package info parsing works (Cargo.toml, package.json, pyproject.toml, go.mod)
- [ ] Protobuf encoding/decoding works
- [ ] 8+ tests passing
- [ ] Cross-repository navigation works (package ownership)

---

## 🔗 References

- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](../shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Integration doc
- [`research/prompts/13-influence-graph-business-rules.md`](../research/prompts/13-influence-graph-business-rules.md) — R-13 research
- SCIP Protocol Spec: https://github.com/sourcegraph/scip

---

**Start AFTER Sprint 11 (Context + RAG Pivot) is complete.**

**Priority: HIGH — this is foundation for Influence Graph.**

**Dependencies:**
- None (can start independently)

**Blocks:**
- Sprint 17 (Influence Vector Calculation)
- Sprint 20 (Context Pruning)
