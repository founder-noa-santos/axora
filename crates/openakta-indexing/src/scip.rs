//! SCIP (Source Code Intelligence Protocol) Index
//!
//! This module implements SCIP protocol for language-agnostic code indexing:
//! - Protocol Buffers format (not JSON)
//! - Human-readable string identifiers (not opaque numeric IDs)
//! - Package ownership (manager, name, version, symbol)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     SCIP Index                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Package Info              │  Symbols                      │
//! │  - manager: cargo/npm/pip  │  - Function definitions       │
//! │  - name: crate/package     │  - Class definitions          │
//! │  - version: semver         │  - Interface definitions      │
//! │                            │  - Variables, macros, etc.    │
//! │                            │                               │
//! │  Occurrences               │  Language Parsers             │
//! │  - File path               │  - Rust (rust-analyzer)       │
//! │  - Line/column             │  - TypeScript (scip-ts)       │
//! │  - Symbol reference        │  - Python (scip-python)       │
//! │  - Is definition?          │  - Go (scip-go)               │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use openakta_indexing::scip::{ParserRegistry, Language};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = ParserRegistry::new();
//!
//! // Generate SCIP index for Rust codebase
//! let scip_index = registry.parse(
//!     Language::Rust,
//!     Path::new("/path/to/rust/project"),
//! )?;
//!
//! println!("Package: {}.{}", scip_index.package.manager, scip_index.package.name);
//! println!("Symbols: {}", scip_index.symbols.len());
//! println!("Occurrences: {}", scip_index.occurrences.len());
//! # Ok(())
//! # }
//! ```

use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

/// SCIP Index error types
#[derive(Error, Debug)]
pub enum SCIPError {
    /// SCIP generation failed
    #[error("SCIP generation failed: {0}")]
    ScipGenerationFailed(String),

    /// Unsupported language
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    /// Protobuf encoding error
    #[error("protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    /// Protobuf decoding error
    #[error("protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML parsing error
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Package info extraction failed
    #[error("failed to extract package info: {0}")]
    PackageInfo(String),

    /// Symbol not found
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
}

/// Result type for SCIP operations
pub type Result<T> = std::result::Result<T, SCIPError>;

/// SCIP Index (language-agnostic code metadata)
///
/// This is the main data structure representing a codebase's metadata
/// in SCIP format. It contains package info, symbols, and occurrences.
#[derive(Clone, Message, Serialize, Deserialize)]
pub struct SCIPIndex {
    /// Protocol version (e.g., "0.3.0")
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

    /// External symbols (from dependencies)
    #[prost(message, repeated, tag = "5")]
    pub external_symbols: Vec<ExternalSymbol>,
}

impl SCIPIndex {
    /// Creates a new empty SCIP index
    pub fn new(package: PackageInfo) -> Self {
        Self {
            version: "0.3.0".to_string(),
            package,
            symbols: Vec::new(),
            occurrences: Vec::new(),
            external_symbols: Vec::new(),
        }
    }

    /// Merges another SCIP index into this one
    pub fn merge(&mut self, other: SCIPIndex) {
        self.symbols.extend(other.symbols);
        self.occurrences.extend(other.occurrences);
        self.external_symbols.extend(other.external_symbols);
    }

    /// Gets all symbols of a specific kind
    pub fn symbols_by_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.kind() == kind).collect()
    }

    /// Gets all occurrences of a specific symbol
    pub fn occurrences_of(&self, symbol_name: &str) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| o.symbol == symbol_name)
            .collect()
    }

    /// Gets definitions of a specific symbol
    pub fn definitions_of(&self, symbol_name: &str) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| o.symbol == symbol_name && o.is_definition)
            .collect()
    }

    /// Gets references to a specific symbol
    pub fn references_of(&self, symbol_name: &str) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| o.symbol == symbol_name && !o.is_definition)
            .collect()
    }

    /// Serializes the index to protobuf bytes
    pub fn encode_to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        Message::encode(self, &mut bytes).map_err(SCIPError::ProtobufEncode)?;
        Ok(bytes)
    }

    /// Deserializes the index from protobuf bytes
    pub fn decode_from_bytes(bytes: &[u8]) -> Result<Self> {
        Message::decode(bytes).map_err(SCIPError::ProtobufDecode)
    }
}

