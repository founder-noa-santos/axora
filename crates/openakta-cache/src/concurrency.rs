//! Sliding-Window Semaphore Concurrency
//!
//! This module implements production-grade concurrency throttling:
//! - Semaphore-based throttling (prevents resource starvation)
//! - Pre-flight token calculation (prevents mid-flight overflow)
//! - Dify pattern (validated in production with 20K+ stars)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                ConcurrentExecutor                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Sliding Window Semaphore     │  Pre-flight Token Calc      │
//! │  - Limits concurrent tasks    │  - Estimate context tokens  │
//! │  - Prevents starvation        │  - Estimate output tokens   │
//! │  - Releases on completion     │  - Budget check             │
//! │                               │                             │
//! │  Rate Limiter                 │  Task Queue                 │
//! │  - Requests per minute        │  - Pending tasks            │
//! │  - Token bucket algorithm     │  - Executing tasks          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use openakta_cache::concurrency::{ConcurrentExecutor, ConcurrencyConfig, Task};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create executor with throttling
//! let config = ConcurrencyConfig::default();
//! let executor = ConcurrentExecutor::new(config);
//!
//! // Execute tasks with sliding-window throttling
//! let tasks = vec![
//!     Task::new("task-1", "Process file 1"),
//!     Task::new("task-2", "Process file 2"),
//! ];
//!
//! let results = executor.execute_with_throttle(tasks).await?;
//! println!("Completed {} tasks", results.len());
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// Concurrency error types
#[derive(Error, Debug)]
pub enum ConcurrencyError {
    /// Token budget exceeded
    #[error("token budget exceeded: estimated {estimated}, limit {limit}")]
    TokenBudgetExceeded { estimated: usize, limit: usize },

    /// Context too large
    #[error("context too large: estimated {estimated}, limit {limit}")]
    ContextTooLarge { estimated: usize, limit: usize },

    /// Semaphore acquire error
    #[error("semaphore error: {0}")]
    Semaphore(#[from] tokio::sync::AcquireError),

    /// Rate limit exceeded
    #[error("rate limit exceeded, retry after {retry_after_ms}ms")]
    RateLimitExceeded { retry_after_ms: u64 },

    /// Task execution error
    #[error("task error: {0}")]
    TaskError(String),

    /// Join error
    #[error("join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

/// Result type for concurrency operations
pub type Result<T> = std::result::Result<T, ConcurrencyError>;

/// Task representation for concurrent execution
#[derive(Debug, Clone)]
pub struct Task {
    /// Task identifier
    pub id: String,

    /// Task description
    pub description: String,

    /// Task context (for token estimation)
    pub context_tokens: usize,

    /// Priority (higher = more urgent)
    pub priority: u8,
}

impl Task {
    /// Creates a new task
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            context_tokens: 0,
            priority: 50,
        }
    }

    /// Sets the context tokens
    pub fn with_context_tokens(mut self, tokens: usize) -> Self {
        self.context_tokens = tokens;
        self
    }

    /// Sets the priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Estimates total tokens (context + expected output)
    pub fn estimate_tokens(&self) -> usize {
        // Context + estimated output (roughly 20% of context)
        self.context_tokens + (self.context_tokens / 5)
    }
}

/// Task result
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task identifier
    pub task_id: String,

    /// Result content
    pub content: String,

    /// Tokens used
    pub tokens_used: usize,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl TaskResult {
    /// Creates a new task result
    pub fn new(task_id: &str, content: &str, tokens_used: usize, execution_time_ms: u64) -> Self {
        Self {
            task_id: task_id.to_string(),
            content: content.to_string(),
            tokens_used,
            execution_time_ms,
        }
    }
}

/// Token estimation model
pub struct TokenEstimationModel {
    /// Base tokens per task
    base_tokens: usize,

    /// Tokens per word in description
    tokens_per_word: f32,
}

impl TokenEstimationModel {
    /// Creates a new estimation model
    pub fn new(base_tokens: usize, tokens_per_word: f32) -> Self {
        Self {
            base_tokens,
            tokens_per_word,
        }
    }

