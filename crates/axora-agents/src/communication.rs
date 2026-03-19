//! Inter-agent communication using NATS

use crate::patch_protocol::{
    ContextPack, PatchEnvelope, PatchReceipt, ValidationResult as PatchValidationResult,
};
use crate::transport::{
    InternalBlockerAlert, InternalProgressUpdate, InternalResultSubmission, InternalTaskAssignment,
    InternalWorkflowTransitionEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID
    pub message_id: String,
    /// Conversation thread ID
    pub thread_id: String,
    /// Message type
    pub message_type: MessageType,
    /// Sender agent ID
    pub sender_id: String,
    /// Recipient agent ID (None for broadcast)
    pub recipient_id: Option<String>,
    /// Message content
    pub content: String,
    /// Typed patch envelope.
    pub patch: Option<PatchEnvelope>,
    /// Typed deterministic patch receipt.
    pub patch_receipt: Option<PatchReceipt>,
    /// Typed context pack.
    pub context_pack: Option<ContextPack>,
    /// Typed validation result.
    pub validation_result: Option<PatchValidationResult>,
    /// Typed task assignment.
    pub task_assignment: Option<InternalTaskAssignment>,
    /// Typed progress update.
    pub progress_update: Option<InternalProgressUpdate>,
    /// Typed result submission.
    pub result_submission: Option<InternalResultSubmission>,
    /// Typed blocker alert.
    pub blocker_alert: Option<InternalBlockerAlert>,
    /// Typed workflow transition event.
    pub workflow_transition: Option<InternalWorkflowTransitionEvent>,
    /// Timestamp
    pub timestamp: u64,
    /// Time to live in seconds
    pub ttl_seconds: Option<u32>,
}

/// Type of message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Task assignment
    TaskAssign,
    /// Task result
    TaskResult,
    /// Status update
    StatusUpdate,
    /// Information request
    InfoRequest,
    /// Information response
    InfoResponse,
    /// Coordination (lock request, etc.)
    Coordination,
    /// Meta-communication (capability advertisement, etc.)
    Meta,
    /// Diff or AST patch envelope
    Patch,
    /// Deterministic patch application result
    PatchResult,
    /// Context pack prepared for a worker
    ContextPack,
    /// Validation result for an agent output
    ValidationResult,
    /// Typed task assignment
    TypedTaskAssignment,
    /// Typed progress update
    TypedProgressUpdate,
    /// Typed result submission
    TypedResultSubmission,
    /// Typed blocker alert
    TypedBlockerAlert,
    /// Typed workflow transition event
    TypedWorkflowTransition,
}

/// Message with delivery metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// The message
    pub message: AgentMessage,
    /// Delivery attempts
    pub delivery_attempts: u32,
    /// Last delivery attempt
    pub last_attempt: Option<u64>,
    /// Acknowledged
    pub acknowledged: bool,
}

/// NATS-based message bus
pub struct MessageBus {
    /// Pending messages
    pending: HashMap<String, Envelope>,
    /// Subscriptions (subject -> handlers)
    subscriptions: HashMap<String, Vec<Box<dyn Fn(&AgentMessage) + Send + Sync>>>,
    /// Message counter
    message_counter: u64,
    /// Default TTL
    default_ttl: Duration,
}

impl MessageBus {
    /// Create new message bus
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            subscriptions: HashMap::new(),
            message_counter: 0,
            default_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a new message
    pub fn create_message(
        &mut self,
        sender_id: &str,
        recipient_id: Option<&str>,
        message_type: MessageType,
        content: &str,
    ) -> AgentMessage {
        self.message_counter += 1;

        AgentMessage {
            message_id: format!("msg-{}-{}", self.message_counter, uuid::Uuid::new_v4()),
            thread_id: uuid::Uuid::new_v4().to_string(),
            message_type,
            sender_id: sender_id.to_string(),
            recipient_id: recipient_id.map(|s| s.to_string()),
            content: content.to_string(),
            patch: None,
            patch_receipt: None,
            context_pack: None,
            validation_result: None,
            task_assignment: None,
            progress_update: None,
            result_submission: None,
            blocker_alert: None,
            workflow_transition: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: Some(self.default_ttl.as_secs() as u32),
        }
    }