/// Package info (cross-repository navigation)
///
/// Contains the package manager, name, and version for navigating
/// across repositories and understanding symbol ownership.
#[derive(Clone, Message, Serialize, Deserialize)]
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

impl PackageInfo {
    /// Creates a new package info
    pub fn new(manager: &str, name: &str, version: &str) -> Self {
        Self {
            manager: manager.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    /// Creates a fully qualified package identifier
    pub fn qualified_name(&self) -> String {
        format!("{}:{}/{}", self.manager, self.name, self.version)
    }
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}/{}", self.manager, self.name, self.version)
    }
}

/// Symbol metadata (function, class, interface, etc.)
///
/// Represents a code symbol with its kind, signature, and documentation.
#[derive(Clone, Message, Serialize, Deserialize)]
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

    /// Relationships to other symbols (parent, children, etc.)
    #[prost(message, repeated, tag = "5")]
    pub relationships: Vec<SymbolRelationship>,
}

impl Symbol {
    /// Creates a new symbol
    pub fn new(symbol: &str, kind: SymbolKind, signature: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            kind: kind as i32,
            signature: signature.to_string(),
            documentation: String::new(),
            relationships: Vec::new(),
        }
    }

    /// Gets the symbol kind as enum
    pub fn kind_enum(&self) -> SymbolKind {
        SymbolKind::from(self.kind)
    }

    /// Adds documentation
    pub fn with_documentation(mut self, docs: &str) -> Self {
        self.documentation = docs.to_string();
        self
    }

    /// Adds a relationship
    pub fn with_relationship(mut self, rel: SymbolRelationship) -> Self {
        self.relationships.push(rel);
        self
    }
}

/// Symbol relationship (parent, children, etc.)
#[derive(Clone, Message, Serialize, Deserialize)]
pub struct SymbolRelationship {
    /// Related symbol
    #[prost(string, tag = "1")]
    pub symbol: String,

    /// Relationship type
    #[prost(enumeration = "RelationshipKind", tag = "2")]
    pub kind: i32,
}

impl SymbolRelationship {
    /// Creates a new relationship
    pub fn new(symbol: &str, kind: RelationshipKind) -> Self {
        Self {
            symbol: symbol.to_string(),
            kind: kind as i32,
        }
    }
}

/// Relationship kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum RelationshipKind {
    #[default]
    Unspecified = 0,
    Parent = 1,
    Child = 2,
    Implements = 3,
    Extends = 4,
    Uses = 5,
    Imports = 6,
    Exports = 7,
}

impl From<i32> for RelationshipKind {
    fn from(value: i32) -> Self {
        match value {
            1 => RelationshipKind::Parent,
            2 => RelationshipKind::Child,
            3 => RelationshipKind::Implements,
            4 => RelationshipKind::Extends,
            5 => RelationshipKind::Uses,
            6 => RelationshipKind::Imports,
            7 => RelationshipKind::Exports,
            _ => RelationshipKind::Unspecified,
        }
    }
}

/// Occurrence (where symbol appears in code)
///
/// Represents a specific location in the codebase where a symbol
/// is defined or referenced.
#[derive(Clone, Message, Serialize, Deserialize)]
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

    /// End line (0-indexed, inclusive)
    #[prost(int32, tag = "4")]
    pub end_line: i32,

    /// End column (0-indexed, exclusive)
    #[prost(int32, tag = "5")]
    pub end_column: i32,

    /// Symbol reference (fully qualified name)
    #[prost(string, tag = "6")]
    pub symbol: String,

    /// Is this a definition or reference?
    #[prost(bool, tag = "7")]
    pub is_definition: bool,

    /// Snippet of surrounding code (for context)
    #[prost(string, tag = "8")]
    pub snippet: String,
}

impl Occurrence {
    /// Creates a new occurrence
    pub fn new(file_path: &str, line: i32, column: i32, symbol: &str, is_definition: bool) -> Self {
        Self {
            file_path: file_path.to_string(),
            line,
            column,
            end_line: line,
            end_column: column,
            symbol: symbol.to_string(),
            is_definition,
            snippet: String::new(),
        }
    }

    /// Sets the end position
    pub fn with_end_position(mut self, end_line: i32, end_column: i32) -> Self {
        self.end_line = end_line;
        self.end_column = end_column;
        self
    }

    /// Sets the snippet
    pub fn with_snippet(mut self, snippet: &str) -> Self {
        self.snippet = snippet.to_string();
        self
    }

