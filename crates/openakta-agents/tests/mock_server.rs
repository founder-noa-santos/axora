//! Mock API Server for Integration Tests - Phase 6.3
//!
//! Provides a WireMock-style mock server for testing API client interactions
//! without requiring a real API server.

use openakta_proto::provider_v1::{
    embedding_service_server::{EmbeddingService, EmbeddingServiceServer},
    provider_service_server::{ProviderService, ProviderServiceServer},
    BatchEmbedRequest, BatchEmbedResponse, Choice as ProtoChoice, EmbedRequest, EmbedResponse,
    Message as ProtoMessage, ProviderRequest, ProviderResponse, ProviderResponseChunk, Usage,
};
use openakta_proto::research_v1::{
    research_service_server::{ResearchService, ResearchServiceServer},
    SearchRequest, SearchResponse,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

/// Type alias for boxed stream
type BoxStream<T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send>>;

/// Mock response configuration
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub response: ProviderResponse,
    pub delay_ms: u64,
    pub should_fail: bool,
}

/// Mock streaming response configuration
#[derive(Debug, Clone)]
pub struct MockStreamConfig {
    pub chunks: Vec<ProviderResponseChunk>,
    pub delay_between_chunks_ms: u64,
    pub should_fail: bool,
}

/// Embedded response configuration
#[derive(Debug, Clone)]
pub struct MockEmbedResponse {
    pub response: EmbedResponse,
    pub delay_ms: u64,
}

/// Batch embed response configuration
#[derive(Debug, Clone)]
pub struct MockBatchEmbedResponse {
    pub response: BatchEmbedResponse,
    pub delay_ms: u64,
}

/// Search response configuration
#[derive(Debug, Clone)]
pub struct MockSearchResponse {
    pub response: SearchResponse,
    pub delay_ms: u64,
}

/// State for the mock server
#[derive(Default)]
pub struct MockServerState {
    /// Queue of responses for completion requests
    pub completion_responses: VecDeque<MockResponse>,
    /// Queue of streaming configurations
    pub stream_configs: VecDeque<MockStreamConfig>,
    /// Queue of embed responses
    pub embed_responses: VecDeque<MockEmbedResponse>,
    /// Queue of batch embed responses
    pub batch_embed_responses: VecDeque<MockBatchEmbedResponse>,
    /// Queue of search responses
    pub search_responses: VecDeque<MockSearchResponse>,
    /// Request history for verification
    pub request_history: Vec<String>,
}

impl MockServerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mock response to the queue
    pub fn add_completion_response(&mut self, response: MockResponse) {
        self.completion_responses.push_back(response);
    }

    /// Add a mock stream configuration
    pub fn add_stream_config(&mut self, config: MockStreamConfig) {
        self.stream_configs.push_back(config);
    }

    /// Add a mock embed response
    pub fn add_embed_response(&mut self, response: MockEmbedResponse) {
        self.embed_responses.push_back(response);
    }

    /// Add a mock batch embed response
    pub fn add_batch_embed_response(&mut self, response: MockBatchEmbedResponse) {
        self.batch_embed_responses.push_back(response);
    }

    /// Add a mock search response
    pub fn add_search_response(&mut self, response: MockSearchResponse) {
        self.search_responses.push_back(response);
    }

    /// Get the next completion response
    pub fn next_completion_response(&mut self) -> Option<MockResponse> {
        self.completion_responses.pop_front()
    }

    /// Get the next stream configuration
    pub fn next_stream_config(&mut self) -> Option<MockStreamConfig> {
        self.stream_configs.pop_front()
    }

    /// Record a request for history
    pub fn record_request(&mut self, request_id: String) {
        self.request_history.push(request_id);
    }

    /// Get request count
    pub fn request_count(&self) -> usize {
        self.request_history.len()
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.completion_responses.clear();
        self.stream_configs.clear();
        self.embed_responses.clear();
        self.batch_embed_responses.clear();
        self.search_responses.clear();
        self.request_history.clear();
    }
}

/// Mock Provider Service implementation
#[derive(Clone, Default)]
pub struct MockProviderService {
    pub state: Arc<Mutex<MockServerState>>,
}

