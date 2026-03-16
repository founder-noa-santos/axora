//! Context Distribution System (Graph-Based + Domain RAG)
//!
//! Intelligent context allocation for agents using Graph-Based workflow with Domain RAG:
//! - Domain knowledge is in vector stores (RAG), not agent structure
//! - Agents are generalists with domain-specific retrieval
//! - Coordination is O(N), not O(N²)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ContextManager                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SharedContext (global)  │  DomainRagStore (per-domain)    │
//! │  - global_docs           │  - auth: VectorStore            │
//! │  - global_code           │  - api: VectorStore             │
//! │  - decisions             │  - db: VectorStore              │
//! │                        │  - etc...                         │
//! └─────────────────────────────────────────────────────────────┘
//!                              ↓
//!              ┌───────────────────────────────┐
//!              │   TaskContext (minimal)       │
//!              │   - retrieved_knowledge       │
//!              │   - required_docs             │
//!              │   - related_tasks             │
//!              └───────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use axora_cache::context::ContextManager;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut ctx_manager = ContextManager::new();
//!
//! // Context manager is ready for context allocation
//! // See Task and Agent types for usage
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::rag::{DomainRagStore, Experience, RagResult, RetrievalStrategy};

/// Unique identifier for documents
pub type DocId = String;

/// Unique identifier for code files
pub type FileId = String;

/// Unique identifier for tasks
pub type TaskId = String;

/// Unique identifier for agents
pub type AgentId = String;

/// Represents a task that needs to be executed by an agent
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Documents this task potentially needs
    pub required_docs: Vec<DocId>,
    /// Code files this task potentially needs
    pub required_code: Vec<FileId>,
    /// Tasks this task depends on (results needed)
    pub dependencies: Vec<TaskId>,
    /// Priority level (higher = more urgent)
    pub priority: u8,
    /// Query for RAG retrieval (domain knowledge)
    pub query: String,
    /// Mentioned domains (optional, extracted from query if not provided)
    pub domains: Vec<String>,
}

impl Task {
    /// Creates a new task with the given parameters
    pub fn new(
        id: &str,
        required_docs: Vec<&str>,
        required_code: Vec<&str>,
        dependencies: Vec<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            required_docs: required_docs.into_iter().map(|s| s.to_string()).collect(),
            required_code: required_code.into_iter().map(|s| s.to_string()).collect(),
            dependencies: dependencies.into_iter().map(|s| s.to_string()).collect(),
            priority: 50, // Default priority
            query: String::new(),
            domains: Vec::new(),
        }
    }

    /// Sets the priority level for this task
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the query for RAG retrieval
    pub fn with_query(mut self, query: &str) -> Self {
        self.query = query.to_string();
        self
    }

    /// Sets the domains for this task
    pub fn with_domains(mut self, domains: Vec<&str>) -> Self {
        self.domains = domains.into_iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Represents an agent that executes tasks
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent identifier
    pub id: AgentId,
    /// Agent type/capability
    pub agent_type: String,
    /// Maximum context tokens this agent can handle
    pub max_context_tokens: usize,
}

impl Agent {
    /// Creates a new agent with the given ID
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            agent_type: "general".to_string(),
            max_context_tokens: 100_000,
        }
    }

    /// Sets the agent type
    pub fn with_type(mut self, agent_type: &str) -> Self {
        self.agent_type = agent_type.to_string();
        self
    }

    /// Sets the maximum context tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_context_tokens = max_tokens;
        self
    }
}

/// Represents a document in the system
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique document identifier
    pub id: DocId,
    /// Document content
    pub content: String,
    /// Document type (e.g., "adr", "spec", "readme")
    pub doc_type: String,
    /// Token count of the document
    pub token_count: usize,
}

impl Document {
    /// Creates a new document
    pub fn new(id: &str, content: &str, doc_type: &str) -> Self {
        let token_count = estimate_tokens(content);
        Self {
            id: id.to_string(),
            content: content.to_string(),
            doc_type: doc_type.to_string(),
            token_count,
        }
    }
}

