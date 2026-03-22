use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use openakta_agents::{
    default_local_transport, transport_for_instance, CloudModelRef, Coordinator,
    CoordinatorConfig, InternalResultSubmission, LocalModelRef, ProviderRegistry,
    ProviderRuntimeBundle, ProviderTransport, RuntimeBlackboard,
};
use openakta_core::config_resolve::{
    build_model_registry_snapshot, build_provider_bundle, resolve_secrets,
};
use openakta_core::CoreConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeResolutionFlag {
    pub doc_path: String,
    pub code_path: Option<String>,
    pub symbol_name: Option<String>,
    pub rule_ids: Vec<String>,
    pub severity: String,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeResolutionRequest {
    pub review_id: String,
    pub report_id: String,
    pub workspace_root: PathBuf,
    pub primary_doc_path: Option<String>,
    pub code_paths: Vec<String>,
    pub flags: Vec<CodeResolutionFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeResolutionResult {
    pub patch_receipt_id: String,
    pub summary: String,
}

pub trait CodeResolutionRunner: Send + Sync {
    fn run(&self, request: &CodeResolutionRequest) -> Result<CodeResolutionResult>;
}

#[derive(Clone)]
pub struct CoordinatorCodeResolutionRunner {
    config: CoreConfig,
}

impl CoordinatorCodeResolutionRunner {
    pub fn new(config: CoreConfig) -> Self {
        Self { config }
    }
}

impl CodeResolutionRunner for CoordinatorCodeResolutionRunner {
    fn run(&self, request: &CodeResolutionRequest) -> Result<CodeResolutionResult> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build runtime for code resolution")?;
        runtime.block_on(self.run_async(request))
    }
}

impl CoordinatorCodeResolutionRunner {
    async fn run_async(&self, request: &CodeResolutionRequest) -> Result<CodeResolutionResult> {
        let secrets = resolve_secrets(&self.config.workspace_root, &self.config.providers)
            .context("resolve provider secrets for code resolution")?;
        let provider_bundle = Arc::new(
            build_provider_bundle(&self.config, &secrets)
                .context("build provider bundle for code resolution")?,
        );
        let registry = Arc::new(
            build_model_registry_snapshot(&self.config)
                .await
                .context("build model registry for code resolution")?,
        );

        let provider_registry =
            Arc::new(build_provider_registry(&self.config, provider_bundle.clone(), registry)?);
        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let mut coordinator = Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                default_cloud: default_cloud_ref(&self.config, provider_bundle.as_ref()),
                default_local: default_local_ref(&self.config, provider_bundle.as_ref()),
                model_instance_priority: self.config.providers.model_instance_priority.clone(),
                provider_bundle,
                registry: provider_registry.model_registry.clone(),
                fallback_policy: self.config.fallback_policy,
                routing_enabled: self.config.routing_enabled
                    || (self.config.providers.default_cloud_instance.is_some()
                        && self.config.providers.default_local_instance.is_some()),
                local_validation_retry_budget: self.config.local_validation_retry_budget,
                local_enabled_for: vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
                workspace_root: request.workspace_root.clone(),
                task_timeout: Duration::from_secs(30),
                hitl_gate: None,
                context_use_ratio: self.config.provider_context_use_ratio,
                context_margin_tokens: self.config.provider_context_margin_tokens,
                retrieval_share: self.config.provider_retrieval_share,
                ..Default::default()
            },
            blackboard.clone(),
            provider_registry,
        )
        .map_err(anyhow::Error::msg)?;

        let mission = build_code_resolution_mission(request);
        coordinator
            .execute_mission(&mission)
            .await
            .map_err(anyhow::Error::msg)?;

        let blackboard = blackboard.lock().await;
        let typed_results = blackboard
            .get_accessible("coordinator")
            .iter()
            .filter_map(|entry| serde_json::from_str::<InternalResultSubmission>(&entry.content).ok())
            .collect::<Vec<_>>();
        let result = typed_results
            .iter()
            .rev()
            .find(|entry| entry.success && entry.patch_receipt.is_some())
            .ok_or_else(|| anyhow!("coordinator mission finished without a patch receipt"))?;
        let receipt = result
            .patch_receipt
            .as_ref()
            .ok_or_else(|| anyhow!("missing patch receipt on successful coordinator result"))?;
        let receipt_json =
            serde_json::to_string(receipt).context("serialize patch receipt for stable id")?;
        let receipt_id = format!("patch-receipt-{}", blake3::hash(receipt_json.as_bytes()).to_hex());

        Ok(CodeResolutionResult {
            patch_receipt_id: receipt_id,
            summary: result.summary.clone(),
        })
    }
}