    /// Send a message
    pub fn send(&mut self, message: AgentMessage) -> Result<(), String> {
        debug!(
            "Sending message: {} -> {:?}",
            message.sender_id, message.recipient_id
        );

        validate_message_structure(&message)?;

        // Check TTL
        if let Some(ttl) = message.ttl_seconds {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if (message.timestamp + ttl as u64) < now {
                warn!("Message {} expired", message.message_id);
                return Err("Message expired".to_string());
            }
        }

        // Create envelope
        let envelope = Envelope {
            message,
            delivery_attempts: 0,
            last_attempt: None,
            acknowledged: false,
        };

        // Store in pending
        let message_id = envelope.message.message_id.clone();
        self.pending.insert(message_id.clone(), envelope);

        // Notify subscribers
        self.notify_subscribers(&message_id);

        Ok(())
    }

    /// Publish to a subject (broadcast)
    pub fn publish(&mut self, subject: &str, message: AgentMessage) -> Result<(), String> {
        info!("Publishing to {}: {}", subject, message.message_id);
        // In a full implementation, this would use NATS subjects
        // For now, just send the message
        self.send(message)
    }

    /// Subscribe to a subject
    pub fn subscribe<F>(&mut self, subject: &str, handler: F)
    where
        F: Fn(&AgentMessage) + Send + Sync + 'static,
    {
        info!("Subscribing to {}", subject);
        self.subscriptions
            .entry(subject.to_string())
            .or_default()
            .push(Box::new(handler));
    }

    /// Acknowledge message receipt
    pub fn acknowledge(&mut self, message_id: &str) -> Result<(), String> {
        if let Some(envelope) = self.pending.get_mut(message_id) {
            envelope.acknowledged = true;
            debug!("Acknowledged message: {}", message_id);
            Ok(())
        } else {
            Err(format!("Message {} not found", message_id))
        }
    }

    /// Get pending messages for an agent
    pub fn get_pending(&self, agent_id: &str) -> Vec<&AgentMessage> {
        self.pending
            .values()
            .filter(|envelope| {
                !envelope.acknowledged
                    && envelope
                        .message
                        .recipient_id
                        .as_ref()
                        .map(|r| r == agent_id)
                        .unwrap_or(false)
            })
            .map(|e| &e.message)
            .collect()
    }

    /// Retry pending messages
    pub fn retry_pending(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut to_retry: Vec<String> = Vec::new();

        for (id, envelope) in &self.pending {
            if !envelope.acknowledged && envelope.delivery_attempts > 0 {
                if let Some(last_attempt) = envelope.last_attempt {
                    // Retry after 30 seconds
                    if now - last_attempt > 30 {
                        to_retry.push(id.clone());
                    }
                }
            }
        }

        for id in to_retry {
            if let Some(envelope) = self.pending.get_mut(&id) {
                envelope.delivery_attempts += 1;
                envelope.last_attempt = Some(now);

                if envelope.delivery_attempts > 3 {
                    warn!("Message {} failed {} times", id, envelope.delivery_attempts);
                }

                self.notify_subscribers(&id);
            }
        }
    }

    /// Clean up acknowledged messages
    pub fn cleanup(&mut self) {
        let acknowledged: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, e)| e.acknowledged)
            .map(|(id, _)| id.clone())
            .collect();

        let count = acknowledged.len();

        for id in acknowledged {
            self.pending.remove(&id);
        }

        if count > 0 {
            debug!("Cleaned up {} acknowledged messages", count);
        }
    }

    /// Notify subscribers
    fn notify_subscribers(&self, message_id: &str) {
        if let Some(envelope) = self.pending.get(message_id) {
            // In a full NATS implementation, this would publish to NATS
            // For now, just log
            debug!("Notifying subscribers of message {}", message_id);
        }
    }

    /// Set default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Get pending message count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Communication protocol for agent-to-agent communication
pub struct CommunicationProtocol {
    /// Message bus
    bus: MessageBus,
    /// Agent ID
    agent_id: String,
}

impl CommunicationProtocol {
    /// Create new protocol
    pub fn new(agent_id: &str) -> Self {
        Self {
            bus: MessageBus::new(),
            agent_id: agent_id.to_string(),
        }
    }