    /// Gets the span (start to end)
    pub fn span(&self) -> (i32, i32, i32, i32) {
        (self.line, self.column, self.end_line, self.end_column)
    }
}

/// External symbol (from dependencies)
#[derive(Clone, Message, Serialize, Deserialize)]
pub struct ExternalSymbol {
    /// Fully qualified symbol name
    #[prost(string, tag = "1")]
    pub symbol: String,

    /// Package where symbol is defined
    #[prost(message, required, tag = "2")]
    pub package: PackageInfo,

    /// Symbol kind
    #[prost(enumeration = "SymbolKind", tag = "3")]
    pub kind: i32,
}

impl ExternalSymbol {
    /// Creates a new external symbol
    pub fn new(symbol: &str, package: PackageInfo, kind: SymbolKind) -> Self {
        Self {
            symbol: symbol.to_string(),
            package,
            kind: kind as i32,
        }
    }

    /// Gets the symbol kind as enum
    pub fn kind_enum(&self) -> SymbolKind {
        SymbolKind::from(self.kind)
    }
}

/// Symbol kind enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(i32)]
pub enum SymbolKind {
    #[default]
    Unspecified = 0,
    Class = 1,
    Interface = 2,
    Method = 3,
    Function = 4,
    Variable = 5,
    Macro = 6,
    Parameter = 7,
    Type = 8,
    Module = 9,
    Namespace = 10,
    Constant = 11,
    Field = 12,
    Enum = 13,
    EnumMember = 14,
    Struct = 15,
    Trait = 16,
    Impl = 17,
}

impl From<i32> for SymbolKind {
    fn from(value: i32) -> Self {
        match value {
            1 => SymbolKind::Class,
            2 => SymbolKind::Interface,
            3 => SymbolKind::Method,
            4 => SymbolKind::Function,
            5 => SymbolKind::Variable,
            6 => SymbolKind::Macro,
            7 => SymbolKind::Parameter,
            8 => SymbolKind::Type,
            9 => SymbolKind::Module,
            10 => SymbolKind::Namespace,
            11 => SymbolKind::Constant,
            12 => SymbolKind::Field,
            13 => SymbolKind::Enum,
            14 => SymbolKind::EnumMember,
            15 => SymbolKind::Struct,
            16 => SymbolKind::Trait,
            17 => SymbolKind::Impl,
            _ => SymbolKind::Unspecified,
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Macro => write!(f, "macro"),
            SymbolKind::Parameter => write!(f, "parameter"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Module => write!(f, "module"),
            SymbolKind::Namespace => write!(f, "namespace"),
            SymbolKind::Constant => write!(f, "constant"),
            SymbolKind::Field => write!(f, "field"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::EnumMember => write!(f, "enum member"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Impl => write!(f, "impl"),
            SymbolKind::Unspecified => write!(f, "unspecified"),
        }
    }
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Python => write!(f, "python"),
            Language::Go => write!(f, "go"),
        }
    }
}

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
    /// Creates a new parser registry with all supported languages
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

    /// Registers a parser for a language
    pub fn register(&mut self, language: Language, parser: Box<dyn CodeParser>) {
        self.parsers.insert(language, parser);
    }

    /// Parses a codebase and generates SCIP index
    pub fn parse(&self, language: Language, codebase: &Path) -> Result<SCIPIndex> {
        let parser = self
            .parsers
            .get(&language)
            .ok_or_else(|| SCIPError::UnsupportedLanguage(language.to_string()))?;

        parser.generate_scip(codebase)
    }

    /// Gets package info for a codebase
    pub fn get_package_info(&self, language: Language, codebase: &Path) -> Result<PackageInfo> {
        let parser = self
            .parsers
            .get(&language)
            .ok_or_else(|| SCIPError::UnsupportedLanguage(language.to_string()))?;

        parser.get_package_info(codebase)
    }

    /// Checks if a language is supported
    pub fn supports(&self, language: Language) -> bool {
        self.parsers.contains_key(&language)
    }

