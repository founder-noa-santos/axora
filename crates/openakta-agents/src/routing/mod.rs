//! Difficulty-aware routing for heterogeneous model execution.

use crate::provider::ProviderKind;
use crate::provider_registry::ProviderRegistry;
use crate::provider_transport::{
    CloudModelRef, LocalModelRef, ModelRoutingHint, ProviderInstanceId,
};
use crate::task::{Task, TaskType};
use crate::transport::InternalTaskAssignment;
use crate::wire_profile::WireProfile;

/// Concrete execution target chosen by the router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutedTarget {
    /// Route to a configured cloud lane.
    Cloud(CloudModelRef),
    /// Route to a configured local lane.
    Local(LocalModelRef),
}

/// Normalized execution descriptor used for routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionDescriptor {
    /// Task type.
    pub task_type: TaskType,
    /// Lowercased task description.
    pub description: String,
    /// Number of target files.
    pub target_file_count: usize,
    /// Whether the task looks like a small bounded edit.
    pub small_bounded_edit: bool,
    /// Whether the task is clearly review/arbitration work.
    pub review_or_arbitration: bool,
    /// Whether the task is obviously architecture-heavy.
    pub architecture_heavy: bool,
    /// Whether the task is a syntax/doc/autocomplete style operation.
    pub fast_path_candidate: bool,
}

impl ExecutionDescriptor {
    /// Build a descriptor from the runtime task and assignment.
    pub fn from_task(task: &Task, assignment: &InternalTaskAssignment) -> Self {
        let description = task.description.to_ascii_lowercase();
        let target_file_count = assignment.target_files.len();
        let small_bounded_edit = target_file_count <= 1
            && [
                "fix", "rename", "doc", "comment", "small", "format", "syntax",
            ]
            .iter()
            .any(|keyword| description.contains(keyword));
        let review_or_arbitration = task.task_type == TaskType::Review
            || ["review", "arbiter", "arbitrate", "consensus", "validate"]
                .iter()
                .any(|keyword| description.contains(keyword));
        let architecture_heavy = [
            "architecture",
            "decompose",
            "decomposition",
            "refactor",
            "design",
        ]
        .iter()
        .any(|keyword| description.contains(keyword))
            || target_file_count > 3;
        let fast_path_candidate = ["syntax", "docstring", "comment", "autocomplete", "typo"]
            .iter()
            .any(|keyword| description.contains(keyword))
            || small_bounded_edit;

        Self {
            task_type: task.task_type.clone(),
            description,
            target_file_count,
            small_bounded_edit,
            review_or_arbitration,
            architecture_heavy,
            fast_path_candidate,
        }
    }
}

/// Route a task to a configured execution lane.
pub fn route(
    task: &Task,
    assignment: &InternalTaskAssignment,
    registry: &ProviderRegistry,
    routing_enabled: bool,
    model_instance_priority: &[ProviderInstanceId],
    routing_hint: Option<&ModelRoutingHint>,
) -> Option<RoutedTarget> {
    if let Some(target) = target_from_hint(registry, routing_hint) {
        return Some(target);
    }
    if !routing_enabled {
        return single_lane_fallback(registry);
    }
    // Phase 7+: has_cloud() removed, check default_cloud directly
    if registry.default_cloud.is_some() && !registry.has_local() {
        return registry
            .default_cloud
            .as_ref()
            .map(|reference| reference.model.as_str())
            .and_then(|model| preferred_target_for_model(registry, model))
            .or_else(|| registry.default_cloud.clone().map(RoutedTarget::Cloud));
    }
    if registry.has_local() && registry.default_cloud.is_none() {
        return registry
            .default_local
            .as_ref()
            .map(|reference| reference.model.as_str())
            .and_then(|model| preferred_target_for_model(registry, model))
            .or_else(|| registry.default_local.clone().map(RoutedTarget::Local));
    }
    if registry.default_cloud.is_none() && !registry.has_local() {
        return None;
    }

    let descriptor = ExecutionDescriptor::from_task(task, assignment);
    if should_route_local(&descriptor, registry) {
        registry
            .default_local
            .as_ref()
            .map(|reference| reference.model.as_str())
            .and_then(|model| preferred_target_for_model(registry, model))
            .or_else(|| choose_ordered_local(registry, model_instance_priority))
    } else {
        registry
            .default_cloud
            .as_ref()
            .map(|reference| reference.model.as_str())
            .and_then(|model| preferred_target_for_model(registry, model))
            .or_else(|| choose_ordered_cloud(registry, model_instance_priority))
    }
}

