//! AXORA Documentation Management System
//!
//! Agent-native documentation system that enables automatic documentation
//! updates when code changes, with support for Architecture Decision Records (ADRs).
//!
//! # Features
//!
//! - **Structured Documentation**: Schema-based docs that agents can read and write
//! - **Living Documentation**: Auto-update docs when source code changes
//! - **Document Index**: Search and retrieval of documentation
//! - **ADR System**: Track architectural decisions with linking and consequences
//!
//! # Example
//!
//! ```rust,no_run
//! use axora_docs::{DocIndex, DocQuery, LivingDocs, Document, DocSchema};
//!
//! // Create living docs manager
//! let mut living_docs = LivingDocs::new();
//!
//! // Add documentation
//! let doc = Document::new(
//!     "api-docs",
//!     DocSchema::new("api", "1.0", "agent-a"),
//!     "# API Documentation".to_string(),
//!     "1.0.0",
//! );
//! living_docs.add_document(doc).expect("Failed to add");
//!
//! // Register source file
//! living_docs.register_file(
//!     std::path::Path::new("src/api.rs"),
//!     "api-docs",
//!     "pub fn handler() {}"
//! );
//!
//! // Detect changes and get update suggestions
//! let updates = living_docs.on_code_change(
//!     std::path::Path::new("src/api.rs"),
//!     "pub fn handler() {}",
//!     "pub fn handler() { /* new implementation */ }"
//! );
//! ```

#![warn(missing_docs)]

pub mod schema;
pub mod index;
pub mod living;
pub mod adr;

// Re-export main types
pub use schema::{
    DocSchema, DocSection, Document, DocId,
    Endpoint, Parameter, CodeExample, TestCase,
    SchemaError,
};

pub use index::{
    DocIndex, DocQuery, SearchResult, IndexError,
};

pub use living::{
    LivingDocs, DocUpdate, UpdateType, LivingDocsError, DiffStats,
};

pub use adr::{
    Adr, AdrStatus, AdrLog, AdrError,
};
