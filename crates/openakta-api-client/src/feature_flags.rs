//! Feature flags for gradual rollout

use serde::{Deserialize, Serialize};

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Kill switch for all API traffic
    pub api_enabled: bool,

    /// Hosted completion enabled
    pub hosted_completion_enabled: bool,

    /// Hosted search enabled
    pub hosted_search_enabled: bool,

    /// Remote embedding fallback enabled
    pub remote_embedding_fallback: bool,

    /// Canary percentage (0-100)
    pub canary_percentage: u8,

    /// Fallback enabled (during migration only)
    pub fallback_enabled: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            api_enabled: true,
            hosted_completion_enabled: false,
            hosted_search_enabled: false,
            remote_embedding_fallback: false,
            canary_percentage: 0,
            fallback_enabled: true,
        }
    }
}

impl FeatureFlags {
    /// Check if hosted completion should be used
    pub fn should_use_hosted_completion(&self, tenant_id: &str) -> bool {
        if !self.hosted_completion_enabled {
            return false;
        }

        // Check canary percentage
        if !self.is_in_canary(tenant_id) {
            return false;
        }

        true
    }

    /// Check if hosted search should be used
    pub fn should_use_hosted_search(&self, tenant_id: &str) -> bool {
        if !self.hosted_search_enabled {
            return false;
        }

        // Check canary percentage
        if !self.is_in_canary(tenant_id) {
            return false;
        }

        true
    }

    /// Check if tenant is in canary group
    fn is_in_canary(&self, tenant_id: &str) -> bool {
        if self.canary_percentage >= 100 {
            return true;
        }

        if self.canary_percentage == 0 {
            return false;
        }

        // Hash tenant ID to determine canary inclusion
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tenant_id.hash(&mut hasher);
        let hash = hasher.finish();

        (hash % 100) < self.canary_percentage as u64
    }

    /// Check if API is globally enabled
    pub fn is_api_enabled(&self) -> bool {
        self.api_enabled
    }
}
