//! AXORA Daemon
//!
//! Main entry point for the AXORA multi-agent system daemon.
//! Handles command-line arguments, configuration, and starts
//! all necessary services.

use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};

use axora_core::{init_tracing, CollectiveServer, CoreConfig, DocSyncService, MemoryServices};
use axora_mcp_server::{McpService, McpServiceConfig};
use axora_proto::mcp::v1::tool_service_server::ToolServiceServer;
use axora_storage::{Database, DatabaseConfig};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "axora-daemon")]
#[command(about = "AXORA Multi-Agent System Daemon")]
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
    #[arg(short, long, default_value = "axora.db")]
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
        std::env::set_var("RUST_LOG", format!("axora={}", level));
    }
    init_tracing();

    info!("Starting AXORA Daemon v{}", env!("CARGO_PKG_VERSION"));

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
    std::env::set_var("AXORA_MCP_ENDPOINT", format!("http://{}", config.mcp_server_address()));

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

    // Start MCP service
    let mcp_addr = config
        .mcp_server_address()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid MCP address: {}", e))?;
    let mcp_service = McpService::with_config(McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
    });
    let mcp_server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ToolServiceServer::new(mcp_service))
            .serve(mcp_addr)
            .await
    });

    // Create and start the collective server
    let server = CollectiveServer::new(config.clone());

    info!("AXORA Daemon started successfully");

    tokio::select! {
        collective = server.serve() => {
            if let Err(e) = collective {
                error!("Collective server error: {}", e);
                return Err(e.into());
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
