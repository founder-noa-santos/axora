//! Debounced filesystem notifications enqueue work only on the SQLite-backed job queue
//! (`IncrementalSync` / rescan). No Node/chokidar or external watcher stack.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, info, warn};

use crate::background::queue::{
    EnqueueRequest, JobKind, SqliteJobQueue, SyncJobPayload, SyncScope,
};

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub debounce_window: Duration,
    pub max_batch_paths: usize,
    pub overflow_path_limit: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_window: Duration::from_millis(350),
            max_batch_paths: 256,
            overflow_path_limit: 4096,
        }
    }
}

pub struct DebouncedFileWatcher {
    root: PathBuf,
    canonical_root: Option<PathBuf>,
    config: WatcherConfig,
    queue: SqliteJobQueue,
}

impl DebouncedFileWatcher {
    pub fn new(root: PathBuf, queue: SqliteJobQueue, config: WatcherConfig) -> Self {
        Self {
            canonical_root: std::fs::canonicalize(&root).ok(),
            root,
            config,
            queue,
        }
    }

    pub fn run(self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                let _ = tx.send(result);
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(false),
        )?;
        watcher.watch(&self.root, RecursiveMode::Recursive)?;

        info!(root = %self.root.display(), "livingdocs watcher started");
        self.event_loop(rx)
    }

    fn event_loop(self, rx: Receiver<notify::Result<Event>>) -> Result<()> {
        let mut paths = BTreeSet::new();
        let mut last_event_at = None::<Instant>;
        let mut overflowed = false;
        let mut event_count = 0u32;

        loop {
            match rx.recv_timeout(self.config.debounce_window) {
                Ok(Ok(event)) => {
                    if should_ignore_event(&event) {
                        continue;
                    }

                    event_count = event_count.saturating_add(1);
                    last_event_at = Some(Instant::now());

                    for path in event.paths {
                        if let Some(relative) =
                            normalize_path(&self.root, self.canonical_root.as_deref(), &path)
                        {
                            if should_ignore_path(&relative) {
                                continue;
                            }
                            paths.insert(relative);
                        }
                    }

                    if paths.len() >= self.config.overflow_path_limit {
                        overflowed = true;
                        paths.clear();
                    }
                }
                Ok(Err(err)) => {
                    warn!(error = %err, "livingdocs watcher received filesystem error");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_event_at.is_none() {
                        continue;
                    }

                    if last_event_at
                        .map(|instant| instant.elapsed() < self.config.debounce_window)
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    self.flush_batch(&mut paths, overflowed, event_count)?;
                    overflowed = false;
                    event_count = 0;
                    last_event_at = None;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("filesystem watcher channel disconnected");
                }
            }
        }
    }

    fn flush_batch(
        &self,
        paths: &mut BTreeSet<String>,
        overflowed: bool,
        event_count: u32,
    ) -> Result<()> {
        if overflowed {
            self.queue.enqueue(EnqueueRequest {
                kind: JobKind::RescanWorkspace,
                priority: 10,
                dedupe_key: "livingdocs:rescan".to_string(),
                payload: SyncJobPayload {
                    workspace_root: self.root.display().to_string(),
                    scope: SyncScope::Rescan,
                    reason: "watcher_overflow".to_string(),
                    event_count,
                },
                max_attempts: 5,
                run_after: Duration::from_secs(2),
            })?;
            warn!(
                event_count,
                limit = self.config.overflow_path_limit,
                "livingdocs watcher squashed burst into rescan job"
            );
            return Ok(());
        }

        if paths.is_empty() {
            return Ok(());
        }

        let batch: Vec<String> = if paths.len() > self.config.max_batch_paths {
            let truncated: Vec<String> = paths
                .iter()
                .take(self.config.max_batch_paths)
                .cloned()
                .collect();
            self.queue.enqueue(EnqueueRequest {
                kind: JobKind::RescanWorkspace,
                priority: 12,
                dedupe_key: "livingdocs:rescan".to_string(),
                payload: SyncJobPayload {
                    workspace_root: self.root.display().to_string(),
                    scope: SyncScope::Rescan,
                    reason: "batch_truncated".to_string(),
                    event_count,
                },
                max_attempts: 5,
                run_after: Duration::from_secs(2),
            })?;
            truncated
        } else {
            paths.iter().cloned().collect()
        };

        let dedupe_key = if batch.len() == 1 {
            format!("livingdocs:path:{}", batch[0])
        } else {
            format!(
                "livingdocs:batch:{}",
                blake3::hash(batch.join("\n").as_bytes()).to_hex()
            )
        };

        self.queue.enqueue(EnqueueRequest {
            kind: JobKind::IncrementalSync,
            priority: priority_for_paths(&batch),
            dedupe_key,
            payload: SyncJobPayload {
                workspace_root: self.root.display().to_string(),
                scope: SyncScope::Paths(batch.clone()),
                reason: "fs_batch".to_string(),
                event_count,
            },
            max_attempts: 3,
            run_after: Duration::from_millis(50),
        })?;

        debug!(
            paths = batch.len(),
            event_count, "livingdocs watcher enqueued sync batch"
        );
        paths.clear();
        Ok(())
    }
}

