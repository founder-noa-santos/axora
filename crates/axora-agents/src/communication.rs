//! Inter-agent communication using NATS

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

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
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: Some(self.default_ttl.as_secs() as u32),
        }
    }

    /// Send a message
    pub fn send(&mut self, message: AgentMessage) -> Result<(), String> {
        debug!("Sending message: {} -> {:?}", message.sender_id, message.recipient_id);

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
        
        let message1 = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TaskAssign,
            "Task 1",
        );
        let message2 = bus.create_message(
            "agent1",
            Some("agent2"),
            MessageType::TaskAssign,
            "Task 2",
        );
        let message3 = bus.create_message(
            "agent1",
            Some("agent3"),
            MessageType::TaskAssign,
            "Task 3",
        );

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
}