fn build_provider_registry(
    config: &CoreConfig,
    bundle: Arc<ProviderRuntimeBundle>,
    model_registry: Arc<openakta_agents::ModelRegistrySnapshot>,
) -> Result<ProviderRegistry> {
    let mut cloud = HashMap::new();
    let mut local = HashMap::new();
    for (instance_id, instance) in &bundle.instances {
        if instance.is_local {
            let local_config = openakta_agents::LocalProviderConfig {
                provider: openakta_agents::LocalProviderKind::Ollama,
                base_url: instance.base_url.clone(),
                default_model: instance
                    .default_model
                    .clone()
                    .unwrap_or_else(|| "qwen2.5-coder:7b".to_string()),
                enabled_for: vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
            };
            local.insert(
                instance_id.clone(),
                Arc::from(
                    default_local_transport(&local_config, Duration::from_secs(30))
                        .map_err(|err| anyhow!(err.to_string()))?,
                ),
            );
        } else {
            let transport: Arc<dyn ProviderTransport> = Arc::from(
                transport_for_instance(instance, &bundle.http)
                    .map_err(|err| anyhow!(err.to_string()))?,
            );
            cloud.insert(instance_id.clone(), transport);
        }
    }

    Ok(ProviderRegistry::new(
        cloud,
        local,
        default_cloud_ref(config, bundle.as_ref()),
        default_local_ref(config, bundle.as_ref()),
        config.fallback_policy,
        bundle,
        model_registry,
    ))
}

fn default_cloud_ref(
    config: &CoreConfig,
    bundle: &ProviderRuntimeBundle,
) -> Option<CloudModelRef> {
    let instance_id = config.providers.default_cloud_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    Some(CloudModelRef {
        instance_id,
        model: instance.default_model.clone()?,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

fn default_local_ref(
    config: &CoreConfig,
    bundle: &ProviderRuntimeBundle,
) -> Option<LocalModelRef> {
    let instance_id = config.providers.default_local_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    Some(LocalModelRef {
        instance_id,
        model: instance.default_model.clone()?,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

fn build_code_resolution_mission(request: &CodeResolutionRequest) -> String {
    let mut lines = vec![
        format!(
            "Update the implementation to match the documentation for LivingDocs review {}.",
            request.review_id
        ),
        format!("Workspace root: {}", request.workspace_root.display()),
    ];
    if let Some(primary_doc) = &request.primary_doc_path {
        lines.push(format!("Primary documentation file: {primary_doc}"));
    }
    if !request.code_paths.is_empty() {
        lines.push(format!(
            "Target code files: {}",
            request.code_paths.join(", ")
        ));
    }
    lines.push(
        "Make the smallest safe code changes needed so the code matches the documented SSOT. Return only valid patch output."
            .to_string(),
    );
    lines.push("Drift findings:".to_string());
    for flag in &request.flags {
        let code_path = flag.code_path.as_deref().unwrap_or("n/a");
        let symbol = flag.symbol_name.as_deref().unwrap_or("n/a");
        let rules = if flag.rule_ids.is_empty() {
            "n/a".to_string()
        } else {
            flag.rule_ids.join(", ")
        };
        lines.push(format!(
            "- kind={} severity={} doc={} code={} symbol={} rules={} message={}",
            flag.kind, flag.severity, flag.doc_path, code_path, symbol, rules, flag.message
        ));
    }
    lines.join("\n")
}
