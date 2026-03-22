//! Human-in-the-loop (HITL) mission gate: questions, answers, checkpoints, metrics.
//!
//! Drives `pending_answer` ↔ `running` transitions, enforces caps and duplicate
//! detection, and optionally fans out `QuestionEnvelope` on the collective bus.

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use openakta_proto::collective::v1::{
    AnswerEnvelope, Message, MessageType, MissionLifecycleState, QuestionEnvelope, QuestionKind,
};
use prost::Message as ProtoMessage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Configuration for HITL behavior.
#[derive(Debug, Clone)]
pub struct HitlConfig {
    /// Hard cap on questions per mission (default 5).
    pub max_questions_per_mission: u32,
    /// Directory for crash-safe checkpoints (JSON + prost payloads).
    pub checkpoint_dir: PathBuf,
    /// When set, `expiry_token` is HMAC-signed on raise and verified on submit (`OPENAKTA_HITL_HMAC_SECRET`, hex).
    pub answer_hmac_secret: Option<Vec<u8>>,
}

impl Default for HitlConfig {
    fn default() -> Self {
        Self {
            max_questions_per_mission: 5,
            checkpoint_dir: PathBuf::from(".openakta/checkpoints"),
            answer_hmac_secret: None,
        }
    }
}

impl HitlConfig {
    /// Load `answer_hmac_secret` from `OPENAKTA_HITL_HMAC_SECRET` (hex-encoded key material).
    pub fn with_hmac_secret_from_env(mut self) -> Self {
        if let Ok(raw) = std::env::var("OPENAKTA_HITL_HMAC_SECRET") {
            let t = raw.trim();
            if !t.is_empty() {
                if let Ok(bytes) = hex::decode(t) {
                    if !bytes.is_empty() {
                        self.answer_hmac_secret = Some(bytes);
                    }
                }
            }
        }
        self
    }
}

/// Vendor-neutral counters (inspected in tests).
#[derive(Debug, Default)]
pub struct HitlMetrics {
    /// `agent.question.raised_total` — increment on successful raise.
    pub raised_total: AtomicU64,
    /// `agent.question.answered_total`
    pub answered_total: AtomicU64,
    /// `agent.question.timed_out_total`
    pub timed_out_total: AtomicU64,
    /// Sum of answer latencies in ms (histogram stand-in).
    pub pending_latency_ms_sum: AtomicU64,
    /// Samples for latency average / tests.
    pub pending_latency_samples: AtomicU64,
    /// Missions currently in `pending_answer` (best-effort gauge).
    pub pending_missions: AtomicU64,
}

/// HITL / mission lifecycle errors.
#[derive(Debug, Error)]
pub enum HitlError {
    #[error("DUPLICATE_QUESTION")]
    DuplicateQuestion,

    #[error("MAX_QUESTIONS_EXCEEDED")]
    MaxQuestionsExceeded,

    #[error("ALREADY_PENDING")]
    AlreadyPending,

    #[error("INVALID_STATE: {0}")]
    InvalidState(String),

    #[error("VALIDATION: {0}")]
    Validation(String),

    #[error("UNKNOWN_QUESTION")]
    UnknownQuestion,

    /// Second `register_answer_waiter` for the same `question_id` (V-011).
    #[error("WAITER_ALREADY_REGISTERED")]
    WaiterAlreadyRegistered,

    #[error("CHECKPOINT_IO: {0}")]
    CheckpointIo(String),

    #[error("AUTH: {0}")]
    Auth(String),
}

/// Result metadata from a successful `submit_answer` (routing / observability).
#[derive(Debug, Clone)]
pub struct HitlSubmitAnswerOutcome {
    /// When true, do not publish the answer onto global `SharedBlackboard` (sensitive mission/session path).
    pub suppress_global_blackboard: bool,
}

struct PendingQuestion {
    envelope: QuestionEnvelope,
    expires_at: Option<Instant>,
    raised_at: Instant,
}

struct MissionRecord {
    lifecycle: i32,
    questions_raised: u32,
    normalized_texts: HashSet<String>,
    option_fingerprints: HashSet<String>,
    pending: Option<PendingQuestion>,
    /// Total ReAct turns observed (optional hint for checkpoint).
    last_turn_index: u32,
}

