//! Progress Monitoring Module
//!
//! Real-time progress tracking with ETA calculation, blocker detection,
//! and user-facing status reports.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::Mutex;

use crate::coordinator::v2::BlackboardV2;

// Use a local TaskId alias to avoid conflicts
type LocalTaskId = usize;

/// Task status in the progress monitoring system
#[derive(Debug, Clone)]
pub enum TaskStatus {
    /// Task is waiting to be started
    Pending,
    /// Task is in progress with a progress percentage (0-100)
    InProgress { progress: f32 },
    /// Task completed successfully
    Completed,
    /// Task failed with an error message
    Failed(String),
    /// Task is blocked with a reason
    Blocked(String),
}

impl TaskStatus {
    /// Get progress percentage for this status
    pub fn progress_percentage(&self) -> f32 {
        match self {
            TaskStatus::Pending => 0.0,
            TaskStatus::InProgress { progress } => *progress,
            TaskStatus::Completed => 100.0,
            TaskStatus::Failed(..) => 0.0,
            TaskStatus::Blocked(..) => 0.0,
        }
    }

    /// Check if task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed(..))
    }
}

/// Progress information for a single task
#[derive(Debug, Clone)]
pub struct TaskProgress {
    pub task_id: LocalTaskId,
    pub status: TaskStatus,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub last_updated: Instant,
}

impl TaskProgress {
    pub fn new(task_id: LocalTaskId) -> Self {
        let now = Instant::now();
        Self {
            task_id,
            status: TaskStatus::Pending,
            started_at: None,
            completed_at: None,
            last_updated: now,
        }
    }

    pub fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.last_updated = Instant::now();

        if matches!(self.status, TaskStatus::InProgress { .. }) && self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }

        if self.status.is_terminal() && self.completed_at.is_none() {
            self.completed_at = Some(Instant::now());
        }
    }

    /// Get duration since last update
    pub fn time_since_update(&self) -> Duration {
        self.last_updated.elapsed()
    }
}

/// Information about a blocker
#[derive(Debug, Clone)]
pub struct BlockerInfo {
    pub task_id: LocalTaskId,
    pub reason: String,
    pub stalled_since: Duration,
}

/// Status report for a mission
#[derive(Debug, Clone)]
pub struct StatusReport {
    pub mission_id: String,
    pub progress_percentage: f32,
    pub eta: Duration,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub blocked_tasks: Vec<BlockerInfo>,
    pub elapsed_time: Duration,
    pub in_progress_tasks: usize,
    pub pending_tasks: usize,
}

impl StatusReport {
    pub fn new(mission_id: String) -> Self {
        Self {
            mission_id,
            progress_percentage: 0.0,
            eta: Duration::ZERO,
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            blocked_tasks: Vec::new(),
            elapsed_time: Duration::ZERO,
            in_progress_tasks: 0,
            pending_tasks: 0,
        }
    }
}

