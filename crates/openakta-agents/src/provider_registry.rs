//! Heterogeneous provider registry for instance-based cloud and local execution lanes.
//!
//! ## Phase 7 Architecture
//!
//! Cloud provider calls are unified behind the `openakta-api` service via `api_client_pool`.
//! Local execution lanes remain direct (permanent local-first exception).

use crate::provider::ProviderKind;
use crate::provider_transport::{
    CloudModelRef, FallbackPolicy, LocalModelRef, ModelRegistryEntry, ModelRegistrySnapshot,
    ProviderInstanceId, ProviderRuntimeBundle, ResolvedProviderInstance,
};
use openakta_api_client::ApiClientPool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of configured execution lanes.
///
/// ## Architecture Note (Phase 7+)
///
/// Cloud execution is API-backed via `api_client_pool`. Local execution remains direct
/// (Candle, Ollama, etc.).
pub struct ProviderRegistry {
    /// Local execution lanes keyed by provider instance id.
    ///
    /// **Permanent:** Local execution (Candle, Ollama, sqlite-vec) remains direct.
    pub local:
        HashMap<ProviderInstanceId, Arc<dyn crate::provider_transport::LocalProviderTransport>>,
    /// Default cloud instance/model binding.
    pub default_cloud: Option<CloudModelRef>,
    /// Default local instance/model binding.
    pub default_local: Option<LocalModelRef>,
    /// Runtime fallback behavior when cloud execution is unavailable.
    pub fallback_policy: FallbackPolicy,
    /// Shared resolved provider bundle.
    pub bundle: Arc<ProviderRuntimeBundle>,
    /// Shared model-registry snapshot.
    pub model_registry: Arc<ModelRegistrySnapshot>,
    /// API client pool for cloud execution (Phase 5+).
    ///
    /// This is the primary mechanism for cloud provider calls post-Phase 5.
    pub api_client_pool: Arc<ApiClientPool>,
}

impl ProviderRegistry {
    /// Create a new registry with API client pool (Phase 5+).
    ///
    /// This is the preferred constructor for Phase 5+ architecture.
    /// Cloud execution uses the API client pool; local execution remains direct.
    pub fn new_with_api_client(
        local: HashMap<
            ProviderInstanceId,
            Arc<dyn crate::provider_transport::LocalProviderTransport>,
        >,
        default_cloud: Option<CloudModelRef>,
        default_local: Option<LocalModelRef>,
        fallback_policy: FallbackPolicy,
        bundle: Arc<ProviderRuntimeBundle>,
        model_registry: Arc<ModelRegistrySnapshot>,
        api_client_pool: Arc<ApiClientPool>,
    ) -> Self {
        Self {
            local,
            default_cloud,
            default_local,
            fallback_policy,
            bundle,
            model_registry,
            api_client_pool,
        }
    }

    /// Returns true when the registry has a local lane.
    pub fn has_local(&self) -> bool {
        self.default_local
            .as_ref()
            .map(|reference| self.local.contains_key(&reference.instance_id))
            .unwrap_or(false)
    }

    /// Get a configured local transport by instance id.
    pub fn local_transport(
        &self,
        instance_id: &ProviderInstanceId,
    ) -> Option<Arc<dyn crate::provider_transport::LocalProviderTransport>> {
        self.local.get(instance_id).cloned()
    }

    /// Resolve a configured instance by id.
    pub fn instance(&self, instance_id: &ProviderInstanceId) -> Option<&ResolvedProviderInstance> {
        self.bundle.instances.get(instance_id)
    }

    /// Derive the coarse provider kind from an instance id.
    pub fn provider_kind(&self, instance_id: &ProviderInstanceId) -> Option<ProviderKind> {
        self.instance(instance_id)
            .map(ResolvedProviderInstance::provider_kind)
    }

    /// Resolve wire protocol profile for an instance.
    pub fn wire_profile(
        &self,
        instance_id: &ProviderInstanceId,
    ) -> Option<crate::wire_profile::WireProfile> {
        self.instance(instance_id)
            .map(ResolvedProviderInstance::wire_profile)
    }

    /// Resolve model metadata from the active registry snapshot.
    pub fn model_metadata(&self, model: &str) -> Option<&ModelRegistryEntry> {
        self.model_registry.models.get(model)
    }

    /// Get the API client pool for cloud execution (Phase 5+).
    pub fn api_client_pool(&self) -> &ApiClientPool {
        &self.api_client_pool
    }

    /// Check if cloud execution should use API client (Phase 5+).
    ///
    /// Returns true if the registry is configured for API-backed cloud execution.
    pub fn uses_api_for_cloud(&self) -> bool {
        // Phase 5+: Always use API for cloud execution
        // (unless explicitly disabled for testing/development)
        true
    }
}
