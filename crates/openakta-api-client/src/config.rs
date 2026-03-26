//! API client configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::execution_strategy::ExecutionStrategy;
use crate::feature_flags::FeatureFlags;

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// API endpoint URL
    pub endpoint: String,

    /// Connection timeout
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,

    /// Request timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Enable TLS
    pub use_tls: bool,

    /// Execution strategy
    pub execution_strategy: ExecutionStrategy,

    /// Feature flags
    pub feature_flags: FeatureFlags,

    /// Migration mode (enables fallback)
    pub migration_mode: bool,

    /// Enable telemetry forwarding
    pub telemetry_forwarding_enabled: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:3030".to_string(),
            connect_timeout: Duration::from_secs(10),
            timeout: Duration::from_secs(30),
            use_tls: false,
            execution_strategy: ExecutionStrategy::default(),
            feature_flags: FeatureFlags::default(),
            migration_mode: true,
            telemetry_forwarding_enabled: false,
        }
    }
}

impl ClientConfig {
    /// Load configuration from file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

// Helper for humantime serialization
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = humantime::format_duration(*duration).to_string();
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        humantime::parse_duration(&s).map_err(serde::de::Error::custom)
    }
}
