//! gRPC server implementation

use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, debug, error};
use uuid::Uuid;

use axora_proto::collective::v1::{
    collective_service_server::{CollectiveService, CollectiveServiceServer},
    Agent, AgentStatus, Message, MessageType, RegisterAgentRequest, RegisterAgentResponse,
    UnregisterAgentRequest, ListAgentsRequest, ListAgentsResponse,
    SubmitTaskRequest, SubmitTaskResponse, GetTaskRequest, GetTaskResponse,
    ListTasksRequest, ListTasksResponse, StreamMessagesRequest, SendMessageRequest,
    Task, TaskStatus,
};

use crate::{CoreConfig, CoreError, Result};

/// Collective service implementation
pub struct CollectiveServer {
    config: CoreConfig,
    agents: Arc<tokio::sync::RwLock<Vec<Agent>>>,
    message_tx: mpsc::Sender<Message>,
    message_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>,
}

impl CollectiveServer {
    /// Create a new collective server
    pub fn new(config: CoreConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(1000);
        Self {
            config,
            agents: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            message_tx,
            message_rx: Arc::new(tokio::sync::Mutex::new(message_rx)),
        }
    }

    /// Get the gRPC server
    pub fn into_service(self) -> CollectiveServiceServer<Self> {
        CollectiveServiceServer::new(self)
    }

    /// Start the server
    pub async fn serve(self) -> Result<()> {
        let addr = self.config.server_address().parse().map_err(|e| {
            CoreError::Server(format!("Invalid address: {}", e))
        })?;
        
        info!("Starting Collective server on {}", addr);
        
        tonic::transport::Server::builder()
            .add_service(self.into_service())
            .serve(addr)
            .await
            .map_err(|e| CoreError::Server(e.to_string()))?;
        
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
        
        Ok(Response::new(RegisterAgentResponse {
            agent: Some(agent),
        }))
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
        
        Ok(Response::new(SubmitTaskResponse {
            task: Some(task),
        }))
    }

    async fn get_task(
        &self,
        _request: Request<GetTaskRequest>,
    ) -> std::result::Result<Response<GetTaskResponse>, Status> {
        // Placeholder implementation
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn list_tasks(
        &self,
        _request: Request<ListTasksRequest>,
    ) -> std::result::Result<Response<ListTasksResponse>, Status> {
        // Placeholder implementation
        Ok(Response::new(ListTasksResponse {
            tasks: vec![],
        }))
    }

    type StreamMessagesStream = Pin<Box<dyn Stream<Item = std::result::Result<Message, Status>> + Send>>;

    async fn stream_messages(
        &self,
        _request: Request<StreamMessagesRequest>,
    ) -> std::result::Result<Response<Self::StreamMessagesStream>, Status> {
        let rx = Arc::clone(&self.message_rx);
        
        let stream = async_stream::stream! {
            let mut rx = rx.lock().await;
            while let Some(message) = rx.recv().await {
                yield Ok(message);
            }
        };
        
        Ok(Response::new(Box::pin(stream)))
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let req = request.into_inner();
        
        let message = Message {
            id: Uuid::new_v4().to_string(),
            sender_id: req.sender_id,
            recipient_id: req.recipient_id,
            message_type: req.message_type,
            content: req.content,
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };
        
        if let Err(e) = self.message_tx.send(message).await {
            error!("Failed to send message: {}", e);
            return Err(Status::internal("Failed to send message"));
        }
        
        Ok(Response::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = CoreConfig::default();
        let server = CollectiveServer::new(config);
        // Server created successfully
        assert!(server.agents.read().await.is_empty());
    }
}
