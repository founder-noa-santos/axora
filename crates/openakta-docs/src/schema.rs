//! Documentation schema definitions for agent-native documentation.
//!
//! This module provides the core data structures for representing documentation
//! in a format that agents can read, write, and update automatically.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema errors
#[derive(Error, Debug)]
pub enum SchemaError {
    /// Invalid schema version
    #[error("invalid schema version: {0}")]
    InvalidVersion(String),

    /// Missing required field
    #[error("missing required field: {0}")]
    MissingField(String),
}

/// Unique identifier for a document
pub type DocId = String;

/// A document with structured schema and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document identifier
    pub id: DocId,
    /// The schema this document follows
    pub schema: DocSchema,
    /// The document content (markdown, JSON, etc.)
    pub content: String,
    /// Unix timestamp when document was created
    pub created_at: u64,
    /// Unix timestamp when document was last updated
    pub updated_at: u64,
    /// Version string (e.g., "1.0.0")
    pub version: String,
}

impl Document {
    /// Create a new document
    pub fn new(id: &str, schema: DocSchema, content: String, version: &str) -> Self {
        let now = Utc::now().timestamp() as u64;
        Self {
            id: id.to_string(),
            schema,
            content,
            created_at: now,
            updated_at: now,
            version: version.to_string(),
        }
    }

    /// Update document content and timestamp
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now().timestamp() as u64;
    }

    /// Check if document is stale (older than max_age_seconds)
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        let now = Utc::now().timestamp() as u64;
        now - self.updated_at > max_age_seconds
    }

    /// Get document age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = Utc::now().timestamp() as u64;
        now - self.updated_at
    }

    /// Get document age in days
    pub fn age_days(&self) -> u64 {
        self.age_seconds() / 86400
    }
}

/// Schema for agent-native documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSchema {
    /// Module or component this documentation belongs to
    pub module: String,
    /// Schema version (e.g., "1.0")
    pub version: String,
    /// Unix timestamp of last update
    pub last_updated: u64,
    /// Maintainer (agent or human) responsible for this documentation
    pub maintainer: String,
    /// Sections within this documentation
    pub sections: Vec<DocSection>,
}

impl DocSchema {
    /// Create a new schema
    pub fn new(module: &str, version: &str, maintainer: &str) -> Self {
        Self {
            module: module.to_string(),
            version: version.to_string(),
            last_updated: Utc::now().timestamp() as u64,
            maintainer: maintainer.to_string(),
            sections: Vec::new(),
        }
    }

    /// Add a section to the schema
    pub fn add_section(&mut self, section: DocSection) {
        self.sections.push(section);
        self.last_updated = Utc::now().timestamp() as u64;
    }

    /// Update the last_updated timestamp
    pub fn touch(&mut self) {
        self.last_updated = Utc::now().timestamp() as u64;
    }
}

/// A section within documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DocSection {
    /// API contract documentation with endpoint definitions
    ApiContract {
        /// API endpoints
        endpoints: Vec<Endpoint>,
    },
    /// Architecture documentation with design decisions
    Architecture {
        /// References to ADRs or decision IDs
        decisions: Vec<String>,
    },
    /// Code patterns and examples
    Patterns {
        /// Code examples demonstrating patterns
        examples: Vec<CodeExample>,
    },
    /// Test documentation
    Tests {
        /// Test case descriptions
        test_cases: Vec<TestCase>,
    },
    /// General documentation section
    General {
        /// Section title
        title: String,
        /// Section content (markdown)
        content: String,
    },
}

/// An API endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Endpoint path
    pub path: String,
    /// Description of what the endpoint does
    pub description: String,
    /// Request parameters
    pub parameters: Vec<Parameter>,
    /// Response schema
    pub response: String,
}

/// A parameter for an API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, int, etc.)
    pub param_type: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Parameter description
    pub description: String,
}

/// A code example demonstrating a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    /// Example title
    pub title: String,
    /// Programming language
    pub language: String,
    /// The code snippet
    pub code: String,
    /// Explanation of what the code does
    pub explanation: String,
}

