use openakta_agents::{QueueStats, Task, TaskQueue, TaskQueueConfig, TaskQueueStatus};

fn queue() -> TaskQueue {
    TaskQueue::new(TaskQueueConfig::default())
}

fn task(description: &str) -> Task {
    Task::new(description)
}

#[test]
fn priority_ordering_prefers_high_priority_ready_task() {
    let queue = queue();
    let low = queue.add_task(task("low"), 10, vec![]).unwrap();
    let high = queue.add_task(task("high"), 90, vec![]).unwrap();

    assert_eq!(queue.get_next_ready_task().unwrap().task_id, high);
    assert_eq!(queue.get_next_ready_task().unwrap().task_id, low);
}

#[test]
fn dependency_resolution_requires_parent_completion() {
    let queue = queue();
    let parent = queue.add_task(task("parent"), 60, vec![]).unwrap();
    let child = queue
        .add_task(task("child"), 95, vec![parent.clone()])
        .unwrap();

    assert_eq!(queue.get_next_ready_task().unwrap().task_id, parent);
    queue.mark_completed(&parent).unwrap();
    assert_eq!(queue.get_next_ready_task().unwrap().task_id, child);
}

#[test]
fn queue_stats_reflect_reserved_completed_and_failed_tasks() {
    let queue = queue();
    let ready = queue.add_task(task("ready"), 90, vec![]).unwrap();
    let completed = queue.add_task(task("completed"), 70, vec![]).unwrap();
    let failed = queue.add_task(task("failed"), 50, vec![]).unwrap();

    let reserved = queue.get_next_ready_task().unwrap();
    queue.mark_completed(&completed).unwrap();
    queue.mark_failed(&failed, "boom".to_string()).unwrap();

    let stats = queue.get_queue_stats();

    assert_eq!(reserved.task_id, ready);
    assert_queue_stats(&stats, 3, 0, 0, 1, 1, 1);
}

#[test]
fn critical_path_calculation_matches_longest_chain() {
    let queue = queue();
    let a = queue.add_task(task("a"), 10, vec![]).unwrap();
    let b = queue.add_task(task("b"), 20, vec![a.clone()]).unwrap();
    let c = queue.add_task(task("c"), 30, vec![b.clone()]).unwrap();
    let d = queue.add_task(task("d"), 40, vec![a.clone()]).unwrap();

    assert_eq!(queue.get_critical_path(), vec![a, b, c]);
    assert!(!queue.get_critical_path().contains(&d));
}

#[test]
fn load_balancing_distributes_ready_work_evenly() {
    let queue = queue();
    queue.add_task(task("a"), 100, vec![]).unwrap();
    queue.add_task(task("b"), 90, vec![]).unwrap();
    queue.add_task(task("c"), 80, vec![]).unwrap();
    queue.add_task(task("d"), 70, vec![]).unwrap();

    let assignments = queue.balance_load(&["w1".to_string(), "w2".to_string()]);

    assert_eq!(assignments.len(), 2);
    let diff = assignments[0]
        .task_ids
        .len()
        .abs_diff(assignments[1].task_ids.len());
    assert!(diff <= 1);
}

#[test]
fn blocked_tasks_transition_to_ready_after_dependency_completion() {
    let queue = queue();
    let root = queue.add_task(task("root"), 80, vec![]).unwrap();
    let child = queue
        .add_task(task("child"), 70, vec![root.clone()])
        .unwrap();

    let child_before = queue
        .get_next_ready_task()
        .map(|task| task.task_id)
        .unwrap();
    assert_eq!(child_before, root);
    queue.mark_completed(&root).unwrap();

    let next = queue.get_next_ready_task().unwrap();
    assert_eq!(next.task_id, child);
    assert!(matches!(next.status, TaskQueueStatus::InProgress { .. }));
}

#[test]
fn queue_rejects_missing_dependency_at_integration_level() {
    let queue = queue();
    let error = queue
        .add_task(task("orphan"), 50, vec!["missing".to_string()])
        .unwrap_err();

    assert_eq!(error.to_string(), "dependency not found: missing");
}

#[test]
fn queue_rejects_invalid_priority_at_integration_level() {
    let queue = queue();
    let error = queue.add_task(task("invalid"), 101, vec![]).unwrap_err();

    assert_eq!(error.to_string(), "invalid priority: 101");
}

#[test]
fn queue_enforces_capacity_at_integration_level() {
    let queue = TaskQueue::new(TaskQueueConfig {
        max_queue_size: 1,
        ..TaskQueueConfig::default()
    });

    queue.add_task(task("first"), 50, vec![]).unwrap();
    let error = queue.add_task(task("second"), 50, vec![]).unwrap_err();

    assert_eq!(error.to_string(), "task queue is at capacity");
}

#[test]
fn disabled_load_balancing_returns_empty_assignments() {
    let queue = TaskQueue::new(TaskQueueConfig {
        enable_load_balancing: false,
        ..TaskQueueConfig::default()
    });
    queue.add_task(task("a"), 100, vec![]).unwrap();
    queue.add_task(task("b"), 90, vec![]).unwrap();

    let assignments = queue.balance_load(&["w1".to_string(), "w2".to_string()]);

    assert_eq!(assignments.len(), 2);
    assert!(assignments
        .iter()
        .all(|assignment| assignment.task_ids.is_empty()));
}

fn assert_queue_stats(
    stats: &QueueStats,
    total: usize,
    pending: usize,
    ready: usize,
    in_progress: usize,
    completed: usize,
    failed: usize,
) {
    assert_eq!(stats.total_tasks, total);
    assert_eq!(stats.pending_tasks, pending);
    assert_eq!(stats.ready_tasks, ready);
    assert_eq!(stats.in_progress_tasks, in_progress);
    assert_eq!(stats.completed_tasks, completed);
    assert_eq!(stats.failed_tasks, failed);
}