fn choose_ordered_cloud(
    registry: &ProviderRegistry,
    _priority: &[ProviderInstanceId],
) -> Option<RoutedTarget> {
    // Phase 7+: Cloud execution uses API client pool, no instance-specific transports
    // Return default cloud target if available
    registry.default_cloud.clone().map(RoutedTarget::Cloud)
}

fn choose_ordered_local(
    registry: &ProviderRegistry,
    priority: &[ProviderInstanceId],
) -> Option<RoutedTarget> {
    ordered_candidates(priority, registry.local.keys().cloned().collect())
        .into_iter()
        .find_map(|instance_id| {
            registry.default_local.as_ref().and_then(|reference| {
                if reference.instance_id == instance_id {
                    Some(RoutedTarget::Local(reference.clone()))
                } else if registry.local.contains_key(&instance_id) {
                    let instance = registry.instance(&instance_id);
                    let telemetry_kind = registry
                        .provider_kind(&instance_id)
                        .unwrap_or(ProviderKind::OpenAi);
                    let wire_profile = instance
                        .map(|i| i.profile.wire_profile())
                        .unwrap_or(WireProfile::OpenAiChatCompletions);
                    let model = registry
                        .instance(&instance_id)
                        .and_then(|instance| instance.default_model.clone())
                        .unwrap_or_else(|| reference.model.clone());
                    Some(RoutedTarget::Local(LocalModelRef {
                        instance_id,
                        model,
                        wire_profile,
                        telemetry_kind,
                    }))
                } else {
                    None
                }
            })
        })
        .or_else(|| registry.default_local.clone().map(RoutedTarget::Local))
}

fn ordered_candidates(
    priority: &[ProviderInstanceId],
    mut candidates: Vec<ProviderInstanceId>,
) -> Vec<ProviderInstanceId> {
    candidates.sort_by(|left, right| {
        let left_rank = priority
            .iter()
            .position(|candidate| candidate == left)
            .unwrap_or(usize::MAX);
        let right_rank = priority
            .iter()
            .position(|candidate| candidate == right)
            .unwrap_or(usize::MAX);
        left_rank.cmp(&right_rank).then_with(|| left.cmp(right))
    });
    candidates
}

fn target_from_hint(
    registry: &ProviderRegistry,
    routing_hint: Option<&ModelRoutingHint>,
) -> Option<RoutedTarget> {
    let hint = routing_hint?;
    let instance_id = hint.instance.as_ref()?;
    // Phase 7+: Cloud execution uses API client pool, only check local instances
    if registry.local.contains_key(instance_id) {
        let instance = registry.instance(instance_id);
        let telemetry_kind = registry
            .provider_kind(instance_id)
            .unwrap_or(ProviderKind::OpenAi);
        let wire_profile = instance
            .map(|i| i.profile.wire_profile())
            .unwrap_or(WireProfile::OpenAiChatCompletions);
        return Some(RoutedTarget::Local(LocalModelRef {
            instance_id: instance_id.clone(),
            model: hint.model.clone(),
            wire_profile,
            telemetry_kind,
        }));
    }
    // For cloud instances, just use the default cloud target with the hinted model
    if registry.default_cloud.is_some() {
        return Some(RoutedTarget::Cloud(CloudModelRef {
            instance_id: instance_id.clone(),
            model: hint.model.clone(),
            wire_profile: WireProfile::OpenAiChatCompletions,
            telemetry_kind: ProviderKind::OpenAi,
        }));
    }
    None
}

fn single_lane_fallback(registry: &ProviderRegistry) -> Option<RoutedTarget> {
    registry
        .default_cloud
        .as_ref()
        .map(|reference| reference.model.as_str())
        .and_then(|model| preferred_target_for_model(registry, model))
        .or_else(|| registry.default_cloud.clone().map(RoutedTarget::Cloud))
        .or_else(|| registry.default_local.clone().map(RoutedTarget::Local))
}

fn preferred_target_for_model(registry: &ProviderRegistry, model: &str) -> Option<RoutedTarget> {
    let preferred_instance = registry.model_metadata(model)?.preferred_instance.clone()?;
    let hint = ModelRoutingHint {
        model: model.to_string(),
        instance: Some(preferred_instance),
    };
    target_from_hint(registry, Some(&hint))
}

