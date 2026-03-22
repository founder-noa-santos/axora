use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use tracing::{error, info, warn};

use openakta_core::CoreConfig;

use crate::background::governor::{GovernorConfig, ResourceGovernor};
use crate::background::processor::LivingDocsProcessor;
use crate::background::queue::{
    failed_status_for_choice, EnqueueRequest, JobKind, SqliteJobQueue, SyncJobPayload, SyncScope,
};
use crate::background::review_resolution::CoordinatorCodeResolutionRunner;
use crate::background::watcher::{DebouncedFileWatcher, WatcherConfig};

pub struct LivingDocsEngine;

impl LivingDocsEngine {
    /// Background LivingDocs worker. When `shutdown` becomes true, the main loop exits cooperatively
    /// (see `PLAN_TO_ACTION` / V-009 — daemon graceful shutdown).
    pub fn start(config: CoreConfig, shutdown: Arc<AtomicBool>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            if let Err(err) = run_engine(config, shutdown) {
                error!("livingdocs engine exited: {err}");
            }
        })
    }
}

fn run_engine(config: CoreConfig, shutdown: Arc<AtomicBool>) -> Result<()> {
    let queue_path = SqliteJobQueue::path_for_workspace(&config.workspace_root);
    let queue = SqliteJobQueue::open(&queue_path)?;
    let merkle_state_path = config
        .workspace_root
        .join(".openakta")
        .join("livingdocs-merkle.json");
    if !merkle_state_path.exists() {
        queue.enqueue(EnqueueRequest {
            kind: JobKind::RescanWorkspace,
            priority: 10,
            dedupe_key: "livingdocs:bootstrap-rescan".to_string(),
            payload: SyncJobPayload {
                workspace_root: config.workspace_root.display().to_string(),
                scope: SyncScope::Rescan,
                reason: "bootstrap_state".to_string(),
                event_count: 0,
            },
            max_attempts: 5,
            run_after: Duration::from_secs(1),
        })?;
    }
    let watcher_queue = queue.clone();
    let watcher_root = config.workspace_root.clone();

    thread::spawn(move || {
        let watcher =
            DebouncedFileWatcher::new(watcher_root, watcher_queue, WatcherConfig::default());
        if let Err(err) = watcher.run() {
            error!("livingdocs watcher stopped: {err}");
        }
    });

    info!(queue = %queue_path.display(), "livingdocs engine online");

    let governor_config = GovernorConfig::default();
    let ast_budget_bytes = governor_config.recommended_ast_cache_budget_bytes();
    let mut governor = ResourceGovernor::new(governor_config)?;
    let mut processor = LivingDocsProcessor::new(
        config.workspace_root.clone(),
        config.semantic_store_path.clone(),
        queue.clone(),
        ast_budget_bytes,
        std::sync::Arc::new(CoordinatorCodeResolutionRunner::new(config.clone())),
    )?;
    let lease = Duration::from_secs(60);
    let mut had_recent_job = false;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            info!("livingdocs engine stopping (daemon shutdown)");
            return Ok(());
        }

        if let Err(err) = queue.recover_stale_leases() {
            warn!("livingdocs lease recovery failed: {err}");
        }
        if let Err(err) = queue.recover_stale_resolutions() {
            warn!("livingdocs resolution lease recovery failed: {err}");
        }

        governor.wait_for_budget();

        let resolution_lease = Duration::from_secs(30 * 60);
        if let Some(work) = queue.claim_next_resolution_work(resolution_lease)? {
            had_recent_job = true;
            match processor.process_review_resolution(&work) {
                Ok(()) => {}
                Err(err) => {
                    queue.fail_resolution(
                        &work.review_id,
                        failed_status_for_choice(&work.choice)?,
                        &err.to_string(),
                    )?;
                    warn!(
                        review_id = %work.review_id,
                        report_id = %work.report_id,
                        choice = %work.choice,
                        error = %err,
                        "livingdocs review resolution failed"
                    );
                }
            }
            continue;
        }

        let Some(job) = queue.dequeue(lease)? else {
            if had_recent_job {
                processor.release_idle_parser_state();
                had_recent_job = false;
            }
            thread::sleep(Duration::from_millis(250));
            continue;
        };
        had_recent_job = true;

        match processor.process(&job, &mut governor) {
            Ok(()) => {
                queue.complete(&job.id)?;
            }
            Err(err) => {
                let retry_after = Duration::from_millis(250 * 2_u64.saturating_pow(job.attempts));
                queue.fail(&job, &err.to_string(), retry_after)?;
                warn!(job_id = %job.id, error = %err, "livingdocs job failed");
            }
        }
    }
}
