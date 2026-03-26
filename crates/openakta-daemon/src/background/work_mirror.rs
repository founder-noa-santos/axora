//! Local SQLite mirror for work-management read models and daemon-only state.
//!
//! **Path:** `.openakta/work-management.db` under the workspace root ([`WorkMirror::path_for_workspace`]).
//!
//! # Mirror vs canonical (A14)
//!
//! **Canonical source of truth** for Mission Operating Layer aggregates is the hosted API (Postgres +
//! events). This file persists a **local-first cache** plus tables that exist **only** on disk.
//!
//! | Storage | Role | Canonical? | Notes |
//! |---------|------|------------|--------|
//! | `wm_read_models` | Full JSON snapshot of [`ReadModelResponse`] + `etag` + `checkpoint_seq` | **Yes** when refreshed from `WorkManagementGrpc::synced_read_model` or after a successful `submit_command` | Can **diverge** temporarily: [`WorkMirror::resolve_clarifications`] and `update_item_states` patch the embedded JSON **without** a round-trip to the API. The next successful API fetch overwrites the snapshot. |
//! | `wm_pending_commands` | Outbox: serialized [`CommandEnvelope`] per `client_command_id` | Command **application** is canonical on the server; this table tracks sync status (`pending` / `applied` / `failed`) | Written before the HTTP submit; marked after response. |
//! | `wm_local_clarification_answers` | Answers keyed by `(session_id, clarification_item_id)` | **Local-only** | Used when [`WorkMirror::resolve_clarifications`] runs (offline fallback). The normal gRPC path submits `record_clarification_answers` to the hosted API first, then refreshes the mirror from the canonical read model (AB9). |
//! | `wm_local_evidence` | [`EvidenceLinkView`] rows (traces, compiled plan, mission result, …) | **Local-only** | Not replicated to Postgres by this module; `storage_scope` is often `"local"`. |
//! | `wm_local_verification_index` | Denormalized index of verification runs | **Derived** from the last canonical read model passed to [`WorkMirror::upsert_verification_index`] | Convenience for local queries; rebuild from API snapshot. |
//! | `wm_persona_memory_index` | Denormalized persona / assignment / artifact index | **Derived** from the read model in [`WorkMirror::refresh_persona_memory_index`] | Rebuilt from canonical snapshot; not a separate source of truth. |
//!
//! # Implications for AB9 (clarifications)
//!
//! **Canonical path:** `WorkManagementGrpc::resolve_clarifications` submits command
//! `record_clarification_answers`, which appends to `wm_clarification_answers` and updates
//! `wm_clarification_items` in Postgres, then replaces the local read model from the API response.
//!
//! **Offline fallback:** if `submit_work_command` fails with transport-style [`openakta_api_client::ApiError`]
//! (unavailable / timeout / connection refused / circuit open), the handler patches the mirror only
//! (`wm_local_clarification_answers` + embedded JSON). Those answers are **not** in Postgres until a
//! later successful sync—gates that require server-side state should treat offline resolution as
//! non-canonical.
//!
//! **Compile readiness (AB11):** `work_plan_compiler::enforce_readiness` checks clarification rows in the
//! embedded read model; local `resolve_clarifications` patches set `status` to `answered` on those rows
//! before `compile_work_plan` runs, so unresolved items block compilation when they apply to the story.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use openakta_api_client::{CommandEnvelope, EvidenceLinkView, ReadModelResponse};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct WorkMirror {
    db_path: PathBuf,
    busy_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct StoredReadModel {
    pub model: ReadModelResponse,
    pub etag: String,
    pub checkpoint_seq: i64,
}

