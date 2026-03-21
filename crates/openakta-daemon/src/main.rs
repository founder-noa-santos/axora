//! OPENAKTA Daemon
//!
//! Main entry point for the OPENAKTA multi-agent system daemon.
//! Handles command-line arguments, configuration, and starts
//! all necessary services.

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

use openakta_agents::hitl::{HitlConfig, MissionHitlGate};
use openakta_agents::RuntimeBlackboard;
use openakta_core::{init_tracing, CollectiveServer, CoreConfig, DocSyncService, MemoryServices};
use openakta_mcp_server::{McpService, McpServiceConfig};
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
use openakta_proto::mcp::v1::tool_service_server::ToolServiceServer;
use openakta_storage::{Database, DatabaseConfig};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "openakta-daemon")]
#[command(about = "OPENAKTA Multi-Agent System Daemon")]
#[command(version)]
struct Args {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Server bind address
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,

    /// Server port
    #[arg(short, long, default_value = "50051")]
    port: u16,

    /// Database file path
    #[arg(short, long, default_value = "openakta.db")]
    database: PathBuf,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize tracing
    if std::env::var("RUST_LOG").is_err() {
        let level = if args.debug { "debug" } else { "info" };
        std::env::set_var("RUST_LOG", format!("openakta={}", level));
    }
    init_tracing();

    info!("Starting OPENAKTA Daemon v{}", env!("CARGO_PKG_VERSION"));

    // Load or create configuration
    let config = if let Some(config_path) = args.config {
        info!("Loading configuration from: {:?}", config_path);
        CoreConfig::from_file(&config_path)?
    } else {
        let workspace_root = std::env::current_dir()?;
        let mut config = CoreConfig::for_workspace(&workspace_root);
        config.bind_address = args.bind;
        config.port = args.port;
        config.database_path = args.database.clone();
        config
    };

    info!("Configuration: {:?}", config);
    std::env::set_var(
        "OPENAKTA_MCP_ENDPOINT",
        format!("http://{}", config.mcp_server_address()),
    );

    // Initialize database
    let db_config = DatabaseConfig {
        path: config.database_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let db = Database::new(db_config);
    let _conn = db.init()?;
    info!("Database initialized");

    // Start memory and documentation services
    let memory_services = MemoryServices::new(&config).await?;
    let _memory_handles = memory_services.start(&config);
    let _doc_sync_handle = DocSyncService::start(config.clone());

    let (message_bus, hitl_bus_rx) = tokio::sync::broadcast::channel(1024);
    let blackboard = Arc::new(tokio::sync::Mutex::new(RuntimeBlackboard::new()));
    let hitl_gate = Arc::new(MissionHitlGate::new(
        HitlConfig {
            checkpoint_dir: config.workspace_root.join(".openakta/checkpoints"),
            ..Default::default()
        },
        Some((message_bus.clone(), hitl_bus_rx)),
    ));

    // Start MCP service
    let mcp_addr = config
        .mcp_server_address()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid MCP address: {}", e))?;
    let mcp_service = McpService::with_config(McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
        execution_mode: config.execution_mode,
        container_executor: config.container_executor.clone(),
        wasi_executor: config.wasi_executor.clone(),
        dense_backend: config.retrieval.backend,
        dense_qdrant_url: config.retrieval.qdrant_url.clone(),
        dense_store_path: config.retrieval.sqlite_path.clone(),
        code_collection: config.retrieval.code.collection_spec(),
        code_embedding: config.retrieval.code.embedding_config(),
        code_retrieval_budget_tokens: config.retrieval.code.token_budget,
        skill_config: openakta_memory::SkillRetrievalConfig {
            corpus_root: config.retrieval.skills.corpus_root.clone(),
            catalog_db_path: config.retrieval.skills.catalog_db_path.clone(),
            dense_backend: config.retrieval.backend,
            dense_store_path: config.retrieval.sqlite_path.clone(),
            qdrant_url: config.retrieval.qdrant_url.clone(),
            dense_collection: config.retrieval.skills.collection_spec(),
            embedding: config.retrieval.skills.embedding_config(),
            bm25_dir: config.retrieval.skills.bm25_dir.clone(),
            skill_token_budget: config.retrieval.skills.token_budget,
            dense_limit: 64,
            bm25_limit: 64,
        },
        hitl_gate: Some(Arc::clone(&hitl_gate)),
    });
    let mcp_server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GraphRetrievalServiceServer::new(mcp_service.clone()))
            .add_service(ToolServiceServer::new(mcp_service))
            .serve(mcp_addr)
            .await
    });

    let collective =
        CollectiveServer::with_hitl_runtime(config.clone(), message_bus, hitl_gate, blackboard);
    let collective_task = tokio::spawn(async move { collective.serve().await });

    info!("OPENAKTA Daemon started successfully");

    tokio::select! {
        collective = collective_task => {
            match collective {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!("Collective server error: {}", e);
                    return Err(e.into());
                }
                Err(e) => {
                    error!("Collective task error: {}", e);
                    return Err(e.into());
                }
            }
        }
        mcp = mcp_server => {
            match mcp {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!("MCP server error: {}", e);
                    return Err(e.into());
                }
                Err(e) => {
                    error!("MCP task join error: {}", e);
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
