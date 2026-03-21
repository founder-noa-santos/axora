//! Living Documentation system for auto-updating docs when code changes.
//!
//! This module provides automatic documentation updates when source code
//! changes are detected, ensuring documentation stays synchronized with code.

use crate::index::DocIndex;
use crate::schema::{DocId, Document};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Living docs errors
#[derive(Error, Debug)]
pub enum LivingDocsError {
    /// Document not found
    #[error("document not found: {0}")]
    NotFound(String),

    /// Failed to parse code changes
    #[error("failed to parse code changes: {0}")]
    ParseError(String),

    /// Failed to generate update suggestion
    #[error("failed to generate update: {0}")]
    UpdateError(String),
}

/// Result type for living docs operations
pub type Result<T> = std::result::Result<T, LivingDocsError>;

/// Type of update needed for a document
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateType {
    /// Safe to auto-apply (minor changes)
    AutoUpdate,
    /// Needs human/agent review (significant changes)
    FlagForReview,
    /// Document is now obsolete
    Deprecate,
}

/// Suggested update for a document
#[derive(Debug, Clone)]
pub struct DocUpdate {
    /// Document ID to update
    pub doc_id: String,
    /// Type of update needed
    pub update_type: UpdateType,
    /// Suggested changes (markdown or full content)
    pub suggested_changes: String,
    /// Reason for the update
    pub reason: String,
}

/// File hash for change detection
#[derive(Debug, Clone)]
pub struct FileHash {
    /// Blake3 hash of file content
    pub hash: String,
    /// File size in bytes
    pub size: usize,
    /// Last modified timestamp
    pub modified_at: u64,
}

/// Living Documentation manager
pub struct LivingDocs {
    /// Document index
    docs: DocIndex,
    /// Map of file path to file hash
    codebase_hash: HashMap<PathBuf, FileHash>,
    /// Map of file path to associated document IDs
    file_to_docs: HashMap<PathBuf, Vec<DocId>>,
    /// Pending reviews (doc_id -> reason)
    pending_reviews: HashMap<DocId, String>,
}

impl LivingDocs {
    /// Create a new LivingDocs instance
    pub fn new() -> Self {
        Self {
            docs: DocIndex::new(),
            codebase_hash: HashMap::new(),
            file_to_docs: HashMap::new(),
            pending_reviews: HashMap::new(),
        }
    }

    /// Create with an existing document index
    pub fn with_index(index: DocIndex) -> Self {
        Self {
            docs: index,
            codebase_hash: HashMap::new(),
            file_to_docs: HashMap::new(),
            pending_reviews: HashMap::new(),
        }
    }

    /// Register a file and its associated document
    pub fn register_file(&mut self, path: &Path, doc_id: &str, content: &str) {
        let hash = self.compute_hash(content);

        self.codebase_hash.insert(path.to_path_buf(), hash);
        self.file_to_docs
            .entry(path.to_path_buf())
            .or_default()
            .push(doc_id.to_string());
    }

    /// Handle a code change and return suggested doc updates
    pub fn on_code_change(
        &mut self,
        file: &Path,
        old_content: &str,
        new_content: &str,
    ) -> Vec<DocUpdate> {
        let mut updates: Vec<DocUpdate> = Vec::new();

        // Compute new hash
        let new_hash = self.compute_hash(new_content);

        // Check if file was previously tracked
        let _old_hash = self.codebase_hash.get(file).cloned();

        // Update hash
        self.codebase_hash.insert(file.to_path_buf(), new_hash);

        // Find associated documents
        let doc_ids = self.file_to_docs.get(file).cloned().unwrap_or_default();
        let doc_ids_empty = doc_ids.is_empty();

        for doc_id in doc_ids {
            if let Some(doc) = self.docs.get(&doc_id) {
                let update = self.analyze_change(&doc_id, doc, old_content, new_content, file);
                if let Some(u) = update {
                    updates.push(u);
                }
            }
        }

        // If no docs are associated but file changed significantly, flag for review
        if doc_ids_empty && self.significant_change_detected(old_content, new_content) {
            updates.push(DocUpdate {
                doc_id: format!("pending:{}", file.display()),
                update_type: UpdateType::FlagForReview,
                suggested_changes: format!(
                    "New or significantly changed file: {}\nConsider creating documentation.",
                    file.display()
                ),
                reason: "Significant code change without associated documentation".to_string(),
            });
        }

        updates
    }