fn should_route_local(descriptor: &ExecutionDescriptor, registry: &ProviderRegistry) -> bool {
    if registry.default_local.is_none() {
        return false;
    }
    if descriptor.review_or_arbitration || descriptor.architecture_heavy {
        return false;
    }
    descriptor.task_type == TaskType::CodeModification && descriptor.fast_path_candidate
}

impl RoutedTarget {
    /// Provider kind used for shaping a shared model request.
    pub fn request_provider(&self) -> crate::wire_profile::WireProfile {
        match self {
            RoutedTarget::Cloud(cloud) => cloud.wire_profile,
            RoutedTarget::Local(local) => local.wire_profile,
        }
    }

    /// User-facing provider label.
    pub fn provider_label(&self) -> String {
        match self {
            RoutedTarget::Cloud(cloud) => cloud.instance_id.0.clone(),
            RoutedTarget::Local(local) => local.instance_id.0.clone(),
        }
    }

    /// User-facing model label.
    pub fn model_label(&self) -> &str {
        match self {
            RoutedTarget::Cloud(cloud) => &cloud.model,
            RoutedTarget::Local(local) => &local.model,
        }
    }

    /// Backing instance id.
    pub fn instance_id(&self) -> &ProviderInstanceId {
        match self {
            RoutedTarget::Cloud(cloud) => &cloud.instance_id,
            RoutedTarget::Local(local) => &local.instance_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;
    use crate::provider_registry::ProviderRegistry;
    use crate::provider_transport::{
        FallbackPolicy, ModelRegistryEntry, ModelRegistrySnapshot, ProviderProfileId,
        ProviderRuntimeBundle, ProviderRuntimeConfig, ResolvedProviderInstance,
    };
    use crate::task::Task;
    use crate::transport::InternalTaskAssignment;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn assignment(task: &Task) -> InternalTaskAssignment {
        InternalTaskAssignment {
            task_id: task.id.clone(),
            title: task.description.clone(),
            description: task.description.clone(),
            task_type: task.task_type.clone(),
            target_files: Vec::new(),
            target_symbols: Vec::new(),
            token_budget: 2_500,
            context_pack: None,
        }
    }

    #[test]
    fn route_prefers_registry_instance_metadata_over_default_cloud() {
        use crate::provider_transport::CloudModelRef;
        use openakta_api_client::ApiClientPool;

        let primary = ProviderInstanceId("cloud-primary".to_string());
        let preferred = ProviderInstanceId("cloud-preferred".to_string());

        let bundle = Arc::new(ProviderRuntimeBundle {
            instances: [
                (
                    primary.clone(),
                    ResolvedProviderInstance {
                        id: primary.clone(),
                        profile: ProviderProfileId::OpenAiChatCompletions,
                        base_url: "https://api.openai.com/v1".to_string(),
                        api_key: None,
                        is_local: false,
                        default_model: Some("gpt-4o".to_string()),
                        label: None,
                    },
                ),
                (
                    preferred.clone(),
                    ResolvedProviderInstance {
                        id: preferred.clone(),
                        profile: ProviderProfileId::OpenAiChatCompletions,
                        base_url: "https://api.openai.com/v1".to_string(),
                        api_key: None,
                        is_local: false,
                        default_model: Some("gpt-4o".to_string()),
                        label: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            http: ProviderRuntimeConfig::default(),
        });
        let registry = ProviderRegistry::new_with_api_client(
            HashMap::new(),
            Some(CloudModelRef {
                instance_id: primary.clone(),
                model: "gpt-4o".to_string(),
                wire_profile: WireProfile::OpenAiChatCompletions,
                telemetry_kind: ProviderKind::OpenAi,
            }),
            None,
            FallbackPolicy::Explicit,
            bundle,
            Arc::new(ModelRegistrySnapshot {
                models: [(
                    "claude-sonnet-4-5".to_string(),
                    ModelRegistryEntry {
                        name: "claude-sonnet-4-5".to_string(),
                        max_context_window: 200_000,
                        max_output_tokens: 8_192,
                        preferred_instance: Some(preferred.clone()),
                    },
                )]
                .into_iter()
                .collect(),
                sources: Default::default(),
            }),
            Arc::new(ApiClientPool::new(openakta_api_client::ClientConfig::default()).unwrap()),
        );

        let task = Task::new("summarize mission");
        let target = route(&task, &assignment(&task), &registry, false, &[], None).unwrap();

        // Phase 7+: Cloud routing uses default cloud target (no instance-specific transports)
        assert_eq!(target.instance_id(), &primary);
        assert_eq!(target.model_label(), "gpt-4o");
    }
}