fn should_ignore_event(event: &Event) -> bool {
    matches!(event.kind, EventKind::Access(_) | EventKind::Other)
}

fn normalize_path(root: &Path, canonical_root: Option<&Path>, path: &Path) -> Option<String> {
    if let Ok(relative) = path.strip_prefix(root) {
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if !normalized.is_empty() {
            return Some(normalized);
        }
    }

    let canonical_path = std::fs::canonicalize(path).ok()?;
    let normalized = canonical_path
        .strip_prefix(canonical_root?)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn should_ignore_path(relative: &str) -> bool {
    const IGNORED_SEGMENTS: &[&str] = &[
        ".git",
        ".jj",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".turbo",
        ".idea",
        ".vscode",
        ".openakta/cache",
    ];

    if relative.ends_with('~') || relative.ends_with(".swp") || relative.ends_with(".tmp") {
        return true;
    }

    IGNORED_SEGMENTS.iter().any(|segment| {
        relative == *segment
            || relative.starts_with(&format!("{segment}/"))
            || relative.contains(&format!("/{segment}/"))
    })
}

fn priority_for_paths(paths: &[String]) -> i64 {
    let mut priority = 50;

    for path in paths {
        let lower = path.to_ascii_lowercase();
        if lower.starts_with("akta-docs/03-business-logic/")
            || lower.contains("/business/")
            || lower.contains("/domain/")
            || lower.contains("/rules/")
            || lower.contains("/policy/")
        {
            return 80;
        }

        if lower.starts_with("akta-docs/06-technical/")
            || lower.contains("/contracts/")
            || lower.ends_with("/route.ts")
            || lower.ends_with("/route.tsx")
            || lower.contains("/schema/")
        {
            priority = priority.max(65);
            continue;
        }

        if lower.contains("/test/")
            || lower.contains("/tests/")
            || lower.ends_with(".spec.ts")
            || lower.ends_with(".test.ts")
            || lower.ends_with(".spec.tsx")
            || lower.ends_with(".test.tsx")
        {
            priority = priority.min(40);
        }
    }

    priority
}

#[cfg(test)]
mod tests {
    use super::{priority_for_paths, should_ignore_path};

    #[test]
    fn escalates_business_rule_batches() {
        let priority = priority_for_paths(&["src/domain/rules/plan.ts".to_string()]);
        assert_eq!(priority, 80);
    }

    #[test]
    fn keeps_rescan_related_incremental_batches_above_default_for_technical_paths() {
        let priority = priority_for_paths(&["akta-docs/06-technical/api.md".to_string()]);
        assert_eq!(priority, 65);
    }

    #[test]
    fn lowers_test_only_batches() {
        let priority = priority_for_paths(&["src/lib/plan.test.ts".to_string()]);
        assert_eq!(priority, 40);
    }

    #[test]
    fn ignores_node_modules_and_editor_temp_files() {
        assert!(should_ignore_path("node_modules/react/index.js"));
        assert!(should_ignore_path("src/main.rs.swp"));
        assert!(!should_ignore_path("src/main.rs"));
    }
}
