//! AXORA Daemon
//!
//! Main entry point for the AXORA multi-agent system daemon.
//! Handles command-line arguments, configuration, and starts
//! all necessary services.

use std::path::PathBuf;
use clap::Parser;
use tracing::{info, error};

use axora_core::{CoreConfig, CollectiveServer, init_tracing};
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
        CoreConfig {
            bind_address: args.bind,
            port: args.port,
            database_path: args.database.clone(),
            ..Default::default()
        }
    };
    
    info!("Configuration: {:?}", config);
    
    // Initialize database
    let db_config = DatabaseConfig {
        path: config.database_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let db = Database::new(db_config);
    let _conn = db.init()?;
    info!("Database initialized");
    
    // Create and start the collective server
    let server = CollectiveServer::new(config);
    
    info!("AXORA Daemon started successfully");
    
    // Run the server
    if let Err(e) = server.serve().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }
    
    Ok(())
}