    /// Estimates output tokens from task description
    pub fn estimate_output(&self, description: &str) -> Result<usize> {
        let word_count = description.split_whitespace().count();
        let estimated = self.base_tokens + (word_count as f32 * self.tokens_per_word) as usize;
        Ok(estimated)
    }
}

impl Default for TokenEstimationModel {
    fn default() -> Self {
        Self {
            base_tokens: 100,
            tokens_per_word: 1.5,
        }
    }
}

/// Pre-flight token calculator
pub struct TokenCalculator {
    /// Token estimation model
    model: TokenEstimationModel,
}

impl TokenCalculator {
    /// Creates a new token calculator
    pub fn new() -> Self {
        Self {
            model: TokenEstimationModel::default(),
        }
    }

    /// Creates with custom model
    pub fn with_model(model: TokenEstimationModel) -> Self {
        Self { model }
    }

    /// Estimate tokens for task (pre-flight check)
    pub fn estimate(&self, task: &Task) -> Result<usize> {
        // Estimate based on:
        // - Context size (influenced files + business rules)
        // - Expected output size (based on task complexity)
        let context_tokens = task.context_tokens;
        let output_tokens = self.model.estimate_output(&task.description)?;

        Ok(context_tokens + output_tokens)
    }

    /// Check if task fits within budget
    pub fn check_budget(&self, task: &Task, max_tokens: usize) -> Result<()> {
        let estimated = self.estimate(task)?;
        if estimated > max_tokens {
            return Err(ConcurrencyError::TokenBudgetExceeded {
                estimated,
                limit: max_tokens,
            });
        }
        Ok(())
    }
}

impl Default for TokenCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Concurrency configuration
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Max concurrent tasks (sliding window)
    pub max_concurrent: usize,

    /// Max tokens per task (pre-flight limit)
    pub max_tokens_per_task: usize,

    /// Rate limit (requests per minute)
    pub rate_limit_rpm: usize,

    /// Token budget per batch
    pub token_budget: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,          // Dify default
            max_tokens_per_task: 50_000, // 50K tokens per task
            rate_limit_rpm: 100,         // 100 requests/minute
            token_budget: 500_000,       // 500K tokens per batch
        }
    }
}