    /// Gets all supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        self.parsers.keys().copied().collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rust Parser (rust-analyzer SCIP)
// ============================================================================

/// Rust parser (uses rust-analyzer SCIP)
pub struct RustParser;

impl RustParser {
    /// Creates a new Rust parser
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for RustParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Try to run rust-analyzer scip command
        // rust-analyzer scip > index.scip
        let output = std::process::Command::new("rust-analyzer")
            .arg("scip")
            .current_dir(codebase)
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return Err(SCIPError::ScipGenerationFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                // Parse Protobuf output
                let scip_bytes = output.stdout;
                SCIPIndex::decode_from_bytes(&scip_bytes[..])
            }
            Err(_e) => {
                let package = self.get_package_info(codebase)?;
                generate_fallback_scip(Language::Rust, codebase, package)
            }
        }
    }

    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse Cargo.toml
        let cargo_toml = codebase.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)
            .map_err(|_| SCIPError::PackageInfo(format!("Could not read {:?}", cargo_toml)))?;

        // Parse TOML
        let toml_value: toml::Value = toml::from_str(&content)?;

        // Extract package info
        let name = toml_value
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let version = toml_value
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        Ok(PackageInfo::new("cargo", name, version))
    }
}

// ============================================================================
// TypeScript Parser (ts-morph + scip-typescript)
// ============================================================================

/// TypeScript parser (uses ts-morph + scip-typescript)
pub struct TypeScriptParser;

impl TypeScriptParser {
    /// Creates a new TypeScript parser
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for TypeScriptParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Try to run scip-typescript CLI
        // npx scip-typescript > index.scip
        let output = std::process::Command::new("npx")
            .arg("scip-typescript")
            .current_dir(codebase)
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return Err(SCIPError::ScipGenerationFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                // Parse Protobuf output
                let scip_bytes = output.stdout;
                SCIPIndex::decode_from_bytes(&scip_bytes[..])
            }
            Err(_e) => {
                let package = self.get_package_info(codebase)?;
                generate_fallback_scip(Language::TypeScript, codebase, package)
            }
        }
    }

    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse package.json
        let package_json = codebase.join("package.json");
        let content = std::fs::read_to_string(&package_json)
            .map_err(|_| SCIPError::PackageInfo(format!("Could not read {:?}", package_json)))?;

        let package: serde_json::Value = serde_json::from_str(&content)?;

        let name = package
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        Ok(PackageInfo::new("npm", name, version))
    }
}

// ============================================================================
// Python Parser (pyan3 + scip-python)
// ============================================================================

/// Python parser (uses pyan3 + scip-python)
pub struct PythonParser;

impl PythonParser {
    /// Creates a new Python parser
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for PythonParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Try to run scip-python CLI
        let output = std::process::Command::new("scip-python")
            .current_dir(codebase)
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return Err(SCIPError::ScipGenerationFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                // Parse Protobuf output
                let scip_bytes = output.stdout;
                SCIPIndex::decode_from_bytes(&scip_bytes[..])
            }
            Err(_e) => {
                let package = self.get_package_info(codebase)?;
                generate_fallback_scip(Language::Python, codebase, package)
            }
        }
    }

    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Try pyproject.toml first
        let pyproject_toml = codebase.join("pyproject.toml");

        if pyproject_toml.exists() {
            let content = std::fs::read_to_string(&pyproject_toml)?;
            let toml_value: toml::Value = toml::from_str(&content)?;

            let name = toml_value
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");

            let version = toml_value
                .get("project")
                .and_then(|p| p.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0");

            return Ok(PackageInfo::new("pip", name, version));
        }

        // Fallback to setup.py parsing (simplified)
        let setup_py = codebase.join("setup.py");
        if setup_py.exists() {
            // Simple regex-based extraction (in production, use ast parsing)
            let content = std::fs::read_to_string(&setup_py)?;

            let name =
                extract_setup_value(&content, "name").unwrap_or_else(|| "unknown".to_string());
            let version =
                extract_setup_value(&content, "version").unwrap_or_else(|| "0.0.0".to_string());

            return Ok(PackageInfo::new("pip", &name, &version));
        }

        // Default fallback
        Ok(PackageInfo::new("pip", "unknown", "0.0.0"))
    }
}

/// Extracts a value from setup.py (simplified)
fn extract_setup_value(content: &str, key: &str) -> Option<String> {
    // Simple pattern matching (in production, use proper AST parsing)
    let pattern = format!("{}\\s*=\\s*[\"']([^\"']+)[\"']", key);
    if let Ok(re) = regex::Regex::new(&pattern) {
        re.captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    } else {
        None
    }
}

