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
    /// Database path
    pub database_path: PathBuf,
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u64,
    /// Enable debug mode
    pub debug: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 50051,
            database_path: PathBuf::from("axora.db"),
            max_concurrent_agents: 10,
            frame_duration_ms: 16, // ~60 FPS
            debug: false,
        }
    }
}

impl CoreConfig {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CoreConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 50051);
        assert_eq!(config.max_concurrent_agents, 10);
        assert_eq!(config.frame_duration_ms, 16);
    }

    #[test]
    fn test_server_address() {
        let config = CoreConfig::default();
        assert_eq!(config.server_address(), "127.0.0.1:50051");
    }
}