impl MissionRecord {
    fn new_running() -> Self {
        Self {
            lifecycle: MissionLifecycleState::Running as i32,
            questions_raised: 0,
            normalized_texts: HashSet::new(),
            option_fingerprints: HashSet::new(),
            pending: None,
            last_turn_index: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CheckpointFileV1 {
    version: u32,
    mission_id: String,
    lifecycle: i32,
    questions_raised: u32,
    normalized_texts: Vec<String>,
    option_fingerprints: Vec<String>,
    last_turn_index: u32,
    pending_envelope_proto: Option<Vec<u8>>,
    /// Wall-clock expiry from the original `QuestionEnvelope.expires_at`.
    pending_expires_secs: Option<i64>,
    pending_expires_nanos: Option<i32>,
}

/// Shared gate: one runtime typically exposes a single `Arc<MissionHitlGate>`.
pub struct MissionHitlGate {
    inner: Mutex<HashMap<String, MissionRecord>>,
    /// Fast lookup for in-flight waiters keyed by `question_id`.
    waiters: DashMap<String, tokio::sync::oneshot::Sender<AnswerEnvelope>>,
    config: HitlConfig,
    stream: Option<broadcast::Sender<Message>>,
    /// Holds one subscriber so `broadcast::Sender::send` does not fail with zero receivers (H4).
    _broadcast_hold: Option<broadcast::Receiver<Message>>,
    pub metrics: HitlMetrics,
}

impl fmt::Debug for MissionHitlGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MissionHitlGate { .. }")
    }
}

impl MissionHitlGate {
    /// New gate. Pass `Some((tx, rx))` from [`broadcast::channel`] so one receiver is retained and
    /// transactional publish semantics can rely on `send` not failing due to an empty subscriber set (H4).
    pub fn new(
        config: HitlConfig,
        bus: Option<(broadcast::Sender<Message>, broadcast::Receiver<Message>)>,
    ) -> Self {
        let (stream, _broadcast_hold) = match bus {
            Some((tx, rx)) => (Some(tx), Some(rx)),
            None => (None, None),
        };
        Self {
            inner: Mutex::new(HashMap::new()),
            waiters: DashMap::new(),
            config,
            stream,
            _broadcast_hold,
            metrics: HitlMetrics::default(),
        }
    }

    /// Register a mission as running (Coordinator v2 entry).
    pub fn register_mission_start(&self, mission_id: &str) -> Result<(), HitlError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
        g.insert(mission_id.to_string(), MissionRecord::new_running());
        Ok(())
    }

    /// Mark mission completed (best-effort; does not fail if unknown).
    pub fn register_mission_complete(&self, mission_id: &str, success: bool) {
        let Ok(mut g) = self.inner.lock() else {
            return;
        };
        let Some(rec) = g.get_mut(mission_id) else {
            return;
        };
        if success {
            rec.lifecycle = MissionLifecycleState::Completed as i32;
        }
        let _ = fs_safe_remove(&self.checkpoint_path(mission_id));
    }

    /// True when destructive tools must be refused for this mission.
    pub fn should_block_destructive_tools(&self, mission_id: &str) -> bool {
        let Ok(g) = self.inner.lock() else {
            return false;
        };
        let Some(rec) = g.get(mission_id) else {
            return false;
        };
        rec.lifecycle == MissionLifecycleState::PendingAnswer as i32
    }

    /// Lifecycle snapshot (prost enum discriminant).
    pub fn lifecycle_of(&self, mission_id: &str) -> Option<i32> {
        let g = self.inner.lock().ok()?;
        g.get(mission_id).map(|r| r.lifecycle)
    }

    /// Transition to `pending_answer`, enforce anti-loop rules, checkpoint, optional stream.
    pub async fn raise_question(
        &self,
        mut envelope: QuestionEnvelope,
        mission_id: &str,
    ) -> Result<String, HitlError> {
        self.check_expiry_for_mission(mission_id)?;

        validate_envelope_shape(&envelope)?;

        let qid = if envelope.question_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            envelope.question_id.clone()
        };
        envelope.question_id = qid.clone();
        envelope.mission_id = mission_id.to_string();

        if let Some(ref secret) = self.config.answer_hmac_secret {
            let exp = envelope.expires_at.as_ref().ok_or_else(|| {
                HitlError::Validation(
                    "expires_at is required when HMAC-backed HITL is enabled".into(),
                )
            })?;
            envelope.expiry_token = Some(sign_hitl_token(secret, mission_id, &qid, exp)?);
        }

        let norm = normalize_text(&envelope.text);
        let opt_fp = fingerprint_options(&envelope.options);

        {
            let mut g = self
                .inner
                .lock()
                .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
            let rec = g
                .entry(mission_id.to_string())
                .or_insert_with(MissionRecord::new_running);

            if terminal_lifecycle(rec.lifecycle) {
                return Err(HitlError::InvalidState(
                    "mission is not accepting questions".to_string(),
                ));
            }

            if rec.pending.is_some() {
                return Err(HitlError::AlreadyPending);
            }

            if rec.questions_raised >= self.config.max_questions_per_mission {
                return Err(HitlError::MaxQuestionsExceeded);
            }

            if rec.normalized_texts.contains(&norm) {
                return Err(HitlError::DuplicateQuestion);
            }

            if rec.option_fingerprints.contains(&opt_fp) {
                return Err(HitlError::DuplicateQuestion);
            }

            rec.normalized_texts.insert(norm.clone());
            rec.option_fingerprints.insert(opt_fp.clone());
            rec.questions_raised += 1;
            rec.last_turn_index = envelope.turn_index;
            rec.lifecycle = MissionLifecycleState::PendingAnswer as i32;

            let expires_at = envelope.expires_at.as_ref().and_then(prost_ts_to_instant);

            rec.pending = Some(PendingQuestion {
                envelope: envelope.clone(),
                expires_at,
                raised_at: Instant::now(),
            });
        }

        if let Err(e) = self.persist_checkpoint(mission_id) {
            self.rollback_raise(mission_id, &norm, &opt_fp)?;
            return Err(e);
        }
        if let Err(e) = self.publish_question(&envelope) {
            self.rollback_raise(mission_id, &norm, &opt_fp)?;
            let _ = fs_safe_remove(&self.checkpoint_path(mission_id));
            return Err(e);
        }