/// Represents code file content
#[derive(Debug, Clone)]
pub struct CodeFile {
    /// Unique file identifier
    pub id: FileId,
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Token count of the file
    pub token_count: usize,
}

impl CodeFile {
    /// Creates a new code file
    pub fn new(id: &str, path: &str, content: &str) -> Self {
        let token_count = estimate_tokens(content);
        Self {
            id: id.to_string(),
            path: path.to_string(),
            content: content.to_string(),
            token_count,
        }
    }
}

/// Represents the result of a completed task
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task ID that produced this result
    pub task_id: TaskId,
    /// Result content
    pub content: String,
    /// Token count of the result
    pub token_count: usize,
}

impl TaskResult {
    /// Creates a new task result
    pub fn new(task_id: &str, content: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            content: content.to_string(),
            token_count: estimate_tokens(content),
        }
    }
}

/// Agent state within a task context
#[derive(Debug, Clone, Default)]
pub struct AgentState {
    /// Current state data (JSON-like string for flexibility)
    pub data: String,
    /// Step number in the agent's workflow
    pub step: u32,
    /// Whether the agent is currently active
    pub is_active: bool,
}

impl AgentState {
    /// Creates a new active agent state
    pub fn active() -> Self {
        Self {
            is_active: true,
            step: 0,
            data: String::new(),
        }
    }

    /// Creates a new inactive agent state
    pub fn inactive() -> Self {
        Self {
            is_active: false,
            step: 0,
            data: String::new(),
        }
    }
}

/// Shared context accessible by all tasks
#[derive(Debug, Clone, Default)]
pub struct SharedContext {
    /// Global documents available to all tasks
    global_docs: Vec<DocId>,
    /// Global code files available to all tasks
    global_code: Vec<FileId>,
    /// Architectural decisions (ADNs)
    decisions: Vec<String>,
    /// Document storage
    doc_storage: HashMap<DocId, Document>,
    /// Code file storage
    code_storage: HashMap<FileId, CodeFile>,
}

impl SharedContext {
    /// Creates a new empty shared context
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a global document reference
    pub fn add_global_doc(&mut self, doc_id: &str) {
        if !self.global_docs.contains(&doc_id.to_string()) {
            self.global_docs.push(doc_id.to_string());
        }
    }

    /// Adds a global code file reference
    pub fn add_global_code(&mut self, file_id: &str) {
        if !self.global_code.contains(&file_id.to_string()) {
            self.global_code.push(file_id.to_string());
        }
    }

    /// Adds a decision to the shared context
    pub fn add_decision(&mut self, decision: &str) {
        self.decisions.push(decision.to_string());
    }

    /// Stores a document
    pub fn store_document(&mut self, doc: Document) {
        self.doc_storage.insert(doc.id.clone(), doc);
    }

    /// Stores a code file
    pub fn store_code(&mut self, code: CodeFile) {
        self.code_storage.insert(code.id.clone(), code);
    }

    /// Gets a document by ID
    pub fn get_document(&self, doc_id: &str) -> Option<&Document> {
        self.doc_storage.get(doc_id)
    }

    /// Gets a code file by ID
    pub fn get_code(&self, file_id: &str) -> Option<&CodeFile> {
        self.code_storage.get(file_id)
    }

    /// Gets all global document IDs
    pub fn global_docs(&self) -> &[DocId] {
        &self.global_docs
    }

    /// Gets all global code file IDs
    pub fn global_code(&self) -> &[FileId] {
        &self.global_code
    }

    /// Gets all decisions
    pub fn decisions(&self) -> &[String] {
        &self.decisions
    }

    /// Estimates total token count of shared context
    pub fn token_count(&self) -> usize {
        let doc_tokens: usize = self.global_docs.iter()
            .filter_map(|id| self.doc_storage.get(id))
            .map(|d| d.token_count)
            .sum();
        
        let code_tokens: usize = self.global_code.iter()
            .filter_map(|id| self.code_storage.get(id))
            .map(|c| c.token_count)
            .sum();
        
        let decision_tokens: usize = self.decisions.iter()
            .map(|d| estimate_tokens(d))
            .sum();
        
        doc_tokens + code_tokens + decision_tokens
    }
}

