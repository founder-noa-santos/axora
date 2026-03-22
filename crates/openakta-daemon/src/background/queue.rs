use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use openakta_docs::{DriftDomain, DriftKind, DriftReport, DriftSeverity};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    IncrementalSync,
    RescanWorkspace,
}

impl JobKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::IncrementalSync => "incremental_sync",
            Self::RescanWorkspace => "rescan_workspace",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "incremental_sync" => Ok(Self::IncrementalSync),
            "rescan_workspace" => Ok(Self::RescanWorkspace),
            other => anyhow::bail!("unknown job kind: {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncScope {
    Paths(Vec<String>),
    Rescan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobPayload {
    pub workspace_root: String,
    pub scope: SyncScope,
    pub reason: String,
    pub event_count: u32,
}

#[derive(Debug, Clone)]
pub struct EnqueueRequest {
    pub kind: JobKind,
    pub priority: i64,
    pub dedupe_key: String,
    pub payload: SyncJobPayload,
    pub max_attempts: u32,
    pub run_after: Duration,
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub payload: SyncJobPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDriftFlag {
    pub flag_id: String,
    pub domain: DriftDomain,
    pub kind: DriftKind,
    pub severity: DriftSeverity,
    pub fingerprint: String,
    pub doc_path: PathBuf,
    pub code_path: Option<PathBuf>,
    pub symbol_name: Option<String>,
    pub rule_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDriftReport {
    pub report_id: String,
    pub workspace_root: PathBuf,
    pub trigger: String,
    pub created_at_ms: i64,
    pub total_flags: usize,
    pub api_surface_flags: usize,
    pub business_rule_flags: usize,
    pub code_reference_flags: usize,
    pub critical_flags: usize,
    pub warning_flags: usize,
    pub info_flags: usize,
    pub highest_severity: Option<DriftSeverity>,
    pub flags: Vec<StoredDriftFlag>,
}

/// One row from [`SqliteJobQueue::latest_confidence_audit_for_report`].
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct StoredConfidenceAudit {
    pub action_id: String,
    pub report_id: String,
    pub decision: String,
    pub confidence_score: Option<f64>,
    pub breakdown_json: String,
    pub canonical_toon_json: Option<String>,
    pub commit_id: Option<String>,
    pub external_changelog_relpath: Option<String>,
    pub autocommit_error: Option<String>,
}

/// One row from `livingdocs_reconcile_reviews` (Plan 6 gRPC).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ReconcileReviewRow {
    pub review_id: String,
    pub report_id: String,
    pub workspace_root: PathBuf,
    pub created_at_ms: i64,
    pub confidence_score: f64,
    pub breakdown_json: String,
    pub decision: String,
    pub status: String,
    pub notes: Option<String>,
    pub resolution_choice: Option<String>,
    pub server_resolution_id: Option<String>,
    pub patch_receipt_id: Option<String>,
    pub toon_changelog_entry_id: Option<String>,
    pub resolution_error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PendingResolutionWorkItem {
    pub review_id: String,
    pub report_id: String,
    pub workspace_root: PathBuf,
    pub choice: String,
    pub server_resolution_id: String,
    pub confidence_score: f64,
    pub breakdown_json: String,
    pub notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StoredResolutionResult {
    pub review_id: String,
    pub status: String,
    pub server_resolution_id: Option<String>,
    pub patch_receipt_id: Option<String>,
    pub toon_changelog_entry_id: Option<String>,
    pub resolution_error: Option<String>,
}

/// Result of [`SqliteJobQueue::submit_resolution`].
#[derive(Debug, Clone)]
pub enum SubmitResolutionOutcome {
    Ok {
        server_resolution_id: String,
    },
    Duplicate {
        server_resolution_id: String,
    },
    NotFound,
    Conflict {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct SqliteJobQueue {
    db_path: PathBuf,
    busy_timeout: Duration,
}

impl SqliteJobQueue {
    /// Default on-disk path for the workspace LivingDocs queue (shared with [`crate::background::engine::LivingDocsEngine`]).
    pub fn path_for_workspace(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".openakta").join("livingdocs-queue.db")
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let queue = Self {
            db_path: path.into(),
            busy_timeout: Duration::from_secs(5),
        };
        queue.init_schema()?;
        Ok(queue)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA cache_size = -2048;

            CREATE TABLE IF NOT EXISTS livingdocs_jobs (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                dedupe_key TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                available_at_ms INTEGER NOT NULL,
                lease_expires_at_ms INTEGER,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                started_at_ms INTEGER,
                completed_at_ms INTEGER,
                last_error TEXT
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_livingdocs_jobs_dedupe_queued
            ON livingdocs_jobs(dedupe_key)
            WHERE status = 'queued';

            CREATE INDEX IF NOT EXISTS idx_livingdocs_jobs_dequeue
            ON livingdocs_jobs(status, available_at_ms, priority DESC, created_at_ms ASC);

            CREATE INDEX IF NOT EXISTS idx_livingdocs_jobs_recover
            ON livingdocs_jobs(status, lease_expires_at_ms);

            CREATE TABLE IF NOT EXISTS livingdocs_drift_reports (
                report_id TEXT PRIMARY KEY,
                workspace_root TEXT NOT NULL,
                trigger TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                total_flags INTEGER NOT NULL,
                api_surface_flags INTEGER NOT NULL,
                business_rule_flags INTEGER NOT NULL,
                code_reference_flags INTEGER NOT NULL,
                critical_flags INTEGER NOT NULL,
                warning_flags INTEGER NOT NULL,
                info_flags INTEGER NOT NULL,
                highest_severity TEXT
            );

            CREATE TABLE IF NOT EXISTS livingdocs_drift_flags (
                flag_id TEXT PRIMARY KEY,
                report_id TEXT NOT NULL,
                domain TEXT NOT NULL,
                kind TEXT NOT NULL,
                severity TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                doc_path TEXT NOT NULL,
                code_path TEXT,
                symbol_name TEXT,
                rule_ids_json TEXT NOT NULL,
                message TEXT NOT NULL,
                FOREIGN KEY(report_id) REFERENCES livingdocs_drift_reports(report_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_livingdocs_drift_reports_latest
            ON livingdocs_drift_reports(workspace_root, created_at_ms DESC);

            CREATE INDEX IF NOT EXISTS idx_livingdocs_drift_flags_report
            ON livingdocs_drift_flags(report_id);

            CREATE TABLE IF NOT EXISTS livingdocs_reconcile_reviews (
                review_id TEXT PRIMARY KEY,
                report_id TEXT NOT NULL UNIQUE,
                workspace_root TEXT NOT NULL,
                job_id TEXT,
                created_at_ms INTEGER NOT NULL,
                confidence_score REAL NOT NULL,
                breakdown_json TEXT NOT NULL,
                status TEXT NOT NULL,
                resolved_at_ms INTEGER,
                resolver TEXT,
                notes TEXT,
                decision TEXT NOT NULL DEFAULT 'review_required',
                resolution_choice TEXT,
                server_resolution_id TEXT,
                patch_receipt_id TEXT,
                toon_changelog_entry_id TEXT,
                resolution_error TEXT,
                FOREIGN KEY(report_id) REFERENCES livingdocs_drift_reports(report_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_livingdocs_reconcile_reviews_workspace
            ON livingdocs_reconcile_reviews(workspace_root, status, created_at_ms DESC);

            CREATE TABLE IF NOT EXISTS livingdocs_confidence_audit (
                action_id TEXT PRIMARY KEY,
                report_id TEXT NOT NULL,
                workspace_root TEXT NOT NULL,
                decision TEXT NOT NULL,
                confidence_score REAL,
                breakdown_json TEXT NOT NULL,
                canonical_toon_json TEXT,
                commit_id TEXT,
                external_changelog_relpath TEXT,
                autocommit_error TEXT,
                created_at_ms INTEGER NOT NULL,
                FOREIGN KEY(report_id) REFERENCES livingdocs_drift_reports(report_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_livingdocs_confidence_audit_workspace
            ON livingdocs_confidence_audit(workspace_root, created_at_ms DESC);

            CREATE TABLE IF NOT EXISTS livingdocs_autocommit_log (
                commit_id TEXT PRIMARY KEY,
                report_id TEXT NOT NULL,
                workspace_root TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                FOREIGN KEY(report_id) REFERENCES livingdocs_drift_reports(report_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_livingdocs_autocommit_workspace
            ON livingdocs_autocommit_log(workspace_root, created_at_ms DESC);

            CREATE TABLE IF NOT EXISTS livingdocs_resolution_dedupe (
                client_resolution_id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                server_resolution_id TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );
            "#,
        )?;
        Self::migrate_livingdocs_schema(&conn)?;
        Ok(())
    }

    fn migrate_livingdocs_schema(conn: &Connection) -> Result<()> {
        if !Self::column_exists(conn, "livingdocs_reconcile_reviews", "decision")? {
            conn.execute(
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN decision TEXT NOT NULL DEFAULT 'review_required'",
                [],
            )?;
        }
        for (column, ddl) in [
            (
                "resolution_choice",
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN resolution_choice TEXT",
            ),
            (
                "server_resolution_id",
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN server_resolution_id TEXT",
            ),
            (
                "patch_receipt_id",
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN patch_receipt_id TEXT",
            ),
            (
                "toon_changelog_entry_id",
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN toon_changelog_entry_id TEXT",
            ),
            (
                "resolution_error",
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN resolution_error TEXT",
            ),
        ] {
            if !Self::column_exists(conn, "livingdocs_reconcile_reviews", column)? {
                conn.execute(ddl, [])?;
            }
        }
        if !Self::column_exists(
            conn,
            "livingdocs_reconcile_reviews",
            "resolution_lease_expires_at_ms",
        )? {
            conn.execute(
                "ALTER TABLE livingdocs_reconcile_reviews ADD COLUMN resolution_lease_expires_at_ms INTEGER",
                [],
            )?;
        }
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS livingdocs_resolution_dedupe (
                client_resolution_id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                server_resolution_id TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let pragma = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn enqueue(&self, request: EnqueueRequest) -> Result<String> {
        let conn = self.connect()?;
        let now = now_ms();
        let available_at = now + request.run_after.as_millis() as i64;
        let payload_json = serde_json::to_string(&request.payload)?;
        let id = Uuid::new_v4().to_string();

        conn.execute(
            r#"
            INSERT INTO livingdocs_jobs (
                id,
                kind,
                status,
                priority,
                attempts,
                max_attempts,
                dedupe_key,
                payload_json,
                available_at_ms,
                created_at_ms,
                updated_at_ms
            )
            VALUES (?1, ?2, 'queued', ?3, 0, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(dedupe_key) WHERE status = 'queued'
            DO UPDATE SET
                priority = MAX(livingdocs_jobs.priority, excluded.priority),
                payload_json = excluded.payload_json,
                available_at_ms = MIN(livingdocs_jobs.available_at_ms, excluded.available_at_ms),
                updated_at_ms = excluded.updated_at_ms,
                status = 'queued',
                last_error = NULL
            "#,
            params![
                id,
                request.kind.as_str(),
                request.priority,
                request.max_attempts,
                request.dedupe_key,
                payload_json,
                available_at,
                now,
            ],
        )?;

        Ok(id)
    }

    pub fn recover_stale_leases(&self) -> Result<usize> {
        let conn = self.connect()?;
        let now = now_ms();
        let updated = conn.execute(
            r#"
            UPDATE livingdocs_jobs
            SET status = 'queued',
                lease_expires_at_ms = NULL,
                updated_at_ms = ?1,
                last_error = COALESCE(last_error, 'lease expired before completion')
            WHERE status = 'running'
              AND lease_expires_at_ms IS NOT NULL
              AND lease_expires_at_ms <= ?1
            "#,
            params![now],
        )?;
        Ok(updated)
    }

    /// Re-queue review resolution work that stayed in `*_running` past lease expiry (crash/kill).
    pub fn recover_stale_resolutions(&self) -> Result<usize> {
        let conn = self.connect()?;
        let now = now_ms();
        let updated = conn.execute(
            r#"
            UPDATE livingdocs_reconcile_reviews
            SET status = CASE status
                    WHEN ?1 THEN ?2
                    WHEN ?3 THEN ?4
                    ELSE status
                END,
                resolution_lease_expires_at_ms = NULL
            WHERE status IN (?1, ?3)
              AND resolution_lease_expires_at_ms IS NOT NULL
              AND resolution_lease_expires_at_ms <= ?5
            "#,
            params![
                review_status_doc_update_running(),
                review_status_doc_update_queued(),
                review_status_code_update_running(),
                review_status_code_update_queued(),
                now,
            ],
        )?;
        Ok(updated)
    }

    pub fn dequeue(&self, lease_for: Duration) -> Result<Option<JobRecord>> {
        let mut conn = self.connect()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = now_ms();

        let row = tx
            .query_row(
                r#"
                SELECT id, kind, status, priority, attempts, max_attempts, payload_json
                FROM livingdocs_jobs
                WHERE status = 'queued'
                  AND available_at_ms <= ?1
                ORDER BY priority DESC, created_at_ms ASC
                LIMIT 1
                "#,
                params![now],
                |row| {
                    let kind: String = row.get(1)?;
                    let status: String = row.get(2)?;
                    let payload_json: String = row.get(6)?;
                    let payload = serde_json::from_str(&payload_json).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            payload_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?;
                    Ok(JobRecord {
                        id: row.get(0)?,
                        kind: JobKind::from_str(&kind).map_err(to_sql_err)?,
                        status: status_from_str(&status).map_err(to_sql_err)?,
                        attempts: row.get::<_, u32>(4)?,
                        max_attempts: row.get::<_, u32>(5)?,
                        payload,
                    })
                },
            )
            .optional()?;

        let Some(mut job) = row else {
            tx.commit()?;
            return Ok(None);
        };

        let lease_expires_at = now + lease_for.as_millis() as i64;
        tx.execute(
            r#"
            UPDATE livingdocs_jobs
            SET status = 'running',
                attempts = attempts + 1,
                started_at_ms = ?2,
                updated_at_ms = ?2,
                lease_expires_at_ms = ?3
            WHERE id = ?1
            "#,
            params![job.id, now, lease_expires_at],
        )?;
        tx.commit()?;

        job.status = JobStatus::Running;
        job.attempts += 1;
        Ok(Some(job))
    }

    pub fn complete(&self, job_id: &str) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        conn.execute(
            r#"
            UPDATE livingdocs_jobs
            SET status = 'completed',
                completed_at_ms = ?2,
                updated_at_ms = ?2,
                lease_expires_at_ms = NULL
            WHERE id = ?1
            "#,
            params![job_id, now],
        )?;
        Ok(())
    }

    pub fn fail(&self, job: &JobRecord, error: &str, retry_after: Duration) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        let should_retry = job.attempts < job.max_attempts;
        let next_status = if should_retry {
            JobStatus::Queued
        } else {
            JobStatus::Failed
        };
        let available_at = now + retry_after.as_millis() as i64;

        conn.execute(
            r#"
            UPDATE livingdocs_jobs
            SET status = ?2,
                available_at_ms = ?3,
                updated_at_ms = ?4,
                lease_expires_at_ms = NULL,
                last_error = ?5
            WHERE id = ?1
            "#,
            params![job.id, next_status.as_str(), available_at, now, error],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub fn queued_count(&self) -> Result<i64> {
        let conn = self.connect()?;
        let count = conn.query_row(
            "SELECT COUNT(*) FROM livingdocs_jobs WHERE status = 'queued'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Persists a drift report. Returns `(report_id, inserted)` where `inserted` is false when the
    /// same drift fingerprint was already stored (idempotent retry after partial failure — V-003).
    pub fn persist_drift_report(
        &self,
        workspace_root: &Path,
        trigger: &str,
        report: &DriftReport,
    ) -> Result<(String, bool)> {
        let mut conn = self.connect()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = now_ms();
        let report_id = drift_report_stable_id(workspace_root, trigger, report);

        let exists = tx
            .query_row(
                "SELECT 1 FROM livingdocs_drift_reports WHERE report_id = ?1",
                params![&report_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            tx.commit()?;
            return Ok((report_id, false));
        }

        tx.execute(
            r#"
            INSERT INTO livingdocs_drift_reports (
                report_id,
                workspace_root,
                trigger,
                created_at_ms,
                total_flags,
                api_surface_flags,
                business_rule_flags,
                code_reference_flags,
                critical_flags,
                warning_flags,
                info_flags,
                highest_severity
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                report_id,
                workspace_root.display().to_string(),
                trigger,
                now,
                report.total_flags as i64,
                report.api_surface_flags as i64,
                report.business_rule_flags as i64,
                report.code_reference_flags as i64,
                report.critical_flags as i64,
                report.warning_flags as i64,
                report.info_flags as i64,
                report.highest_severity.as_ref().map(drift_severity_as_str),
            ],
        )?;

        for flag in &report.flags {
            tx.execute(
                r#"
                INSERT INTO livingdocs_drift_flags (
                    flag_id,
                    report_id,
                    domain,
                    kind,
                    severity,
                    fingerprint,
                    doc_path,
                    code_path,
                    symbol_name,
                    rule_ids_json,
                    message
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    report_id,
                    drift_domain_as_str(&flag.domain),
                    drift_kind_as_str(&flag.kind),
                    drift_severity_as_str(&flag.severity),
                    flag.fingerprint,
                    flag.doc_path.display().to_string(),
                    flag.code_path
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    flag.symbol_name.as_deref(),
                    serde_json::to_string(&flag.rule_ids)?,
                    flag.message,
                ],
            )?;
        }

        tx.commit()?;
        Ok((report_id, true))
    }

    /// Queue a human review when confidence routing is `ReviewRequired`.
    /// Returns `None` when a row for `report_id` already exists (idempotent).
    pub fn enqueue_reconcile_review(
        &self,
        report_id: &str,
        workspace_root: &Path,
        job_id: Option<&str>,
        confidence_score: f64,
        breakdown_json: &str,
        decision: &str,
    ) -> Result<Option<String>> {
        let conn = self.connect()?;
        let now = now_ms();
        let review_id = Uuid::new_v4().to_string();
        let rows = conn.execute(
            r#"
            INSERT OR IGNORE INTO livingdocs_reconcile_reviews (
                review_id,
                report_id,
                workspace_root,
                job_id,
                created_at_ms,
                confidence_score,
                breakdown_json,
                status,
                decision
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                review_id,
                report_id,
                workspace_root.display().to_string(),
                job_id,
                now,
                confidence_score,
                breakdown_json,
                review_status_pending(),
                decision,
            ],
        )?;
        if rows == 0 {
            return Ok(None);
        }
        Ok(Some(review_id))
    }

    /// Append-only audit for confidence routing, TOON payloads, and autocommit outcomes.
    pub fn record_confidence_action(
        &self,
        report_id: &str,
        workspace_root: &Path,
        decision: &str,
        confidence_score: Option<f64>,
        breakdown_json: &str,
        canonical_toon_json: Option<&str>,
        commit_id: Option<&str>,
        external_changelog_relpath: Option<&str>,
        autocommit_error: Option<&str>,
    ) -> Result<String> {
        let conn = self.connect()?;
        let now = now_ms();
        let action_id = Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO livingdocs_confidence_audit (
                action_id,
                report_id,
                workspace_root,
                decision,
                confidence_score,
                breakdown_json,
                canonical_toon_json,
                commit_id,
                external_changelog_relpath,
                autocommit_error,
                created_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                action_id,
                report_id,
                workspace_root.display().to_string(),
                decision,
                confidence_score,
                breakdown_json,
                canonical_toon_json,
                commit_id,
                external_changelog_relpath,
                autocommit_error,
                now,
            ],
        )?;
        Ok(action_id)
    }

    /// Latest persisted confidence audit row for a drift report (tests, CLI, diagnostics).
    #[cfg(test)]
    pub fn latest_confidence_audit_for_report(
        &self,
        report_id: &str,
    ) -> Result<Option<StoredConfidenceAudit>> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                r#"
                SELECT
                    action_id,
                    report_id,
                    decision,
                    confidence_score,
                    breakdown_json,
                    canonical_toon_json,
                    commit_id,
                    external_changelog_relpath,
                    autocommit_error
                FROM livingdocs_confidence_audit
                WHERE report_id = ?1
                ORDER BY created_at_ms DESC, rowid DESC
                LIMIT 1
                "#,
                params![report_id],
                |row| {
                    Ok(StoredConfidenceAudit {
                        action_id: row.get(0)?,
                        report_id: row.get(1)?,
                        decision: row.get(2)?,
                        confidence_score: row.get(3)?,
                        breakdown_json: row.get(4)?,
                        canonical_toon_json: row.get(5)?,
                        commit_id: row.get(6)?,
                        external_changelog_relpath: row.get(7)?,
                        autocommit_error: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Record a dangling commit OID produced by alternate-index LivingDocs commits (no branch update).
    pub fn record_autocommit(&self, commit_id: &str, report_id: &str, workspace_root: &Path) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        conn.execute(
            r#"
            INSERT INTO livingdocs_autocommit_log (commit_id, report_id, workspace_root, created_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![commit_id, report_id, workspace_root.display().to_string(), now],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub fn latest_drift_report(&self, workspace_root: &Path) -> Result<Option<StoredDriftReport>> {
        let conn = self.connect()?;
        let workspace_root = workspace_root.display().to_string();
        let report = conn
            .query_row(
                r#"
                SELECT
                    report_id,
                    workspace_root,
                    trigger,
                    created_at_ms,
                    total_flags,
                    api_surface_flags,
                    business_rule_flags,
                    code_reference_flags,
                    critical_flags,
                    warning_flags,
                    info_flags,
                    highest_severity
                FROM livingdocs_drift_reports
                WHERE workspace_root = ?1
                ORDER BY created_at_ms DESC, rowid DESC
                LIMIT 1
                "#,
                params![workspace_root],
                |row| {
                    let highest: Option<String> = row.get(11)?;
                    Ok(StoredDriftReport {
                        report_id: row.get(0)?,
                        workspace_root: PathBuf::from(row.get::<_, String>(1)?),
                        trigger: row.get(2)?,
                        created_at_ms: row.get(3)?,
                        total_flags: row.get::<_, i64>(4)? as usize,
                        api_surface_flags: row.get::<_, i64>(5)? as usize,
                        business_rule_flags: row.get::<_, i64>(6)? as usize,
                        code_reference_flags: row.get::<_, i64>(7)? as usize,
                        critical_flags: row.get::<_, i64>(8)? as usize,
                        warning_flags: row.get::<_, i64>(9)? as usize,
                        info_flags: row.get::<_, i64>(10)? as usize,
                        highest_severity: highest
                            .as_deref()
                            .map(drift_severity_from_str)
                            .transpose()
                            .map_err(to_sql_err)?,
                        flags: Vec::new(),
                    })
                },
            )
            .optional()?;

        let Some(mut report) = report else {
            return Ok(None);
        };

        let mut stmt = conn.prepare(
            r#"
            SELECT
                flag_id,
                domain,
                kind,
                severity,
                fingerprint,
                doc_path,
                code_path,
                symbol_name,
                rule_ids_json,
                message
            FROM livingdocs_drift_flags
            WHERE report_id = ?1
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 0
                    WHEN 'warning' THEN 1
                    ELSE 2
                END,
                doc_path ASC,
                COALESCE(symbol_name, '') ASC,
                fingerprint ASC
            "#,
        )?;
        let flags = stmt.query_map(params![report.report_id.clone()], |row| {
            let domain: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let severity: String = row.get(3)?;
            let code_path: Option<String> = row.get(6)?;
            let symbol_name: Option<String> = row.get(7)?;
            let rule_ids_json: String = row.get(8)?;
            Ok(StoredDriftFlag {
                flag_id: row.get(0)?,
                domain: drift_domain_from_str(&domain).map_err(to_sql_err)?,
                kind: drift_kind_from_str(&kind).map_err(to_sql_err)?,
                severity: drift_severity_from_str(&severity).map_err(to_sql_err)?,
                fingerprint: row.get(4)?,
                doc_path: PathBuf::from(row.get::<_, String>(5)?),
                code_path: code_path.map(PathBuf::from),
                symbol_name,
                rule_ids: serde_json::from_str(&rule_ids_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        rule_ids_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?,
                message: row.get(9)?,
            })
        })?;

        report.flags = flags.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Some(report))
    }

    /// Load a stored drift report and flags by `report_id` (Plan 6 detail API).
    pub fn drift_report_by_report_id(&self, report_id: &str) -> Result<Option<StoredDriftReport>> {
        let conn = self.connect()?;
        let report = conn
            .query_row(
                r#"
                SELECT
                    report_id,
                    workspace_root,
                    trigger,
                    created_at_ms,
                    total_flags,
                    api_surface_flags,
                    business_rule_flags,
                    code_reference_flags,
                    critical_flags,
                    warning_flags,
                    info_flags,
                    highest_severity
                FROM livingdocs_drift_reports
                WHERE report_id = ?1
                "#,
                params![report_id],
                |row| {
                    let highest: Option<String> = row.get(11)?;
                    Ok(StoredDriftReport {
                        report_id: row.get(0)?,
                        workspace_root: PathBuf::from(row.get::<_, String>(1)?),
                        trigger: row.get(2)?,
                        created_at_ms: row.get(3)?,
                        total_flags: row.get::<_, i64>(4)? as usize,
                        api_surface_flags: row.get::<_, i64>(5)? as usize,
                        business_rule_flags: row.get::<_, i64>(6)? as usize,
                        code_reference_flags: row.get::<_, i64>(7)? as usize,
                        critical_flags: row.get::<_, i64>(8)? as usize,
                        warning_flags: row.get::<_, i64>(9)? as usize,
                        info_flags: row.get::<_, i64>(10)? as usize,
                        highest_severity: highest
                            .as_deref()
                            .map(drift_severity_from_str)
                            .transpose()
                            .map_err(to_sql_err)?,
                        flags: Vec::new(),
                    })
                },
            )
            .optional()?;

        let Some(mut report) = report else {
            return Ok(None);
        };

        let mut stmt = conn.prepare(
            r#"
            SELECT
                flag_id,
                domain,
                kind,
                severity,
                fingerprint,
                doc_path,
                code_path,
                symbol_name,
                rule_ids_json,
                message
            FROM livingdocs_drift_flags
            WHERE report_id = ?1
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 0
                    WHEN 'warning' THEN 1
                    ELSE 2
                END,
                doc_path ASC,
                COALESCE(symbol_name, '') ASC,
                fingerprint ASC
            "#,
        )?;
        let flags = stmt.query_map(params![report.report_id.clone()], |row| {
            let domain: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let severity: String = row.get(3)?;
            let code_path: Option<String> = row.get(6)?;
            let symbol_name: Option<String> = row.get(7)?;
            let rule_ids_json: String = row.get(8)?;
            Ok(StoredDriftFlag {
                flag_id: row.get(0)?,
                domain: drift_domain_from_str(&domain).map_err(to_sql_err)?,
                kind: drift_kind_from_str(&kind).map_err(to_sql_err)?,
                severity: drift_severity_from_str(&severity).map_err(to_sql_err)?,
                fingerprint: row.get(4)?,
                doc_path: PathBuf::from(row.get::<_, String>(5)?),
                code_path: code_path.map(PathBuf::from),
                symbol_name,
                rule_ids: serde_json::from_str(&rule_ids_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        rule_ids_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?,
                message: row.get(9)?,
            })
        })?;

        report.flags = flags.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Some(report))
    }

    pub fn latest_confidence_audit_action_id(
        &self,
        report_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                r#"
                SELECT action_id
                FROM livingdocs_confidence_audit
                WHERE report_id = ?1
                ORDER BY created_at_ms DESC, rowid DESC
                LIMIT 1
                "#,
                params![report_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(row)
    }

    pub fn pending_review_count(&self, workspace_root: &Path) -> Result<i32> {
        let conn = self.connect()?;
        let ws = workspace_root.display().to_string();
        let n: i64 = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM livingdocs_reconcile_reviews
            WHERE workspace_root = ?1 AND status = ?2
            "#,
            params![ws, review_status_pending()],
            |row| row.get(0),
        )?;
        Ok(n as i32)
    }

    pub fn list_pending_reviews(
        &self,
        workspace_root: &Path,
        page_size: u32,
        page_offset: u32,
    ) -> Result<Vec<(ReconcileReviewRow, Option<String>, Option<String>, Option<String>)>> {
        let conn = self.connect()?;
        let ws = workspace_root.display().to_string();
        let limit = page_size.max(1).min(500);
        let offset = page_offset;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                r.review_id,
                r.report_id,
                r.workspace_root,
                r.created_at_ms,
                r.confidence_score,
                r.breakdown_json,
                r.decision,
                r.status,
                (
                    SELECT f.doc_path
                    FROM livingdocs_drift_flags f
                    WHERE f.report_id = r.report_id
                    ORDER BY
                        CASE f.severity
                            WHEN 'critical' THEN 0
                            WHEN 'warning' THEN 1
                            ELSE 2
                        END,
                        f.doc_path
                    LIMIT 1
                ) AS primary_doc,
                (
                    SELECT dr.highest_severity
                    FROM livingdocs_drift_reports dr
                    WHERE dr.report_id = r.report_id
                    LIMIT 1
                ) AS highest_severity,
                (
                    SELECT f.message
                    FROM livingdocs_drift_flags f
                    WHERE f.report_id = r.report_id
                    ORDER BY
                        CASE f.severity
                            WHEN 'critical' THEN 0
                            WHEN 'warning' THEN 1
                            ELSE 2
                        END,
                        f.doc_path
                    LIMIT 1
                ) AS summary_line
            FROM livingdocs_reconcile_reviews r
            WHERE r.workspace_root = ?1 AND r.status = ?4
            ORDER BY r.created_at_ms ASC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;
        let rows = stmt.query_map(
            params![ws, limit as i64, offset as i64, review_status_pending()],
            |row| {
                let summary_raw: Option<String> = row.get(10)?;
                let summary = summary_raw.map(|s| {
                    if s.chars().count() > 200 {
                        format!("{}…", s.chars().take(200).collect::<String>())
                    } else {
                    s
                }
            });
            Ok((
                ReconcileReviewRow {
                    review_id: row.get(0)?,
                    report_id: row.get(1)?,
                    workspace_root: PathBuf::from(row.get::<_, String>(2)?),
                    created_at_ms: row.get(3)?,
                    confidence_score: row.get(4)?,
                    breakdown_json: row.get(5)?,
                    decision: row.get(6)?,
                    status: row.get(7)?,
                    notes: None,
                    resolution_choice: None,
                    server_resolution_id: None,
                    patch_receipt_id: None,
                    toon_changelog_entry_id: None,
                    resolution_error: None,
                },
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                summary,
            ))
            },
        )?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn reconcile_review_by_id(&self, review_id: &str) -> Result<Option<ReconcileReviewRow>> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                r#"
                SELECT
                    review_id,
                    report_id,
                    workspace_root,
                    created_at_ms,
                    confidence_score,
                    breakdown_json,
                    decision,
                    status,
                    notes,
                    resolution_choice,
                    server_resolution_id,
                    patch_receipt_id,
                    toon_changelog_entry_id,
                    resolution_error
                FROM livingdocs_reconcile_reviews
                WHERE review_id = ?1
                "#,
                params![review_id],
                |row| {
                    Ok(ReconcileReviewRow {
                        review_id: row.get(0)?,
                        report_id: row.get(1)?,
                        workspace_root: PathBuf::from(row.get::<_, String>(2)?),
                        created_at_ms: row.get(3)?,
                        confidence_score: row.get(4)?,
                        breakdown_json: row.get(5)?,
                        decision: row.get(6)?,
                        status: row.get(7)?,
                        notes: row.get(8)?,
                        resolution_choice: row.get(9)?,
                        server_resolution_id: row.get(10)?,
                        patch_receipt_id: row.get(11)?,
                        toon_changelog_entry_id: row.get(12)?,
                        resolution_error: row.get(13)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn submit_resolution(
        &self,
        review_id: &str,
        ssot_label: &str,
        client_resolution_id: &str,
        user_note: Option<&str>,
    ) -> Result<SubmitResolutionOutcome> {
        let mut conn = self.connect()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT server_resolution_id FROM livingdocs_resolution_dedupe WHERE client_resolution_id = ?1",
                params![client_resolution_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(sid) = existing {
            return Ok(SubmitResolutionOutcome::Duplicate {
                server_resolution_id: sid,
            });
        }

        let row = self.reconcile_review_by_id(review_id)?;
        let Some(review) = row else {
            return Ok(SubmitResolutionOutcome::NotFound);
        };
        if review.status != review_status_pending() {
            return Ok(SubmitResolutionOutcome::Conflict {
                reason: format!("review status is {}", review.status),
            });
        }

        let now = now_ms();
        let server_resolution_id = Uuid::new_v4().to_string();
        let notes = user_note.map(ToOwned::to_owned);
        let queued_status = queued_status_for_choice(ssot_label)?;

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            r#"
            UPDATE livingdocs_reconcile_reviews
            SET status = ?1,
                resolved_at_ms = NULL,
                resolver = 'grpc',
                notes = ?2,
                resolution_choice = ?3,
                server_resolution_id = ?4,
                patch_receipt_id = NULL,
                toon_changelog_entry_id = NULL,
                resolution_error = NULL
            WHERE review_id = ?5 AND status = ?6
            "#,
            params![
                queued_status,
                notes,
                ssot_label,
                &server_resolution_id,
                review_id,
                review_status_pending(),
            ],
        )?;
        if updated == 0 {
            tx.rollback()?;
            return Ok(SubmitResolutionOutcome::Conflict {
                reason: "review no longer pending".into(),
            });
        }
        tx.execute(
            r#"
            INSERT INTO livingdocs_resolution_dedupe (client_resolution_id, review_id, server_resolution_id, created_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                client_resolution_id,
                review_id,
                &server_resolution_id,
                now
            ],
        )?;
        tx.commit()?;

        info!(
            review_id = %review_id,
            server_resolution_id = %server_resolution_id,
            ssot = %ssot_label,
            "livingdocs review resolution queued via grpc"
        );

        Ok(SubmitResolutionOutcome::Ok {
            server_resolution_id,
        })
    }

    pub fn claim_next_resolution_work(
        &self,
        lease: Duration,
    ) -> Result<Option<PendingResolutionWorkItem>> {
        let mut conn = self.connect()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = now_ms();
        let lease_expires = now + lease.as_millis() as i64;
        let row = tx
            .query_row(
                r#"
                SELECT
                    review_id,
                    report_id,
                    workspace_root,
                    resolution_choice,
                    server_resolution_id,
                    confidence_score,
                    breakdown_json,
                    notes,
                    status
                FROM livingdocs_reconcile_reviews
                WHERE status IN (?1, ?2)
                ORDER BY created_at_ms ASC
                LIMIT 1
                "#,
                params![review_status_doc_update_queued(), review_status_code_update_queued()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        PathBuf::from(row.get::<_, String>(2)?),
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            review_id,
            report_id,
            workspace_root,
            choice,
            server_resolution_id,
            confidence_score,
            breakdown_json,
            notes,
            current_status,
        )) = row
        else {
            tx.commit()?;
            return Ok(None);
        };

        let choice = choice.ok_or_else(|| anyhow::anyhow!("missing resolution_choice"))?;
        let server_resolution_id =
            server_resolution_id.ok_or_else(|| anyhow::anyhow!("missing server_resolution_id"))?;
        let running_status = running_status_for_status(&current_status)?;
        tx.execute(
            r#"
            UPDATE livingdocs_reconcile_reviews
            SET status = ?2,
                resolution_lease_expires_at_ms = ?3
            WHERE review_id = ?1
            "#,
            params![review_id, running_status, lease_expires],
        )?;
        tx.commit()?;

        Ok(Some(PendingResolutionWorkItem {
            review_id,
            report_id,
            workspace_root,
            choice,
            server_resolution_id,
            confidence_score,
            breakdown_json,
            notes,
        }))
    }

    pub fn complete_resolution(
        &self,
        review_id: &str,
        final_status: &str,
        patch_receipt_id: Option<&str>,
        toon_changelog_entry_id: Option<&str>,
        resolution_error: Option<&str>,
    ) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        conn.execute(
            r#"
            UPDATE livingdocs_reconcile_reviews
            SET status = ?2,
                resolved_at_ms = ?3,
                patch_receipt_id = ?4,
                toon_changelog_entry_id = ?5,
                resolution_error = ?6,
                resolution_lease_expires_at_ms = NULL
            WHERE review_id = ?1
            "#,
            params![
                review_id,
                final_status,
                now,
                patch_receipt_id,
                toon_changelog_entry_id,
                resolution_error,
            ],
        )?;
        Ok(())
    }

    pub fn fail_resolution(&self, review_id: &str, failed_status: &str, error: &str) -> Result<()> {
        self.complete_resolution(review_id, failed_status, None, None, Some(error))
    }

    pub fn resolution_result(&self, review_id: &str) -> Result<Option<StoredResolutionResult>> {
        let conn = self.connect()?;
        let row = conn
            .query_row(
                r#"
                SELECT
                    review_id,
                    status,
                    server_resolution_id,
                    patch_receipt_id,
                    toon_changelog_entry_id,
                    resolution_error
                FROM livingdocs_reconcile_reviews
                WHERE review_id = ?1
                "#,
                params![review_id],
                |row| {
                    Ok(StoredResolutionResult {
                        review_id: row.get(0)?,
                        status: row.get(1)?,
                        server_resolution_id: row.get(2)?,
                        patch_receipt_id: row.get(3)?,
                        toon_changelog_entry_id: row.get(4)?,
                        resolution_error: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open queue database {}", self.db_path.display()))?;
        conn.busy_timeout(self.busy_timeout)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            "#,
        )?;
        Ok(conn)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Deterministic primary key for drift reports so SQLite + semantic retries do not duplicate rows (V-003).
fn drift_report_stable_id(workspace_root: &Path, trigger: &str, report: &DriftReport) -> String {
    let mut lines: BTreeSet<String> = BTreeSet::new();
    for f in &report.flags {
        lines.insert(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            f.fingerprint,
            drift_domain_as_str(&f.domain),
            drift_kind_as_str(&f.kind),
            drift_severity_as_str(&f.severity),
            f.doc_path.display(),
            f.code_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            f.symbol_name.as_deref().unwrap_or(""),
            f.rule_ids.join(","),
            f.message
        ));
    }
    let mut payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        workspace_root.display(),
        trigger,
        report.total_flags,
        report.api_surface_flags,
        report.business_rule_flags,
        report.code_reference_flags,
        report.critical_flags,
        report.warning_flags,
        report.info_flags,
        report
            .highest_severity
            .as_ref()
            .map(drift_severity_as_str)
            .unwrap_or("none"),
    );
    for line in lines {
        payload.push('\n');
        payload.push_str(&line);
    }
    let digest = blake3::hash(payload.as_bytes());
    format!("drift_{}", digest.to_hex())
}

fn status_from_str(value: &str) -> Result<JobStatus> {
    match value {
        "queued" => Ok(JobStatus::Queued),
        "running" => Ok(JobStatus::Running),
        "completed" => Ok(JobStatus::Completed),
        "failed" => Ok(JobStatus::Failed),
        other => anyhow::bail!("unknown job status: {other}"),
    }
}

pub(crate) fn review_status_pending() -> &'static str {
    "pending"
}

pub(crate) fn review_status_doc_update_queued() -> &'static str {
    "doc_update_queued"
}

pub(crate) fn review_status_doc_update_running() -> &'static str {
    "doc_update_running"
}

pub(crate) fn review_status_resolved_with_doc_update() -> &'static str {
    "resolved_with_doc_update"
}

pub(crate) fn review_status_doc_update_failed() -> &'static str {
    "doc_update_failed"
}

pub(crate) fn review_status_code_update_queued() -> &'static str {
    "code_update_queued"
}

pub(crate) fn review_status_code_update_running() -> &'static str {
    "code_update_running"
}

pub(crate) fn review_status_resolved_with_code_update() -> &'static str {
    "resolved_with_code_update"
}

pub(crate) fn review_status_code_update_failed() -> &'static str {
    "code_update_failed"
}

pub(crate) fn queued_status_for_choice(choice: &str) -> Result<&'static str> {
    match choice {
        "update_doc" => Ok(review_status_doc_update_queued()),
        "update_code" => Ok(review_status_code_update_queued()),
        other => anyhow::bail!("unknown ssot choice: {other}"),
    }
}

pub(crate) fn running_status_for_status(status: &str) -> Result<&'static str> {
    match status {
        "doc_update_queued" => Ok(review_status_doc_update_running()),
        "code_update_queued" => Ok(review_status_code_update_running()),
        other => anyhow::bail!("unknown queued resolution status: {other}"),
    }
}

pub(crate) fn resolved_status_for_choice(choice: &str) -> Result<&'static str> {
    match choice {
        "update_doc" => Ok(review_status_resolved_with_doc_update()),
        "update_code" => Ok(review_status_resolved_with_code_update()),
        other => anyhow::bail!("unknown ssot choice: {other}"),
    }
}

pub(crate) fn failed_status_for_choice(choice: &str) -> Result<&'static str> {
    match choice {
        "update_doc" => Ok(review_status_doc_update_failed()),
        "update_code" => Ok(review_status_code_update_failed()),
        other => anyhow::bail!("unknown ssot choice: {other}"),
    }
}

fn drift_domain_as_str(domain: &DriftDomain) -> &'static str {
    match domain {
        DriftDomain::ApiSurface => "api_surface",
        DriftDomain::BusinessRule => "business_rule",
        DriftDomain::CodeReference => "code_reference",
    }
}

fn drift_domain_from_str(value: &str) -> Result<DriftDomain> {
    match value {
        "api_surface" => Ok(DriftDomain::ApiSurface),
        "business_rule" => Ok(DriftDomain::BusinessRule),
        "code_reference" => Ok(DriftDomain::CodeReference),
        other => anyhow::bail!("unknown drift domain: {other}"),
    }
}

fn drift_kind_as_str(kind: &DriftKind) -> &'static str {
    match kind {
        DriftKind::MissingSymbol => "missing_symbol",
        DriftKind::SignatureMismatch => "signature_mismatch",
        DriftKind::MissingRuleBinding => "missing_rule_binding",
        DriftKind::StructuralDrift => "structural_drift",
        DriftKind::DeadCodeReference => "dead_code_reference",
    }
}

fn drift_kind_from_str(value: &str) -> Result<DriftKind> {
    match value {
        "missing_symbol" => Ok(DriftKind::MissingSymbol),
        "signature_mismatch" => Ok(DriftKind::SignatureMismatch),
        "missing_rule_binding" => Ok(DriftKind::MissingRuleBinding),
        "structural_drift" => Ok(DriftKind::StructuralDrift),
        "dead_code_reference" => Ok(DriftKind::DeadCodeReference),
        other => anyhow::bail!("unknown drift kind: {other}"),
    }
}

fn drift_severity_as_str(severity: &DriftSeverity) -> &'static str {
    match severity {
        DriftSeverity::Critical => "critical",
        DriftSeverity::Warning => "warning",
        DriftSeverity::Info => "info",
    }
}

fn drift_severity_from_str(value: &str) -> Result<DriftSeverity> {
    match value {
        "critical" => Ok(DriftSeverity::Critical),
        "warning" => Ok(DriftSeverity::Warning),
        "info" => Ok(DriftSeverity::Info),
        other => anyhow::bail!("unknown drift severity: {other}"),
    }
}

fn to_sql_err(err: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(err.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_docs::{DriftDomain, DriftKind, DriftSeverity, InconsistencyFlag};

    #[test]
    fn queue_supports_enqueue_dequeue_complete() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        queue
            .enqueue(EnqueueRequest {
                kind: JobKind::IncrementalSync,
                priority: 10,
                dedupe_key: "incremental:a".to_string(),
                payload: SyncJobPayload {
                    workspace_root: dir.path().display().to_string(),
                    scope: SyncScope::Paths(vec!["src/main.rs".to_string()]),
                    reason: "test".to_string(),
                    event_count: 1,
                },
                max_attempts: 3,
                run_after: Duration::ZERO,
            })
            .unwrap();

        let job = queue.dequeue(Duration::from_secs(30)).unwrap().unwrap();
        assert_eq!(job.kind, JobKind::IncrementalSync);
        assert_eq!(job.attempts, 1);

        queue.complete(&job.id).unwrap();
        assert_eq!(queue.queued_count().unwrap(), 0);
    }

    #[test]
    fn queue_retries_failed_jobs_until_budget_exhausted() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        queue
            .enqueue(EnqueueRequest {
                kind: JobKind::IncrementalSync,
                priority: 10,
                dedupe_key: "incremental:b".to_string(),
                payload: SyncJobPayload {
                    workspace_root: dir.path().display().to_string(),
                    scope: SyncScope::Paths(vec!["src/lib.rs".to_string()]),
                    reason: "test".to_string(),
                    event_count: 1,
                },
                max_attempts: 1,
                run_after: Duration::ZERO,
            })
            .unwrap();

        let job = queue.dequeue(Duration::from_secs(30)).unwrap().unwrap();
        queue.fail(&job, "boom", Duration::ZERO).unwrap();
        assert!(queue.dequeue(Duration::from_secs(30)).unwrap().is_none());
    }

    #[test]
    fn queue_persists_and_reads_latest_drift_report() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let workspace_root = dir.path().join("workspace");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "Documented symbol `resolvePlan` no longer exists.".to_string(),
                doc_path: workspace_root.join("akta-docs/03-business-logic/rules.md"),
                code_path: Some(workspace_root.join("src/lib/rules.ts")),
                symbol_name: Some("resolvePlan".to_string()),
                rule_ids: vec!["BR-001".to_string()],
                fingerprint: "fingerprint-1".to_string(),
            }],
        };

        queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap();

        let stored = queue
            .latest_drift_report(&workspace_root)
            .unwrap()
            .expect("stored report");
        assert_eq!(stored.trigger, "incremental_sync");
        assert_eq!(stored.total_flags, 1);
        assert_eq!(stored.highest_severity, Some(DriftSeverity::Critical));
        assert_eq!(stored.flags.len(), 1);
        assert_eq!(stored.flags[0].symbol_name.as_deref(), Some("resolvePlan"));
        assert_eq!(stored.flags[0].rule_ids, vec!["BR-001"]);
    }

    #[test]
    fn reconcile_review_enqueue_is_idempotent_per_report() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let workspace_root = dir.path().join("workspace");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "missing".into(),
                doc_path: workspace_root.join("akta-docs/x.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "fp".into(),
            }],
        };

        let report_id = queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap()
            .0;

        let first = queue
            .enqueue_reconcile_review(
                &report_id,
                &workspace_root,
                None,
                0.2,
                "{}",
                "review_required",
            )
            .unwrap();
        let second = queue
            .enqueue_reconcile_review(
                &report_id,
                &workspace_root,
                None,
                0.2,
                "{}",
                "review_required",
            )
            .unwrap();
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn record_confidence_action_round_trips_via_latest() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let workspace_root = dir.path().join("workspace");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "m".into(),
                doc_path: workspace_root.join("akta-docs/x.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "fp".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "t", &report)
            .unwrap()
            .0;

        let action_id = queue
            .record_confidence_action(
                &report_id,
                &workspace_root,
                "noop",
                Some(1.0),
                r#"{"after_severity":0.9}"#,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert!(!action_id.is_empty());

        let audit = queue
            .latest_confidence_audit_for_report(&report_id)
            .unwrap()
            .expect("audit row");
        assert_eq!(audit.decision, "noop");
        assert_eq!(audit.confidence_score, Some(1.0));
        assert_eq!(audit.breakdown_json, r#"{"after_severity":0.9}"#);
        assert!(audit.canonical_toon_json.is_none());
        assert!(audit.commit_id.is_none());
    }

    #[test]
    fn record_confidence_action_stores_update_fields() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let workspace_root = dir.path().join("workspace");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 0,
            business_rule_flags: 0,
            code_reference_flags: 1,
            critical_flags: 0,
            warning_flags: 0,
            info_flags: 1,
            highest_severity: Some(DriftSeverity::Info),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::CodeReference,
                kind: DriftKind::DeadCodeReference,
                severity: DriftSeverity::Info,
                message: "dead".into(),
                doc_path: workspace_root.join("akta-docs/06-technical/a.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "f".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "t", &report)
            .unwrap()
            .0;

        queue
            .record_confidence_action(
                &report_id,
                &workspace_root,
                "update_required",
                Some(0.88),
                "{}",
                Some(r#"{"v":1,"ts":"20250716143022","ty":"changed","d":"x","doc":"doc","slug":"slug","sha256_16":"abcd1234ef567890"}"#),
                None,
                Some("akta-docs/10-changelog/20250716143022_doc_changed_slug.md"),
                Some("autocommit_skipped_no_git_or_head"),
            )
            .unwrap();

        let audit = queue
            .latest_confidence_audit_for_report(&report_id)
            .unwrap()
            .expect("audit");
        assert_eq!(audit.decision, "update_required");
        assert_eq!(audit.confidence_score, Some(0.88));
        assert!(audit.canonical_toon_json.is_some());
        assert_eq!(
            audit.external_changelog_relpath.as_deref(),
            Some("akta-docs/10-changelog/20250716143022_doc_changed_slug.md")
        );
        assert_eq!(
            audit.autocommit_error.as_deref(),
            Some("autocommit_skipped_no_git_or_head")
        );
    }

    #[test]
    fn submit_resolution_resolves_pending_review_and_dedupes_by_client_id() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().join("workspace");
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "m".into(),
                doc_path: workspace_root.join("akta-docs/x.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "fp".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "t", &report)
            .unwrap()
            .0;
        let review_id = queue
            .enqueue_reconcile_review(
                &report_id,
                &workspace_root,
                None,
                0.4,
                r#"{"k":1}"#,
                "review_required",
            )
            .unwrap()
            .expect("review row");

        let first = queue
            .submit_resolution(&review_id, "update_doc", "client-a", None)
            .unwrap();
        let SubmitResolutionOutcome::Ok {
            server_resolution_id: sid1,
        } = first
        else {
            panic!("expected Ok, got {first:?}");
        };
        assert!(!sid1.is_empty());

        let dup = queue
            .submit_resolution(&review_id, "update_doc", "client-a", None)
            .unwrap();
        let SubmitResolutionOutcome::Duplicate {
            server_resolution_id: sid_dup,
        } = dup
        else {
            panic!("expected Duplicate, got {dup:?}");
        };
        assert_eq!(sid_dup, sid1);

        let conflict = queue
            .submit_resolution(&review_id, "update_code", "client-b", None)
            .unwrap();
        assert!(
            matches!(conflict, SubmitResolutionOutcome::Conflict { .. }),
            "expected Conflict, got {conflict:?}"
        );
    }

    #[test]
    fn submit_resolution_not_found_for_unknown_review() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let out = queue
            .submit_resolution("no-such-review", "update_doc", "c1", None)
            .unwrap();
        assert!(matches!(out, SubmitResolutionOutcome::NotFound));
    }

    #[test]
    fn drift_report_by_report_id_loads_flags() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().join("workspace");
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 0,
            warning_flags: 1,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Warning),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::BusinessRule,
                kind: DriftKind::SignatureMismatch,
                severity: DriftSeverity::Warning,
                message: "sig".into(),
                doc_path: workspace_root.join("docs/a.md"),
                code_path: Some(workspace_root.join("src/lib.rs")),
                symbol_name: Some("foo".into()),
                rule_ids: vec!["R1".into()],
                fingerprint: "fp2".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap()
            .0;
        let stored = queue
            .drift_report_by_report_id(&report_id)
            .unwrap()
            .expect("report");
        assert_eq!(stored.report_id, report_id);
        assert_eq!(stored.flags.len(), 1);
        assert_eq!(stored.flags[0].kind, DriftKind::SignatureMismatch);
        assert_eq!(stored.flags[0].symbol_name.as_deref(), Some("foo"));
    }

    #[test]
    fn persist_drift_report_same_content_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let workspace_root = dir.path().join("workspace");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "idempotent-msg".into(),
                doc_path: workspace_root.join("docs/x.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "idem-fp".into(),
            }],
        };
        let (id1, ins1) = queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap();
        let (id2, ins2) = queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap();
        assert_eq!(id1, id2);
        assert!(ins1);
        assert!(!ins2);
    }

    #[test]
    fn recover_stale_resolutions_requeues_expired_running_review() {
        use rusqlite::Connection;

        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().join("workspace");
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "lease-test".into(),
                doc_path: workspace_root.join("akta-docs/x.md"),
                code_path: None,
                symbol_name: None,
                rule_ids: vec![],
                fingerprint: "lease-fp".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "t", &report)
            .unwrap()
            .0;
        let review_id = queue
            .enqueue_reconcile_review(
                &report_id,
                &workspace_root,
                None,
                0.4,
                "{}",
                "review_required",
            )
            .unwrap()
            .expect("review");

        let SubmitResolutionOutcome::Ok { .. } = queue
            .submit_resolution(&review_id, "update_doc", "client-a", None)
            .unwrap()
        else {
            panic!("expected Ok");
        };

        let work = queue
            .claim_next_resolution_work(Duration::from_secs(60))
            .unwrap()
            .expect("claimed work");
        assert_eq!(work.review_id, review_id);

        let conn = Connection::open(dir.path().join("queue.db")).unwrap();
        conn.execute(
            "UPDATE livingdocs_reconcile_reviews SET resolution_lease_expires_at_ms = ?1 WHERE review_id = ?2",
            rusqlite::params![super::now_ms() - 60_000, review_id],
        )
        .unwrap();
        drop(conn);

        assert_eq!(queue.recover_stale_resolutions().unwrap(), 1);
        let work2 = queue
            .claim_next_resolution_work(Duration::from_secs(60))
            .unwrap()
            .expect("re-claimed after recovery");
        assert_eq!(work2.review_id, review_id);
    }
}
