//! Heterogeneous provider registry for instance-based cloud and local execution lanes.

use crate::provider::ProviderKind;
use crate::provider_transport::{
    CloudModelRef, FallbackPolicy, LocalModelRef, ModelRegistryEntry, ModelRegistrySnapshot,
    ProviderInstanceId, ProviderRuntimeBundle, ProviderTransport, ResolvedProviderInstance,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of configured execution lanes.
pub struct ProviderRegistry {
    /// Cloud execution lanes keyed by provider instance id.
    pub cloud: HashMap<ProviderInstanceId, Arc<dyn ProviderTransport>>,
    /// Local execution lanes keyed by provider instance id.
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
}

impl ProviderRegistry {
    /// Create a new heterogeneous registry.
    pub fn new(
        cloud: HashMap<ProviderInstanceId, Arc<dyn ProviderTransport>>,
        local: HashMap<
            ProviderInstanceId,
            Arc<dyn crate::provider_transport::LocalProviderTransport>,
        >,
        default_cloud: Option<CloudModelRef>,
        default_local: Option<LocalModelRef>,
        fallback_policy: FallbackPolicy,
        bundle: Arc<ProviderRuntimeBundle>,
        model_registry: Arc<ModelRegistrySnapshot>,
    ) -> Self {
        Self {
            cloud,
            local,
            default_cloud,
            default_local,
            fallback_policy,
            bundle,
            model_registry,
        }
    }

    /// Returns true when the registry has a cloud lane.
    pub fn has_cloud(&self) -> bool {
        self.default_cloud
            .as_ref()
            .map(|reference| self.cloud.contains_key(&reference.instance_id))
            .unwrap_or(false)
    }

    /// Returns true when the registry has a local lane.
    pub fn has_local(&self) -> bool {
        self.default_local
            .as_ref()
            .map(|reference| self.local.contains_key(&reference.instance_id))
            .unwrap_or(false)
    }

    /// Get a configured cloud transport by instance id.
    pub fn cloud_transport(
        &self,
        instance_id: &ProviderInstanceId,
    ) -> Option<Arc<dyn ProviderTransport>> {
        self.cloud.get(instance_id).cloned()
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
}