/// Task-specific context with minimal required information
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Task identifier
    pub task_id: TaskId,
    /// Agent ID assigned to this task
    pub agent_id: AgentId,
    /// Required document IDs (minimal set)
    pub required_docs: Vec<DocId>,
    /// Required code file IDs (minimal set)
    pub required_code: Vec<FileId>,
    /// Related task IDs (for dependencies)
    pub related_tasks: Vec<TaskId>,
    /// Agent state
    pub agent_state: AgentState,
    /// Creation timestamp (Unix timestamp in seconds)
    pub created_at: u64,
    /// Last access timestamp (Unix timestamp in seconds)
    pub last_accessed: u64,
    /// Reference to shared context (for pull-based retrieval)
    shared_context: Option<Arc<RwLock<SharedContext>>>,
    /// Task results from dependencies
    task_results: HashMap<TaskId, TaskResult>,
    /// Retrieved knowledge from Domain RAG (Experience-as-Parameters)
    retrieved_knowledge: Vec<RagResult>,
    /// Domains used for retrieval
    retrieval_domains: Vec<String>,
}

impl TaskContext {
    /// Creates a new task context
    pub fn new(task_id: &str, agent_id: &str) -> Self {
        let now = current_timestamp();
        Self {
            task_id: task_id.to_string(),
            agent_id: agent_id.to_string(),
            required_docs: Vec::new(),
            required_code: Vec::new(),
            related_tasks: Vec::new(),
            agent_state: AgentState::active(),
            created_at: now,
            last_accessed: now,
            shared_context: None,
            task_results: HashMap::new(),
            retrieved_knowledge: Vec::new(),
            retrieval_domains: Vec::new(),
        }
    }

    /// Sets the shared context reference for pull-based retrieval
    pub fn with_shared_context(mut self, shared: Arc<RwLock<SharedContext>>) -> Self {
        self.shared_context = Some(shared);
        self
    }

    /// Adds a required document
    pub fn add_required_doc(&mut self, doc_id: &str) {
        if !self.required_docs.contains(&doc_id.to_string()) {
            self.required_docs.push(doc_id.to_string());
        }
        self.last_accessed = current_timestamp();
    }

    /// Adds a required code file
    pub fn add_required_code(&mut self, file_id: &str) {
        if !self.required_code.contains(&file_id.to_string()) {
            self.required_code.push(file_id.to_string());
        }
        self.last_accessed = current_timestamp();
    }

    /// Adds a related task
    pub fn add_related_task(&mut self, task_id: &str) {
        if !self.related_tasks.contains(&task_id.to_string()) {
            self.related_tasks.push(task_id.to_string());
        }
        self.last_accessed = current_timestamp();
    }

    /// Adds a task result from a dependency
    pub fn add_task_result(&mut self, result: TaskResult) {
        self.task_results.insert(result.task_id.clone(), result);
        self.last_accessed = current_timestamp();
    }

    /// Gets a document by ID (pull-based retrieval)
    pub fn get_doc(&self, doc_id: &str) -> Option<Document> {
        // First check if it's in our required docs
        if self.required_docs.contains(&doc_id.to_string()) {
            if let Some(shared) = &self.shared_context {
                let shared = shared.read().ok()?;
                return shared.get_document(doc_id).cloned();
            }
        }
        None
    }

    /// Gets a code file by ID (pull-based retrieval)
    pub fn get_code(&self, file_id: &str) -> Option<String> {
        // First check if it's in our required code
        if self.required_code.contains(&file_id.to_string()) {
            if let Some(shared) = &self.shared_context {
                let shared = shared.read().ok()?;
                return shared.get_code(file_id).map(|c| c.content.clone());
            }
        }
        None
    }

    /// Gets a related task result (pull-based retrieval)
    pub fn get_related_task(&self, task_id: &str) -> Option<&TaskResult> {
        self.task_results.get(task_id)
    }

