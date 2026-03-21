//! gRPC server implementation

use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};
use uuid::Uuid;

use openakta_agents::blackboard_runtime::{BlackboardEntry, RuntimeBlackboard};
use openakta_agents::hitl::MissionHitlGate;
use openakta_proto::collective::v1::{
    collective_service_server::{CollectiveService, CollectiveServiceServer},
    Agent, AgentStatus, AnswerEnvelope, GetTaskRequest, GetTaskResponse, ListAgentsRequest,
    ListAgentsResponse, ListTasksRequest, ListTasksResponse, Message, MessageType,
    RegisterAgentRequest, RegisterAgentResponse, SendMessageRequest, StreamMessagesRequest,
    SubmitHitlAnswerRequest, SubmitHitlAnswerResponse, SubmitTaskRequest, SubmitTaskResponse, Task,
    TaskStatus, UnregisterAgentRequest,
};

use crate::{CoreConfig, CoreError, Result};

/// Collective service implementation
pub struct CollectiveServer {
    config: CoreConfig,
    agents: Arc<tokio::sync::RwLock<Vec<Agent>>>,
    message_bus: broadcast::Sender<Message>,
    hitl_gate: Option<Arc<MissionHitlGate>>,
    blackboard: Option<Arc<Mutex<RuntimeBlackboard>>>,
}

impl CollectiveServer {
    /// Create a new collective server (broadcast fan-out, no HITL/blackboard wiring).
    pub fn new(config: CoreConfig) -> Self {
        let (message_bus, _) = broadcast::channel(1024);
        Self {
            config,
            agents: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            message_bus,
            hitl_gate: None,
            blackboard: None,
        }
    }

    /// Collective server with shared HITL gate, message bus, and blackboard (daemon / integration).
    pub fn with_hitl_runtime(
        config: CoreConfig,
        message_bus: broadcast::Sender<Message>,
        hitl_gate: Arc<MissionHitlGate>,
        blackboard: Arc<Mutex<RuntimeBlackboard>>,
    ) -> Self {
        Self {
            config,
            agents: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            message_bus,
            hitl_gate: Some(hitl_gate),
            blackboard: Some(blackboard),
        }
    }

    /// Clone of the message bus sender for wiring `MissionHitlGate::new(..., Some((tx, rx)))`.
    pub fn message_bus(&self) -> broadcast::Sender<Message> {
        self.message_bus.clone()
    }

    /// Get the gRPC server
    pub fn into_service(self) -> CollectiveServiceServer<Self> {
        CollectiveServiceServer::new(self)
    }

    /// Start the server
    pub async fn serve(self) -> Result<()> {
        let addr = self
            .config
            .server_address()
            .parse()
            .map_err(|e| CoreError::Server(format!("Invalid address: {}", e)))?;

        info!("Starting Collective server on {}", addr);

        tonic::transport::Server::builder()
            .add_service(self.into_service())
            .serve(addr)
            .await
            .map_err(|e| CoreError::Server(e.to_string()))?;

        Ok(())
    }