/// A test case description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Test case name
    pub name: String,
    /// What the test verifies
    pub description: String,
    /// Test input
    pub input: String,
    /// Expected output
    pub expected_output: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_schema_creation() {
        let schema = DocSchema::new("auth", "1.0", "agent-a");

        assert_eq!(schema.module, "auth");
        assert_eq!(schema.version, "1.0");
        assert_eq!(schema.maintainer, "agent-a");
        assert!(schema.sections.is_empty());
    }

    #[test]
    fn test_document_creation() {
        let schema = DocSchema::new("core", "1.0", "agent-b");
        let doc = Document::new(
            "core-api",
            schema.clone(),
            "# Core API Documentation".to_string(),
            "1.0.0",
        );

        assert_eq!(doc.id, "core-api");
        assert_eq!(doc.schema.module, "core");
        assert_eq!(doc.version, "1.0.0");
        assert!(!doc.content.is_empty());
        assert_eq!(doc.created_at, doc.updated_at);
    }

    #[test]
    fn test_document_update() {
        let schema = DocSchema::new("cache", "1.0", "agent-c");
        let mut doc = Document::new("cache-docs", schema, "Initial content".to_string(), "1.0.0");

        let original_updated = doc.updated_at;

        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(10));

        doc.update_content("Updated content".to_string());

        assert_eq!(doc.content, "Updated content");
        assert!(doc.updated_at >= original_updated);
    }

    #[test]
    fn test_document_staleness() {
        let schema = DocSchema::new("test", "1.0", "agent-d");
        let doc = Document::new("test-doc", schema, "content".to_string(), "1.0.0");

        // Document should not be stale with very large max_age
        assert!(!doc.is_stale(u64::MAX));

        // Document just created, so it's not stale with 0 max_age (age is 0)
        assert!(!doc.is_stale(0));

        // Document should be stale with negative max_age (impossible, but tests the logic)
        // In practice, a document becomes stale after max_age seconds have passed
    }

    #[test]
    fn test_doc_section_api_contract() {
        let endpoint = Endpoint {
            method: "POST".to_string(),
            path: "/mcp.v1.ToolService/CallTool".to_string(),
            description: "Invoke a sandboxed tool against the local workspace".to_string(),
            parameters: vec![Parameter {
                name: "limit".to_string(),
                param_type: "int".to_string(),
                required: false,
                description: "Max results".to_string(),
            }],
            response: "UserList".to_string(),
        };

        let section = DocSection::ApiContract {
            endpoints: vec![endpoint],
        };

        match section {
            DocSection::ApiContract { endpoints } => {
                assert_eq!(endpoints.len(), 1);
                assert_eq!(endpoints[0].method, "POST");
            }
            _ => panic!("Expected ApiContract section"),
        }
    }

    #[test]
    fn test_doc_section_patterns() {
        let example = CodeExample {
            title: "Basic Usage".to_string(),
            language: "rust".to_string(),
            code: "fn main() { println!(\"hello\"); }".to_string(),
            explanation: "Prints hello world".to_string(),
        };

        let section = DocSection::Patterns {
            examples: vec![example],
        };

        match section {
            DocSection::Patterns { examples } => {
                assert_eq!(examples.len(), 1);
                assert_eq!(examples[0].title, "Basic Usage");
            }
            _ => panic!("Expected Patterns section"),
        }
    }

    #[test]
    fn test_doc_section_tests() {
        let test_case = TestCase {
            name: "test_addition".to_string(),
            description: "Verifies addition works".to_string(),
            input: "1 + 1".to_string(),
            expected_output: "2".to_string(),
        };

        let section = DocSection::Tests {
            test_cases: vec![test_case],
        };

        match section {
            DocSection::Tests { test_cases } => {
                assert_eq!(test_cases.len(), 1);
                assert_eq!(test_cases[0].name, "test_addition");
            }
            _ => panic!("Expected Tests section"),
        }
    }

    #[test]
    fn test_doc_section_architecture() {
        let section = DocSection::Architecture {
            decisions: vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        };

        match section {
            DocSection::Architecture { decisions } => {
                assert_eq!(decisions.len(), 2);
                assert_eq!(decisions[0], "AUTH-001");
            }
            _ => panic!("Expected Architecture section"),
        }
    }

    #[test]
    fn test_doc_section_general() {
        let section = DocSection::General {
            title: "Getting Started".to_string(),
            content: "# Getting Started\n\nThis is the guide.".to_string(),
        };

        match section {
            DocSection::General { title, content } => {
                assert_eq!(title, "Getting Started");
                assert!(content.contains("Getting Started"));
            }
            _ => panic!("Expected General section"),
        }
    }

    #[test]
    fn test_schema_add_section() {
        let mut schema = DocSchema::new("test", "1.0", "agent-e");

        schema.add_section(DocSection::General {
            title: "Section 1".to_string(),
            content: "Content 1".to_string(),
        });

        assert_eq!(schema.sections.len(), 1);
        assert!(schema.last_updated > 0);
    }

    #[test]
    fn test_document_age() {
        let schema = DocSchema::new("test", "1.0", "agent-f");
        let doc = Document::new("test-doc", schema, "content".to_string(), "1.0.0");

        // Age should be very small (just created)
        assert!(doc.age_seconds() < 60);
        assert!(doc.age_days() < 1);
    }

    #[test]
    fn test_document_serialization() {
        let schema = DocSchema::new("test", "1.0", "agent-g");
        let doc = Document::new("test-doc", schema, "content".to_string(), "1.0.0");

        // Serialize to JSON
        let json = serde_json::to_string(&doc).expect("Failed to serialize");
        assert!(!json.is_empty());

        // Deserialize back
        let deserialized: Document = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.id, doc.id);
        assert_eq!(deserialized.content, doc.content);
    }
}