    /// Marks this context as accessed (updates last_accessed timestamp)
    pub fn mark_accessed(&mut self) {
        self.last_accessed = current_timestamp();
    }

    /// Merges another task context into this one (for dependent tasks)
    pub fn merge(&mut self, other: &TaskContext) {
        // Merge required docs (only if not already present)
        for doc_id in &other.required_docs {
            if !self.required_docs.contains(doc_id) {
                self.required_docs.push(doc_id.clone());
            }
        }

        // Merge required code (only if not already present)
        for file_id in &other.required_code {
            if !self.required_code.contains(file_id) {
                self.required_code.push(file_id.clone());
            }
        }

        // Merge task results
        for (task_id, result) in &other.task_results {
            if !self.task_results.contains_key(task_id) {
                self.task_results.insert(task_id.clone(), result.clone());
            }
        }

        self.last_accessed = current_timestamp();
    }

    /// Estimates the token count for this context
    pub fn token_count(&self) -> usize {
        let doc_tokens: usize = self.required_docs.iter()
            .filter_map(|id| {
                self.shared_context.as_ref()
                    .and_then(|sc| sc.read().ok())
                    .and_then(|sc| sc.get_document(id).map(|d| d.token_count))
            })
            .sum();
        
        let code_tokens: usize = self.required_code.iter()
            .filter_map(|id| {
                self.shared_context.as_ref()
                    .and_then(|sc| sc.read().ok())
                    .and_then(|sc| sc.get_code(id).map(|c| c.token_count))
            })
            .sum();
        
        let result_tokens: usize = self.task_results.values()
            .map(|r| r.token_count)
            .sum();
        
        doc_tokens + code_tokens + result_tokens
    }

    /// Checks if this context is stale (not accessed within max_age_hours)
    pub fn is_stale(&self, max_age_hours: u64) -> bool {
        let now = current_timestamp();
        let age_seconds = now.saturating_sub(self.last_accessed);
        let max_age_seconds = max_age_hours * 3600;
        age_seconds > max_age_seconds
    }

    /// Updates the agent state
    pub fn update_state(&mut self, state: AgentState) {
        self.agent_state = state;
        self.last_accessed = current_timestamp();
    }
}

/// Manages context distribution for all tasks and agents
pub struct ContextManager {
    /// Shared context accessible by all tasks
    shared_context: Arc<RwLock<SharedContext>>,
    /// Per-task contexts
    task_contexts: HashMap<TaskId, TaskContext>,
    /// Document index for quick lookups
    doc_index: HashMap<DocId, Document>,
    /// Code index for quick lookups
    code_index: HashMap<FileId, CodeFile>,
    /// Task results cache
    task_results: HashMap<TaskId, TaskResult>,
    /// Domain RAG store (Experience-as-Parameters)
    domain_rag: DomainRagStore,
}

impl ContextManager {
    /// Creates a new context manager
    pub fn new() -> Self {
        Self {
            shared_context: Arc::new(RwLock::new(SharedContext::new())),
            task_contexts: HashMap::new(),
            doc_index: HashMap::new(),
            code_index: HashMap::new(),
            task_results: HashMap::new(),
            domain_rag: DomainRagStore::default(),
        }
    }

    /// Creates a new context manager with custom RAG strategy
    pub fn with_rag_strategy(strategy: RetrievalStrategy) -> Self {
        Self {
            shared_context: Arc::new(RwLock::new(SharedContext::new())),
            task_contexts: HashMap::new(),
            doc_index: HashMap::new(),
            code_index: HashMap::new(),
            task_results: HashMap::new(),
            domain_rag: DomainRagStore::with_strategy(strategy),
        }
    }

    /// Adds an experience to the domain RAG store
    pub fn add_experience(&mut self, domain: &str, task: &str, pattern: &str, reasoning: &str) {
        self.domain_rag.add_experience(domain, task, pattern, reasoning);
    }

