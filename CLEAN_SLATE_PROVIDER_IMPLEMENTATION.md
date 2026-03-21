# CLEAN_SLATE_PROVIDER_IMPLEMENTATION

## Purge Commands

```bash
rm crates/openakta-agents/src/coordinator.rs
rm crates/openakta-agents/src/memory.rs
mkdir -p crates/openakta-agents/src/coordinator
```

## Executed Clean-Slate Changes

- Deleted legacy `crates/openakta-agents/src/memory.rs`
- Deleted file-based `crates/openakta-agents/src/coordinator.rs`
- Replaced coordinator entrypoint with `crates/openakta-agents/src/coordinator/mod.rs`
- Added `crates/openakta-agents/src/blackboard_runtime.rs` as a thin runtime facade over `openakta-cache` Blackboard v2
- Removed env/synthetic provider bootstrap fallback from `crates/openakta-agents/src/provider_transport.rs`
- Removed provider env overrides from `crates/openakta-core/src/config_resolve.rs`
- Stopped bootstrap from mutating merged config via provider env overrides in `crates/openakta-core/src/bootstrap.rs`
- Split lane references into `CloudModelRef` and `LocalModelRef`
- Added `DynamicModelRegistry` / `DynamicModelMetadata` in `crates/openakta-agents/src/model_registry/mod.rs`

## Provider Env Purge Diff

Note: provider env loading was no longer in `config.rs` itself; the live env mutation path was in `config_resolve.rs` plus the bootstrap callsite. This is the exact purge shape that was applied.

```diff
diff --git a/crates/openakta-core/src/config_resolve.rs b/crates/openakta-core/src/config_resolve.rs
@@
-/// Apply non-secret environment overrides after TOML merge.
-pub fn apply_env_overrides(config: &mut CoreConfig) {
-    if let Ok(instance) = std::env::var("OPENAKTA_DEFAULT_CLOUD_INSTANCE") {
-        config.providers.default_cloud_instance = Some(ProviderInstanceId(instance));
-    }
-    if let Ok(instance) = std::env::var("OPENAKTA_DEFAULT_LOCAL_INSTANCE") {
-        config.providers.default_local_instance = Some(ProviderInstanceId(instance));
-    }
-    if let Ok(model) = std::env::var("OPENAKTA_DEFAULT_CLOUD_MODEL") {
-        if let Some(instance_id) = config.providers.default_cloud_instance.clone() {
-            if let Some(instance) = config.providers.instances.get_mut(&instance_id) {
-                instance.default_model = Some(model);
-            }
-        }
-    }
-    if let Ok(model) = std::env::var("OPENAKTA_DEFAULT_LOCAL_MODEL") {
-        if let Some(instance_id) = config.providers.default_local_instance.clone() {
-            if let Some(instance) = config.providers.instances.get_mut(&instance_id) {
-                instance.default_model = Some(model);
-            }
-        }
-    }
-    if let Ok(url) = std::env::var("OPENAKTA_OLLAMA_URL") {
-        if let Some(instance_id) = config.providers.default_local_instance.clone() {
-            if let Some(instance) = config.providers.instances.get_mut(&instance_id) {
-                instance.base_url = url;
-            }
-        }
-    }
-    if let Ok(routing) = std::env::var("OPENAKTA_ROUTING_ENABLED") {
-        config.routing_enabled = matches!(routing.as_str(), "1" | "true" | "TRUE" | "yes" | "YES");
-    }
-    if let Ok(retry_budget) = std::env::var("OPENAKTA_LOCAL_RETRY_BUDGET") {
-        if let Ok(parsed) = retry_budget.parse::<u32>() {
-            config.local_validation_retry_budget = parsed;
-        }
-    }
-}
```

```diff
diff --git a/crates/openakta-core/src/bootstrap.rs b/crates/openakta-core/src/bootstrap.rs
@@
-use crate::config_resolve::{
-    apply_env_overrides, build_model_registry_snapshot, build_provider_bundle,
-    load_project_config, load_workspace_overlay, merge_config_layers, resolve_secrets,
-};
+use crate::config_resolve::{
+    build_model_registry_snapshot, build_provider_bundle, load_project_config,
+    load_workspace_overlay, merge_config_layers, resolve_secrets,
+};
@@
-    let mut merged = merge_config_layers(defaults, workspace, project)?;
-    apply_env_overrides(&mut merged);
-    Ok(merged)
+    merge_config_layers(defaults, workspace, project)
```