    /// Flag a document for review
    pub fn flag_for_review(&mut self, doc_id: &str, reason: &str) {
        self.pending_reviews
            .insert(doc_id.to_string(), reason.to_string());
    }

    /// Get pending reviews
    pub fn get_pending_reviews(&self) -> Vec<(&str, &str)> {
        self.pending_reviews
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// Clear a pending review
    pub fn clear_review(&mut self, doc_id: &str) {
        self.pending_reviews.remove(doc_id);
    }

    /// Add a document to the index
    pub fn add_document(&mut self, doc: Document) -> crate::index::Result<()> {
        self.docs.add(doc)
    }

    /// Get the document index
    pub fn index(&self) -> &DocIndex {
        &self.docs
    }

    /// Get a mutable reference to the document index
    pub fn index_mut(&mut self) -> &mut DocIndex {
        &mut self.docs
    }

    /// Get all tracked files
    pub fn tracked_files(&self) -> Vec<&Path> {
        self.codebase_hash.keys().map(|p| p.as_path()).collect()
    }

    /// Check if a file is tracked
    pub fn is_tracked(&self, path: &Path) -> bool {
        self.codebase_hash.contains_key(path)
    }

    /// Get documents associated with a file
    pub fn docs_for_file(&self, path: &Path) -> Vec<&Document> {
        self.file_to_docs
            .get(path)
            .map(|ids| ids.iter().filter_map(|id| self.docs.get(id)).collect())
            .unwrap_or_default()
    }

    /// Detect if documentation is stale compared to code
    pub fn is_doc_stale(&self, doc_id: &str) -> bool {
        if let Some(doc) = self.docs.get(doc_id) {
            // Check if doc is old (more than 7 days)
            if doc.age_days() > 7 {
                return true;
            }

            // Check if associated files have changed
            for path in self.file_to_docs.keys() {
                if let Some(doc_ids) = self.file_to_docs.get(path) {
                    if doc_ids.contains(&doc_id.to_string()) {
                        // File has been modified since doc was updated
                        if let Some(file_hash) = self.codebase_hash.get(path) {
                            if file_hash.modified_at > doc.updated_at {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Get stale documents
    pub fn find_stale_docs(&self) -> Vec<&Document> {
        self.docs
            .all()
            .into_iter()
            .filter(|doc| {
                // Check doc age
                if doc.age_days() > 7 {
                    return true;
                }

                // Check if associated files changed
                for (path, doc_ids) in &self.file_to_docs {
                    if doc_ids.contains(&doc.id) {
                        if let Some(file_hash) = self.codebase_hash.get(path) {
                            if file_hash.modified_at > doc.updated_at {
                                return true;
                            }
                        }
                    }
                }
                false
            })
            .collect()
    }

    /// Get all documents
    pub fn all_docs(&self) -> Vec<&Document> {
        self.docs.all()
    }

    // === Internal Implementation ===

    /// Compute hash of content
    fn compute_hash(&self, content: &str) -> FileHash {
        use blake3::hash;

        let hash_result = hash(content.as_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        FileHash {
            hash: hash_result.to_hex().to_string(),
            size: content.len(),
            modified_at: now,
        }
    }

    /// Analyze a code change and suggest doc updates
    fn analyze_change(
        &self,
        doc_id: &str,
        _doc: &Document,
        old_content: &str,
        new_content: &str,
        file: &Path,
    ) -> Option<DocUpdate> {
        let diff_stats = self.compute_diff_stats(old_content, new_content);

        // Determine update type based on change magnitude
        let change_lines = diff_stats.lines_added + diff_stats.lines_removed;
        let update_type = if change_lines > 10 {
            UpdateType::FlagForReview
        } else {
            UpdateType::AutoUpdate
        };

        // Check for specific patterns that need doc updates
        let mut reasons: Vec<String> = Vec::new();

        if diff_stats.functions_added > 0 {
            reasons.push(format!(
                "{} new function(s) added",
                diff_stats.functions_added
            ));
        }

        if diff_stats.functions_removed > 0 {
            reasons.push(format!(
                "{} function(s) removed",
                diff_stats.functions_removed
            ));
        }

        if diff_stats.lines_added > 100 {
            reasons.push("Large addition (>100 lines)".to_string());
        }

        if diff_stats.lines_removed > 50 {
            reasons.push("Significant removal (>50 lines)".to_string());
        }

        if reasons.is_empty() && diff_stats.lines_added + diff_stats.lines_removed > 0 {
            reasons.push("Minor code changes".to_string());
        }

        if !reasons.is_empty() {
            Some(DocUpdate {
                doc_id: doc_id.to_string(),
                update_type,
                suggested_changes: self.generate_update_suggestion(&reasons, file, &diff_stats),
                reason: reasons.join("; "),
            })
        } else {
            None
        }
    }

    /// Compute diff statistics
    fn compute_diff_stats(&self, old: &str, new: &str) -> DiffStats {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut stats = DiffStats::default();

        // Simple line-based diff
        let mut old_idx = 0;
        let mut new_idx = 0;

        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if old_idx >= old_lines.len() {
                stats.lines_added += 1;
                if new_lines[new_idx].trim().starts_with("fn ")
                    || new_lines[new_idx].trim().starts_with("pub fn ")
                    || new_lines[new_idx].trim().starts_with("async fn ")
                    || new_lines[new_idx].trim().starts_with("def ")
                    || new_lines[new_idx].trim().starts_with("function ")
                {
                    stats.functions_added += 1;
                }
                new_idx += 1;
            } else if new_idx >= new_lines.len() {
                stats.lines_removed += 1;
                if old_lines[old_idx].trim().starts_with("fn ")
                    || old_lines[old_idx].trim().starts_with("pub fn ")
                    || old_lines[old_idx].trim().starts_with("async fn ")
                    || old_lines[old_idx].trim().starts_with("def ")
                    || old_lines[old_idx].trim().starts_with("function ")
                {
                    stats.functions_removed += 1;
                }
                old_idx += 1;
            } else if old_lines[old_idx] == new_lines[new_idx] {
                old_idx += 1;
                new_idx += 1;
            } else {
                // Check if line was modified vs added/removed
                if is_similar_line(old_lines[old_idx], new_lines[new_idx]) {
                    stats.lines_modified += 1;
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    stats.lines_added += 1;
                    stats.lines_removed += 1;
                    old_idx += 1;
                    new_idx += 1;
                }
            }
        }

        stats
    }

    /// Check if code change is significant
    fn significant_change_detected(&self, old: &str, new: &str) -> bool {
        let stats = self.compute_diff_stats(old, new);

        // Consider significant if:
        // - More than 20 lines changed
        // - Functions added or removed
        // - More than 100 bytes difference
        stats.lines_added + stats.lines_removed > 20
            || stats.functions_added > 0
            || stats.functions_removed > 0
            || (old.len() as i64 - new.len() as i64).abs() > 100
    }

    /// Generate update suggestion based on changes
    fn generate_update_suggestion(
        &self,
        reasons: &[String],
        file: &Path,
        stats: &DiffStats,
    ) -> String {
        format!(
            r#"## Documentation Update Suggestion

**File:** `{}`

**Changes detected:**
{}

**Diff statistics:**
- Lines added: {}
- Lines removed: {}
- Lines modified: {}
- Functions added: {}
- Functions removed: {}

**Recommended actions:**
1. Review the code changes
2. Update API documentation if function signatures changed
3. Update examples if behavior changed
4. Add changelog entry if this is a user-facing change
"#,
            file.display(),
            reasons
                .iter()
                .map(|r| format!("- {}", r))
                .collect::<Vec<_>>()
                .join("\n"),
            stats.lines_added,
            stats.lines_removed,
            stats.lines_modified,
            stats.functions_added,
            stats.functions_removed,
        )
    }
}

impl Default for LivingDocs {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about code changes
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    /// Number of lines added
    pub lines_added: usize,
    /// Number of lines removed
    pub lines_removed: usize,
    /// Number of lines modified
    pub lines_modified: usize,
    /// Number of functions added
    pub functions_added: usize,
    /// Number of functions removed
    pub functions_removed: usize,
}

/// Check if two lines are similar (for diff detection)
fn is_similar_line(old: &str, new: &str) -> bool {
    let old_trimmed = old.trim();
    let new_trimmed = new.trim();

    // Check if lines have similar structure
    let old_words: Vec<&str> = old_trimmed.split_whitespace().collect();
    let new_words: Vec<&str> = new_trimmed.split_whitespace().collect();

    // If same number of words and >50% match, consider similar
    if old_words.len() == new_words.len() && !old_words.is_empty() {
        let matches = old_words
            .iter()
            .zip(new_words.iter())
            .filter(|(a, b)| a == b)
            .count();
        matches > old_words.len() / 2
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::DocSchema;

    fn create_test_doc(id: &str, module: &str, content: &str) -> Document {
        let schema = DocSchema::new(module, "1.0", "test-agent");
        Document::new(id, schema, content.to_string(), "1.0.0")
    }

    #[test]
    fn test_living_docs_creation() {
        let living_docs = LivingDocs::new();

        assert!(living_docs.tracked_files().is_empty());
        assert!(living_docs.get_pending_reviews().is_empty());
    }

    #[test]
    fn test_living_docs_code_change_detection() {
        let mut living_docs = LivingDocs::new();

        let old_code = r#"
fn hello() {
    println!("Hello");
}
"#;

        let new_code = r#"
fn hello() {
    println!("Hello, World!");
}

fn goodbye() {
    println!("Goodbye");
}
"#;

        let updates = living_docs.on_code_change(Path::new("src/test.rs"), old_code, new_code);

        // Should detect changes even without registered docs
        assert!(!updates.is_empty());
    }

    #[test]
    fn test_living_docs_file_registration() {
        let mut living_docs = LivingDocs::new();

        let doc = create_test_doc("test-doc", "test", "Documentation content");
        living_docs.add_document(doc).expect("Failed to add doc");

        let code = "fn test() {}";
        living_docs.register_file(Path::new("src/test.rs"), "test-doc", code);

        assert!(living_docs.is_tracked(Path::new("src/test.rs")));
        assert_eq!(living_docs.docs_for_file(Path::new("src/test.rs")).len(), 1);
    }

    #[test]
    fn test_living_docs_auto_update_detection() {
        let mut living_docs = LivingDocs::new();

        // Add and register a document
        let doc = create_test_doc("api-doc", "api", "API documentation");
        living_docs.add_document(doc).expect("Failed to add doc");
        living_docs.register_file(Path::new("src/api.rs"), "api-doc", "fn old() {}");

        // Simulate code change
        let updates = living_docs.on_code_change(
            Path::new("src/api.rs"),
            "fn old() {}",
            "fn new() {} fn another() {}",
        );

        assert!(!updates.is_empty());
        assert_eq!(updates[0].doc_id, "api-doc");
    }

    #[test]
    fn test_living_docs_flag_for_review() {
        let mut living_docs = LivingDocs::new();

        living_docs.flag_for_review("doc-1", "Needs API review");
        living_docs.flag_for_review("doc-2", "Outdated examples");

        let reviews = living_docs.get_pending_reviews();
        assert_eq!(reviews.len(), 2);

        living_docs.clear_review("doc-1");
        assert_eq!(living_docs.get_pending_reviews().len(), 1);
    }

    #[test]
    fn test_living_docs_staleness() {
        let mut living_docs = LivingDocs::new();

        let mut doc = create_test_doc("stale-doc", "test", "Old documentation");
        // Make doc older by setting updated_at to the past
        doc.updated_at -= 86400; // 1 day ago
        living_docs.add_document(doc).expect("Failed to add doc");

        // Register file after doc is created (simulating file change)
        living_docs.register_file(Path::new("src/test.rs"), "stale-doc", "fn test() {}");

        // Doc should be stale because file was registered after doc creation
        assert!(living_docs.is_doc_stale("stale-doc"));
    }

    #[test]
    fn test_living_docs_find_stale_docs() {
        let mut living_docs = LivingDocs::new();

        // Add fresh doc
        let doc1 = create_test_doc("fresh-doc", "test", "Fresh docs");
        living_docs.add_document(doc1).expect("Failed to add doc");

        // Add and register another doc with older timestamp
        let mut doc2 = create_test_doc("stale-doc", "test", "Stale docs");
        doc2.updated_at -= 86400; // 1 day ago
        living_docs.add_document(doc2).expect("Failed to add doc");
        living_docs.register_file(Path::new("src/test.rs"), "stale-doc", "fn test() {}");

        let stale = living_docs.find_stale_docs();

        // At least stale-doc should be in the list
        assert!(stale.iter().any(|d| d.id == "stale-doc"));
    }

    #[test]
    fn test_diff_stats_calculation() {
        let living_docs = LivingDocs::new();

        let old = "fn hello() {\n    println!(\"Hi\");\n}";
        let new = "fn hello() {\n    println!(\"Hello\");\n}\n\nfn goodbye() {\n    println!(\"Bye\");\n}";

        let stats = living_docs.compute_diff_stats(old, new);

        assert!(stats.lines_added > 0);
        assert!(stats.functions_added >= 1);
    }

    #[test]
    fn test_significant_change_detection() {
        let living_docs = LivingDocs::new();

        // Small change
        let old1 = "fn test() { let x = 1; }";
        let new1 = "fn test() { let x = 2; }";
        assert!(!living_docs.significant_change_detected(old1, new1));

        // Large change (new function)
        let old2 = "fn test() {}";
        let new2 = "fn test() {}\nfn new_function() {\n    // lots of code\n    // more code\n    // even more\n}";
        assert!(living_docs.significant_change_detected(old2, new2));
    }

    #[test]
    fn test_full_workflow() {
        let mut living_docs = LivingDocs::new();

        // Step 1: Create initial documentation
        let api_doc = create_test_doc(
            "auth-api",
            "auth",
            "# Auth API\n\nFunctions:\n- `login()`: Authenticate user",
        );
        living_docs
            .add_document(api_doc)
            .expect("Failed to add doc");

        // Step 2: Register source file
        let initial_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token> {
    // Authenticate user
    Ok(Token::new())
}
"#;
        living_docs.register_file(Path::new("src/auth/login.rs"), "auth-api", initial_code);

        // Step 3: Simulate code change (add new function)
        let updated_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token> {
    // Authenticate user with improved security
    Ok(Token::new())
}

pub fn logout(token: &Token) -> Result<()> {
    // Invalidate token
    Ok(())
}
"#;

        let updates =
            living_docs.on_code_change(Path::new("src/auth/login.rs"), initial_code, updated_code);

        // Step 4: Verify update was detected
        assert!(!updates.is_empty());
        assert_eq!(updates[0].doc_id, "auth-api");
        assert!(updates[0].reason.contains("function"));

        // Step 5: Flag for review (simulating agent decision)
        living_docs.flag_for_review("auth-api", "New function needs documentation");

        // Step 6: Verify review is pending
        let reviews = living_docs.get_pending_reviews();
        assert!(reviews.iter().any(|(_, r)| r.contains("New function")));
    }

    #[test]
    fn test_update_type_classification() {
        let mut living_docs = LivingDocs::new();

        let doc = create_test_doc("test-doc", "test", "docs");
        living_docs.add_document(doc).expect("Failed to add");
        living_docs.register_file(Path::new("test.rs"), "test-doc", "fn a() {}");

        // Small change -> AutoUpdate
        let updates1 = living_docs.on_code_change(
            Path::new("test.rs"),
            "fn a() {}",
            "fn a() { /* comment */ }",
        );
        assert!(updates1
            .iter()
            .any(|u| u.update_type == UpdateType::AutoUpdate));

        // Large change -> FlagForReview
        let large_addition = (0..60)
            .map(|i| format!("fn func_{}() {{}}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let updates2 =
            living_docs.on_code_change(Path::new("test.rs"), "fn a() {}", &large_addition);
        assert!(updates2
            .iter()
            .any(|u| u.update_type == UpdateType::FlagForReview));
    }

    #[test]
    fn test_docs_for_file() {
        let mut living_docs = LivingDocs::new();

        let doc1 = create_test_doc("doc1", "test", "content1");
        let doc2 = create_test_doc("doc2", "test", "content2");
        living_docs.add_document(doc1).expect("Failed");
        living_docs.add_document(doc2).expect("Failed");

        living_docs.register_file(Path::new("shared.rs"), "doc1", "code");
        living_docs.register_file(Path::new("shared.rs"), "doc2", "code");

        let docs = living_docs.docs_for_file(Path::new("shared.rs"));
        assert_eq!(docs.len(), 2);
    }
}