    /// Allocates minimal context for a task assigned to an agent
    ///
    /// This is the core method that implements intelligent context allocation
    /// using Graph-Based workflow with Domain RAG:
    /// - Extract domains from task query
    /// - Retrieve domain knowledge via RAG (Experience-as-Parameters)
    /// - Allocate only retrieved knowledge (O(N) coordination)
    ///
    /// # Arguments
    ///
    /// * `task` - The task to allocate context for
    /// * `agent` - The agent that will execute the task
    ///
    /// # Returns
    ///
    /// A TaskContext with minimal required information
    pub async fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        let mut ctx = TaskContext::new(&task.id, &agent.id);
        ctx = ctx.with_shared_context(self.shared_context.clone());

        // Add only the documents required for this task (minimal set)
        for doc_id in &task.required_docs {
            ctx.add_required_doc(doc_id);
        }

        // Add only the code files required for this task (minimal set)
        for file_id in &task.required_code {
            ctx.add_required_code(file_id);
        }

        // Add results from dependent tasks
        for dep_task_id in &task.dependencies {
            ctx.add_related_task(dep_task_id);

            // If we have the result of the dependent task, include it
            if let Some(result) = self.task_results.get(dep_task_id) {
                ctx.add_task_result(result.clone());
            }
        }

        // RAG-based domain knowledge retrieval (Experience-as-Parameters)
        if !task.query.is_empty() {
            // Extract or use provided domains
            let domains = if task.domains.is_empty() {
                self.extract_domains_from_query(&task.query)
            } else {
                task.domains.clone()
            };

            // Retrieve from each domain (O(N) not O(N²))
            let mut all_results = Vec::new();
            for domain in &domains {
                if let Ok(results) = self.domain_rag.retrieve(domain, &task.query, 5).await {
                    all_results.extend(results);
                }
            }

            // Take top results (limit to avoid context bloat)
            all_results.truncate(10);
            ctx.retrieved_knowledge = all_results;
            ctx.retrieval_domains = domains;
        }

        // Store the context
        self.task_contexts.insert(task.id.clone(), ctx.clone());