fn generate_fallback_scip(
    language: Language,
    codebase: &Path,
    package: PackageInfo,
) -> Result<SCIPIndex> {
    let mut index = SCIPIndex::new(package);
    let extensions = fallback_extensions(language);

    for entry in WalkDir::new(codebase)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if !path.is_file() || !has_supported_extension(path, extensions) {
            continue;
        }

        let relative_path = path
            .strip_prefix(codebase)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(path)?;
        let module_name = module_name_from_path(&relative_path);

        for (line_index, line) in content.lines().enumerate() {
            if let Some((name, kind)) = detect_definition(language, line) {
                let symbol_name = format!("{module_name}::{name}");
                index
                    .symbols
                    .push(Symbol::new(&symbol_name, kind, line.trim()).with_documentation(""));
                index.occurrences.push(
                    Occurrence::new(&relative_path, line_index as i32, 0, &symbol_name, true)
                        .with_end_position(line_index as i32, line.len() as i32)
                        .with_snippet(line.trim()),
                );
            }

            for referenced_symbol in detect_references(language, line) {
                index.occurrences.push(
                    Occurrence::new(
                        &relative_path,
                        line_index as i32,
                        0,
                        &referenced_symbol,
                        false,
                    )
                    .with_end_position(line_index as i32, line.len() as i32)
                    .with_snippet(line.trim()),
                );
            }
        }
    }

    deduplicate_symbols(&mut index.symbols);
    Ok(index)
}

fn fallback_extensions(language: Language) -> &'static [&'static str] {
    match language {
        Language::Rust => &["rs"],
        Language::TypeScript => &["ts", "tsx", "js", "jsx"],
        Language::Python => &["py"],
        Language::Go => &["go"],
    }
}

fn has_supported_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.iter().any(|candidate| candidate == &ext))
        .unwrap_or(false)
}

fn module_name_from_path(relative_path: &str) -> String {
    relative_path
        .trim_end_matches(".rs")
        .trim_end_matches(".ts")
        .trim_end_matches(".tsx")
        .trim_end_matches(".js")
        .trim_end_matches(".jsx")
        .trim_end_matches(".py")
        .trim_start_matches("src/")
        .trim_start_matches("./")
        .replace('/', "::")
}

fn detect_definition(language: Language, line: &str) -> Option<(String, SymbolKind)> {
    let trimmed = line.trim();
    let (prefix, kind) = match language {
        Language::Rust if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") => {
            ("fn ", SymbolKind::Function)
        }
        Language::Rust if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") => {
            ("struct ", SymbolKind::Struct)
        }
        Language::Rust if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") => {
            ("enum ", SymbolKind::Enum)
        }
        Language::Rust if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") => {
            ("trait ", SymbolKind::Trait)
        }
        Language::TypeScript
            if trimmed.starts_with("function ") || trimmed.starts_with("export function ") =>
        {
            ("function ", SymbolKind::Function)
        }
        Language::TypeScript
            if trimmed.starts_with("class ") || trimmed.starts_with("export class ") =>
        {
            ("class ", SymbolKind::Class)
        }
        Language::TypeScript
            if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") =>
        {
            ("interface ", SymbolKind::Interface)
        }
        Language::TypeScript
            if trimmed.starts_with("type ") || trimmed.starts_with("export type ") =>
        {
            ("type ", SymbolKind::Type)
        }
        Language::Python if trimmed.starts_with("def ") => ("def ", SymbolKind::Function),
        Language::Python if trimmed.starts_with("class ") => ("class ", SymbolKind::Class),
        _ => return None,
    };

    let name = trimmed
        .split(prefix)
        .nth(1)?
        .split(|ch: char| ch == '(' || ch == ':' || ch == '<' || ch.is_whitespace())
        .next()?;
    Some((name.trim().to_string(), kind))
}

fn detect_references(language: Language, line: &str) -> Vec<String> {
    let trimmed = line.trim();
    match language {
        Language::Rust => detect_rust_references(trimmed),
        Language::TypeScript => detect_typescript_references(trimmed),
        Language::Python => detect_python_references(trimmed),
        Language::Go => Vec::new(),
    }
}

fn detect_rust_references(line: &str) -> Vec<String> {
    if let Some(rest) = line.strip_prefix("use ") {
        return rest
            .trim_end_matches(';')
            .split(',')
            .filter_map(|segment| {
                let normalized = segment
                    .trim()
                    .trim_start_matches("crate::")
                    .trim_start_matches("self::");
                let parts = normalized.split("::").collect::<Vec<_>>();
                if parts.len() >= 2 {
                    Some(parts.join("::"))
                } else {
                    None
                }
            })
            .collect();
    }
    Vec::new()
}