/// Tracks progress of tasks with ETA calculation
#[derive(Debug)]
pub struct ProgressTracker {
    tasks: DashMap<LocalTaskId, TaskProgress>,
    start_time: Instant,
    historical_velocity: Mutex<Vec<f32>>, // tasks per second
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            start_time: Instant::now(),
            historical_velocity: Mutex::new(Vec::with_capacity(10)),
        }
    }

    /// Track or update a task's status
    pub fn track_task(&self, task_id: LocalTaskId, status: TaskStatus) {
        let mut task = self
            .tasks
            .entry(task_id)
            .or_insert_with(|| TaskProgress::new(task_id));
        task.update_status(status.clone());

        // Update velocity if task completed
        if status.is_terminal() {
            if let Some(started_at) = task.started_at {
                let duration = task
                    .completed_at
                    .unwrap_or_else(Instant::now)
                    .duration_since(started_at);
                if duration.as_secs_f32() > 0.0 {
                    let velocity = 1.0 / duration.as_secs_f32();
                    let mut velocities = self.historical_velocity.lock();
                    velocities.push(velocity);
                    // Keep last 10 measurements
                    if velocities.len() > 10 {
                        velocities.remove(0);
                    }
                }
            }
        }
    }

    /// Get progress for a specific task
    pub fn get_task_progress(&self, task_id: LocalTaskId) -> Option<TaskProgress> {
        self.tasks.get(&task_id).map(|entry| entry.clone())
    }

    /// Get overall progress percentage (0-100)
    pub fn get_progress_percentage(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }

        let total_progress: f32 = self
            .tasks
            .iter()
            .map(|entry| entry.value().status.progress_percentage())
            .sum();

        total_progress / self.tasks.len() as f32
    }

    /// Calculate ETA based on historical velocity
    pub fn calculate_eta(&self) -> Option<Duration> {
        let velocities = self.historical_velocity.lock();

        if velocities.is_empty() {
            return None;
        }

        // Calculate average velocity
        let avg_velocity: f32 = velocities.iter().sum::<f32>() / velocities.len() as f32;

        if avg_velocity <= 0.0 {
            return None;
        }

        // Count remaining work
        let remaining_work: f32 = self
            .tasks
            .iter()
            .map(|entry| {
                let progress = entry.value().status.progress_percentage();
                (100.0 - progress) / 100.0
            })
            .sum();

        if remaining_work <= 0.0 {
            return Some(Duration::ZERO);
        }

        // ETA = remaining work / velocity
        let eta_secs = remaining_work / avg_velocity;
        Some(Duration::from_secs_f32(eta_secs))
    }

    /// Get all tasks
    pub fn get_all_tasks(&self) -> HashMap<LocalTaskId, TaskProgress> {
        self.tasks
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Get elapsed time since tracker started
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get task counts by status
    pub fn get_task_counts(&self) -> TaskCounts {
        let mut counts = TaskCounts::default();

        for entry in self.tasks.iter() {
            match &entry.value().status {
                TaskStatus::Pending => counts.pending += 1,
                TaskStatus::InProgress { .. } => counts.in_progress += 1,
                TaskStatus::Completed => counts.completed += 1,
                TaskStatus::Failed { .. } => counts.failed += 1,
                TaskStatus::Blocked { .. } => counts.blocked += 1,
            }
        }

        counts.total = self.tasks.len();
        counts
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Task counts by status
#[derive(Debug, Clone, Default)]
pub struct TaskCounts {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub blocked: usize,
}

/// Detects blocked/stalled tasks
#[derive(Debug, Clone)]
pub struct BlockerDetector {
    stall_threshold: Duration,
}

impl BlockerDetector {
    pub fn new(stall_threshold: Duration) -> Self {
        Self { stall_threshold }
    }

    /// Detect blockers in the given tasks
    pub fn detect_blockers(&self, tasks: &HashMap<LocalTaskId, TaskProgress>) -> Vec<BlockerInfo> {
        let mut blockers = Vec::new();

        for (task_id, task) in tasks {
            if let TaskStatus::InProgress { .. } = &task.status {
                let time_since_update = task.time_since_update();

                if time_since_update > self.stall_threshold {
                    blockers.push(BlockerInfo {
                        task_id: *task_id,
                        reason: format!(
                            "Task stalled for {:?}",
                            time_since_update.as_secs() / 60
                        ),
                        stalled_since: time_since_update,
                    });
                }
            }

            if let TaskStatus::Blocked(reason) = &task.status {
                blockers.push(BlockerInfo {
                    task_id: *task_id,
                    reason: reason.clone(),
                    stalled_since: task.time_since_update(),
                });
            }
        }

        blockers
    }

    /// Check if a specific task is blocked
    pub fn is_blocked(&self, task: &TaskProgress) -> bool {
        matches!(task.status, TaskStatus::Blocked(..))
            || (matches!(task.status, TaskStatus::InProgress { .. })
                && task.time_since_update() > self.stall_threshold)
    }
}

impl Default for BlockerDetector {
    fn default() -> Self {
        // Default: 5 minutes stall threshold
        Self::new(Duration::from_secs(300))
    }
}

/// Generates status reports
#[derive(Debug)]
pub struct StatusReporter {
    tracker: Arc<ProgressTracker>,
    detector: BlockerDetector,
    mission_id: String,
}

impl StatusReporter {
    pub fn new(tracker: Arc<ProgressTracker>, detector: BlockerDetector, mission_id: String) -> Self {
        Self {
            tracker,
            detector,
            mission_id,
        }
    }

    /// Generate a status report
    pub fn generate_report(&self) -> StatusReport {
        let tasks = self.tracker.get_all_tasks();
        let counts = self.tracker.get_task_counts();
        let blockers = self.detector.detect_blockers(&tasks);

        let mut report = StatusReport::new(self.mission_id.clone());
        report.progress_percentage = self.tracker.get_progress_percentage();
        report.eta = self.tracker.calculate_eta().unwrap_or(Duration::ZERO);
        report.total_tasks = counts.total;
        report.completed_tasks = counts.completed;
        report.failed_tasks = counts.failed;
        report.blocked_tasks = blockers;
        report.elapsed_time = self.tracker.elapsed_time();
        report.in_progress_tasks = counts.in_progress;
        report.pending_tasks = counts.pending;

        report
    }
}

/// Main progress monitor that coordinates all components
pub struct ProgressMonitor {
    tracker: Arc<ProgressTracker>,
    detector: BlockerDetector,
    reporter: Mutex<Option<StatusReporter>>,
    #[allow(dead_code)]
    blackboard: Arc<BlackboardV2>,
    mission_id: String,
}

impl ProgressMonitor {
    /// Create a new progress monitor
    pub fn new(blackboard: Arc<BlackboardV2>, mission_id: String) -> Self {
        let tracker = Arc::new(ProgressTracker::new());
        let detector = BlockerDetector::default();

        Self {
            tracker: tracker.clone(),
            detector: detector.clone(),
            reporter: Mutex::new(Some(StatusReporter::new(tracker, detector, mission_id.clone()))),
            blackboard,
            mission_id,
        }
    }

    /// Create with custom stall threshold
    pub fn with_stall_threshold(
        blackboard: Arc<BlackboardV2>,
        mission_id: String,
        stall_threshold: Duration,
    ) -> Self {
        let tracker = Arc::new(ProgressTracker::new());
        let detector = BlockerDetector::new(stall_threshold);

        Self {
            tracker: tracker.clone(),
            detector: detector.clone(),
            reporter: Mutex::new(Some(StatusReporter::new(tracker, detector, mission_id.clone()))),
            blackboard,
            mission_id,
        }
    }

    /// Update progress for a task
    pub fn update_progress(&self, task_id: LocalTaskId, status: TaskStatus) {
        self.tracker.track_task(task_id, status);
    }

    /// Get current status report
    pub fn get_report(&self) -> StatusReport {
        let reporter = self.reporter.lock();
        reporter
            .as_ref()
            .map(|r| r.generate_report())
            .unwrap_or_else(|| StatusReport::new(self.mission_id.clone()))
    }

    /// Get the tracker
    pub fn tracker(&self) -> Arc<ProgressTracker> {
        self.tracker.clone()
    }

    /// Get the mission ID
    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    /// Get elapsed time
    pub fn elapsed_time(&self) -> Duration {
        self.tracker.elapsed_time()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_progress_new() {
        let task = TaskProgress::new(1);
        assert_eq!(task.task_id, 1);
        assert!(matches!(task.status, TaskStatus::Pending));
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_progress_update_status() {
        let mut task = TaskProgress::new(1);

        // Update to in progress
        task.update_status(TaskStatus::InProgress { progress: 50.0 });
        assert!(matches!(task.status, TaskStatus::InProgress { .. }));
        assert!(task.started_at.is_some());

        // Update to completed
        task.update_status(TaskStatus::Completed);
        assert!(matches!(task.status, TaskStatus::Completed));
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_status_progress_percentage() {
        assert_eq!(TaskStatus::Pending.progress_percentage(), 0.0);
        assert_eq!(TaskStatus::Completed.progress_percentage(), 100.0);
        assert_eq!(
            TaskStatus::InProgress { progress: 75.0 }.progress_percentage(),
            75.0
        );
        assert_eq!(TaskStatus::Failed(String::new()).progress_percentage(), 0.0);
        assert_eq!(TaskStatus::Blocked(String::new()).progress_percentage(), 0.0);
    }

    #[test]
    fn test_progress_tracker_track_task() {
        let tracker = ProgressTracker::new();
        tracker.track_task(1, TaskStatus::InProgress { progress: 50.0 });

        let task = tracker.get_task_progress(1);
        assert!(task.is_some());
        assert!(matches!(task.unwrap().status, TaskStatus::InProgress { .. }));
    }

    #[test]
    fn test_progress_tracker_get_progress_percentage() {
        let tracker = ProgressTracker::new();

        // Empty tracker
        assert_eq!(tracker.get_progress_percentage(), 0.0);

        // Add tasks with different progress
        tracker.track_task(1, TaskStatus::Completed);
        tracker.track_task(2, TaskStatus::InProgress { progress: 50.0 });
        tracker.track_task(3, TaskStatus::Pending);

        // (100 + 50 + 0) / 3 = 50
        let progress = tracker.get_progress_percentage();
        assert!((progress - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_progress_tracker_calculate_eta() {
        let tracker = ProgressTracker::new();

        // No history
        assert!(tracker.calculate_eta().is_none());

        // Add completed tasks to build velocity history
        tracker.track_task(1, TaskStatus::InProgress { progress: 10.0 });
        std::thread::sleep(Duration::from_millis(100));
        tracker.track_task(1, TaskStatus::Completed);

        tracker.track_task(2, TaskStatus::InProgress { progress: 10.0 });
        std::thread::sleep(Duration::from_millis(100));
        tracker.track_task(2, TaskStatus::Completed);

        // Now we have velocity, should have ETA for remaining work
        tracker.track_task(3, TaskStatus::InProgress { progress: 50.0 });
        let eta = tracker.calculate_eta();
        assert!(eta.is_some());
    }

    #[test]
    fn test_blocker_detector_detect_stalled_task() {
        let detector = BlockerDetector::new(Duration::from_millis(100));
        let mut tasks = HashMap::new();

        // Create a stalled task
        let mut task = TaskProgress::new(1);
        task.update_status(TaskStatus::InProgress { progress: 50.0 });
        std::thread::sleep(Duration::from_millis(150));

        tasks.insert(1, task);

        let blockers = detector.detect_blockers(&tasks);
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].task_id, 1);
    }

    #[test]
    fn test_blocker_detector_detect_blocked_status() {
        let detector = BlockerDetector::default();
        let mut tasks = HashMap::new();

        let mut task = TaskProgress::new(1);
        task.update_status(TaskStatus::Blocked(
            "Waiting for dependency".to_string(),
        ));
        tasks.insert(1, task);

        let blockers = detector.detect_blockers(&tasks);
        assert_eq!(blockers.len(), 1);
        assert!(blockers[0].reason.contains("Waiting for dependency"));
    }

    #[test]
    fn test_status_reporter_generate_report() {
        let tracker = Arc::new(ProgressTracker::new());
        let detector = BlockerDetector::default();
        let reporter = StatusReporter::new(tracker.clone(), detector, "mission-1".to_string());

        // Add some tasks
        tracker.track_task(1, TaskStatus::Completed);
        tracker.track_task(2, TaskStatus::InProgress { progress: 50.0 });
        tracker.track_task(3, TaskStatus::Pending);

        let report = reporter.generate_report();

        assert_eq!(report.mission_id, "mission-1");
        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.completed_tasks, 1);
        assert_eq!(report.in_progress_tasks, 1);
        assert_eq!(report.pending_tasks, 1);
        assert!(report.progress_percentage > 0.0);
    }

    #[test]
    fn test_progress_monitor_update_and_report() {
        let blackboard = Arc::new(BlackboardV2::default());
        let monitor = ProgressMonitor::new(blackboard, "mission-1".to_string());

        // Update progress
        monitor.update_progress(1, TaskStatus::InProgress { progress: 25.0 });
        monitor.update_progress(2, TaskStatus::Pending);

        // Get report
        let report = monitor.get_report();

        assert_eq!(report.mission_id, "mission-1");
        assert_eq!(report.total_tasks, 2);
        assert!(report.progress_percentage > 0.0);
    }

    #[test]
    fn test_task_counts() {
        let tracker = ProgressTracker::new();

        tracker.track_task(1, TaskStatus::Completed);
        tracker.track_task(2, TaskStatus::InProgress { progress: 50.0 });
        tracker.track_task(3, TaskStatus::Failed("error".to_string()));
        tracker.track_task(4, TaskStatus::Blocked("blocked".to_string()));
        tracker.track_task(5, TaskStatus::Pending);

        let counts = tracker.get_task_counts();

        assert_eq!(counts.total, 5);
        assert_eq!(counts.completed, 1);
        assert_eq!(counts.in_progress, 1);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.blocked, 1);
        assert_eq!(counts.pending, 1);
    }

    #[test]
    fn test_eta_calculation_with_velocity() {
        let tracker = ProgressTracker::new();

        // Complete a task quickly
        tracker.track_task(1, TaskStatus::InProgress { progress: 10.0 });
        std::thread::sleep(Duration::from_millis(50));
        tracker.track_task(1, TaskStatus::Completed);

        // Add remaining work
        tracker.track_task(2, TaskStatus::InProgress { progress: 50.0 });

        let eta = tracker.calculate_eta();
        assert!(eta.is_some());
    }
}
