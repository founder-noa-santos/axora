//! API client implementation

use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::{Code, Request, Status};
use tracing::warn;
use uuid::Uuid;

use crate::auth::{static_provider, AuthProvider, EnvAuthProvider};
use crate::config::ClientConfig;
use crate::error::{ApiError, Result};
use crate::execution_strategy::ExecutionStrategy;
use crate::work_management::{
    ClosureReportView, CommandEnvelope, CommandResponse, EventsResponse,
    PersonaAssignmentsListView, ReadModelResponse, RequirementGraphView,
};
use crate::work_proto_convert::{
    closure_report_from_response, persona_assignments_from_response,
    requirement_graph_from_response,
};

use openakta_proto::provider_v1::{
    provider_service_client::ProviderServiceClient, ProviderRequest, ProviderResponse,
    ProviderResponseChunk,
};

use openakta_proto::provider_v1::{
    embedding_service_client::EmbeddingServiceClient, BatchEmbedRequest as ProtoBatchEmbedRequest,
    BatchEmbedResponse as ProtoBatchEmbedResponse, EmbedRequest as ProtoEmbedRequest,
    EmbedResponse as ProtoEmbedResponse, ExecutionStrategy as ProtoExecutionStrategy,
};

use openakta_proto::research_v1::{
    research_service_client::ResearchServiceClient, SearchRequest, SearchResponse,
};

use openakta_proto::work_v1::work_management_service_client::WorkManagementServiceClient;
use openakta_proto::work_v1::{
    GetClosureReportRequest, GetRequirementGraphRequest, ListPersonaAssignmentsRequest,
};

