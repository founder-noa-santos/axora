//! Canonical execution events and per-session observability sinks.

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Canonical execution phase.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTracePhase {
    Requested,
    Approved,
    Started,
    Progress,
    Completed,
    Failed,
    Denied,
}

/// Canonical execution event kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventKind {
    Mission,
    Task,
    ProviderRequest,
    ToolCall,
    Retrieval,
    Approval,
    AgentAssignment,
    AgentResult,
}

/// Structured execution event used by runtime sinks and replay consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionTraceEvent {
    pub event_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub action_id: String,
    pub parent_action_id: Option<String>,
    pub event_kind: ExecutionEventKind,
    pub phase: ExecutionTracePhase,
    pub display_name: String,
    pub mission_id: String,
    pub task_id: String,
    pub turn_id: String,
    pub agent_id: String,
    pub provider_request_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message_count: Option<u32>,
    pub tool_call_count: Option<u32>,
    pub stop_reason: Option<String>,
    pub usage_preview: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_kind: Option<String>,
    pub tool_name: Option<String>,
    pub read_only: bool,
    pub mutating: bool,
    pub requires_approval: bool,
    pub target_path: Option<String>,
    pub target_symbol: Option<String>,
    pub query: Option<String>,
    pub args_preview: Option<String>,
    pub result_preview: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

impl ExecutionTraceEvent {
    /// Create a new canonical event with deterministic defaults.
    pub fn new(
        session_id: impl Into<String>,
        mission_id: impl Into<String>,
        task_id: impl Into<String>,
        turn_id: impl Into<String>,
        agent_id: impl Into<String>,
        event_kind: ExecutionEventKind,
        phase: ExecutionTracePhase,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            sequence: 0,
            timestamp: Utc::now().to_rfc3339(),
            action_id: Uuid::new_v4().to_string(),
            parent_action_id: None,
            event_kind,
            phase,
            display_name: display_name.into(),
            mission_id: mission_id.into(),
            task_id: task_id.into(),
            turn_id: turn_id.into(),
            agent_id: agent_id.into(),
            provider_request_id: None,
            provider: None,
            model: None,
            message_count: None,
            tool_call_count: None,
            stop_reason: None,
            usage_preview: None,
            tool_call_id: None,
            tool_kind: None,
            tool_name: None,
            read_only: true,
            mutating: false,
            requires_approval: false,
            target_path: None,
            target_symbol: None,
            query: None,
            args_preview: None,
            result_preview: None,
            error: None,
            duration_ms: None,
        }
    }
}

/// Per-session trace writer + broadcaster.
#[derive(Debug)]
pub struct ExecutionTraceService {
    session_id: String,
    log_path: PathBuf,
    sequence: AtomicU64,
    writer: Mutex<BufWriter<File>>,
    events: Mutex<Vec<ExecutionTraceEvent>>,
    tx: broadcast::Sender<ExecutionTraceEvent>,
    render_terminal: bool,
}

