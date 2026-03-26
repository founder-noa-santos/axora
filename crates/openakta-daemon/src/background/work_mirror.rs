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

    pub fn record_pending_command(&self, workspace_id: Uuid, command: &CommandEnvelope) -> Result<()> {
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
        self.update_item_states(workspace_id, work_item_ids, Some("in_progress"), "queued_for_llm")
    }

    pub fn mark_items_executing(&self, workspace_id: Uuid, work_item_ids: &[Uuid]) -> Result<()> {
        self.update_item_states(workspace_id, work_item_ids, Some("in_progress"), "executing")
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