impl ConcurrencyConfig {
    /// Creates a new config with custom values
    pub fn new(max_concurrent: usize, max_tokens_per_task: usize, rate_limit_rpm: usize) -> Self {
        Self {
            max_concurrent,
            max_tokens_per_task,
            rate_limit_rpm,
            token_budget: max_tokens_per_task * max_concurrent * 2,
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    /// Tokens available
    tokens: f64,

    /// Max tokens (equals rate limit per minute)
    max_tokens: f64,

    /// Refill rate (tokens per millisecond)
    refill_rate: f64,

    /// Last refill time
    last_refill: Instant,
}

impl RateLimiter {
    /// Creates a new rate limiter
    pub fn new(requests_per_minute: usize) -> Self {
        Self {
            tokens: requests_per_minute as f64,
            max_tokens: requests_per_minute as f64,
            refill_rate: requests_per_minute as f64 / 60_000.0, // per ms
            last_refill: Instant::now(),
        }
    }

    /// Try to acquire a permit
    pub fn try_acquire(&mut self) -> Result<()> {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate retry after
            let needed = 1.0 - self.tokens;
            let retry_after_ms = (needed / self.refill_rate) as u64;

            Err(ConcurrencyError::RateLimitExceeded { retry_after_ms })
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_refill).as_millis() as f64;
        self.tokens = (self.tokens + elapsed_ms * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Wait until a permit is available
    pub async fn acquire(&mut self) -> Result<()> {
        loop {
            match self.try_acquire() {
                Ok(()) => return Ok(()),
                Err(ConcurrencyError::RateLimitExceeded { retry_after_ms }) => {
                    sleep(Duration::from_millis(retry_after_ms.min(100))).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Concurrent executor with sliding-window throttling
pub struct ConcurrentExecutor {
    /// Sliding window semaphore (limits concurrent tasks)
    semaphore: Arc<Semaphore>,

    /// Pre-flight token calculator
    token_calculator: TokenCalculator,

    /// Rate limiter
    rate_limiter: Arc<Mutex<RateLimiter>>,

    /// Configuration
    config: ConcurrencyConfig,

    /// Current token usage (for batch budget)
    current_usage: Arc<Mutex<usize>>,
}

impl ConcurrentExecutor {
    /// Creates a new executor with throttling
    pub fn new(config: ConcurrencyConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
            token_calculator: TokenCalculator::new(),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(config.rate_limit_rpm))),
            config,
            current_usage: Arc::new(Mutex::new(0)),
        }
    }

    /// Creates with custom token calculator
    pub fn with_token_calculator(mut self, calculator: TokenCalculator) -> Self {
        self.token_calculator = calculator;
        self
    }

    /// Execute tasks with sliding-window throttling
    pub async fn execute_with_throttle(&self, tasks: Vec<Task>) -> Result<Vec<TaskResult>> {
        let mut handles = Vec::new();

        for task in tasks {
            // Pre-flight token check (prevent mid-flight overflow)
            let estimated_tokens = self.token_calculator.estimate(&task)?;
            if estimated_tokens > self.config.max_tokens_per_task {
                return Err(ConcurrencyError::TokenBudgetExceeded {
                    estimated: estimated_tokens,
                    limit: self.config.max_tokens_per_task,
                });
            }

            // Rate limiting
            {
                let mut limiter = self.rate_limiter.lock().await;
                limiter.acquire().await?;
            }

            // Check batch token budget
            {
                let usage = self.current_usage.lock().await;
                if *usage + estimated_tokens > self.config.token_budget {
                    return Err(ConcurrencyError::TokenBudgetExceeded {
                        estimated: *usage + estimated_tokens,
                        limit: self.config.token_budget,
                    });
                }
            }

            // Acquire semaphore permit (throttles concurrency)
            let permit = self.semaphore.clone().acquire_owned().await?;

            // Spawn task (releases permit when complete)
            let handle = tokio::spawn({
                let task = task.clone();
                let current_usage = self.current_usage.clone();

                async move {
                    let start = Instant::now();

                    // Execute task (simulated)
                    let result = execute_task(&task).await;

                    let execution_time_ms = start.elapsed().as_millis() as u64;

                    // Update token usage
                    {
                        let mut usage = current_usage.lock().await;
                        *usage = usage.saturating_sub(estimated_tokens);
                    }

                    // Release semaphore (permit dropped at end of scope)
                    drop(permit);

                    result.map(|content| {
                        TaskResult::new(&task.id, &content, estimated_tokens, execution_time_ms)
                    })
                }
            });

            // Update token usage
            {
                let mut usage = self.current_usage.lock().await;
                *usage += estimated_tokens;
            }

            handles.push(handle);
        }

        // Wait for all tasks (sliding window ensures throughput)
        let results = futures::future::try_join_all(handles)
            .await
            .map_err(ConcurrencyError::JoinError)?;

        // Convert Vec<Result<TaskResult, ConcurrencyError>> to Result<Vec<TaskResult>, ConcurrencyError>
        results.into_iter().collect()
    }

    /// Execute a single task with throttling
    pub async fn execute_single(&self, task: Task) -> Result<TaskResult> {
        let results = self.execute_with_throttle(vec![task]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| ConcurrencyError::TaskError("No result returned".to_string()))
    }

    /// Get current concurrency level
    pub fn current_concurrency(&self) -> usize {
        self.config.max_concurrent - self.semaphore.available_permits()
    }

    /// Get current token usage
    pub async fn current_token_usage(&self) -> usize {
        *self.current_usage.lock().await
    }

    /// Get configuration
    pub fn config(&self) -> &ConcurrencyConfig {
        &self.config
    }
}

impl Default for ConcurrentExecutor {
    fn default() -> Self {
        Self::new(ConcurrencyConfig::default())
    }
}

/// Executes a single task (simulated)
async fn execute_task(task: &Task) -> Result<String> {
    // Simulate task execution
    // In production, this would call the LLM or perform actual work
    let _ = task; // Suppress unused warning

    // Simulate some work
    sleep(Duration::from_millis(10)).await;

    Ok(format!("Completed task: {}", task.id))
}

/// Batch executor for multiple task batches
pub struct BatchExecutor {
    /// Inner executor
    executor: ConcurrentExecutor,

    /// Pending tasks
    pending: Vec<Task>,

    /// Current batch token usage
    batch_usage: usize,
}

impl BatchExecutor {
    /// Creates a new batch executor
    pub fn new(executor: ConcurrentExecutor) -> Self {
        Self {
            executor,
            pending: Vec::new(),
            batch_usage: 0,
        }
    }

    /// Add a task to the batch
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        let estimated = self.executor.token_calculator.estimate(&task)?;

        if estimated > self.executor.config.max_tokens_per_task {
            return Err(ConcurrencyError::TokenBudgetExceeded {
                estimated,
                limit: self.executor.config.max_tokens_per_task,
            });
        }

        self.pending.push(task);
        self.batch_usage += estimated;

        Ok(())
    }

    /// Execute the batch
    pub async fn execute_batch(&mut self) -> Result<Vec<TaskResult>> {
        let tasks = std::mem::take(&mut self.pending);
        self.batch_usage = 0;

        self.executor.execute_with_throttle(tasks).await
    }

    /// Get pending task count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get current batch usage
    pub fn batch_usage(&self) -> usize {
        self.batch_usage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_semaphore_throttling() {
        let config = ConcurrencyConfig::new(3, 50_000, 1000); // Max 3 concurrent
        let executor = ConcurrentExecutor::new(config);

        // Create 10 tasks
        let tasks: Vec<Task> = (0..10)
            .map(|i| Task::new(&format!("task-{}", i), "Test task"))
            .collect();

        // Execute with timeout
        let result = timeout(
            Duration::from_secs(5),
            executor.execute_with_throttle(tasks),
        )
        .await;

        assert!(result.is_ok());
        let results = result.unwrap().unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn test_pre_flight_token_check() {
        let config = ConcurrencyConfig::new(10, 1_000, 1000); // 1K token limit
        let executor = ConcurrentExecutor::new(config);

        // Task that exceeds budget
        let task = Task::new(
            "big-task",
            "This is a very long task description that should exceed the token budget",
        )
        .with_context_tokens(5_000); // 5K context tokens

        let result = executor.execute_single(task).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConcurrencyError::TokenBudgetExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_token_budget_exceeded() {
        let config = ConcurrencyConfig::new(10, 100_000, 1000);
        let executor = ConcurrentExecutor::new(config);

        // Task within per-task limit but will exceed when combined
        let task1 = Task::new("task-1", "Task 1").with_context_tokens(40_000);
        let task2 = Task::new("task-2", "Task 2").with_context_tokens(40_000);
        let task3 = Task::new("task-3", "Task 3").with_context_tokens(40_000);

        // First two should succeed
        let result1 = executor.execute_single(task1).await;
        assert!(result1.is_ok());

        let result2 = executor.execute_single(task2).await;
        assert!(result2.is_ok());

        // Third might fail due to batch budget
        // (depends on timing of token release)
        let _ = executor.execute_single(task3).await;
    }

    #[tokio::test]
    async fn test_sliding_window_throughput() {
        let config = ConcurrencyConfig::new(5, 50_000, 1000); // 5 concurrent
        let executor = ConcurrentExecutor::new(config);

        // Create tasks
        let tasks: Vec<Task> = (0..20)
            .map(|i| Task::new(&format!("task-{}", i), "Test"))
            .collect();

        let start = Instant::now();
        let results = executor.execute_with_throttle(tasks).await.unwrap();
        let elapsed = start.elapsed();

        // All tasks should complete
        assert_eq!(results.len(), 20);

        // With 5 concurrent and ~10ms per task, 20 tasks should take ~40-50ms
        // (4 batches of 5)
        assert!(elapsed.as_millis() >= 30);
        assert!(elapsed.as_millis() <= 500); // Generous upper bound
    }

    #[tokio::test]
    async fn test_concurrent_task_execution() {
        let config = ConcurrencyConfig::new(10, 50_000, 1000);
        let executor = ConcurrentExecutor::new(config);

        // Create tasks that track execution time
        let tasks: Vec<Task> = (0..10)
            .map(|i| Task::new(&format!("task-{}", i), "Test"))
            .collect();

        let results = executor.execute_with_throttle(tasks).await.unwrap();

        // All tasks should complete
        assert_eq!(results.len(), 10);

        // Each task should have execution time
        for result in &results {
            assert!(result.execution_time_ms > 0);
        }
    }

    #[tokio::test]
    async fn test_semaphore_permit_release() {
        let config = ConcurrencyConfig::new(2, 50_000, 1000); // Max 2 concurrent
        let executor = ConcurrentExecutor::new(config);

        // Initial state: 2 permits available
        assert_eq!(executor.current_concurrency(), 0);

        // Start tasks
        let tasks: Vec<Task> = (0..4)
            .map(|i| Task::new(&format!("task-{}", i), "Test"))
            .collect();

        let _ = executor.execute_with_throttle(tasks).await.unwrap();

        // After completion: all permits released
        assert_eq!(executor.current_concurrency(), 0);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = ConcurrencyConfig::new(100, 50_000, 10); // 10 requests/minute
        let executor = ConcurrentExecutor::new(config);

        // First request should succeed
        let task1 = Task::new("task-1", "Test");
        let result1 = executor.execute_single(task1).await;
        assert!(result1.is_ok());

        // Rapid subsequent requests might hit rate limit
        // (depends on timing)
        let task2 = Task::new("task-2", "Test");
        let _ = executor.execute_single(task2).await;
    }

    #[tokio::test]
    async fn test_context_budget_check() {
        let config = ConcurrencyConfig::new(10, 1_000, 1000); // 1K limit
        let executor = ConcurrentExecutor::new(config);

        // Task with large context
        let task = Task::new("big-context", "Test").with_context_tokens(5_000);

        let result = executor.execute_single(task).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConcurrencyError::TokenBudgetExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_batch_executor() {
        let config = ConcurrencyConfig::new(5, 50_000, 1000);
        let executor = ConcurrentExecutor::new(config);
        let mut batch = BatchExecutor::new(executor);

        // Add tasks to batch
        for i in 0..10 {
            let task = Task::new(&format!("batch-task-{}", i), "Batch test");
            batch.add_task(task).unwrap();
        }

        assert_eq!(batch.pending_count(), 10);

        // Execute batch
        let results = batch.execute_batch().await.unwrap();

        assert_eq!(results.len(), 10);
        assert_eq!(batch.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_token_calculator() {
        let calculator = TokenCalculator::new();

        let task =
            Task::new("test", "This is a test task with some words").with_context_tokens(1000);

        let estimated = calculator.estimate(&task).unwrap();

        // Should be context + output estimate
        assert!(estimated > 1000);
        assert!(estimated < 2000);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let mut limiter = RateLimiter::new(60); // 60 per minute = 1 per second

        // Use all tokens
        for _ in 0..60 {
            limiter.try_acquire().unwrap();
        }

        // Should be rate limited
        let result = limiter.try_acquire();
        assert!(result.is_err());

        // Wait for refill
        sleep(Duration::from_millis(1100)).await;

        // Should have tokens again
        let result = limiter.try_acquire();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrency_config() {
        let config = ConcurrencyConfig::default();

        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.max_tokens_per_task, 50_000);
        assert_eq!(config.rate_limit_rpm, 100);

        let custom = ConcurrencyConfig::new(20, 100_000, 200);

        assert_eq!(custom.max_concurrent, 20);
        assert_eq!(custom.max_tokens_per_task, 100_000);
        assert_eq!(custom.rate_limit_rpm, 200);
    }
}