## Provider Bootstrap Purge Diff

```diff
diff --git a/crates/openakta-agents/src/provider_transport.rs b/crates/openakta-agents/src/provider_transport.rs
@@
-    #[error("synthetic fallback is disabled for {0:?}; inject SyntheticTransport explicitly or set OPENAKTA_ALLOW_SYNTHETIC_PROVIDER_FALLBACK=1 for dev mode")]
-    SyntheticFallbackDisabled(ProviderKind),
@@
-pub fn default_transport(
-    provider: ProviderKind,
-    workspace_root: impl Into<PathBuf>,
-) -> std::result::Result<Box<dyn ProviderTransport>, ProviderTransportError> {
-    let http = ProviderRuntimeConfig::default();
-    if let Some(instance) = legacy_env_instance(provider) {
-        Ok(Box::new(LiveHttpTransport::new(instance, http)?))
-    } else if synthetic_fallback_enabled() {
-        Ok(Box::new(SyntheticTransport::new(workspace_root)))
-    } else {
-        Err(ProviderTransportError::SyntheticFallbackDisabled(provider))
-    }
-}
-
 pub fn transport_for_instance(
     instance: &ResolvedProviderInstance,
     http: &ProviderRuntimeConfig,
-    workspace_root: impl Into<PathBuf>,
 ) -> std::result::Result<Box<dyn ProviderTransport>, ProviderTransportError> {
-    if instance.api_key.is_some() {
-        Ok(Box::new(LiveHttpTransport::new(
-            instance.clone(),
-            http.clone(),
-        )?))
-    } else if synthetic_fallback_enabled() {
-        Ok(Box::new(SyntheticTransport::new(workspace_root)))
-    } else {
-        Err(ProviderTransportError::SyntheticFallbackDisabled(
-            instance.provider_kind(),
-        ))
-    }
+    if instance.api_key.is_none() {
+        return Err(ProviderTransportError::MissingCredentials(
+            instance.provider_kind(),
+        ));
+    }
+    Ok(Box::new(LiveHttpTransport::new(
+        instance.clone(),
+        http.clone(),
+    )?))
 }
@@
-fn synthetic_fallback_enabled() -> bool { ... }
-fn legacy_env_instance(provider: ProviderKind) -> Option<ResolvedProviderInstance> { ... }
```

## New Canonical Runtime Types

### Dynamic Model Registry

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicModelMetadata {
    pub name: String,
    pub max_context_window: u32,
    pub max_output_tokens: u32,
    pub preferred_instance: Option<ProviderInstanceId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicModelRegistry {
    pub models: HashMap<String, DynamicModelMetadata>,
    pub sources: RegistryProvenance,
}
```

### Isolated Lane References

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudModelRef {
    pub instance_id: ProviderInstanceId,
    pub model: String,
    pub telemetry_kind: ProviderKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelRef {
    pub instance_id: ProviderInstanceId,
    pub model: String,
    pub telemetry_kind: ProviderKind,
}
```

## Source of Truth After Purge

- Coordinator runtime: `crates/openakta-agents/src/coordinator/v2.rs`
- Coordinator module root: `crates/openakta-agents/src/coordinator/mod.rs`
- Shared runtime blackboard state: `crates/openakta-cache/src/blackboard/v2.rs`
- Runtime blackboard facade for agent/core integration: `crates/openakta-agents/src/blackboard_runtime.rs`
- Provider instance transport authority: `crates/openakta-agents/src/provider_transport.rs`
- Instance registry: `crates/openakta-agents/src/provider_registry.rs`
- Routing: `crates/openakta-agents/src/routing/mod.rs`
- Registry metadata: `crates/openakta-agents/src/model_registry/mod.rs`

## Validation Run

```bash
cargo check -p openakta-agents -p openakta-core -p openakta-daemon
cargo test -p openakta-core --lib --quiet
```

## Immediate Remaining Strictness Gaps

- `route()` still needs stronger model-registry-first selection logic if `preferred_instance` must fully outrank static lane heuristics for every path.
- `build_model_registry_snapshot()` still merges builtin + TOML synchronously; remote registry fetch is not yet authoritative in bootstrap.
- `ProviderKind` is reduced, but still used for request shaping; the next purge step is to push shape selection fully behind `ProviderProfileId` + lane type, keeping `ProviderKind` telemetry-only.
