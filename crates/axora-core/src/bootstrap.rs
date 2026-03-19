//! Batteries-included runtime bootstrap for AXORA CLI flows.

use axora_agents::{
    BlackboardV2, Coordinator, CoordinatorConfig, MissionResult, ProviderKind,
    ProviderRuntimeConfig,
};
use axora_mcp_server::{McpService, McpServiceConfig};
use axora_proto::mcp::v1::tool_service_server::ToolServiceServer;
use axora_storage::{Database, DatabaseConfig};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

use crate::{CoreConfig, DocSyncService, MemoryServices};

/// Runtime bootstrap options for CLI entrypoints.
#[derive(Debug, Clone)]
pub struct RuntimeBootstrapOptions {
    /// Workspace root containing the codebase to operate on.
    pub workspace_root: std::path::PathBuf,
    /// Provider backend to use for model execution.
    pub provider: ProviderKind,
    /// Model identifier to request from the provider.
    pub model: String,
    /// Whether to start background memory/doc services.
    pub start_background_services: bool,
}

impl Default for RuntimeBootstrapOptions {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            provider: ProviderKind::Anthropic,
            model: "claude-sonnet-4-5".to_string(),
            start_background_services: true,
        }
    }
}

/// Running batteries-included AXORA runtime.
pub struct RuntimeBootstrap {
    config: CoreConfig,
    blackboard: Arc<BlackboardV2>,
    _mcp_task: JoinHandle<Result<(), tonic::transport::Error>>,
    _memory_handles: Vec<std::thread::JoinHandle<()>>,
    _doc_sync_handle: Option<std::thread::JoinHandle<()>>,
}

impl RuntimeBootstrap {
    /// Bootstrap a ready-to-run runtime for the given options.
    pub async fn new(options: RuntimeBootstrapOptions) -> anyhow::Result<Self> {
        let config = CoreConfig::for_workspace(&options.workspace_root);
        config.ensure_runtime_layout()?;

        let provider_config = ProviderRuntimeConfig::default();
        if !provider_config.has_credentials(options.provider) {
            return Err(anyhow::anyhow!(
                "missing provider credentials for {:?}; set {}",
                options.provider,
                match options.provider {
                    ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
                    ProviderKind::OpenAi => "OPENAI_API_KEY",
                }
            ));
        }

        let db = Database::new(DatabaseConfig {
            path: config.database_path.to_string_lossy().to_string(),
            ..Default::default()
        });
        let _conn = db.init()?;

        let memory_services = MemoryServices::new(&config).await?;
        let memory_handles = if options.start_background_services {
            memory_services.start(&config)
        } else {
            Vec::new()
        };
        let doc_sync_handle = if options.start_background_services {
            Some(DocSyncService::start(config.clone()))
        } else {
            None
        };

        let (mcp_addr, mcp_task) = start_embedded_mcp_server(&config).await?;
        std::env::set_var("AXORA_MCP_ENDPOINT", format!("http://{}", mcp_addr));

        Ok(Self {
            config,
            blackboard: Arc::new(Mutex::new(axora_agents::SharedBlackboard::new())),
            _mcp_task: mcp_task,
            _memory_handles: memory_handles,
            _doc_sync_handle: doc_sync_handle,
        })
    }

    /// Bootstrap the runtime and execute a mission immediately.
    pub async fn run_mission(
        options: RuntimeBootstrapOptions,
        mission: &str,
    ) -> anyhow::Result<MissionResult> {
        let runtime = Self::new(options.clone()).await?;
        let mut coordinator = Coordinator::new(
            CoordinatorConfig {
                provider: options.provider,
                model: options.model,
                workspace_root: runtime.config.workspace_root.clone(),
                ..Default::default()
            },
            Arc::clone(&runtime.blackboard),
        )
        .map_err(anyhow::Error::msg)?;

        coordinator
            .execute_mission(mission)
            .await
            .map_err(anyhow::Error::msg)
    }
}

async fn start_embedded_mcp_server(
    config: &CoreConfig,
) -> anyhow::Result<(SocketAddr, JoinHandle<Result<(), tonic::transport::Error>>)> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
    let local_addr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);
    let service = McpService::with_config(McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
    });
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ToolServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
    });

    Ok((local_addr, task))
}