    /// Send task assignment
    pub fn send_task_assignment(
        &mut self,
        recipient_id: &str,
        task_description: &str,
    ) -> Result<(), String> {
        let message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TaskAssign,
            task_description,
        );
        self.bus.send(message)
    }

    /// Send task result
    pub fn send_task_result(
        &mut self,
        recipient_id: &str,
        thread_id: &str,
        result: &str,
        success: bool,
    ) -> Result<(), String> {
        let content = if success {
            format!("SUCCESS: {}", result)
        } else {
            format!("FAILURE: {}", result)
        };

        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TaskResult,
            &content,
        );
        message.thread_id = thread_id.to_string();
        self.bus.send(message)
    }

    /// Request information
    pub fn request_info(&mut self, recipient_id: &str, query: &str) -> Result<(), String> {
        let message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::InfoRequest,
            query,
        );
        self.bus.send(message)
    }

    /// Send information
    pub fn send_info(
        &mut self,
        recipient_id: &str,
        thread_id: &str,
        info: &str,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::InfoResponse,
            info,
        );
        message.thread_id = thread_id.to_string();
        self.bus.send(message)
    }

    /// Send a patch envelope.
    pub fn send_patch_envelope(
        &mut self,
        recipient_id: &str,
        envelope: &PatchEnvelope,
    ) -> Result<(), String> {
        let mut message =
            self.bus
                .create_message(&self.agent_id, Some(recipient_id), MessageType::Patch, "");
        message.patch = Some(envelope.clone());
        self.bus.send(message)
    }

    /// Send a patch receipt.
    pub fn send_patch_receipt(
        &mut self,
        recipient_id: &str,
        receipt: &PatchReceipt,
    ) -> Result<(), String> {
        let mut message =
            self.bus
                .create_message(&self.agent_id, Some(recipient_id), MessageType::PatchResult, "");
        message.patch_receipt = Some(receipt.clone());
        self.bus.send(message)
    }

    /// Send a compact context pack.
    pub fn send_context_pack(
        &mut self,
        recipient_id: &str,
        context_pack: &ContextPack,
    ) -> Result<(), String> {
        let mut message = self
            .bus
            .create_message(&self.agent_id, Some(recipient_id), MessageType::ContextPack, "");
        message.context_pack = Some(context_pack.clone());
        self.bus.send(message)
    }

    /// Send a validation result.
    pub fn send_validation_result(
        &mut self,
        recipient_id: &str,
        result: &PatchValidationResult,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::ValidationResult,
            "",
        );
        message.validation_result = Some(result.clone());
        self.bus.send(message)
    }

    /// Send a typed task assignment.
    pub fn send_typed_task_assignment(
        &mut self,
        recipient_id: &str,
        assignment: &InternalTaskAssignment,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TypedTaskAssignment,
            "",
        );
        message.task_assignment = Some(assignment.clone());
        self.bus.send(message)
    }

    /// Send a typed progress update.
    pub fn send_typed_progress_update(
        &mut self,
        recipient_id: &str,
        update: &InternalProgressUpdate,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TypedProgressUpdate,
            "",
        );
        message.progress_update = Some(update.clone());
        self.bus.send(message)
    }

    /// Send a typed result submission.
    pub fn send_typed_result_submission(
        &mut self,
        recipient_id: &str,
        result: &InternalResultSubmission,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TypedResultSubmission,
            "",
        );
        message.result_submission = Some(result.clone());
        self.bus.send(message)
    }

    /// Send a typed blocker alert.
    pub fn send_typed_blocker_alert(
        &mut self,
        recipient_id: &str,
        alert: &InternalBlockerAlert,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TypedBlockerAlert,
            "",
        );
        message.blocker_alert = Some(alert.clone());
        self.bus.send(message)
    }

    /// Send a typed workflow transition event.
    pub fn send_typed_workflow_transition(
        &mut self,
        recipient_id: &str,
        event: &InternalWorkflowTransitionEvent,
    ) -> Result<(), String> {
        let mut message = self.bus.create_message(
            &self.agent_id,
            Some(recipient_id),
            MessageType::TypedWorkflowTransition,
            "",
        );
        message.workflow_transition = Some(event.clone());
        self.bus.send(message)
    }

    /// Broadcast status update
    pub fn broadcast_status(&mut self, status: &str) -> Result<(), String> {
        let message = self.bus.create_message(
            &self.agent_id,
            None, // Broadcast
            MessageType::StatusUpdate,
            status,
        );
        self.bus.send(message)
    }

    /// Get pending messages
    pub fn get_pending(&self) -> Vec<&AgentMessage> {
        self.bus.get_pending(&self.agent_id)
    }

    /// Acknowledge message
    pub fn acknowledge(&mut self, message_id: &str) -> Result<(), String> {
        self.bus.acknowledge(message_id)
    }

    /// Run cleanup
    pub fn cleanup(&mut self) {
        self.bus.cleanup();
    }
}