/// API client for provider service
#[derive(Clone)]
pub struct ApiClient {
    config: Arc<ClientConfig>,
    channel: Channel,
    http_client: reqwest::Client,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    auth_provider: Option<Arc<dyn AuthProvider>>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: ClientConfig) -> Result<Self> {
        Self::with_auth_provider(config, None)
    }

    /// Create a new API client with authentication token.
    pub fn with_auth_token(config: ClientConfig, auth_token: Option<String>) -> Result<Self> {
        Self::with_auth_provider(config, static_provider(auth_token))
    }

    /// Create a new API client with a pluggable auth provider.
    pub fn with_auth_provider(
        config: ClientConfig,
        auth_provider: Option<Arc<dyn AuthProvider>>,
    ) -> Result<Self> {
        let endpoint_uri = normalize_endpoint(&config);
        let endpoint = if endpoint_uri.starts_with("https://") {
            Channel::from_shared(endpoint_uri).map_err(|e| ApiError::InvalidUri(e.to_string()))?
        } else {
            Channel::from_shared(endpoint_uri).map_err(|e| ApiError::InvalidUri(e.to_string()))?
        };

        let channel = endpoint
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(10))
            .connect_lazy();
        let http_client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .build()
            .map_err(|err| ApiError::Internal(err.to_string()))?;

        Ok(Self {
            config: Arc::new(config),
            channel,
            http_client,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new())),
            auth_provider,
        })
    }

    /// Create a new API client with auth token from environment.
    pub fn new_with_env_auth(config: ClientConfig) -> Result<Self> {
        Self::with_auth_provider(config, Some(Arc::new(EnvAuthProvider)))
    }

    async fn prepare_request<T>(&self, payload: T) -> Result<Request<T>> {
        let mut request = Request::new(payload);
        if let Some(auth_provider) = &self.auth_provider {
            if let Some(token) = auth_provider.bearer_token().await? {
                let auth_header = format!("Bearer {}", token);
                let auth_value = MetadataValue::try_from(auth_header.as_str())
                    .map_err(|e| ApiError::AuthFailed(e.to_string()))?;
                request.metadata_mut().insert("authorization", auth_value);
            }
        }
        Ok(request)
    }

    async fn maybe_refresh_auth(&self, status: &Status) -> Result<bool> {
        if status.code() != Code::Unauthenticated {
            return Ok(false);
        }

        let Some(auth_provider) = &self.auth_provider else {
            return Ok(false);
        };

        auth_provider.on_unauthenticated().await?;
        Ok(true)
    }

    async fn circuit_allow_request(&self) -> Result<()> {
        if !self.circuit_breaker.lock().await.allow_request() {
            return Err(ApiError::CircuitOpen);
        }
        Ok(())
    }

    async fn record_circuit_result<T>(&self, result: &std::result::Result<T, Status>) {
        let mut cb = self.circuit_breaker.lock().await;
        match result {
            Ok(_) => cb.on_success(),
            Err(_) => cb.on_failure(),
        }
    }

    /// Execute a provider request (non-streaming)
    pub async fn execute(&self, request: ProviderRequest) -> Result<ProviderResponse> {
        self.circuit_allow_request().await?;

        let mut client = ProviderServiceClient::new(self.channel.clone());
        let first = self.prepare_request(request.clone()).await?;
        let mut result = client.execute(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(request).await?;
                result = client.execute(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        result.map(|resp| resp.into_inner()).map_err(Into::into)
    }

    /// Execute a provider request (streaming)
    pub async fn execute_stream(
        &self,
        request: ProviderRequest,
    ) -> Result<impl Stream<Item = Result<ProviderResponseChunk>>> {
        self.circuit_allow_request().await?;

        let mut client = ProviderServiceClient::new(self.channel.clone());
        let first = self.prepare_request(request.clone()).await?;
        let mut result = client.execute_stream(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(request).await?;
                result = client.execute_stream(retry).await;
            }
        }

        let stream = result?.into_inner();
        Ok(tokio_stream::StreamExt::map(stream, |item| {
            item.map_err(Into::into)
        }))
    }

    /// Execute with fallback (during migration only)
    pub async fn execute_with_fallback<F, Fut>(
        &self,
        request: ProviderRequest,
        fallback: F,
    ) -> Result<ProviderResponse>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<ProviderResponse>>,
    {
        if !self.config.migration_mode || !self.config.feature_flags.fallback_enabled {
            return self.execute(request).await;
        }

        let request_id = request.request_id.clone();
        match self.execute(request).await {
            Ok(response) => Ok(response),
            Err(e) if e.should_trigger_fallback() => {
                warn!(request_id = %request_id, "API unavailable, falling back");
                fallback().await
            }
            Err(e) => Err(e),
        }
    }

    /// Search via API
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        self.circuit_allow_request().await?;

        let mut client = ResearchServiceClient::new(self.channel.clone());
        let first = self.prepare_request(request.clone()).await?;
        let mut result = client.search(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(request).await?;
                result = client.search(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        result.map(|resp| resp.into_inner()).map_err(Into::into)
    }

    pub async fn submit_work_command(
        &self,
        workspace_id: Uuid,
        command: &CommandEnvelope,
    ) -> Result<CommandResponse> {
        self.send_http_json(
            Method::POST,
            format!("/api/v1/workspaces/{workspace_id}/commands"),
            Some(command),
        )
        .await
    }

    pub async fn get_work_read_model(&self, workspace_id: Uuid) -> Result<ReadModelResponse> {
        self.send_http_json::<(), _>(
            Method::GET,
            format!("/api/v1/workspaces/{workspace_id}/read-model"),
            None,
        )
        .await
    }

    pub async fn list_work_events(
        &self,
        workspace_id: Uuid,
        after_seq: i64,
        limit: i64,
    ) -> Result<EventsResponse> {
        self.send_http_json::<(), _>(
            Method::GET,
            format!("/api/v1/workspaces/{workspace_id}/events?after_seq={after_seq}&limit={limit}"),
            None,
        )
        .await
    }

    /// Requirement graph and related rows for a story or prepared story (`work.v1.GetRequirementGraph`).
    ///
    /// Callers typically point the client at the local daemon endpoint that serves `WorkManagementService`.
    pub async fn get_requirement_graph(
        &self,
        workspace_id: Uuid,
        story_id: Option<Uuid>,
        prepared_story_id: Option<Uuid>,
    ) -> Result<RequirementGraphView> {
        self.circuit_allow_request().await?;

        let mut client = WorkManagementServiceClient::new(self.channel.clone());
        let payload = GetRequirementGraphRequest {
            workspace_id: workspace_id.to_string(),
            story_id: story_id.map(|id| id.to_string()).unwrap_or_default(),
            prepared_story_id: prepared_story_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        };
        let first = self.prepare_request(payload.clone()).await?;
        let mut result = client.get_requirement_graph(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(payload).await?;
                result = client.get_requirement_graph(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        let inner = result.map(|r| r.into_inner()).map_err(ApiError::from)?;
        requirement_graph_from_response(inner)
    }

    /// Closure report: requirements, claims, gates, and verification findings (`work.v1.GetClosureReport`).
    pub async fn get_closure_report(
        &self,
        workspace_id: Uuid,
        story_id: Option<Uuid>,
        prepared_story_id: Option<Uuid>,
    ) -> Result<ClosureReportView> {
        self.circuit_allow_request().await?;

        let mut client = WorkManagementServiceClient::new(self.channel.clone());
        let payload = GetClosureReportRequest {
            workspace_id: workspace_id.to_string(),
            story_id: story_id.map(|id| id.to_string()).unwrap_or_default(),
            prepared_story_id: prepared_story_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        };
        let first = self.prepare_request(payload.clone()).await?;
        let mut result = client.get_closure_report(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(payload).await?;
                result = client.get_closure_report(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        let inner = result.map(|r| r.into_inner()).map_err(ApiError::from)?;
        closure_report_from_response(inner)
    }

    /// Personas and assignments for the workspace (`work.v1.ListPersonaAssignments`).
    pub async fn list_persona_assignments(
        &self,
        workspace_id: Uuid,
    ) -> Result<PersonaAssignmentsListView> {
        self.circuit_allow_request().await?;

        let mut client = WorkManagementServiceClient::new(self.channel.clone());
        let payload = ListPersonaAssignmentsRequest {
            workspace_id: workspace_id.to_string(),
        };
        let first = self.prepare_request(payload.clone()).await?;
        let mut result = client.list_persona_assignments(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(payload).await?;
                result = client.list_persona_assignments(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        let inner = result.map(|r| r.into_inner()).map_err(ApiError::from)?;
        persona_assignments_from_response(inner)
    }

    pub async fn approve_work_plan_version(
        &self,
        workspace_id: Uuid,
        plan_version_id: Uuid,
    ) -> Result<CommandResponse> {
        self.send_http_json::<(), _>(
            Method::POST,
            format!("/api/v1/workspaces/{workspace_id}/plan-versions/{plan_version_id}/approve"),
            None,
        )
        .await
    }

    /// Get execution strategy from config
    pub fn execution_strategy(&self) -> ExecutionStrategy {
        self.config.execution_strategy
    }

    /// Check if hosted execution should be used
    pub fn should_use_hosted(&self, tenant_id: &str, capability: &str) -> bool {
        let flags = &self.config.feature_flags;

        match capability {
            "completion" => flags.should_use_hosted_completion(tenant_id),
            "search" => flags.should_use_hosted_search(tenant_id),
            "embedding" => flags.remote_embedding_fallback,
            _ => false,
        }
    }

    /// Execute a single embedding via API
    pub async fn embed(
        &self,
        text: String,
        model: Option<String>,
        execution_strategy: ExecutionStrategy,
    ) -> Result<ProtoEmbedResponse> {
        self.circuit_allow_request().await?;

        let mut client = EmbeddingServiceClient::new(self.channel.clone());
        let proto_strategy = to_proto_execution_strategy(execution_strategy);
        let payload = ProtoEmbedRequest {
            text,
            model,
            execution_strategy: proto_strategy as i32,
        };

        let first = self.prepare_request(payload.clone()).await?;
        let mut result = client.embed(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(payload).await?;
                result = client.embed(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        result.map(|resp| resp.into_inner()).map_err(Into::into)
    }

    /// Execute a batch embedding via API
    pub async fn embed_batch(
        &self,
        texts: Vec<String>,
        model: Option<String>,
        execution_strategy: ExecutionStrategy,
    ) -> Result<ProtoBatchEmbedResponse> {
        self.circuit_allow_request().await?;

        let mut client = EmbeddingServiceClient::new(self.channel.clone());
        let proto_strategy = to_proto_execution_strategy(execution_strategy);
        let payload = ProtoBatchEmbedRequest {
            texts,
            model,
            execution_strategy: proto_strategy as i32,
        };

        let first = self.prepare_request(payload.clone()).await?;
        let mut result = client.embed_batch(first).await;

        if let Err(status) = &result {
            if self.maybe_refresh_auth(status).await? {
                let retry = self.prepare_request(payload).await?;
                result = client.embed_batch(retry).await;
            }
        }

        self.record_circuit_result(&result).await;
        result.map(|resp| resp.into_inner()).map_err(Into::into)
    }

    async fn bearer_token(&self) -> Result<Option<String>> {
        match &self.auth_provider {
            Some(provider) => provider.bearer_token().await,
            None => Ok(None),
        }
    }

    async fn send_http_json<B, T>(
        &self,
        method: Method,
        path: String,
        body: Option<&B>,
    ) -> Result<T>
    where
        B: serde::Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.circuit_allow_request().await?;
        let token = self.bearer_token().await?;
        let mut response = self
            .send_http_request(method.clone(), &path, body, token.clone())
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(auth_provider) = &self.auth_provider {
                auth_provider.on_unauthenticated().await?;
                let refreshed = self.bearer_token().await?;
                response = self
                    .send_http_request(method, &path, body, refreshed)
                    .await?;
            }
        }

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(map_http_status(status, body_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|err| ApiError::Internal(err.to_string()))
    }

    async fn send_http_request<B: serde::Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        bearer_token: Option<String>,
    ) -> Result<reqwest::Response> {
        let base = normalize_endpoint(&self.config);
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        let mut request = self.http_client.request(method, url);
        if let Some(token) = bearer_token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }

        request.send().await.map_err(map_reqwest_error)
    }
}

fn to_proto_execution_strategy(execution_strategy: ExecutionStrategy) -> ProtoExecutionStrategy {
    match execution_strategy {
        ExecutionStrategy::LocalOnly => ProtoExecutionStrategy::LocalOnly,
        ExecutionStrategy::HostedOnly => ProtoExecutionStrategy::HostedOnly,
        ExecutionStrategy::LocalWithFallback => ProtoExecutionStrategy::LocalWithFallback,
        ExecutionStrategy::HostedWithFallback => ProtoExecutionStrategy::HostedWithFallback,
        ExecutionStrategy::IntelligentRouting => ProtoExecutionStrategy::IntelligentRouting,
    }
}

/// Circuit breaker implementation
struct CircuitBreaker {
    failures: u32,
    last_failure: Option<std::time::Instant>,
    state: CircuitState,
}

enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            failures: 0,
            last_failure: None,
            state: CircuitState::Closed,
        }
    }

    fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() > Duration::from_secs(30) {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    fn on_success(&mut self) {
        self.failures = 0;
        self.state = CircuitState::Closed;
    }

    fn on_failure(&mut self) {
        self.failures += 1;
        self.last_failure = Some(std::time::Instant::now());

        if self.failures >= 5 {
            self.state = CircuitState::Open;
        }
    }
}

/// Shared client pool (singleton per process)
pub struct ApiClientPool {
    pub completion_client: ApiClient,
    pub search_client: ApiClient,
    pub embedding_client: Option<ApiClient>,
}

impl ApiClientPool {
    pub fn new(config: ClientConfig) -> Result<Self> {
        Self::with_auth_provider(config, None)
    }

    pub fn with_auth_provider(
        config: ClientConfig,
        auth_provider: Option<Arc<dyn AuthProvider>>,
    ) -> Result<Self> {
        let embedding_client = if config.feature_flags.remote_embedding_fallback {
            Some(ApiClient::with_auth_provider(
                config.clone(),
                auth_provider.clone(),
            )?)
        } else {
            None
        };

        Ok(Self {
            completion_client: ApiClient::with_auth_provider(
                config.clone(),
                auth_provider.clone(),
            )?,
            search_client: ApiClient::with_auth_provider(config, auth_provider)?,
            embedding_client,
        })
    }

    /// Get or create global client pool
    pub fn global() -> &'static Self {
        use once_cell::sync::OnceCell;

        static POOL: OnceCell<ApiClientPool> = OnceCell::new();

        POOL.get_or_init(|| {
            ApiClientPool::with_auth_provider(load_config(), Some(Arc::new(EnvAuthProvider)))
                .expect("Failed to create global API client pool")
        })
    }

    pub fn has_embedding_client(&self) -> bool {
        self.embedding_client.is_some()
    }

    pub fn embedding(&self) -> Option<&ApiClient> {
        self.embedding_client.as_ref()
    }
}

fn load_config() -> ClientConfig {
    ClientConfig::load_from_file("openakta.toml").unwrap_or_else(|_| ClientConfig::default())
}

fn normalize_endpoint(config: &ClientConfig) -> String {
    if config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://") {
        config.endpoint.clone()
    } else if config.use_tls {
        format!("https://{}", config.endpoint)
    } else {
        format!("http://{}", config.endpoint)
    }
}

fn map_reqwest_error(err: reqwest::Error) -> ApiError {
    if err.is_timeout() {
        ApiError::Timeout(err.to_string())
    } else if err.is_connect() {
        ApiError::ConnectionRefused(err.to_string())
    } else {
        ApiError::Unavailable(err.to_string())
    }
}

fn map_http_status(status: reqwest::StatusCode, body: String) -> ApiError {
    match status {
        reqwest::StatusCode::UNAUTHORIZED => ApiError::Unauthenticated(body),
        reqwest::StatusCode::TOO_MANY_REQUESTS => ApiError::RateLimited(body),
        reqwest::StatusCode::PAYMENT_REQUIRED => ApiError::QuotaExceeded { reset_at: None },
        reqwest::StatusCode::BAD_REQUEST
        | reqwest::StatusCode::UNPROCESSABLE_ENTITY
        | reqwest::StatusCode::NOT_FOUND
        | reqwest::StatusCode::CONFLICT => ApiError::InvalidRequest(body),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
        | reqwest::StatusCode::BAD_GATEWAY
        | reqwest::StatusCode::GATEWAY_TIMEOUT => ApiError::Unavailable(body),
        _ => ApiError::Internal(body),
    }
}