fn detect_typescript_references(line: &str) -> Vec<String> {
    if !line.starts_with("import ") {
        return Vec::new();
    }
    let module = line
        .split(" from ")
        .nth(1)
        .map(|part| {
            part.trim()
                .trim_matches(';')
                .trim_matches('\'')
                .trim_matches('"')
        })
        .unwrap_or_default()
        .trim_start_matches("./")
        .replace('/', "::");
    let imported = line
        .split('{')
        .nth(1)
        .and_then(|part| part.split('}').next())
        .unwrap_or("");
    imported
        .split(',')
        .filter_map(|name| {
            let name = name.trim();
            if name.is_empty() || module.is_empty() {
                None
            } else {
                Some(format!("{module}::{name}"))
            }
        })
        .collect()
}

fn detect_python_references(line: &str) -> Vec<String> {
    if let Some(rest) = line.strip_prefix("from ") {
        let mut segments = rest.split(" import ");
        let module = segments.next().unwrap_or("").trim().replace('.', "::");
        let names = segments.next().unwrap_or("");
        return names
            .split(',')
            .filter_map(|name| {
                let name = name.trim();
                if module.is_empty() || name.is_empty() {
                    None
                } else {
                    Some(format!("{module}::{name}"))
                }
            })
            .collect();
    }
    Vec::new()
}

fn deduplicate_symbols(symbols: &mut Vec<Symbol>) {
    let mut seen = HashMap::new();
    symbols.retain(|symbol| seen.insert(symbol.symbol.clone(), ()).is_none());
}

// ============================================================================
// Go Parser (go/ast + scip-go)
// ============================================================================

/// Go parser (uses go/ast + scip-go)
pub struct GoParser;

impl GoParser {
    /// Creates a new Go parser
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeParser for GoParser {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex> {
        // Try to run scip-go
        let output = std::process::Command::new("scip-go")
            .current_dir(codebase)
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return Err(SCIPError::ScipGenerationFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }

                // Parse Protobuf output
                let scip_bytes = output.stdout;
                SCIPIndex::decode_from_bytes(&scip_bytes[..])
            }
            Err(_e) => {
                // scip-go not available, generate minimal SCIP index
                let package = self.get_package_info(codebase)?;
                Ok(SCIPIndex::new(package))
            }
        }
    }

    fn get_package_info(&self, codebase: &Path) -> Result<PackageInfo> {
        // Parse go.mod
        let go_mod = codebase.join("go.mod");
        let content = std::fs::read_to_string(&go_mod)
            .map_err(|_| SCIPError::PackageInfo(format!("Could not read {:?}", go_mod)))?;

        // Extract module name (first line: module example.com/pkg)
        let module_name = content
            .lines()
            .find(|line| line.starts_with("module "))
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("unknown");

        Ok(PackageInfo::new("go", module_name, "0.0.0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_package_info_qualified_name() {
        let package = PackageInfo::new("cargo", "my-crate", "1.0.0");
        assert_eq!(package.qualified_name(), "cargo:my-crate/1.0.0");
        assert_eq!(package.to_string(), "cargo:my-crate/1.0.0");
    }

    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new(
            "my_function",
            SymbolKind::Function,
            "fn my_function() -> i32",
        );
        assert_eq!(symbol.symbol, "my_function");
        assert_eq!(symbol.kind_enum(), SymbolKind::Function);
        assert_eq!(symbol.signature, "fn my_function() -> i32");
    }

    #[test]
    fn test_occurrence_creation() {
        let occurrence = Occurrence::new("src/main.rs", 10, 5, "my_function", true);
        assert_eq!(occurrence.file_path, "src/main.rs");
        assert_eq!(occurrence.line, 10);
        assert_eq!(occurrence.column, 5);
        assert!(occurrence.is_definition);
    }

    #[test]
    fn test_scip_index_creation() {
        let package = PackageInfo::new("cargo", "test-crate", "0.1.0");
        let index = SCIPIndex::new(package.clone());

        assert_eq!(index.version, "0.3.0");
        assert_eq!(index.package.manager, "cargo");
        assert_eq!(index.package.name, "test-crate");
        assert!(index.symbols.is_empty());
        assert!(index.occurrences.is_empty());
    }

    #[test]
    fn test_scip_index_merge() {
        let package = PackageInfo::new("cargo", "test-crate", "0.1.0");
        let mut index1 = SCIPIndex::new(package.clone());
        let mut index2 = SCIPIndex::new(package.clone());

        index1
            .symbols
            .push(Symbol::new("func1", SymbolKind::Function, "fn func1()"));
        index2
            .symbols
            .push(Symbol::new("func2", SymbolKind::Function, "fn func2()"));

        index1.merge(index2);

        assert_eq!(index1.symbols.len(), 2);
        assert_eq!(index1.symbols[0].symbol, "func1");
        assert_eq!(index1.symbols[1].symbol, "func2");
    }

    #[test]
    fn test_scip_index_filtering() {
        let package = PackageInfo::new("cargo", "test-crate", "0.1.0");
        let mut index = SCIPIndex::new(package);

        // Add symbols
        index
            .symbols
            .push(Symbol::new("MyClass", SymbolKind::Class, "struct MyClass"));
        index
            .symbols
            .push(Symbol::new("my_func", SymbolKind::Function, "fn my_func()"));
        index
            .symbols
            .push(Symbol::new("my_var", SymbolKind::Variable, "let my_var"));

        // Add occurrences
        index
            .occurrences
            .push(Occurrence::new("src/lib.rs", 0, 0, "my_func", true));
        index
            .occurrences
            .push(Occurrence::new("src/main.rs", 5, 10, "my_func", false));
        index
            .occurrences
            .push(Occurrence::new("src/main.rs", 10, 5, "my_func", false));

        // Test filtering
        let functions = index.symbols_by_kind(SymbolKind::Function);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].symbol, "my_func");

        let func_occurrences = index.occurrences_of("my_func");
        assert_eq!(func_occurrences.len(), 3);

        let definitions = index.definitions_of("my_func");
        assert_eq!(definitions.len(), 1);
        assert!(definitions[0].is_definition);

        let references = index.references_of("my_func");
        assert_eq!(references.len(), 2);
        assert!(!references.iter().any(|o| o.is_definition));
    }

    #[test]
    fn test_rust_package_info_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-crate"