        self.metrics.raised_total.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .pending_missions
            .fetch_add(1, Ordering::Relaxed);

        Ok(qid)
    }

    /// Validate and apply an answer; resumes `running` when successful.
    pub async fn submit_answer(
        &self,
        answer: AnswerEnvelope,
    ) -> Result<HitlSubmitAnswerOutcome, HitlError> {
        let mission_id = answer.mission_id.clone();
        self.check_expiry_for_mission(&mission_id)?;

        let (pending_envelope, raised_at) = {
            let mut g = self
                .inner
                .lock()
                .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
            let rec = g
                .get_mut(&mission_id)
                .ok_or_else(|| HitlError::InvalidState("unknown mission".to_string()))?;
            if rec.lifecycle != MissionLifecycleState::PendingAnswer as i32 {
                return Err(HitlError::InvalidState(
                    "mission not waiting for answer".to_string(),
                ));
            }
            let Some(p) = rec.pending.as_ref() else {
                return Err(HitlError::InvalidState("no pending question".to_string()));
            };
            if p.envelope.question_id != answer.question_id {
                return Err(HitlError::UnknownQuestion);
            }
            if let Some(ref secret) = self.config.answer_hmac_secret {
                let exp = p.envelope.expires_at.as_ref().ok_or_else(|| {
                    HitlError::Auth("pending question missing expires_at for HMAC mode".into())
                })?;
                let tok = p
                    .envelope
                    .expiry_token
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| HitlError::Auth("missing expiry_token".into()))?;
                verify_hitl_token(secret, tok, &mission_id, &p.envelope.question_id, exp)?;
            }
            if let Err(e) = validate_answer(&p.envelope, &answer) {
                return Err(e);
            }
            // Persist running state before mutating in-memory mission state so disk never lags
            // behind a successful transition (V-001).
            let file = Self::checkpoint_after_answer_applied(&mission_id, rec);
            self.write_checkpoint_v1(&file)?;
            let p = rec.pending.take().ok_or_else(|| {
                HitlError::InvalidState("no pending question after checkpoint write".to_string())
            })?;
            rec.lifecycle = MissionLifecycleState::Running as i32;
            (p.envelope, p.raised_at)
        };

        let suppress_global_blackboard = pending_envelope.sensitive;
        let session_id = pending_envelope.session_id.clone();

        let latency = raised_at.elapsed().as_millis() as u64;
        self.metrics
            .pending_latency_ms_sum
            .fetch_add(latency, Ordering::Relaxed);
        self.metrics
            .pending_latency_samples
            .fetch_add(1, Ordering::Relaxed);
        self.metrics.answered_total.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .pending_missions
            .fetch_sub(1, Ordering::Relaxed);
        if let Some(tx) = &self.stream {
            if !suppress_global_blackboard {
                let msg = build_answer_message(&answer, &session_id);
                tx.send(msg).map_err(|_| {
                    HitlError::InvalidState("answer event bus publish failed (closed)".into())
                })?;
            }
        }

        if let Some((_, w)) = self.waiters.remove(&pending_envelope.question_id) {
            let _ = w.send(answer);
        }

        Ok(HitlSubmitAnswerOutcome {
            suppress_global_blackboard,
        })
    }

    /// Register a oneshot to be resumed when this `question_id` is answered (Pattern A / CLI).
    /// Returns [`HitlError::WaiterAlreadyRegistered`] if a waiter is already registered for this id (V-011).
    pub fn register_answer_waiter(
        &self,
        question_id: &str,
    ) -> std::result::Result<tokio::sync::oneshot::Receiver<AnswerEnvelope>, HitlError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        match self.waiters.entry(question_id.to_string()) {
            Entry::Occupied(_) => Err(HitlError::WaiterAlreadyRegistered),
            Entry::Vacant(v) => {
                v.insert(tx);
                Ok(rx)
            }
        }
    }

    /// Cancels a mission from any state.
    pub fn cancel_mission(&self, mission_id: &str) -> Result<(), HitlError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
        let rec = g
            .entry(mission_id.to_string())
            .or_insert_with(MissionRecord::new_running);
        if rec.lifecycle == MissionLifecycleState::PendingAnswer as i32 {
            self.metrics
                .pending_missions
                .fetch_sub(1, Ordering::Relaxed);
        }
        rec.lifecycle = MissionLifecycleState::Cancelled as i32;
        rec.pending = None;
        let _ = fs_safe_remove(&self.checkpoint_path(mission_id));
        Ok(())
    }

    /// Scan pending question for expiry (timeout edge).
    pub fn check_expiry_for_mission(&self, mission_id: &str) -> Result<(), HitlError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
        let Some(rec) = g.get_mut(mission_id) else {
            return Ok(());
        };
        let Some(p) = rec.pending.as_ref() else {
            return Ok(());
        };
        // Timeout only when an expiry was declared on the envelope.
        if let Some(deadline) = p.expires_at {
            if Instant::now() >= deadline {
                rec.pending = None;
                rec.lifecycle = MissionLifecycleState::TimedOut as i32;
                self.metrics.timed_out_total.fetch_add(1, Ordering::Relaxed);
                self.metrics
                    .pending_missions
                    .fetch_sub(1, Ordering::Relaxed);
                let _ = fs_safe_remove(&self.checkpoint_path(mission_id));
                return Err(HitlError::InvalidState("TIMEOUT".to_string()));
            }
        }
        Ok(())
    }

    /// Restore mission state from disk (process restart).
    pub fn restore_checkpoint(&self, mission_id: &str) -> Result<(), HitlError> {
        let path = self.checkpoint_path(mission_id);
        let raw = std::fs::read(&path).map_err(|e| HitlError::CheckpointIo(e.to_string()))?;
        let parsed: CheckpointFileV1 =
            serde_json::from_slice(&raw).map_err(|e| HitlError::CheckpointIo(e.to_string()))?;

        if parsed.lifecycle == MissionLifecycleState::PendingAnswer as i32 {
            if let Some(bytes) = &parsed.pending_envelope_proto {
                if let Ok(env) = QuestionEnvelope::decode(&**bytes) {
                    if let Some(ts) = env.expires_at.as_ref() {
                        if let Some(wall) = system_time_from_proto(ts) {
                            if SystemTime::now() >= wall {
                                let mut g = self.inner.lock().map_err(|_| {
                                    HitlError::InvalidState("hitl mutex poisoned".to_string())
                                })?;
                                g.insert(
                                    mission_id.to_string(),
                                    MissionRecord {
                                        lifecycle: MissionLifecycleState::TimedOut as i32,
                                        questions_raised: parsed.questions_raised,
                                        normalized_texts: parsed
                                            .normalized_texts
                                            .iter()
                                            .cloned()
                                            .collect(),
                                        option_fingerprints: parsed
                                            .option_fingerprints
                                            .iter()
                                            .cloned()
                                            .collect(),
                                        pending: None,
                                        last_turn_index: parsed.last_turn_index,
                                    },
                                );
                                self.metrics.timed_out_total.fetch_add(1, Ordering::Relaxed);
                                let _ = fs_safe_remove(&path);
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        let pending = if let Some(bytes) = parsed.pending_envelope_proto {
            let env = QuestionEnvelope::decode(&*bytes)
                .map_err(|e| HitlError::CheckpointIo(e.to_string()))?;
            let expires_at = match (parsed.pending_expires_secs, parsed.pending_expires_nanos) {
                (Some(secs), Some(nanos)) => instant_from_wallclock(&prost_types::Timestamp {
                    seconds: secs,
                    nanos,
                }),
                (Some(secs), None) => instant_from_wallclock(&prost_types::Timestamp {
                    seconds: secs,
                    nanos: 0,
                }),
                _ => None,
            };
            Some(PendingQuestion {
                envelope: env,
                expires_at,
                raised_at: Instant::now(),
            })
        } else {
            None
        };

        let mut g = self
            .inner
            .lock()
            .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
        let lifecycle_was_pending =
            parsed.lifecycle == MissionLifecycleState::PendingAnswer as i32 && pending.is_some();

        g.insert(
            mission_id.to_string(),
            MissionRecord {
                lifecycle: parsed.lifecycle,
                questions_raised: parsed.questions_raised,
                normalized_texts: parsed.normalized_texts.into_iter().collect(),
                option_fingerprints: parsed.option_fingerprints.into_iter().collect(),
                pending,
                last_turn_index: parsed.last_turn_index,
            },
        );
        if lifecycle_was_pending {
            self.metrics
                .pending_missions
                .fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn checkpoint_path(&self, mission_id: &str) -> PathBuf {
        self.config
            .checkpoint_dir
            .join(format!("{mission_id}.json"))
    }

    fn mission_record_to_checkpoint_v1(mission_id: &str, rec: &MissionRecord) -> CheckpointFileV1 {
        let pending_bytes = rec.pending.as_ref().map(|p| p.envelope.encode_to_vec());
        let (pending_expires_secs, pending_expires_nanos) = rec
            .pending
            .as_ref()
            .and_then(|p| p.envelope.expires_at.as_ref())
            .map(|ts| (Some(ts.seconds), Some(ts.nanos)))
            .unwrap_or((None, None));

        CheckpointFileV1 {
            version: 1,
            mission_id: mission_id.to_string(),
            lifecycle: rec.lifecycle,
            questions_raised: rec.questions_raised,
            normalized_texts: rec.normalized_texts.iter().cloned().collect(),
            option_fingerprints: rec.option_fingerprints.iter().cloned().collect(),
            last_turn_index: rec.last_turn_index,
            pending_envelope_proto: pending_bytes,
            pending_expires_secs,
            pending_expires_nanos,
        }
    }

    /// Checkpoint snapshot after a validated answer: running with no pending question.
    fn checkpoint_after_answer_applied(mission_id: &str, rec: &MissionRecord) -> CheckpointFileV1 {
        CheckpointFileV1 {
            version: 1,
            mission_id: mission_id.to_string(),
            lifecycle: MissionLifecycleState::Running as i32,
            questions_raised: rec.questions_raised,
            normalized_texts: rec.normalized_texts.iter().cloned().collect(),
            option_fingerprints: rec.option_fingerprints.iter().cloned().collect(),
            last_turn_index: rec.last_turn_index,
            pending_envelope_proto: None,
            pending_expires_secs: None,
            pending_expires_nanos: None,
        }
    }

    fn write_checkpoint_v1(&self, file: &CheckpointFileV1) -> Result<(), HitlError> {
        let path = self.checkpoint_path(&file.mission_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| HitlError::CheckpointIo(e.to_string()))?;
        }

        let json =
            serde_json::to_vec_pretty(file).map_err(|e| HitlError::CheckpointIo(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| HitlError::CheckpointIo(e.to_string()))?;
        Ok(())
    }

    fn persist_checkpoint(&self, mission_id: &str) -> Result<(), HitlError> {
        let file = {
            let g = self
                .inner
                .lock()
                .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
            let rec = g
                .get(mission_id)
                .ok_or_else(|| HitlError::CheckpointIo("missing mission".to_string()))?;
            Self::mission_record_to_checkpoint_v1(mission_id, rec)
        };
        self.write_checkpoint_v1(&file)
    }

    fn publish_question(&self, envelope: &QuestionEnvelope) -> Result<(), HitlError> {
        let Some(tx) = &self.stream else {
            return Ok(());
        };
        let msg = build_question_message(envelope);
        tx.send(msg).map_err(|_| {
            HitlError::InvalidState(
                "question event bus publish failed (no holders or closed)".into(),
            )
        })?;
        Ok(())
    }

    fn rollback_raise(&self, mission_id: &str, norm: &str, opt_fp: &str) -> Result<(), HitlError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HitlError::InvalidState("hitl mutex poisoned".to_string()))?;
        let Some(rec) = g.get_mut(mission_id) else {
            return Ok(());
        };
        rec.pending = None;
        rec.lifecycle = MissionLifecycleState::Running as i32;
        rec.normalized_texts.remove(norm);
        rec.option_fingerprints.remove(opt_fp);
        rec.questions_raised = rec.questions_raised.saturating_sub(1);
        Ok(())
    }
}

fn terminal_lifecycle(lifecycle: i32) -> bool {
    matches!(
        lifecycle,
        x if x == MissionLifecycleState::Cancelled as i32
            || x == MissionLifecycleState::TimedOut as i32
            || x == MissionLifecycleState::Completed as i32
    )
}

fn normalize_text(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn fingerprint_options(options: &[openakta_proto::collective::v1::QuestionOption]) -> String {
    let mut ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    ids.sort_unstable();
    ids.join("|")
}

fn validate_envelope_shape(envelope: &QuestionEnvelope) -> Result<(), HitlError> {
    if envelope.kind == QuestionKind::Unspecified as i32 {
        return Err(HitlError::Validation("question kind required".into()));
    }
    if envelope.text.trim().is_empty() {
        return Err(HitlError::Validation("question text required".into()));
    }
    if let Some(c) = envelope.constraints.as_ref() {
        if c.min_selections > c.max_selections {
            return Err(HitlError::Validation(
                "constraints: min_selections exceeds max_selections".into(),
            ));
        }
    }

    match QuestionKind::try_from(envelope.kind).unwrap_or(QuestionKind::Unspecified) {
        QuestionKind::Single | QuestionKind::Multi | QuestionKind::Mixed => {
            if envelope.options.is_empty() {
                return Err(HitlError::Validation("options required for kind".into()));
            }
            let mut seen = HashSet::new();
            for o in &envelope.options {
                if o.id.is_empty() || !seen.insert(o.id.clone()) {
                    return Err(HitlError::Validation("duplicate or empty option id".into()));
                }
            }
            if QuestionKind::try_from(envelope.kind).unwrap() == QuestionKind::Single {
                let defaults = envelope.options.iter().filter(|o| o.is_default).count();
                if defaults != 1 {
                    return Err(HitlError::Validation(
                        "single-choice requires exactly one default option".into(),
                    ));
                }
            }
        }
        QuestionKind::FreeText => {}
        QuestionKind::Unspecified => {}
    }

    Ok(())
}

fn validate_answer(q: &QuestionEnvelope, a: &AnswerEnvelope) -> Result<(), HitlError> {
    if a.question_id != q.question_id || a.mission_id != q.mission_id {
        return Err(HitlError::Validation("answer ids mismatch".into()));
    }

    if q.kind != a.mode {
        return Err(HitlError::Validation(
            "answer mode does not match question kind".into(),
        ));
    }

    let mut seen_sel = HashSet::new();
    for id in &a.selected_option_ids {
        if !seen_sel.insert(id.as_str()) {
            return Err(HitlError::Validation(
                "duplicate selected_option_ids".into(),
            ));
        }
    }

    let valid_ids: HashSet<&str> = q.options.iter().map(|o| o.id.as_str()).collect();
    for id in &a.selected_option_ids {
        if !valid_ids.contains(id.as_str()) {
            return Err(HitlError::Validation("unknown option id in answer".into()));
        }
    }

    let c = q.constraints.as_ref();
    let min_s = c.map(|x| x.min_selections).unwrap_or(0);
    let max_s = c.map(|x| x.max_selections).unwrap_or(u32::MAX);

    match QuestionKind::try_from(q.kind).unwrap_or(QuestionKind::Unspecified) {
        QuestionKind::Single => {
            if a.selected_option_ids.len() != 1 {
                return Err(HitlError::Validation(
                    "single requires one selection".into(),
                ));
            }
            if !a.free_text.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                return Err(HitlError::Validation(
                    "single must not include free_text".into(),
                ));
            }
        }
        QuestionKind::Multi => {
            if a.free_text.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
                return Err(HitlError::Validation(
                    "multi must not include free_text".into(),
                ));
            }
            let n = a.selected_option_ids.len() as u32;
            if n < min_s || n > max_s {
                return Err(HitlError::Validation(
                    "multi selection count out of range".into(),
                ));
            }
        }
        QuestionKind::FreeText => {
            if !a.selected_option_ids.is_empty() {
                return Err(HitlError::Validation(
                    "free_text must not include selected_option_ids".into(),
                ));
            }
            let ft = a.free_text.clone().unwrap_or_default();
            if ft.is_empty() {
                return Err(HitlError::Validation("free_text answer required".into()));
            }
            if let Some(m) = c.and_then(|x| x.free_text_max_chars) {
                if ft.chars().count() as u32 > m {
                    return Err(HitlError::Validation("free_text too long".into()));
                }
            }
        }
        QuestionKind::Mixed => {
            let has_opts = !a.selected_option_ids.is_empty();
            let has_text = a.free_text.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
            if !has_opts && !has_text {
                return Err(HitlError::Validation(
                    "mixed requires options and/or free_text".into(),
                ));
            }
            let n = a.selected_option_ids.len() as u32;
            if n > 0 && (n < min_s || n > max_s) {
                return Err(HitlError::Validation(
                    "mixed option count out of range".into(),
                ));
            }
        }
        QuestionKind::Unspecified => {}
    }

    Ok(())
}

type HmacSha256 = Hmac<Sha256>;

fn hitl_hmac_payload(mission_id: &str, question_id: &str, exp: &prost_types::Timestamp) -> Vec<u8> {
    let mut v = Vec::with_capacity(
        mission_id.len() + question_id.len() + 2 + std::mem::size_of::<i64>() * 2,
    );
    v.extend_from_slice(mission_id.as_bytes());
    v.push(0);
    v.extend_from_slice(question_id.as_bytes());
    v.push(0);
    v.extend_from_slice(&exp.seconds.to_be_bytes());
    v.extend_from_slice(&exp.nanos.to_be_bytes());
    v
}

fn sign_hitl_token(
    secret: &[u8],
    mission_id: &str,
    question_id: &str,
    exp: &prost_types::Timestamp,
) -> Result<String, HitlError> {
    let payload = hitl_hmac_payload(mission_id, question_id, exp);
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| HitlError::Auth(format!("hmac key: {e}")))?;
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();
    Ok(format!("v1.{}", hex::encode(sig)))
}

fn verify_hitl_token(
    secret: &[u8],
    token: &str,
    mission_id: &str,
    question_id: &str,
    exp: &prost_types::Timestamp,
) -> Result<(), HitlError> {
    let body = token
        .strip_prefix("v1.")
        .ok_or_else(|| HitlError::Auth("token must start with v1.".into()))?;
    if body.len() != 64 {
        return Err(HitlError::Auth("token MAC length".into()));
    }
    let sig = hex::decode(body).map_err(|_| HitlError::Auth("token hex".into()))?;
    let payload = hitl_hmac_payload(mission_id, question_id, exp);
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| HitlError::Auth(format!("hmac key: {e}")))?;
    mac.update(&payload);
    mac.verify_slice(&sig)
        .map_err(|_| HitlError::Auth("HMAC verify failed".into()))
}

fn build_question_message(envelope: &QuestionEnvelope) -> Message {
    Message {
        id: Uuid::new_v4().to_string(),
        sender_id: "openakta-hitl".to_string(),
        recipient_id: envelope.session_id.clone(),
        message_type: MessageType::Question as i32,
        content: String::new(),
        timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
        patch: None,
        patch_receipt: None,
        context_pack: None,
        validation_result: None,
        task_assignment: None,
        progress_update: None,
        result_submission: None,
        blocker_alert: None,
        workflow_transition: None,
        human_question: Some(envelope.clone()),
        human_answer: None,
    }
}

fn build_answer_message(answer: &AnswerEnvelope, recipient_id: &str) -> Message {
    Message {
        id: Uuid::new_v4().to_string(),
        sender_id: "openakta-hitl".to_string(),
        recipient_id: recipient_id.to_string(),
        message_type: MessageType::Answer as i32,
        content: String::new(),
        timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
        patch: None,
        patch_receipt: None,
        context_pack: None,
        validation_result: None,
        task_assignment: None,
        progress_update: None,
        result_submission: None,
        blocker_alert: None,
        workflow_transition: None,
        human_question: None,
        human_answer: Some(answer.clone()),
    }
}

fn prost_ts_to_instant(ts: &prost_types::Timestamp) -> Option<Instant> {
    let target = system_time_from_proto(ts)?;
    let now = SystemTime::now();
    let until = target.duration_since(now).ok()?;
    Some(Instant::now() + until)
}

fn system_time_from_proto(ts: &prost_types::Timestamp) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::new(ts.seconds.max(0) as u64, ts.nanos as u32))
}

