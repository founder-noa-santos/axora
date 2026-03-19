//! Core configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Core system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// MCP server bind address
    pub mcp_bind_address: String,
    /// MCP server port
    pub mcp_port: u16,
    /// Database path
    pub database_path: PathBuf,
    /// Workspace root exposed to MCP and doc sync services.
    pub workspace_root: PathBuf,
    /// Root directory for procedural skills.
    pub skills_root: PathBuf,
    /// Root directory for repository documentation.
    pub docs_root: PathBuf,
    /// Local semantic store database path.
    pub semantic_store_path: PathBuf,
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u64,
    /// MCP command allowlist enforced by the daemon.
    pub mcp_allowed_commands: Vec<String>,
    /// Maximum execution time for MCP commands.
    pub mcp_command_timeout_secs: u64,
    /// Background pruning cadence in seconds.
    pub pruning_interval_secs: u64,
    /// Background doc sync cadence in seconds.
    pub doc_sync_interval_secs: u64,
    /// Enable debug mode
    pub debug: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 50051,
            mcp_bind_address: "127.0.0.1".to_string(),
            mcp_port: 50061,
            database_path: PathBuf::from("axora.db"),
            workspace_root: PathBuf::from("."),
            skills_root: PathBuf::from("./skills"),
            docs_root: PathBuf::from("./docs"),
            semantic_store_path: PathBuf::from("./semantic-memory.db"),
            max_concurrent_agents: 10,
            frame_duration_ms: 16, // ~60 FPS
            mcp_allowed_commands: vec![
                "cargo".to_string(),
                "git".to_string(),
                "rg".to_string(),
                "rustc".to_string(),
            ],
            mcp_command_timeout_secs: 30,
            pruning_interval_secs: 3600,
            doc_sync_interval_secs: 60,
            debug: false,
        }
    }
}

impl CoreConfig {
    /// Build a batteries-included runtime config rooted in the provided workspace.
    pub fn for_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let runtime_root = workspace_root.join(".axora");

        Self {
            workspace_root: workspace_root.clone(),
            database_path: runtime_root.join("axora.db"),
            skills_root: runtime_root.clone(),
            docs_root: workspace_root.join("docs"),
            semantic_store_path: runtime_root.join("semantic-memory.db"),
            ..Default::default()
        }
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the full server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }

    /// Get the MCP server address.
    pub fn mcp_server_address(&self) -> String {
        format!("{}:{}", self.mcp_bind_address, self.mcp_port)
    }

    /// Ensure runtime directories exist before bootstrapping services.
    pub fn ensure_runtime_layout(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = self.semantic_store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(&self.skills_root)?;
        if self.docs_root.starts_with(&self.workspace_root) {
            std::fs::create_dir_all(&self.docs_root)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CoreConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 50051);
        assert_eq!(config.mcp_port, 50061);
        assert_eq!(config.max_concurrent_agents, 10);
        assert_eq!(config.frame_duration_ms, 16);
        assert_eq!(config.mcp_command_timeout_secs, 30);
    }

    #[test]
    fn test_server_address() {
        let config = CoreConfig::default();
        assert_eq!(config.server_address(), "127.0.0.1:50051");
    }

    #[test]
    fn test_mcp_server_address() {
        let config = CoreConfig::default();
        assert_eq!(config.mcp_server_address(), "127.0.0.1:50061");
    }
}