    async fn ingest_hitl_answer(&self, answer: AnswerEnvelope) -> std::result::Result<(), Status> {
        let gate = self
            .hitl_gate
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("HITL gate not configured"))?;
        let outcome = gate
            .submit_answer(answer.clone())
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        if let Some(bb) = &self.blackboard {
            if outcome.suppress_global_blackboard {
                return Ok(());
            }
            let mut guard = bb.lock().await;
            let summary = serde_json::json!({
                "question_id": answer.question_id,
                "mission_id": answer.mission_id,
                "mode": answer.mode,
                "selected": answer.selected_option_ids,
                "has_free_text": answer.free_text.as_ref().map(|s| !s.is_empty()).unwrap_or(false),
            })
            .to_string();
            guard
                .publish(
                    BlackboardEntry {
                        id: format!("hitl_answer:{}", answer.question_id),
                        content: summary,
                    },
                    vec!["planner".to_string()],
                )
                .map_err(|e| Status::internal(e.to_string()))?;
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl CollectiveService for CollectiveServer {
    async fn register_agent(
        &self,
        request: Request<RegisterAgentRequest>,
    ) -> std::result::Result<Response<RegisterAgentResponse>, Status> {
        let req = request.into_inner();
        debug!("Registering agent: {}", req.name);

        let agent = Agent {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            role: req.role,
            status: AgentStatus::Idle as i32,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            metadata: req.metadata,
        };

        let mut agents = self.agents.write().await;
        agents.push(agent.clone());

        info!("Agent registered: {} ({})", agent.name, agent.id);

        Ok(Response::new(RegisterAgentResponse { agent: Some(agent) }))
    }

    async fn unregister_agent(
        &self,
        request: Request<UnregisterAgentRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let req = request.into_inner();
        debug!("Unregistering agent: {}", req.agent_id);

        let mut agents = self.agents.write().await;
        agents.retain(|a| a.id != req.agent_id);

        info!("Agent unregistered: {}", req.agent_id);

        Ok(Response::new(()))
    }

    async fn list_agents(
        &self,
        _request: Request<ListAgentsRequest>,
    ) -> std::result::Result<Response<ListAgentsResponse>, Status> {
        let agents = self.agents.read().await;
        Ok(Response::new(ListAgentsResponse {
            agents: agents.clone(),
        }))
    }

    async fn submit_task(
        &self,
        request: Request<SubmitTaskRequest>,
    ) -> std::result::Result<Response<SubmitTaskResponse>, Status> {
        let req = request.into_inner();
        debug!("Submitting task: {}", req.title);

        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: req.title,
            description: req.description,
            status: TaskStatus::Pending as i32,
            assignee_id: req.assignee_id,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            completed_at: None,
        };

        Ok(Response::new(SubmitTaskResponse { task: Some(task) }))
    }

    async fn get_task(
        &self,
        _request: Request<GetTaskRequest>,
    ) -> std::result::Result<Response<GetTaskResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn list_tasks(
        &self,
        _request: Request<ListTasksRequest>,
    ) -> std::result::Result<Response<ListTasksResponse>, Status> {
        Ok(Response::new(ListTasksResponse { tasks: vec![] }))
    }

    type StreamMessagesStream =
        Pin<Box<dyn Stream<Item = std::result::Result<Message, Status>> + Send>>;

    async fn stream_messages(
        &self,
        request: Request<StreamMessagesRequest>,
    ) -> std::result::Result<Response<Self::StreamMessagesStream>, Status> {
        let agent_id = request.into_inner().agent_id;
        let mut rx = self.message_bus.subscribe();

        let stream = async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if message_visible_to_subscriber(&msg, &agent_id) {
                            yield Ok(msg);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let req = request.into_inner();
        validate_typed_message_request(&req)?;

        let message_type =
            MessageType::try_from(req.message_type).unwrap_or(MessageType::Unspecified);
        if message_type == MessageType::Answer {
            if let Some(ans) = req.human_answer.clone() {
                if self.hitl_gate.is_some() {
                    self.ingest_hitl_answer(ans).await?;
                    return Ok(Response::new(()));
                }
            }
        }

        let message = Message {
            id: Uuid::new_v4().to_string(),
            sender_id: req.sender_id,
            recipient_id: req.recipient_id,
            message_type: req.message_type,
            content: req.content,
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            patch: req.patch,
            patch_receipt: req.patch_receipt,
            context_pack: req.context_pack,
            validation_result: req.validation_result,
            task_assignment: req.task_assignment,
            progress_update: req.progress_update,
            result_submission: req.result_submission,
            blocker_alert: req.blocker_alert,
            workflow_transition: req.workflow_transition,
            human_question: req.human_question,
            human_answer: req.human_answer,
        };

        self.message_bus.send(message).map_err(|e| {
            error!("message bus closed: {}", e);
            Status::internal("message bus closed")
        })?;

        Ok(Response::new(()))
    }

    async fn submit_hitl_answer(
        &self,
        request: Request<SubmitHitlAnswerRequest>,
    ) -> std::result::Result<Response<SubmitHitlAnswerResponse>, Status> {
        let answer = request
            .into_inner()
            .answer
            .ok_or_else(|| Status::invalid_argument("answer required"))?;
        self.ingest_hitl_answer(answer).await?;
        Ok(Response::new(SubmitHitlAnswerResponse {
            accepted: true,
            detail: String::new(),
        }))
    }
}

/// When `agent_id` is empty, all messages are visible (tests / admin). Otherwise scope delivery.
fn message_visible_to_subscriber(msg: &Message, agent_id: &str) -> bool {
    let sensitive_hitl_question = matches!(
        MessageType::try_from(msg.message_type),
        Ok(MessageType::Question)
    ) && msg.human_question.as_ref().is_some_and(|q| q.sensitive);

    if agent_id.is_empty() {
        // Wildcard taps must not observe session-scoped sensitive HITL (H9).
        return !sensitive_hitl_question;
    }
    let Ok(mt) = MessageType::try_from(msg.message_type) else {
        return false;
    };
    match mt {
        MessageType::Question => msg.recipient_id == agent_id,
        MessageType::Answer => {
            msg.recipient_id == agent_id
                || msg
                    .human_answer
                    .as_ref()
                    .is_some_and(|a| a.mission_id == agent_id)
        }
        _ => {
            msg.recipient_id.is_empty() || msg.recipient_id == agent_id || msg.sender_id == agent_id
        }
    }
}

#[allow(clippy::result_large_err)]
fn validate_typed_message_request(req: &SendMessageRequest) -> std::result::Result<(), Status> {
    let message_type = MessageType::try_from(req.message_type).unwrap_or(MessageType::Unspecified);
    let expects_empty_content = matches!(
        message_type,
        MessageType::Patch
            | MessageType::PatchResult
            | MessageType::ContextPack
            | MessageType::ValidationResult
            | MessageType::TaskAssignment
            | MessageType::ProgressUpdate
            | MessageType::ResultSubmission
            | MessageType::BlockerAlert
            | MessageType::WorkflowTransition
            | MessageType::Question
            | MessageType::Answer
    );

    if expects_empty_content && !req.content.trim().is_empty() {
        return Err(Status::invalid_argument(
            "typed orchestration messages must not use generic content",
        ));
    }

    let typed_present = match message_type {
        MessageType::Patch => req.patch.is_some(),
        MessageType::PatchResult => req.patch_receipt.is_some(),
        MessageType::ContextPack => req.context_pack.is_some(),
        MessageType::ValidationResult => req.validation_result.is_some(),
        MessageType::TaskAssignment => req.task_assignment.is_some(),
        MessageType::ProgressUpdate => req.progress_update.is_some(),
        MessageType::ResultSubmission => req.result_submission.is_some(),
        MessageType::BlockerAlert => req.blocker_alert.is_some(),
        MessageType::WorkflowTransition => req.workflow_transition.is_some(),
        MessageType::Question => req.human_question.is_some(),
        MessageType::Answer => req.human_answer.is_some(),
        _ => true,
    };

    if !typed_present {
        return Err(Status::invalid_argument(
            "typed orchestration payload missing for message type",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_agents::hitl::HitlConfig;
    use openakta_proto::collective::v1::{AnswerAuthor, QuestionKind, QuestionOption};

    #[tokio::test]
    async fn test_server_creation() {
        let config = CoreConfig::default();
        let server = CollectiveServer::new(config);
        assert!(server.agents.read().await.is_empty());
    }

    #[test]
    fn test_server_rejects_typed_message_with_generic_content() {
        let req = SendMessageRequest {
            sender_id: "agent-1".to_string(),
            recipient_id: "agent-2".to_string(),
            message_type: MessageType::TaskAssignment as i32,
            content: "{\"task_id\":\"task-1\"}".to_string(),
            patch: None,
            patch_receipt: None,
            context_pack: None,
            validation_result: None,
            task_assignment: Some(openakta_proto::collective::v1::TaskAssignment {
                task_id: "task-1".to_string(),
                title: "Title".to_string(),
                description: "Desc".to_string(),
                task_type: openakta_proto::collective::v1::TaskPayloadType::General as i32,
                target_files: Vec::new(),
                target_symbols: Vec::new(),
                token_budget: 10,
                context_pack: None,
            }),
            progress_update: None,
            result_submission: None,
            blocker_alert: None,
            workflow_transition: None,
            human_question: None,
            human_answer: None,
        };

        let result = validate_typed_message_request(&req);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn answer_message_submits_to_hitl_gate() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = CoreConfig::for_workspace(tmp.path().to_path_buf());
        let (bus, bus_rx) = broadcast::channel(16);
        let gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                checkpoint_dir: tmp.path().join("cp"),
                ..Default::default()
            },
            Some((bus.clone(), bus_rx)),
        ));
        gate.register_mission_start("m1").unwrap();
        let env = openakta_proto::collective::v1::QuestionEnvelope {
            question_id: String::new(),
            mission_id: "m1".into(),
            session_id: "sess-a".into(),
            turn_index: 0,
            text: "Pick".into(),
            kind: QuestionKind::Single as i32,
            options: vec![
                QuestionOption {
                    id: "x".into(),
                    label: "X".into(),
                    description: "".into(),
                    is_default: true,
                },
                QuestionOption {
                    id: "y".into(),
                    label: "Y".into(),
                    description: "".into(),
                    is_default: false,
                },
            ],
            constraints: Some(openakta_proto::collective::v1::QuestionConstraints {
                min_selections: 1,
                max_selections: 1,
                free_text_max_chars: None,
            }),
            expiry_token: None,
            sensitive: false,
            expires_at: None,
        };
        let qid = gate.raise_question(env, "m1").await.unwrap();

        let bb = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let server = CollectiveServer::with_hitl_runtime(cfg, bus, gate.clone(), bb.clone());

        server
            .send_message(Request::new(SendMessageRequest {
                sender_id: "human".into(),
                recipient_id: String::new(),
                message_type: MessageType::Answer as i32,
                content: String::new(),
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
                human_answer: Some(AnswerEnvelope {
                    question_id: qid.clone(),
                    mission_id: "m1".into(),
                    answered_by: AnswerAuthor::Human as i32,
                    mode: QuestionKind::Single as i32,
                    selected_option_ids: vec!["x".into()],
                    free_text: None,
                    answered_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                }),
            }))
            .await
            .unwrap();

        assert_eq!(
            gate.lifecycle_of("m1"),
            Some(openakta_proto::collective::v1::MissionLifecycleState::Running as i32)
        );
        let bb_guard = bb.lock().await;
        assert!(bb_guard
            .read("planner", &format!("hitl_answer:{qid}"))
            .is_some());
    }

    #[tokio::test]
    async fn stream_messages_scopes_by_agent_id() {
        let cfg = CoreConfig::default();
        let server = CollectiveServer::new(cfg);
        let bus = server.message_bus();

        let mut alice = server
            .stream_messages(Request::new(StreamMessagesRequest {
                agent_id: "alice".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        let mut bob = server
            .stream_messages(Request::new(StreamMessagesRequest {
                agent_id: "bob".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        let q = Message {
            id: "1".into(),
            sender_id: "hitl".into(),
            recipient_id: "alice".into(),
            message_type: MessageType::Question as i32,
            content: String::new(),
            timestamp: None,
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
            human_answer: None,
        };
        bus.send(q).unwrap();

        use tokio_stream::StreamExt;
        let a = tokio::time::timeout(std::time::Duration::from_secs(1), alice.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(a.recipient_id, "alice");

        let bob_waits =
            tokio::time::timeout(std::time::Duration::from_millis(150), bob.next()).await;
        assert!(
            bob_waits.is_err(),
            "bob should not receive alice-scoped message"
        );
    }
}