fn instant_from_wallclock(ts: &prost_types::Timestamp) -> Option<Instant> {
    let target = system_time_from_proto(ts)?;
    let now = SystemTime::now();
    let dur = target.duration_since(now).ok()?;
    Some(Instant::now() + dur)
}

fn fs_safe_remove(path: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(path);
    Ok(())
}

/// Redact sensitive free-text before logging (data governance).
pub fn redact_answer_for_logs(answer: &AnswerEnvelope) -> String {
    let mut out = format!(
        "question_id={} mission_id={} mode={} answered_by={} selected={:?}",
        answer.question_id,
        answer.mission_id,
        answer.mode,
        answer.answered_by,
        answer.selected_option_ids
    );
    if let Some(ref ft) = answer.free_text {
        out.push_str(" free_text=");
        out.push_str(&redact_free_text(ft));
    }
    out
}

fn redact_free_text(s: &str) -> String {
    if credential_heuristic(s) || should_redact_sensitive_tokens(s) {
        "[REDACTED]".to_string()
    } else {
        s.to_string()
    }
}

fn credential_heuristic(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("bearer ")
        || regex_likely_api_key(s)
}

fn regex_likely_api_key(s: &str) -> bool {
    // OpenAI-style sk-…, generic long alnum
    s.contains("sk-")
        || s.split_whitespace()
            .any(|t| t.len() >= 40 && t.chars().all(|c| c.is_alphanumeric()))
}

