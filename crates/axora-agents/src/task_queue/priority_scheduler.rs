//! Priority-based scheduling for ready task identifiers.

use std::collections::{BTreeMap, HashMap, VecDeque};

use thiserror::Error;

/// Scheduler-specific result type.
pub type Result<T> = std::result::Result<T, PrioritySchedulerError>;

/// Errors produced by [`PriorityScheduler`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrioritySchedulerError {
    /// Priority values must remain within the supported range.
    #[error("invalid priority: {0}")]
    InvalidPriority(u8),
}

/// Maintains a stable, priority-ordered queue of ready task identifiers.
#[derive(Debug, Clone, Default)]
pub struct PriorityScheduler {
    buckets: BTreeMap<u8, VecDeque<String>>,
    priorities: HashMap<String, u8>,
}

impl PriorityScheduler {
    /// Create an empty scheduler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a task with the provided priority.
    pub fn add_task(&mut self, task_id: String, priority: u8) -> Result<()> {
        validate_priority(priority)?;
        self.remove_task(&task_id);
        self.buckets
            .entry(priority)
            .or_default()
            .push_back(task_id.clone());
        self.priorities.insert(task_id, priority);
        Ok(())
    }

    /// Return the highest-priority ready task identifier.
    pub fn get_next_task(&mut self) -> Option<String> {
        let priority = *self.buckets.keys().next_back()?;
        let bucket = self.buckets.get_mut(&priority)?;
        let task_id = bucket.pop_front()?;
        if bucket.is_empty() {
            self.buckets.remove(&priority);
        }
        self.priorities.remove(&task_id);
        Some(task_id)
    }

    /// Rebuild queue ordering after external priority changes.
    pub fn reorder_queue(&mut self, priorities: &HashMap<String, u8>) -> Result<()> {
        let ordered = self
            .buckets
            .iter()
            .rev()
            .flat_map(|(_, bucket)| bucket.iter().cloned())
            .collect::<Vec<_>>();

        self.buckets.clear();
        self.priorities.clear();

        for task_id in ordered {
            let priority = priorities.get(&task_id).copied().unwrap_or(0);
            self.add_task(task_id, priority)?;
        }

        Ok(())
    }

    /// Remove a task from the scheduler if present.
    pub fn remove_task(&mut self, task_id: &str) -> Option<u8> {
        let priority = self.priorities.remove(task_id)?;
        if let Some(bucket) = self.buckets.get_mut(&priority) {
            if let Some(position) = bucket.iter().position(|queued| queued == task_id) {
                bucket.remove(position);
            }
            if bucket.is_empty() {
                self.buckets.remove(&priority);
            }
        }
        Some(priority)
    }

    /// Returns true when the task is currently queued.
    pub fn contains(&self, task_id: &str) -> bool {
        self.priorities.contains_key(task_id)
    }

    /// Returns the number of queued tasks.
    pub fn len(&self) -> usize {
        self.priorities.len()
    }

    /// Returns true when no tasks are queued.
    pub fn is_empty(&self) -> bool {
        self.priorities.is_empty()
    }
}

fn validate_priority(priority: u8) -> Result<()> {
    if priority > 100 {
        return Err(PrioritySchedulerError::InvalidPriority(priority));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_task_rejects_out_of_range_priority() {
        let mut scheduler = PriorityScheduler::new();
        let error = scheduler.add_task("task-a".to_string(), 101).unwrap_err();
        assert_eq!(error, PrioritySchedulerError::InvalidPriority(101));
    }

    #[test]
    fn get_next_task_returns_highest_priority_first() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.add_task("low".to_string(), 10).unwrap();
        scheduler.add_task("high".to_string(), 90).unwrap();

        assert_eq!(scheduler.get_next_task().as_deref(), Some("high"));
        assert_eq!(scheduler.get_next_task().as_deref(), Some("low"));
    }

    #[test]
    fn scheduler_preserves_fifo_within_priority_bucket() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.add_task("task-1".to_string(), 50).unwrap();
        scheduler.add_task("task-2".to_string(), 50).unwrap();

        assert_eq!(scheduler.get_next_task().as_deref(), Some("task-1"));
        assert_eq!(scheduler.get_next_task().as_deref(), Some("task-2"));
    }

    #[test]
    fn reorder_queue_applies_new_priorities() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.add_task("task-1".to_string(), 10).unwrap();
        scheduler.add_task("task-2".to_string(), 20).unwrap();
        let priorities = HashMap::from([
            ("task-1".to_string(), 80),
            ("task-2".to_string(), 20),
        ]);

        scheduler.reorder_queue(&priorities).unwrap();

        assert_eq!(scheduler.get_next_task().as_deref(), Some("task-1"));
    }

    #[test]
    fn add_task_updates_existing_priority() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.add_task("task-1".to_string(), 10).unwrap();
        scheduler.add_task("task-1".to_string(), 90).unwrap();
        scheduler.add_task("task-2".to_string(), 50).unwrap();

        assert_eq!(scheduler.get_next_task().as_deref(), Some("task-1"));
        assert_eq!(scheduler.get_next_task().as_deref(), Some("task-2"));
    }

    #[test]
    fn remove_task_deletes_existing_entry() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.add_task("task-1".to_string(), 10).unwrap();

        assert_eq!(scheduler.remove_task("task-1"), Some(10));
        assert!(scheduler.is_empty());
    }
}
