//! OPENAKTA Daemon
//!
//! Main entry point for the OPENAKTA multi-agent system daemon.
//! Handles command-line arguments, configuration, and starts
//! all necessary services.

mod background;

use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{error, info, warn};

use openakta_api_client::ApiClientPool;
use openakta_agents::hitl::{HitlConfig, MissionHitlGate};
use openakta_agents::{ExecutionTraceRegistry, RuntimeBlackboard};
use openakta_core::{
    init_tracing, CollectiveServer, CoreConfig, CoreError, ExecutionObservabilityGrpc,
    MemoryServices,
};
use openakta_mcp_server::{McpService, McpServiceConfig};
use openakta_proto::livingdocs::v1::living_docs_review_service_server::LivingDocsReviewServiceServer;
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
use openakta_proto::mcp::v1::tool_service_server::ToolServiceServer;
use openakta_proto::work::v1::work_management_service_server::WorkManagementServiceServer;
use openakta_storage::{Database, DatabaseConfig};

use crate::background::engine::LivingDocsEngine;
use crate::background::livingdocs_review_service::LivingDocsReviewGrpc;
use crate::background::queue::SqliteJobQueue;
use crate::background::work_management_service::WorkManagementGrpc;
use crate::background::work_mirror::WorkMirror;

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

    // Initialize sqlite-vec BEFORE any SQLite connections
    // This is the canonical, idempotent initialization with two-tier verification
    // End users do NOT need to install sqlite-vec manually - it's statically linked
    openakta_memory::ensure_sqlite_vec_ready()
        .context("sqlite-vec initialization failed - this is a product bug, not user error")?;

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
    config.ensure_runtime_layout()?;
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
    let livingdocs_shutdown = Arc::new(AtomicBool::new(false));
    let livingdocs_handle =
        LivingDocsEngine::start(config.clone(), Arc::clone(&livingdocs_shutdown));

    let (message_bus, hitl_bus_rx) = tokio::sync::broadcast::channel(1024);
    let trace_registry = Arc::new(ExecutionTraceRegistry::new(config.execution_log_dir()));
    let blackboard = Arc::new(tokio::sync::Mutex::new(RuntimeBlackboard::new()));
    let hitl_gate = Arc::new(MissionHitlGate::new(
        HitlConfig {
            checkpoint_dir: config.workspace_root.join(".openakta/checkpoints"),
            execution_trace_registry: Some(Arc::clone(&trace_registry)),
            ..Default::default()
        },
        Some((message_bus.clone(), hitl_bus_rx)),
    ));

    // Start MCP service
    let mcp_addr = config
        .mcp_server_address()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid MCP address: {}", e))?;
    let livingdocs_queue =
        SqliteJobQueue::open(SqliteJobQueue::path_for_workspace(&config.workspace_root))?;
    let livingdocs_review =
        LivingDocsReviewGrpc::open(livingdocs_queue, config.workspace_root.clone());
    let work_mirror =
        WorkMirror::open(WorkMirror::path_for_workspace(&config.workspace_root))?;
    let work_management_service = WorkManagementGrpc::open(
        work_mirror,
        ApiClientPool::global(),
        config.clone(),
        Arc::clone(&trace_registry),
        Arc::clone(&hitl_gate),
        config.workspace_root.clone(),
    );

    let mcp_service = McpService::with_config(McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
        execution_mode: config.execution_mode,
        container_executor: config.container_executor.clone(),
        wasi_executor: config.wasi_executor.clone(),
        mass_refactor_executor: config.mass_refactor_executor.clone(),
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
    // Wakes both gRPC shutdown futures (same idea as `CancellationToken`; uses `tokio::sync::Notify`).
    let shutdown_notify = Arc::new(Notify::new());

    let mut collective_task = tokio::spawn({
        let notify = shutdown_notify.clone();
        let collective =
            CollectiveServer::with_hitl_runtime(config.clone(), message_bus, hitl_gate, blackboard);
        async move {
            collective
                .serve_with_shutdown(async move {
                    notify.notified().await;
                })
                .await
        }
    });

    let mut mcp_task = tokio::spawn({
        let notify = shutdown_notify.clone();
        let observability = ExecutionObservabilityGrpc::new(Arc::clone(&trace_registry));
        async move {
            tonic::transport::Server::builder()
                .add_service(GraphRetrievalServiceServer::new(mcp_service.clone()))
                .add_service(ToolServiceServer::new(mcp_service))
                .add_service(LivingDocsReviewServiceServer::new(livingdocs_review))
                .add_service(WorkManagementServiceServer::new(work_management_service))
                .add_service(observability.into_service())
                .serve_with_shutdown(mcp_addr, async move {
                    notify.notified().await;
                })
                .await
        }
    });

    info!("OPENAKTA Daemon started successfully");

    tokio::select! {
        _ = shutdown_signal() => {
            info!("shutdown signal received, stopping Collective and MCP servers");
            livingdocs_shutdown.store(true, Ordering::SeqCst);
            shutdown_notify.notify_waiters();
            let (c, m) = tokio::join!(collective_task, mcp_task);
            let res = merge_shutdown_results(c, m);
            join_livingdocs(livingdocs_handle);
            info!("OPENAKTA Daemon shut down");
            res
        }
        collective = &mut collective_task => {
            warn!("Collective server task finished before shutdown signal; stopping MCP");
            livingdocs_shutdown.store(true, Ordering::SeqCst);
            shutdown_notify.notify_waiters();
            let mcp = mcp_task.await;
            let res = merge_shutdown_results(collective, mcp);
            join_livingdocs(livingdocs_handle);
            res
        }
        mcp = &mut mcp_task => {
            warn!("MCP server task finished before shutdown signal; stopping Collective");
            livingdocs_shutdown.store(true, Ordering::SeqCst);
            shutdown_notify.notify_waiters();
            let collective = collective_task.await;
            let res = merge_shutdown_results(collective, mcp);
            join_livingdocs(livingdocs_handle);
            res
        }
    }
}

fn merge_shutdown_results(
    collective: Result<Result<(), CoreError>, tokio::task::JoinError>,
    mcp: Result<Result<(), tonic::transport::Error>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    flatten_join("Collective", collective).and(flatten_join("MCP", mcp))
}

fn flatten_join<E: std::fmt::Display>(
    label: &'static str,
    res: Result<Result<(), E>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    match res {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            error!("{label} server error: {e}");
            Err(anyhow::anyhow!("{label}: {e}"))
        }
        Err(e) if e.is_cancelled() => Ok(()),
        Err(e) => {
            error!("{label} task join error: {e}");
            Err(e.into())
        }
    }
}

fn join_livingdocs(handle: std::thread::JoinHandle<()>) {
    if handle.join().is_err() {
        warn!("livingdocs engine thread panicked");
    }
}

/// Wait for SIGINT (Ctrl+C) or, on Unix, SIGTERM — used for coordinated daemon shutdown (V-009).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm = match tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ) {
            Ok(s) => s,
            Err(err) => {
                warn!(error = %err, "failed to install SIGTERM handler; only Ctrl+C will stop the daemon");
                return tokio::signal::ctrl_c()
                    .await
                    .expect("failed to listen for ctrl+c");
            }
        };

        tokio::select! {
            res = tokio::signal::ctrl_c() => {
                if let Err(err) = res {
                    warn!(error = %err, "failed to listen for ctrl+c");
                }
            }
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