fn should_redact_sensitive_tokens(s: &str) -> bool {
    s.to_lowercase().contains("token=")
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_proto::collective::v1::{AnswerAuthor, QuestionOption};
    use std::sync::Arc;

    fn sample_constraints() -> openakta_proto::collective::v1::QuestionConstraints {
        openakta_proto::collective::v1::QuestionConstraints {
            min_selections: 1,
            max_selections: 1,
            free_text_max_chars: None,
        }
    }

    fn sample_envelope(mission: &str) -> QuestionEnvelope {
        QuestionEnvelope {
            question_id: String::new(),
            mission_id: mission.to_string(),
            session_id: "sess".into(),
            turn_index: 3,
            text: "Pick one".into(),
            kind: QuestionKind::Single as i32,
            options: vec![
                QuestionOption {
                    id: "a".into(),
                    label: "A".into(),
                    description: "".into(),
                    is_default: true,
                },
                QuestionOption {
                    id: "b".into(),
                    label: "B".into(),
                    description: "".into(),
                    is_default: false,
                },
            ],
            constraints: Some(sample_constraints()),
            expiry_token: None,
            sensitive: false,
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn reject_duplicate_selected_option_ids() {
        let gate = Arc::new(MissionHitlGate::new(HitlConfig::default(), None));
        gate.register_mission_start("m").unwrap();
        let env = QuestionEnvelope {
            question_id: String::new(),
            mission_id: "m".into(),
            session_id: "sess".into(),
            turn_index: 1,
            text: "Pick many".into(),
            kind: QuestionKind::Multi as i32,
            options: vec![
                QuestionOption {
                    id: "a".into(),
                    label: "A".into(),
                    description: "".into(),
                    is_default: false,
                },
                QuestionOption {
                    id: "b".into(),
                    label: "B".into(),
                    description: "".into(),
                    is_default: false,
                },
            ],
            constraints: Some(openakta_proto::collective::v1::QuestionConstraints {
                min_selections: 1,
                max_selections: 2,
                free_text_max_chars: None,
            }),
            expiry_token: None,
            sensitive: false,
            expires_at: None,
        };
        let qid = gate.raise_question(env, "m").await.unwrap();
        let err = gate
            .submit_answer(AnswerEnvelope {
                question_id: qid,
                mission_id: "m".into(),
                answered_by: AnswerAuthor::Human as i32,
                mode: QuestionKind::Multi as i32,
                selected_option_ids: vec!["a".into(), "a".into()],
                free_text: None,
                answered_at: None,
            })
            .await;
        assert!(matches!(err, Err(HitlError::Validation(_))));
    }

    #[tokio::test]
    async fn state_machine_raise_answer_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                max_questions_per_mission: 5,
                checkpoint_dir: dir.path().to_path_buf(),
                ..Default::default()
            },
            None,
        ));
        gate.register_mission_start("m1").unwrap();
        let mut env = sample_envelope("m1");
        let qid = gate.raise_question(env.clone(), "m1").await.unwrap();
        env.question_id = qid.clone();
        assert_eq!(
            gate.lifecycle_of("m1"),
            Some(MissionLifecycleState::PendingAnswer as i32)
        );

        let answer = AnswerEnvelope {
            question_id: qid,
            mission_id: "m1".into(),
            answered_by: AnswerAuthor::Human as i32,
            mode: QuestionKind::Single as i32,
            selected_option_ids: vec!["a".into()],
            free_text: None,
            answered_at: Some(prost_types::Timestamp::from(SystemTime::now())),
        };
        gate.submit_answer(answer).await.unwrap();
        assert_eq!(
            gate.lifecycle_of("m1"),
            Some(MissionLifecycleState::Running as i32)
        );
        assert!(gate.metrics.answered_total.load(Ordering::Relaxed) >= 1);
    }

    /// If the Running checkpoint cannot be written, memory must stay PendingAnswer (V-001).
    #[cfg(unix)]
    #[tokio::test]
    async fn submit_answer_checkpoint_fail_leaves_pending_answer() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let cfg = HitlConfig {
            checkpoint_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let gate = Arc::new(MissionHitlGate::new(cfg, None));
        gate.register_mission_start("m1").unwrap();
        let env = sample_envelope("m1");
        let qid = gate.raise_question(env, "m1").await.unwrap();

        let checkpoint_file = dir.path().join("m1.json");
        let mut perms = std::fs::metadata(&checkpoint_file)
            .unwrap()
            .permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(&checkpoint_file, perms).unwrap();

        let err = gate
            .submit_answer(AnswerEnvelope {
                question_id: qid,
                mission_id: "m1".into(),
                answered_by: AnswerAuthor::Human as i32,
                mode: QuestionKind::Single as i32,
                selected_option_ids: vec!["a".into()],
                free_text: None,
                answered_at: Some(prost_types::Timestamp::from(SystemTime::now())),
            })
            .await;
        assert!(matches!(err, Err(HitlError::CheckpointIo(_))));

        let mut perms = std::fs::metadata(&checkpoint_file)
            .unwrap()
            .permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&checkpoint_file, perms).unwrap();

        assert_eq!(
            gate.lifecycle_of("m1"),
            Some(MissionLifecycleState::PendingAnswer as i32)
        );
    }

    #[tokio::test]
    async fn sixth_question_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                max_questions_per_mission: 5,
                checkpoint_dir: dir.path().to_path_buf(),
                ..Default::default()
            },
            None,
        ));
        gate.register_mission_start("m").unwrap();
        for i in 0..5 {
            let mut env = sample_envelope("m");
            env.text = format!("Q{i}");
            env.options[0].id = format!("o{i}a");
            env.options[1].id = format!("o{i}b");
            let qid = gate.raise_question(env, "m").await.unwrap();
            gate.submit_answer(AnswerEnvelope {
                question_id: qid,
                mission_id: "m".into(),
                answered_by: AnswerAuthor::Human as i32,
                mode: QuestionKind::Single as i32,
                selected_option_ids: vec![format!("o{i}a")],
                free_text: None,
                answered_at: Some(prost_types::Timestamp::from(SystemTime::now())),
            })
            .await
            .unwrap();
        }
        let mut env = sample_envelope("m");
        env.text = "sixth".into();
        env.options[0].id = "x".into();
        env.options[1].id = "y".into();
        assert!(matches!(
            gate.raise_question(env, "m").await,
            Err(HitlError::MaxQuestionsExceeded)
        ));
    }

    #[tokio::test]
    async fn duplicate_text_rejected() {
        let gate = Arc::new(MissionHitlGate::new(HitlConfig::default(), None));
        gate.register_mission_start("m").unwrap();
        let mut e1 = sample_envelope("m");
        e1.text = "Same?".into();
        let qid = gate.raise_question(e1.clone(), "m").await.unwrap();
        gate.submit_answer(AnswerEnvelope {
            question_id: qid,
            mission_id: "m".into(),
            answered_by: AnswerAuthor::Human as i32,
            mode: QuestionKind::Single as i32,
            selected_option_ids: vec!["a".into()],
            free_text: None,
            answered_at: Some(prost_types::Timestamp::from(SystemTime::now())),
        })
        .await
        .unwrap();

        let mut e2 = sample_envelope("m");
        e2.text = "same?".into();
        assert!(matches!(
            gate.raise_question(e2, "m").await,
            Err(HitlError::DuplicateQuestion)
        ));
    }

    #[tokio::test]
    async fn checkpoint_survives_reload_pending() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = HitlConfig {
            checkpoint_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let gate = MissionHitlGate::new(cfg.clone(), None);
        gate.register_mission_start("m1").unwrap();
        let env = sample_envelope("m1");
        gate.raise_question(env.clone(), "m1").await.unwrap();

        let gate2 = MissionHitlGate::new(cfg, None);
        gate2.restore_checkpoint("m1").unwrap();
        assert_eq!(
            gate2.lifecycle_of("m1"),
            Some(MissionLifecycleState::PendingAnswer as i32)
        );
    }

    #[test]
    fn hitl_hmac_roundtrip_and_rejects_tamper() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let exp = prost_types::Timestamp {
            seconds: 999_999,
            nanos: 0,
        };
        let tok = sign_hitl_token(secret, "m1", "q1", &exp).unwrap();
        verify_hitl_token(secret, &tok, "m1", "q1", &exp).unwrap();
        assert!(verify_hitl_token(secret, &tok, "m2", "q1", &exp).is_err());
    }

    #[tokio::test]
    async fn duplicate_answer_waiter_rejected() {
        let gate = Arc::new(MissionHitlGate::new(HitlConfig::default(), None));
        gate.register_mission_start("m").unwrap();
        let env = sample_envelope("m");
        let qid = gate.raise_question(env, "m").await.unwrap();
        assert!(gate.register_answer_waiter(&qid).is_ok());
        assert!(matches!(
            gate.register_answer_waiter(&qid),
            Err(HitlError::WaiterAlreadyRegistered)
        ));
    }

    #[test]
    fn redaction_masks_secret_like_free_text() {
        let a = AnswerEnvelope {
            question_id: "q".into(),
            mission_id: "m".into(),
            answered_by: AnswerAuthor::Human as i32,
            mode: QuestionKind::FreeText as i32,
            selected_option_ids: vec![],
            free_text: Some("password=supersecret".into()),
            answered_at: None,
        };
        let s = redact_answer_for_logs(&a);
        assert!(s.contains("[REDACTED]"));
        assert!(!s.contains("supersecret"));
    }
}