impl WorkMirror {
    pub fn path_for_workspace(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".openakta").join("work-management.db")
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let mirror = Self {
            db_path: path.into(),
            busy_timeout: Duration::from_secs(5),
        };
        mirror.init_schema()?;
        Ok(mirror)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS wm_read_models (
                workspace_id TEXT PRIMARY KEY,
                read_model_json TEXT NOT NULL,
                etag TEXT NOT NULL,
                checkpoint_seq INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS wm_pending_commands (
                client_command_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                command_json TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                last_error TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_wm_pending_commands_workspace
            ON wm_pending_commands(workspace_id, status, created_at_ms DESC);

            CREATE TABLE IF NOT EXISTS wm_local_clarification_answers (
                session_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                clarification_item_id TEXT NOT NULL,
                answer_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                PRIMARY KEY (session_id, clarification_item_id)
            );

            CREATE TABLE IF NOT EXISTS wm_local_evidence (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                subject_type TEXT NOT NULL,
                subject_id TEXT,
                artifact_kind TEXT NOT NULL,
                locator_json TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                storage_scope TEXT NOT NULL,
                preview_redacted TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_wm_local_evidence_workspace
            ON wm_local_evidence(workspace_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS wm_local_verification_index (
                verification_run_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                story_id TEXT,
                prepared_story_id TEXT,
                status TEXT NOT NULL,
                verification_stage TEXT NOT NULL,
                summary_json TEXT,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_wm_local_verification_index_workspace
            ON wm_local_verification_index(workspace_id, updated_at_ms DESC);

            CREATE TABLE IF NOT EXISTS wm_persona_memory_index (
                workspace_id TEXT NOT NULL,
                persona_id TEXT NOT NULL,
                story_id TEXT,
                prepared_story_id TEXT,
                memory_ref TEXT NOT NULL,
                memory_kind TEXT NOT NULL,
                summary TEXT,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (workspace_id, persona_id, memory_ref)
            );

            CREATE INDEX IF NOT EXISTS idx_wm_persona_memory_index_scope
            ON wm_persona_memory_index(workspace_id, persona_id, updated_at_ms DESC);
            "#,
        )?;
        Ok(())
    }

    pub fn read_model(&self, workspace_id: Uuid) -> Result<Option<StoredReadModel>> {
        let conn = self.connect()?;
        let row: Option<(String, String, i64)> = conn
            .query_row(
                r#"
                SELECT read_model_json, etag, checkpoint_seq
                FROM wm_read_models
                WHERE workspace_id = ?1
                "#,
                params![workspace_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((json, etag, checkpoint_seq)) = row else {
            return Ok(None);
        };
        let model = serde_json::from_str(&json).context("deserialize local work read model")?;
        Ok(Some(StoredReadModel {
            model,
            etag,
            checkpoint_seq,
        }))
    }

    pub fn upsert_read_model(
        &self,
        workspace_id: Uuid,
        etag: &str,
        read_model: &ReadModelResponse,
    ) -> Result<()> {
        let conn = self.connect()?;
        let json = serde_json::to_string(read_model).context("serialize local work read model")?;
        conn.execute(
            r#"
            INSERT INTO wm_read_models (workspace_id, read_model_json, etag, checkpoint_seq, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(workspace_id) DO UPDATE SET
                read_model_json = excluded.read_model_json,
                etag = excluded.etag,
                checkpoint_seq = excluded.checkpoint_seq,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                workspace_id.to_string(),
                json,
                etag,
                read_model.checkpoint_seq,
                now_ms(),
            ],
        )?;
        Ok(())
    }

    pub fn record_pending_command(
        &self,
        workspace_id: Uuid,
        command: &CommandEnvelope,
    ) -> Result<()> {
        let conn = self.connect()?;
        let json = serde_json::to_string(command).context("serialize pending command")?;
        let now = now_ms();
        conn.execute(
            r#"
            INSERT INTO wm_pending_commands
                (client_command_id, workspace_id, command_json, status, created_at_ms, updated_at_ms)
            VALUES (?1, ?2, ?3, 'pending', ?4, ?4)
            ON CONFLICT(client_command_id) DO UPDATE SET
                command_json = excluded.command_json,
                updated_at_ms = excluded.updated_at_ms,
                last_error = NULL
            "#,
            params![command.client_command_id.to_string(), workspace_id.to_string(), json, now],
        )?;
        Ok(())
    }

    pub fn mark_command_applied(&self, client_command_id: Uuid) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE wm_pending_commands SET status = 'applied', updated_at_ms = ?2, last_error = NULL WHERE client_command_id = ?1",
            params![client_command_id.to_string(), now_ms()],
        )?;
        Ok(())
    }

    pub fn mark_command_failed(&self, client_command_id: Uuid, error: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE wm_pending_commands SET status = 'failed', updated_at_ms = ?2, last_error = ?3 WHERE client_command_id = ?1",
            params![client_command_id.to_string(), now_ms(), error],
        )?;
        Ok(())
    }

    pub fn resolve_clarifications(
        &self,
        workspace_id: Uuid,
        session_id: &str,
        answers: &[(String, String)],
    ) -> Result<usize> {
        let mut stored = match self.read_model(workspace_id)? {
            Some(model) => model,
            None => return Ok(0),
        };

        let now = Utc::now();
        let mut resolved = 0usize;
        for (clarification_item_id, _answer_json) in answers {
            if let Some(item) = stored
                .model
                .clarifications
                .iter_mut()
                .find(|item| item.id.to_string() == *clarification_item_id)
            {
                item.status = "answered".to_string();
                item.answered_at = Some(now);
                resolved += 1;
            }
        }
        self.upsert_read_model(workspace_id, &stored.etag, &stored.model)?;

        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        for (clarification_item_id, answer_json) in answers {
            tx.execute(
                r#"
                INSERT INTO wm_local_clarification_answers
                    (session_id, workspace_id, clarification_item_id, answer_json, created_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(session_id, clarification_item_id) DO UPDATE SET
                    answer_json = excluded.answer_json,
                    created_at_ms = excluded.created_at_ms
                "#,
                params![
                    session_id,
                    workspace_id.to_string(),
                    clarification_item_id,
                    answer_json,
                    now_ms(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(resolved)
    }

    pub fn mark_items_queued_for_execution(
        &self,
        workspace_id: Uuid,
        work_item_ids: &[Uuid],
    ) -> Result<()> {
        self.update_item_states(
            workspace_id,
            work_item_ids,
            Some("in_progress"),
            "queued_for_llm",
        )
    }

    pub fn mark_items_executing(&self, workspace_id: Uuid, work_item_ids: &[Uuid]) -> Result<()> {
        self.update_item_states(
            workspace_id,
            work_item_ids,
            Some("in_progress"),
            "executing",
        )
    }

    pub fn mark_items_execution_succeeded(
        &self,
        workspace_id: Uuid,
        work_item_ids: &[Uuid],
    ) -> Result<()> {
        self.update_item_states(workspace_id, work_item_ids, Some("done"), "done")
    }

    pub fn mark_items_execution_failed(
        &self,
        workspace_id: Uuid,
        work_item_ids: &[Uuid],
    ) -> Result<()> {
        self.update_item_states(workspace_id, work_item_ids, None, "failed_terminal")
    }

    fn update_item_states(
        &self,
        workspace_id: Uuid,
        work_item_ids: &[Uuid],
        tracker_state: Option<&str>,
        run_state: &str,
    ) -> Result<()> {
        let mut stored = match self.read_model(workspace_id)? {
            Some(model) => model,
            None => return Ok(()),
        };
        let now = Utc::now();
        for item in &mut stored.model.work_items {
            if work_item_ids.contains(&item.id) {
                if let Some(next_tracker_state) = tracker_state {
                    item.tracker_state = next_tracker_state.to_string();
                }
                item.run_state = run_state.to_string();
                item.updated_at = now;
            }
        }
        self.upsert_read_model(workspace_id, &stored.etag, &stored.model)
    }

    pub fn append_evidence(&self, entry: &EvidenceLinkView) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT OR REPLACE INTO wm_local_evidence
                (id, workspace_id, subject_type, subject_id, artifact_kind, locator_json,
                 content_hash, storage_scope, preview_redacted, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                entry.id.to_string(),
                entry.workspace_id.to_string(),
                entry.subject_type,
                entry.subject_id.map(|value| value.to_string()),
                entry.artifact_kind,
                serde_json::to_string(&entry.locator_json)?,
                entry.content_hash,
                entry.storage_scope,
                entry.preview_redacted,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn upsert_verification_index(
        &self,
        workspace_id: Uuid,
        read_model: &ReadModelResponse,
    ) -> Result<()> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        for run in &read_model.verification_runs {
            tx.execute(
                r#"
                INSERT INTO wm_local_verification_index
                    (verification_run_id, workspace_id, story_id, prepared_story_id, status,
                     verification_stage, summary_json, updated_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(verification_run_id) DO UPDATE SET
                    status = excluded.status,
                    verification_stage = excluded.verification_stage,
                    summary_json = excluded.summary_json,
                    updated_at_ms = excluded.updated_at_ms,
                    story_id = excluded.story_id,
                    prepared_story_id = excluded.prepared_story_id
                "#,
                params![
                    run.id.to_string(),
                    workspace_id.to_string(),
                    run.story_id.map(|value| value.to_string()),
                    run.prepared_story_id.map(|value| value.to_string()),
                    run.status,
                    run.verification_stage,
                    run.summary_json.as_ref().map(serde_json::Value::to_string),
                    now_ms(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn refresh_persona_memory_index(
        &self,
        workspace_id: Uuid,
        read_model: &ReadModelResponse,
    ) -> Result<()> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM wm_persona_memory_index WHERE workspace_id = ?1",
            params![workspace_id.to_string()],
        )?;

        for artifact in &read_model.knowledge_artifacts {
            if let Some(persona_id) = &artifact.persona_id {
                tx.execute(
                    r#"
                    INSERT INTO wm_persona_memory_index
                        (workspace_id, persona_id, story_id, prepared_story_id, memory_ref, memory_kind, summary, updated_at_ms)
                    VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, ?6)
                    "#,
                    params![
                        workspace_id.to_string(),
                        persona_id,
                        artifact.id.to_string(),
                        artifact.artifact_kind,
                        artifact.title,
                        now_ms(),
                    ],
                )?;
            }
        }

        for assignment in &read_model.persona_assignments {
            tx.execute(
                r#"
                INSERT INTO wm_persona_memory_index
                    (workspace_id, persona_id, story_id, prepared_story_id, memory_ref, memory_kind, summary, updated_at_ms)
                VALUES (?1, ?2, NULL, NULL, ?3, 'assignment', ?4, ?5)
                ON CONFLICT(workspace_id, persona_id, memory_ref) DO UPDATE SET
                    summary = excluded.summary,
                    updated_at_ms = excluded.updated_at_ms
                "#,
                params![
                    workspace_id.to_string(),
                    assignment.persona_id,
                    assignment.id.to_string(),
                    format!("{}:{}", assignment.subject_type, assignment.assignment_role),
                    now_ms(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn list_evidence(&self, workspace_id: Uuid) -> Result<Vec<EvidenceLinkView>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workspace_id, subject_type, subject_id, artifact_kind, locator_json,
                   content_hash, storage_scope, preview_redacted, created_at
            FROM wm_local_evidence
            WHERE workspace_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map(params![workspace_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let workspace_id: String = row.get(1)?;
            let subject_id: Option<String> = row.get(3)?;
            let locator_json: String = row.get(5)?;
            let created_at: String = row.get(9)?;
            Ok(EvidenceLinkView {
                id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil()),
                workspace_id: Uuid::parse_str(&workspace_id).unwrap_or_else(|_| Uuid::nil()),
                subject_type: row.get(2)?,
                subject_id: subject_id.and_then(|value| Uuid::parse_str(&value).ok()),
                artifact_kind: row.get(4)?,
                locator_json: serde_json::from_str(&locator_json).unwrap_or_default(),
                content_hash: row.get(6)?,
                storage_scope: row.get(7)?,
                preview_redacted: row.get(8)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|value| value.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open {}", self.db_path.display()))?;
        conn.busy_timeout(self.busy_timeout)?;
        Ok(conn)
    }
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}