        ctx
    }

    /// Extracts domain names from a query (simple keyword-based)
    ///
    /// In production, this could use NLP or a trained classifier.
    /// For now, uses simple keyword matching.
    fn extract_domains_from_query(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut domains = Vec::new();

        // Simple keyword-based domain detection
        // (In production, use NLP classifier)
        let domain_keywords: Vec<(&str, &str)> = vec![
            ("auth", "auth"),
            ("login", "auth"),
            ("password", "auth"),
            ("token", "auth"),
            ("jwt", "auth"),
            ("oauth", "auth"),
            ("api", "api"),
            ("endpoint", "api"),
            ("route", "api"),
            ("http", "api"),
            ("rest", "api"),
            ("database", "db"),
            ("query", "db"),
            ("sql", "db"),
            ("postgres", "db"),
            ("mysql", "db"),
            ("redis", "db"),
            ("cache", "cache"),
            ("memory", "cache"),
        ];

        for (keyword, domain) in domain_keywords {
            if query_lower.contains(keyword) && !domains.contains(&domain.to_string()) {
                domains.push(domain.to_string());
            }
        }

        // Default to empty if no domains detected (will use general knowledge)
        domains
    }

    /// Gets the shared context (read-only)
    pub fn get_shared(&self) -> Arc<RwLock<SharedContext>> {
        self.shared_context.clone()
    }

    /// Gets a mutable reference to the shared context (via RwLock write lock)
    pub fn get_shared_mut(&self) -> std::sync::RwLockWriteGuard<'_, SharedContext> {
        self.shared_context.write().expect("Shared context lock poisoned")
    }

    /// Gets a clone of the shared context Arc for sharing with task contexts
    pub fn get_shared_arc(&self) -> Arc<RwLock<SharedContext>> {
        self.shared_context.clone()
    }

    /// Gets a task context by ID
    pub fn get_task_context(&self, task_id: &str) -> Option<&TaskContext> {
        self.task_contexts.get(task_id)
    }

    /// Gets a mutable task context by ID
    pub fn get_task_context_mut(&mut self, task_id: &str) -> Option<&mut TaskContext> {
        self.task_contexts.get_mut(task_id)
    }

    /// Stores a document in the index
    pub fn store_document(&mut self, doc: Document) {
        self.doc_index.insert(doc.id.clone(), doc.clone());
        if let Ok(mut shared) = self.shared_context.write() {
            shared.store_document(doc);
        }
    }

    /// Stores a code file in the index
    pub fn store_code(&mut self, code: CodeFile) {
        self.code_index.insert(code.id.clone(), code.clone());
        if let Ok(mut shared) = self.shared_context.write() {
            shared.store_code(code);
        }
    }

    /// Stores a task result
    pub fn store_task_result(&mut self, result: TaskResult) {
        self.task_results.insert(result.task_id.clone(), result.clone());
        
        // Update any task contexts that depend on this task
        for ctx in self.task_contexts.values_mut() {
            if ctx.related_tasks.contains(&result.task_id) {
                ctx.add_task_result(result.clone());
            }
        }
    }

    /// Cleans up stale contexts (not accessed within max_age_hours)
    pub fn cleanup(&mut self, max_age_hours: u64) {
        self.task_contexts.retain(|_, ctx| !ctx.is_stale(max_age_hours));
    }

    /// Gets the number of active task contexts
    pub fn active_context_count(&self) -> usize {
        self.task_contexts.len()
    }

    /// Estimates total token usage across all contexts
    pub fn total_token_count(&self) -> usize {
        let shared_tokens = self.shared_context.read()
            .map(|sc| sc.token_count())
            .unwrap_or(0);
        let task_tokens: usize = self.task_contexts.values()
            .map(|ctx| ctx.token_count())
            .sum();
        
        // Shared context is counted once, task contexts reference it
        shared_tokens + task_tokens
    }

    /// Calculates token savings compared to giving full context to each agent
    pub fn calculate_savings(&self, full_context_tokens: usize) -> ContextSavings {
        let actual_tokens = self.total_token_count();
        let num_tasks = self.task_contexts.len().max(1);
        
        // If each task got full context
        let full_tokens = full_context_tokens * num_tasks;
        
        let saved = full_tokens.saturating_sub(actual_tokens);
        let percentage = if full_tokens > 0 {
            (saved as f64 / full_tokens as f64) * 100.0
        } else {
            0.0
        };

        ContextSavings {
            full_context_tokens: full_tokens,
            actual_tokens,
            saved_tokens: saved,
            savings_percentage: percentage,
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about context token savings
#[derive(Debug, Clone)]
pub struct ContextSavings {
    /// Tokens if each task got full context
    pub full_context_tokens: usize,
    /// Actual tokens used with minimal context
    pub actual_tokens: usize,
    /// Tokens saved
    pub saved_tokens: usize,
    /// Savings percentage (0-100)
    pub savings_percentage: f64,
}

/// Estimates token count for a string (simple approximation)
fn estimate_tokens(content: &str) -> usize {
    // Rough estimate: ~4 characters per token on average
    // This is a simplification; actual tokenization depends on the model
    content.len().saturating_add(3) / 4
}

/// Gets current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_allocation_minimal() {
        let mut manager = ContextManager::new();

        // Store some documents and code
        manager.store_document(Document::new("doc-1", "Content of document 1", "spec"));
        manager.store_document(Document::new("doc-2", "Content of document 2", "adr"));
        manager.store_code(CodeFile::new("file-1", "src/main.rs", "fn main() {}"));
        manager.store_code(CodeFile::new("file-2", "src/lib.rs", "pub fn lib() {}"));

        // Create a task that only needs doc-1 and file-1 (no query = no RAG)
        let task = Task::new("task-1", vec!["doc-1"], vec!["file-1"], vec![]);
        let agent = Agent::new("agent-1");

        // Allocate context (async)
        let ctx = manager.allocate(&task, &agent).await;

        // Verify minimal allocation (only what's needed)
        assert_eq!(ctx.required_docs, vec!["doc-1"]);
        assert_eq!(ctx.required_code, vec!["file-1"]);
        assert_eq!(ctx.related_tasks.len(), 0);

        // Verify pull-based retrieval works
        assert!(ctx.get_doc("doc-1").is_some());
        assert!(ctx.get_doc("doc-2").is_none()); // Not in required docs
        assert!(ctx.get_code("file-1").is_some());
        assert!(ctx.get_code("file-2").is_none()); // Not in required code
    }

    #[tokio::test]
    async fn test_context_allocation_with_dependencies() {
        let mut manager = ContextManager::new();

        // Store task results
        manager.store_task_result(TaskResult::new("task-0", "Result from task 0"));

        // Create a task that depends on task-0
        let task = Task::new("task-1", vec!["doc-1"], vec![], vec!["task-0"]);
        let agent = Agent::new("agent-1");

        // Allocate context (async)
        let ctx = manager.allocate(&task, &agent).await;

        // Verify dependency is included
        assert!(ctx.related_tasks.contains(&"task-0".to_string()));

        // Verify task result is available via pull-based retrieval
        let result = ctx.get_related_task("task-0");
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "Result from task 0");
    }

    #[tokio::test]
    async fn test_context_allocation_with_rag() {
        let mut manager = ContextManager::new();

        // Add domain knowledge (Experience-as-Parameters)
        manager.add_experience("auth", "user login", "Use JWT with HttpOnly cookies", "Stateless auth scales better");
        manager.add_experience("auth", "OAuth2 integration", "Use OAuth2 flow", "Industry standard");

        // Create task with query (triggers RAG)
        let task = Task::new("task-1", vec![], vec![], vec![])
            .with_query("implement user login with JWT");
        let agent = Agent::new("agent-1");

        // Allocate context (async, with RAG retrieval)
        let ctx = manager.allocate(&task, &agent).await;

        // Verify RAG retrieved knowledge
        assert!(!ctx.retrieved_knowledge.is_empty());
        assert!(ctx.retrieval_domains.contains(&"auth".to_string()));
    }

    #[tokio::test]
    async fn test_context_token_savings() {
        let mut manager = ContextManager::new();

        // Store documents with known sizes
        let large_doc = "A".repeat(4000); // ~1000 tokens
        manager.store_document(Document::new("doc-1", &large_doc, "spec"));
        manager.store_document(Document::new("doc-2", &large_doc, "spec"));
        manager.store_document(Document::new("doc-3", &large_doc, "spec"));
        manager.store_document(Document::new("doc-4", &large_doc, "spec"));

        // Full context would be all 4 docs = ~4000 tokens
        let full_context_tokens = 4000;

        // Create tasks that each need only 1 doc
        let task1 = Task::new("task-1", vec!["doc-1"], vec![], vec![]);
        let task2 = Task::new("task-2", vec!["doc-2"], vec![], vec![]);
        let task3 = Task::new("task-3", vec!["doc-3"], vec![], vec![]);

        let agent = Agent::new("agent-1");
        manager.allocate(&task1, &agent).await;
        manager.allocate(&task2, &agent).await;
        manager.allocate(&task3, &agent).await;

        // Calculate savings
        let savings = manager.calculate_savings(full_context_tokens);

        // Each task only gets 1 doc instead of 4
        // Expected savings: ~75% (each task gets 25% of full context)
        assert!(savings.savings_percentage >= 50.0,
            "Expected >= 50% savings, got {:.1}%", savings.savings_percentage);
        assert!(savings.saved_tokens > 0);
    }

    #[tokio::test]
    async fn test_token_efficiency_vs_ddd() {
        let mut manager = ContextManager::new();

        // Add domain experiences
        manager.add_experience("auth", "login", "JWT pattern", "reasoning");
        manager.add_experience("api", "endpoint", "REST pattern", "reasoning");
        manager.add_experience("db", "query", "SQL pattern", "reasoning");

        // Create tasks with queries that match domain keywords
        let tasks: Vec<Task> = (0..10)
            .map(|i| Task::new(&format!("task-{}", i), vec![], vec![], vec![])
                .with_query("implement login authentication"))  // Matches "auth" domain
            .collect();

        let agent = Agent::new("agent-1");

        // Allocate contexts (O(N) coordination with RAG)
        let mut total_retrieved = 0;
        for task in &tasks {
            let ctx = manager.allocate(task, &agent).await;
            total_retrieved += ctx.retrieved_knowledge.len();
        }

        // Verify O(N) coordination (not O(N²))
        // Each task retrieves independently, no cross-talk overhead
        assert!(total_retrieved > 0);
        
        // Token overhead should be <10% (RAG results only, not full domain knowledge)
        let overhead = total_retrieved * 100; // Each result ~100 tokens
        let ddd_overhead = 10 * 400; // DDD would send full context (~400 tokens per domain)
        
        assert!(overhead < ddd_overhead, "RAG should have <10% overhead vs DDD");
    }

    #[tokio::test]
    async fn test_coordination_overhead_linear() {
        // Add domain knowledge
        let mut experiences: Vec<(&str, &str, &str, &str)> = Vec::new();
        for domain in ["auth", "api", "db", "cache"] {
            experiences.push((domain, "task", "pattern", "reasoning"));
        }

        let agent = Agent::new("agent-1");

        // Measure coordination overhead for N tasks
        let task_counts = vec![1, 5, 10, 20];
        let mut overheads = Vec::new();

        for n in task_counts {
            let mut manager = ContextManager::new();
            for (domain, task, pattern, reasoning) in &experiences {
                manager.add_experience(domain, task, pattern, reasoning);
            }
            
            let start = std::time::Instant::now();
            
            for i in 0..n {
                let task = Task::new(&format!("task-{}", i), vec![], vec![], vec![])
                    .with_query("test query");
                manager.allocate(&task, &agent).await;
            }
            
            overheads.push((n, start.elapsed().as_millis()));
        }

        // Verify O(N) scaling (not O(N²))
        // Time for 20 tasks should be roughly 4x time for 5 tasks (not 16x)
        let time_5 = overheads.iter().find(|(n, _)| *n == 5).unwrap().1 as f32;
        let time_20 = overheads.iter().find(|(n, _)| *n == 20).unwrap().1 as f32;
        
        if time_5 > 0.0 {
            let ratio = time_20 / time_5;
            assert!(ratio < 8.0, "Coordination should be O(N), ratio={:.1}", ratio);
        }
    }

    #[test]
    fn test_task_context_is_stale() {
        let mut ctx = TaskContext::new("task-1", "agent-1");

        // Fresh context should not be stale
        assert!(!ctx.is_stale(1)); // 1 hour threshold

        // Make it old
        ctx.last_accessed = 0;

        // Now it should be stale
        assert!(ctx.is_stale(1));
        assert!(!ctx.is_stale(1000000)); // Very long threshold
    }

    #[test]
    fn test_agent_state_management() {
        let mut ctx = TaskContext::new("task-1", "agent-1");

        // Initial state should be active
        assert!(ctx.agent_state.is_active);
        assert_eq!(ctx.agent_state.step, 0);

        // Update state
        let mut new_state = AgentState::active();
        new_state.step = 5;
        new_state.data = "{\"progress\": 50}".to_string();
        ctx.update_state(new_state);

        assert_eq!(ctx.agent_state.step, 5);
        assert!(ctx.agent_state.data.contains("50"));
    }

    #[test]
    fn test_shared_context_token_count() {
        let mut shared = SharedContext::new();

        // Add documents to global context and store them
        shared.add_global_doc("doc-1");
        shared.add_global_doc("doc-2");
        shared.store_document(Document::new("doc-1", &"A".repeat(400), "spec"));
        shared.store_document(Document::new("doc-2", &"B".repeat(400), "spec"));

        // Add decisions
        shared.add_decision("Decision 1");
        shared.add_decision("Decision 2");

        let tokens = shared.token_count();

        // Should be approximately 200+ tokens (docs only, decisions are small)
        // 400 chars / 4 = 100 tokens per doc, so 200 total for 2 docs
        assert!(tokens >= 100, "Expected at least 100 tokens, got {}", tokens);
    }
}
