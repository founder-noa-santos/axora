//! OPENAKTA Core
//!
//! Core business logic for the OPENAKTA multi-agent system.
//! Provides the frame-based execution model, agent coordination,
//! and task management.

pub mod bootstrap;
pub mod config;
pub mod config_resolve;
pub mod frame;
pub mod runtime_services;
pub mod server;

pub use bootstrap::{RuntimeBootstrap, RuntimeBootstrapOptions};
pub use config::CoreConfig;
pub use frame::{Frame, FrameContext, FrameExecutor};
pub use runtime_services::{DocSyncService, MemoryServices};
pub use server::{stream_messages_lagged_total, CollectiveServer};

use thiserror::Error;

/// Core-related errors
#[derive(Error, Debug)]
pub enum CoreError {
    /// Configuration error
    #[error("configuration error: {0}")]
    Config(String),

    /// Frame execution error
    #[error("frame execution error: {0}")]
    Frame(String),

    /// Server error
    #[error("server error: {0}")]
    Server(String),

    /// Storage error
    #[error("storage error: {0}")]
    Storage(#[from] openakta_storage::StorageError),
}

/// Result type for core operations
pub type Result<T> = std::result::Result<T, CoreError>;

/// Initialize tracing for the core system
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
}