impl ExecutionTraceService {
    /// Create a session-backed trace service.
    pub fn new(
        session_id: impl Into<String>,
        log_dir: impl AsRef<Path>,
        render_terminal: bool,
    ) -> std::io::Result<Self> {
        let session_id = session_id.into();
        fs::create_dir_all(log_dir.as_ref())?;
        let log_path = log_dir.as_ref().join(format!("{session_id}.jsonl"));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let (tx, _) = broadcast::channel(1024);
        Ok(Self {
            session_id,
            log_path,
            sequence: AtomicU64::new(0),
            writer: Mutex::new(BufWriter::new(file)),
            events: Mutex::new(Vec::new()),
            tx,
            render_terminal,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionTraceEvent> {
        self.tx.subscribe()
    }

    pub fn snapshot(&self) -> Vec<ExecutionTraceEvent> {
        self.events.lock().clone()
    }

    /// Emit an event to JSONL, broadcast, and terminal views.
    pub fn emit(&self, mut event: ExecutionTraceEvent) -> std::io::Result<ExecutionTraceEvent> {
        if event.session_id.is_empty() {
            event.session_id = self.session_id.clone();
        }
        if event.event_id.is_empty() {
            event.event_id = Uuid::new_v4().to_string();
        }
        if event.timestamp.is_empty() {
            event.timestamp = Utc::now().to_rfc3339();
        }
        if event.action_id.is_empty() {
            event.action_id = Uuid::new_v4().to_string();
        }
        event.sequence = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;

        {
            let mut writer = self.writer.lock();
            serde_json::to_writer(&mut *writer, &event)
                .map_err(|err| std::io::Error::other(err.to_string()))?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }

        self.events.lock().push(event.clone());
        let _ = self.tx.send(event.clone());

        if self.render_terminal {
            for line in ExecutionSummaryRenderer::render(&event) {
                eprintln!("{line}");
            }
        }

        Ok(event)
    }
}

/// Session registry used by runtime and gRPC consumers.
#[derive(Debug)]
pub struct ExecutionTraceRegistry {
    log_dir: PathBuf,
    sessions: DashMap<String, Arc<ExecutionTraceService>>,
    mission_sessions: DashMap<String, String>,
}

impl ExecutionTraceRegistry {
    pub fn new(log_dir: impl Into<PathBuf>) -> Self {
        Self {
            log_dir: log_dir.into(),
            sessions: DashMap::new(),
            mission_sessions: DashMap::new(),
        }
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn create_session(
        &self,
        session_id: impl Into<String>,
        render_terminal: bool,
    ) -> std::io::Result<Arc<ExecutionTraceService>> {
        let service = Arc::new(ExecutionTraceService::new(
            session_id.into(),
            &self.log_dir,
            render_terminal,
        )?);
        self.sessions
            .insert(service.session_id().to_string(), Arc::clone(&service));
        Ok(service)
    }

    pub fn register_mission(&self, session_id: &str, mission_id: &str) {
        self.mission_sessions
            .insert(mission_id.to_string(), session_id.to_string());
    }

    pub fn session_for_mission(&self, mission_id: &str) -> Option<String> {
        self.mission_sessions
            .get(mission_id)
            .map(|value| value.value().clone())
            .or_else(|| find_session_id_by_mission(&self.log_dir, mission_id))
    }

    pub fn service(&self, session_id: &str) -> Option<Arc<ExecutionTraceService>> {
        self.sessions
            .get(session_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    pub fn emit(&self, event: ExecutionTraceEvent) -> std::io::Result<Option<ExecutionTraceEvent>> {
        match self.service(&event.session_id) {
            Some(service) => service.emit(event).map(Some),
            None => Ok(None),
        }
    }
}

/// Deterministic terminal projection for canonical events.
pub struct ExecutionSummaryRenderer;

impl ExecutionSummaryRenderer {
    pub fn render(event: &ExecutionTraceEvent) -> Vec<String> {
        let prefix = match event.event_kind {
            ExecutionEventKind::Mission => "MISSION",
            ExecutionEventKind::Task => "TASK",
            ExecutionEventKind::ProviderRequest => "PROVIDER",
            ExecutionEventKind::ToolCall => "TOOL",
            ExecutionEventKind::Retrieval => "RETRIEVAL",
            ExecutionEventKind::Approval => "APPROVAL",
            ExecutionEventKind::AgentAssignment => "ASSIGN",
            ExecutionEventKind::AgentResult => "RESULT",
        };

        let mut headline = format!(
            "[{prefix}] seq={} phase={:?} action={} label={}",
            event.sequence, event.phase, event.action_id, event.display_name
        );
        if !event.mission_id.is_empty() {
            headline.push_str(&format!(" mission={}", event.mission_id));
        }
        if !event.task_id.is_empty() {
            headline.push_str(&format!(" task={}", event.task_id));
        }

        let mut lines = vec![headline];
        let mut details = Vec::new();

        if let Some(provider) = &event.provider {
            details.push(format!("provider={provider}"));
        }
        if let Some(model) = &event.model {
            details.push(format!("model={model}"));
        }
        if let Some(name) = &event.tool_name {
            details.push(format!("tool={name}"));
        }
        if let Some(kind) = &event.tool_kind {
            details.push(format!("tool_kind={kind}"));
        }
        if let Some(path) = &event.target_path {
            details.push(format!("path={path}"));
        }
        if let Some(symbol) = &event.target_symbol {
            details.push(format!("symbol={symbol}"));
        }
        if let Some(query) = &event.query {
            details.push(format!("query={}", truncate_preview(query)));
        }
        if let Some(args) = &event.args_preview {
            details.push(format!("args={}", truncate_preview(args)));
        }
        if let Some(result) = &event.result_preview {
            details.push(format!("result={}", truncate_preview(result)));
        }
        if let Some(error) = &event.error {
            details.push(format!("error={}", truncate_preview(error)));
        }
        if let Some(duration_ms) = event.duration_ms {
            details.push(format!("duration_ms={duration_ms}"));
        }

        if !details.is_empty() {
            lines.push(format!("  {}", details.join(" ")));
        }

        lines
    }
}

pub fn read_session_events(
    log_dir: &Path,
    session_id: &str,
    from_sequence: u64,
) -> std::io::Result<Vec<ExecutionTraceEvent>> {
    let path = log_dir.join(format!("{session_id}.jsonl"));
    read_events_from_path(&path, from_sequence)
}

pub fn read_events_from_path(
    path: &Path,
    from_sequence: u64,
) -> std::io::Result<Vec<ExecutionTraceEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let reader = BufReader::new(File::open(path)?);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: ExecutionTraceEvent =
            serde_json::from_str(&line).map_err(|err| std::io::Error::other(err.to_string()))?;
        if event.sequence >= from_sequence {
            events.push(event);
        }
    }
    Ok(events)
}

pub fn find_session_id_by_mission(log_dir: &Path, mission_id: &str) -> Option<String> {
    let entries = fs::read_dir(log_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        if let Ok(events) = read_events_from_path(&path, 0) {
            if events.iter().any(|event| event.mission_id == mission_id) {
                return path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string);
            }
        }
    }
    None
}

fn truncate_preview(value: &str) -> String {
    const LIMIT: usize = 120;
    let trimmed = value.replace('\n', "\\n");
    if trimmed.len() <= LIMIT {
        trimmed
    } else {
        format!("{}...", &trimmed[..LIMIT])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn execution_trace_service_assigns_strictly_increasing_sequence_and_replays_jsonl() {
        let tempdir = tempdir().unwrap();
        let service = ExecutionTraceService::new("s1", tempdir.path(), false).unwrap();
        let first = service
            .emit(ExecutionTraceEvent::new(
                "s1",
                "m1",
                "t1",
                "turn-1",
                "agent",
                ExecutionEventKind::Mission,
                ExecutionTracePhase::Started,
                "mission",
            ))
            .unwrap();
        let second = service
            .emit(ExecutionTraceEvent::new(
                "s1",
                "m1",
                "t1",
                "turn-1",
                "agent",
                ExecutionEventKind::Task,
                ExecutionTracePhase::Completed,
                "task",
            ))
            .unwrap();

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);

        let replayed = read_session_events(tempdir.path(), "s1", 0).unwrap();
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].sequence, 1);
        assert_eq!(replayed[1].sequence, 2);
    }

    #[test]
    fn renderer_is_deterministic_for_fixed_event_fixture() {
        let mut event = ExecutionTraceEvent::new(
            "s1",
            "m1",
            "t1",
            "turn-1",
            "agent-1",
            ExecutionEventKind::ToolCall,
            ExecutionTracePhase::Completed,
            "apply patch",
        );
        event.sequence = 7;
        event.action_id = "tool-7".to_string();
        event.tool_name = Some("apply_patch".to_string());
        event.tool_kind = Some("mutation".to_string());
        event.target_path = Some("src/lib.rs".to_string());
        event.args_preview = Some("{\"path\":\"src/lib.rs\"}".to_string());
        event.result_preview = Some("updated 1 file".to_string());
        event.duration_ms = Some(42);

        assert_eq!(
            ExecutionSummaryRenderer::render(&event),
            vec![
                "[TOOL] seq=7 phase=Completed action=tool-7 label=apply patch mission=m1 task=t1"
                    .to_string(),
                "  tool=apply_patch tool_kind=mutation path=src/lib.rs args={\"path\":\"src/lib.rs\"} result=updated 1 file duration_ms=42"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn registry_can_resolve_session_by_mission_from_jsonl_history() {
        let tempdir = tempdir().unwrap();
        let service = ExecutionTraceService::new("session-a", tempdir.path(), false).unwrap();
        service
            .emit(ExecutionTraceEvent::new(
                "session-a",
                "mission-a",
                "task-a",
                "turn-1",
                "agent",
                ExecutionEventKind::Mission,
                ExecutionTracePhase::Started,
                "mission",
            ))
            .unwrap();

        assert_eq!(
            find_session_id_by_mission(tempdir.path(), "mission-a").as_deref(),
            Some("session-a")
        );
    }
}
