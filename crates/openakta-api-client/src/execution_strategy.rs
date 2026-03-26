//! Execution strategy for local vs hosted routing

use serde::{Deserialize, Serialize};

/// Execution strategy enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    /// Local-only (offline-capable)
    LocalOnly,

    /// Hosted-only (requires API)
    HostedOnly,

    /// Prefer local, fall back to hosted if local fails
    LocalWithFallback,

    /// Prefer hosted, fall back to local if hosted fails
    HostedWithFallback,

    /// Intelligent routing based on capability, cost, latency
    IntelligentRouting,
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        Self::LocalWithFallback
    }
}

impl ExecutionStrategy {
    /// Check if this strategy allows local execution
    pub fn allows_local(&self) -> bool {
        matches!(
            self,
            Self::LocalOnly
                | Self::LocalWithFallback
                | Self::IntelligentRouting
                | Self::HostedWithFallback
        )
    }

    /// Check if this strategy allows hosted execution
    pub fn allows_hosted(&self) -> bool {
        matches!(
            self,
            Self::HostedOnly
                | Self::HostedWithFallback
                | Self::IntelligentRouting
                | Self::LocalWithFallback
        )
    }

    /// Check if fallback is enabled
    pub fn has_fallback(&self) -> bool {
        matches!(self, Self::LocalWithFallback | Self::HostedWithFallback)
    }
}