impl MockProviderService {
    pub fn new(state: Arc<Mutex<MockServerState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl ProviderService for MockProviderService {
    async fn execute(
        &self,
        request: Request<ProviderRequest>,
    ) -> Result<Response<ProviderResponse>, Status> {
        let req = request.into_inner();

        // Record request
        {
            let mut state = self.state.lock().await;
            state.record_request(req.request_id.clone());
        }

        // Simulate network latency
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Get next mock response
        let mut state = self.state.lock().await;
        match state.next_completion_response() {
            Some(mock) => {
                // Simulate delay
                tokio::time::sleep(tokio::time::Duration::from_millis(mock.delay_ms)).await;

                if mock.should_fail {
                    Err(Status::internal("Mock server error"))
                } else {
                    Ok(Response::new(mock.response))
                }
            }
            None => {
                warn!("No mock response configured, returning default");
                Ok(Response::new(ProviderResponse {
                    response_id: req.request_id,
                    content: "Default mock response".to_string(),
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 20,
                        total_tokens: 30,
                    }),
                    ..Default::default()
                }))
            }
        }
    }

    type ExecuteStreamStream = BoxStream<Result<ProviderResponseChunk, Status>>;

    async fn execute_stream(
        &self,
        request: Request<ProviderRequest>,
    ) -> Result<Response<Self::ExecuteStreamStream>, Status> {
        let req = request.into_inner();

        // Record request
        {
            let mut state = self.state.lock().await;
            state.record_request(req.request_id.clone());
        }

        // Get next stream configuration
        let mut state = self.state.lock().await;
        let config = state.next_stream_config().unwrap_or_else(|| {
            warn!("No stream config configured, using default");
            MockStreamConfig {
                chunks: vec![
                    ProviderResponseChunk {
                        request_id: req.request_id.clone(),
                        delta: Some(ProtoMessage {
                            role: "assistant".to_string(),
                            content: "Default ".to_string(),
                            tool_calls: vec![],
                        }),
                        finish_reason: String::new(),
                        usage: None,
                    },
                    ProviderResponseChunk {
                        request_id: req.request_id.clone(),
                        delta: Some(ProtoMessage {
                            role: "assistant".to_string(),
                            content: "mock response".to_string(),
                            tool_calls: vec![],
                        }),
                        finish_reason: "stop".to_string(),
                        usage: Some(Usage {
                            input_tokens: 5,
                            output_tokens: 10,
                            total_tokens: 15,
                        }),
                    },
                ],
                delay_between_chunks_ms: 10,
                should_fail: false,
            }
        });

        if config.should_fail {
            return Err(Status::internal("Mock streaming error"));
        }

        // Create stream from chunks
        let chunks: Vec<_> = config.chunks.into_iter().map(Ok).collect();
        let stream = tokio_stream::iter(chunks);

        Ok(Response::new(Box::pin(stream)))
    }
}

/// Mock Embedding Service implementation
#[derive(Clone, Default)]
pub struct MockEmbeddingService {
    pub state: Arc<Mutex<MockServerState>>,
}

impl MockEmbeddingService {
    pub fn new(state: Arc<Mutex<MockServerState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(
        &self,
        request: Request<EmbedRequest>,
    ) -> Result<Response<EmbedResponse>, Status> {
        let req = request.into_inner();

        // Simulate network latency
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut state = self.state.lock().await;
        match state.next_embed_response() {
            Some(mock) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(mock.delay_ms)).await;
                Ok(Response::new(mock.response))
            }
            None => {
                // Return default embedding
                Ok(Response::new(EmbedResponse {
                    embeddings: vec![vec![0.1; 1536]],
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 0,
                        total_tokens: 10,
                    }),
                }))
            }
        }
    }

    async fn embed_batch(
        &self,
        request: Request<BatchEmbedRequest>,
    ) -> Result<Response<BatchEmbedResponse>, Status> {
        let req = request.into_inner();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut state = self.state.lock().await;
        match state.next_batch_embed_response() {
            Some(mock) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(mock.delay_ms)).await;
                Ok(Response::new(mock.response))
            }
            None => {
                // Return default batch embeddings
                let count = req.texts.len();
                Ok(Response::new(BatchEmbedResponse {
                    embeddings: (0..count).map(|_| vec![0.1; 1536]).collect(),
                    usage: Some(Usage {
                        input_tokens: count * 10,
                        output_tokens: 0,
                        total_tokens: count * 10,
                    }),
                }))
            }
        }
    }
}