fn validate_message_structure(message: &AgentMessage) -> Result<(), String> {
    let expects_empty_content = matches!(
        message.message_type,
        MessageType::Patch
            | MessageType::PatchResult
            | MessageType::ContextPack
            | MessageType::ValidationResult
            | MessageType::TypedTaskAssignment
            | MessageType::TypedProgressUpdate
            | MessageType::TypedResultSubmission
            | MessageType::TypedBlockerAlert
            | MessageType::TypedWorkflowTransition
    );

    if expects_empty_content && !message.content.trim().is_empty() {
        return Err("typed orchestration messages must not use generic content".to_string());
    }

    let typed_present = match message.message_type {
        MessageType::Patch => message.patch.is_some(),
        MessageType::PatchResult => message.patch_receipt.is_some(),
        MessageType::ContextPack => message.context_pack.is_some(),
        MessageType::ValidationResult => message.validation_result.is_some(),
        MessageType::TypedTaskAssignment => message.task_assignment.is_some(),
        MessageType::TypedProgressUpdate => message.progress_update.is_some(),
        MessageType::TypedResultSubmission => message.result_submission.is_some(),
        MessageType::TypedBlockerAlert => message.blocker_alert.is_some(),
        MessageType::TypedWorkflowTransition => message.workflow_transition.is_some(),
        _ => true,
    };

    if !typed_present {
        return Err(format!(
            "typed payload missing for message type {:?}",
            message.message_type
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let mut bus = MessageBus::new();

        let message = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TaskAssign,
            "Do something",
        );

        assert!(!message.message_id.is_empty());
        assert_eq!(message.sender_id, "agent1");
        assert_eq!(message.recipient_id, Some("agent2".to_string()));
        assert_eq!(message.message_type, MessageType::TaskAssign);
        assert!(message.patch.is_none());
    }

    #[test]
    fn test_message_send() {
        let mut bus = MessageBus::new();

        let message = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TaskAssign,
            "Do something",
        );

        let result = bus.send(message);
        assert!(result.is_ok());
        assert_eq!(bus.pending_count(), 1);
    }

    #[test]
    fn test_message_acknowledgement() {
        let mut bus = MessageBus::new();

        let message = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TaskAssign,
            "Do something",
        );

        let message_id = message.message_id.clone();
        bus.send(message).unwrap();

        let result = bus.acknowledge(&message_id);
        assert!(result.is_ok());

        let pending = bus.get_pending("agent2");
        assert!(pending.is_empty());
    }

    #[test]
    fn test_get_pending() {
        let mut bus = MessageBus::new();

        let message1 =
            bus.create_message("agent1", Some("agent2"), MessageType::TaskAssign, "Task 1");
        let message2 =
            bus.create_message("agent1", Some("agent2"), MessageType::TaskAssign, "Task 2");
        let message3 =
            bus.create_message("agent1", Some("agent3"), MessageType::TaskAssign, "Task 3");

        bus.send(message1).unwrap();
        bus.send(message2).unwrap();
        bus.send(message3).unwrap();

        let pending = bus.get_pending("agent2");
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_communication_protocol() {
        let mut protocol = CommunicationProtocol::new("architect");

        let result = protocol.send_task_assignment("coder", "Implement feature X");
        assert!(result.is_ok());

        let result = protocol.broadcast_status("Ready for tasks");
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_patch_envelope() {
        let mut protocol = CommunicationProtocol::new("architect");
        let envelope = PatchEnvelope {
            task_id: "task-1".to_string(),
            target_files: vec!["src/lib.rs".to_string()],
            format: crate::patch_protocol::PatchFormat::UnifiedDiffZero,
            patch_text: Some(
                "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
"
                .to_string(),
            ),
            search_replace_blocks: Vec::new(),
            base_revision: "rev-1".to_string(),
            validation: Vec::new(),
        };

        let result = protocol.send_patch_envelope("coder", &envelope);
        assert!(result.is_ok());
    }

    #[test]
    fn test_typed_message_rejects_generic_content() {
        let mut bus = MessageBus::new();
        let mut message = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TypedTaskAssignment,
            "{\"task_id\":\"task-1\"}",
        );
        message.task_assignment = Some(InternalTaskAssignment {
            task_id: "task-1".to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            task_type: crate::task::TaskType::General,
            target_files: Vec::new(),
            target_symbols: Vec::new(),
            token_budget: 100,
            context_pack: None,
        });

        let result = bus.send(message);
        assert!(result.is_err());
    }
}