version = "1.2.3"
edition = "2021"
"#,
        )
        .unwrap();

        let parser = RustParser::new();
        let package = parser.get_package_info(temp_dir.path()).unwrap();

        assert_eq!(package.manager, "cargo");
        assert_eq!(package.name, "test-crate");
        assert_eq!(package.version, "1.2.3");
    }

    #[test]
    fn test_typescript_package_info_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"
{
    "name": "@scope/my-package",
    "version": "2.0.0",
    "description": "Test package"
}
"#,
        )
        .unwrap();

        let parser = TypeScriptParser::new();
        let package = parser.get_package_info(temp_dir.path()).unwrap();

        assert_eq!(package.manager, "npm");
        assert_eq!(package.name, "@scope/my-package");
        assert_eq!(package.version, "2.0.0");
    }

    #[test]
    fn test_go_package_info_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let go_mod = temp_dir.path().join("go.mod");

        fs::write(
            &go_mod,
            r#"
module github.com/example/my-go-project

go 1.21

require (
    github.com/some/dependency v1.0.0
)
"#,
        )
        .unwrap();

        let parser = GoParser::new();
        let package = parser.get_package_info(temp_dir.path()).unwrap();

        assert_eq!(package.manager, "go");
        assert_eq!(package.name, "github.com/example/my-go-project");
        assert_eq!(package.version, "0.0.0");
    }

    #[test]
    fn test_parser_registry() {
        let registry = ParserRegistry::new();

        // Check supported languages
        let languages = registry.supported_languages();
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::Go));

        // Check support method
        assert!(registry.supports(Language::Rust));
        assert!(registry.supports(Language::TypeScript));
    }

    #[test]
    fn test_symbol_relationships() {
        let mut symbol = Symbol::new("ChildClass", SymbolKind::Class, "struct ChildClass");
        symbol.relationships.push(SymbolRelationship::new(
            "ParentClass",
            RelationshipKind::Extends,
        ));
        symbol.relationships.push(SymbolRelationship::new(
            "SomeTrait",
            RelationshipKind::Implements,
        ));

        assert_eq!(symbol.relationships.len(), 2);
        assert_eq!(symbol.relationships[0].symbol, "ParentClass");
        assert_eq!(
            symbol.relationships[0].kind,
            RelationshipKind::Extends as i32
        );
    }

    #[test]
    fn test_external_symbol() {
        let package = PackageInfo::new("npm", "lodash", "4.17.21");
        let external = ExternalSymbol::new("lodash.map", package.clone(), SymbolKind::Function);

        assert_eq!(external.symbol, "lodash.map");
        assert_eq!(external.package.name, "lodash");
        assert_eq!(external.package.version, "4.17.21");
        assert_eq!(external.kind_enum(), SymbolKind::Function);
    }

    #[test]
    fn test_occurrence_span() {
        let occurrence = Occurrence::new("src/lib.rs", 10, 5, "my_func", true)
            .with_end_position(10, 15)
            .with_snippet("fn my_func() {}");

        let (start_line, start_col, end_line, end_col) = occurrence.span();
        assert_eq!(start_line, 10);
        assert_eq!(start_col, 5);
        assert_eq!(end_line, 10);
        assert_eq!(end_col, 15);
        assert_eq!(occurrence.snippet, "fn my_func() {}");
    }

    #[test]
    fn test_cross_repository_navigation() {
        // Simulate cross-repo symbol resolution
        let local_package = PackageInfo::new("cargo", "my-app", "1.0.0");
        let external_package = PackageInfo::new("cargo", "serde", "1.0.193");

        let local_symbol = Symbol::new("my_app::main", SymbolKind::Function, "fn main()");
        let external_symbol = ExternalSymbol::new(
            "serde::Deserialize",
            external_package.clone(),
            SymbolKind::Trait,
        );

        // Verify package ownership
        assert_eq!(local_package.qualified_name(), "cargo:my-app/1.0.0");
        assert_eq!(external_package.qualified_name(), "cargo:serde/1.0.193");
        assert_ne!(local_package.name, external_package.name);

        // Symbols belong to different packages
        assert!(local_symbol.symbol.starts_with("my_app::"));
        assert!(external_symbol.symbol.starts_with("serde::"));
    }

    #[test]
    fn test_rust_fallback_scip_extracts_symbols_and_occurrences() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(
            temp.path().join("src/auth.rs"),
            "pub fn login() {}\nuse crate::db::query;\n",
        )
        .unwrap();
        std::fs::write(temp.path().join("src/db.rs"), "pub fn query() {}\n").unwrap();

        let index = generate_fallback_scip(
            Language::Rust,
            temp.path(),
            PackageInfo::new("cargo", "demo", "0.1.0"),
        )
        .unwrap();

        assert!(index
            .symbols
            .iter()
            .any(|symbol| symbol.symbol == "auth::login"));
        assert!(index
            .occurrences
            .iter()
            .any(|occurrence| !occurrence.is_definition));
    }

    #[test]
    fn test_typescript_fallback_scip_extracts_imports() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"name":"demo","version":"0.1.0"}"#,
        )
        .unwrap();
        std::fs::write(temp.path().join("util.ts"), "export function query() {}\n").unwrap();
        std::fs::write(
            temp.path().join("auth.ts"),
            "import { query } from './util';\nexport function login() { return query(); }\n",
        )
        .unwrap();

        let index = generate_fallback_scip(
            Language::TypeScript,
            temp.path(),
            PackageInfo::new("npm", "demo", "0.1.0"),
        )
        .unwrap();

        assert!(index
            .symbols
            .iter()
            .any(|symbol| symbol.symbol == "auth::login"));
        assert!(index
            .occurrences
            .iter()
            .any(|occurrence| occurrence.symbol == "util::query" && !occurrence.is_definition));
    }

    #[test]
    fn test_python_fallback_scip_extracts_imports() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("pyproject.toml"),
            "[project]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        std::fs::write(temp.path().join("db.py"), "def query():\n    return 1\n").unwrap();
        std::fs::write(
            temp.path().join("auth.py"),
            "from db import query\n\ndef login():\n    return query()\n",
        )
        .unwrap();

        let index = generate_fallback_scip(
            Language::Python,
            temp.path(),
            PackageInfo::new("pip", "demo", "0.1.0"),
        )
        .unwrap();

        assert!(index
            .symbols
            .iter()
            .any(|symbol| symbol.symbol == "auth::login"));
        assert!(index
            .occurrences
            .iter()
            .any(|occurrence| occurrence.symbol == "db::query" && !occurrence.is_definition));
    }
}