/// Mock Research Service implementation
#[derive(Clone, Default)]
pub struct MockResearchService {
    pub state: Arc<Mutex<MockServerState>>,
}

impl MockResearchService {
    pub fn new(state: Arc<Mutex<MockServerState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl ResearchService for MockResearchService {
    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut state = self.state.lock().await;
        match state.next_search_response() {
            Some(mock) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(mock.delay_ms)).await;
                Ok(Response::new(mock.response))
            }
            None => {
                // Return default search results
                Ok(Response::new(SearchResponse {
                    results: vec![],
                    total_count: 0,
                }))
            }
        }
    }
}

/// Start a mock server on a random available port
pub async fn start_mock_server() -> (MockServerState, String) {
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tonic::transport::Server;

    let state = Arc::new(Mutex::new(MockServerState::new()));

    let provider_service = MockProviderService::new(state.clone());
    let embedding_service = MockEmbeddingService::new(state.clone());
    let research_service = MockResearchService::new(state.clone());

    // Find available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    info!("Starting mock server on {}", addr);

    // Spawn server in background
    tokio::spawn(async move {
        Server::builder()
            .add_service(ProviderServiceServer::new(provider_service))
            .add_service(EmbeddingServiceServer::new(embedding_service))
            .add_service(ResearchServiceServer::new(research_service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .expect("Mock server failed");
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let endpoint = format!("localhost:{}", addr.port());
    (
        Arc::try_unwrap(state).unwrap_or_else(|arc| (*arc).blocking_lock().clone()),
        endpoint,
    )
}

/// Helper: Create a successful mock response
pub fn create_success_response(request_id: &str) -> MockResponse {
    MockResponse {
        response: ProviderResponse {
            response_id: request_id.to_string(),
            content: "This is a mock response from the API server.".to_string(),
            usage: Some(Usage {
                input_tokens: 15,
                output_tokens: 25,
                total_tokens: 40,
            }),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        },
        delay_ms: 50,
        should_fail: false,
    }
}

/// Helper: Create a failing mock response
pub fn create_failure_response() -> MockResponse {
    MockResponse {
        response: ProviderResponse::default(),
        delay_ms: 10,
        should_fail: true,
    }
}

/// Helper: Create a slow mock response
pub fn create_slow_response(request_id: &str, delay_ms: u64) -> MockResponse {
    MockResponse {
        response: ProviderResponse {
            response_id: request_id.to_string(),
            content: "Slow response".to_string(),
            usage: Some(Usage {
                input_tokens: 5,
                output_tokens: 10,
                total_tokens: 15,
            }),
            ..Default::default()
        },
        delay_ms,
        should_fail: false,
    }
}

/// Helper: Create streaming chunks
pub fn create_stream_chunks(request_id: &str, content: &str) -> MockStreamConfig {
    let words: Vec<&str> = content.split_whitespace().collect();
    let mut chunks = Vec::new();

    for (i, word) in words.iter().enumerate() {
        let is_last = i == words.len() - 1;
        chunks.push(ProviderResponseChunk {
            request_id: request_id.to_string(),
            delta: Some(ProtoMessage {
                role: "assistant".to_string(),
                content: format!("{} ", word),
                tool_calls: vec![],
            }),
            finish_reason: if is_last {
                "stop".to_string()
            } else {
                String::new()
            },
            usage: if is_last {
                Some(Usage {
                    input_tokens: 10,
                    output_tokens: words.len() as u32,
                    total_tokens: (10 + words.len()) as u32,
                })
            } else {
                None
            },
        });
    }

    MockStreamConfig {
        chunks,
        delay_between_chunks_ms: 20,
        should_fail: false,
    }
}
