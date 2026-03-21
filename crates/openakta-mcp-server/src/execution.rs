//! Tool execution routing across direct, containerized, and WASI backends.

pub mod container;
pub mod direct;
pub mod wasi;

use crate::McpError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

pub use container::ContainerExecutor;
pub use direct::DirectExecutor;
pub use wasi::WasiExecutor;

/// Execution mode for mutating tools.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Route commands through containers and patches through WASI.
    #[default]
    Hybrid,
    /// Route both commands and patches through the container backend.
    Containerized,
    /// Execute directly on the host with ambient authority.
    Direct,
}

/// Configuration for containerized execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerExecutorConfig {
    /// Container runtime binary, such as `docker`.
    pub runtime_binary: String,
    /// OCI image used for sandboxed execution.
    pub image: String,
    /// Mount path exposed inside the container.
    pub workspace_mount_path: String,
    /// Extra runtime flags injected before the image.
    pub extra_args: Vec<String>,
}

impl Default for ContainerExecutorConfig {
    fn default() -> Self {
        Self {
            runtime_binary: "docker".to_string(),
            image: "ghcr.io/openakta/aktacode-mcp-sandbox:latest".to_string(),
            workspace_mount_path: "/workspace".to_string(),
            extra_args: Vec::new(),
        }
    }
}

/// Configuration for WASI execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WasiExecutorConfig {
    /// Optional path to a precompiled module.
    pub patch_module_path: Option<PathBuf>,
    /// Maximum linear memory size in bytes.
    pub max_memory_bytes: usize,
}

impl Default for WasiExecutorConfig {
    fn default() -> Self {
        Self {
            patch_module_path: None,
            max_memory_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Request to execute a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    /// Program name.
    pub program: String,
    /// Command arguments.
    pub args: Vec<String>,
    /// Workspace root.
    pub workspace_root: PathBuf,
    /// Timeout in seconds.
    pub timeout_secs: u32,
}

/// Request to apply a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRequest {
    /// Workspace root.
    pub workspace_root: PathBuf,
    /// Target file.
    pub scope: PathBuf,
    /// Current file content.
    pub current: String,
    /// Unified diff patch text.
    pub patch: String,
}

/// Outcome returned by execution backends.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionOutcome {
    /// Whether the execution succeeded.
    pub success: bool,
    /// Standard output content.
    pub stdout: String,
    /// Standard error content.
    pub stderr: String,
    /// Exit code.
    pub exit_code: i32,
    /// Optional structured metadata.
    pub metadata: Option<Value>,
}

/// Tool executor contract.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a command.
    async fn run_command(&self, request: CommandRequest) -> Result<ExecutionOutcome, McpError>;

    /// Apply a patch.
    async fn apply_patch(&self, request: PatchRequest) -> Result<ExecutionOutcome, McpError>;
}

/// Shared executor router.
#[derive(Clone)]
pub struct ExecutorRouter {
    mode: ExecutionMode,
    direct: Arc<DirectExecutor>,
    container: Arc<ContainerExecutor>,
    wasi: Arc<WasiExecutor>,
}

impl ExecutorRouter {
    /// Create a new router.
    pub fn new(
        mode: ExecutionMode,
        container_config: ContainerExecutorConfig,
        wasi_config: WasiExecutorConfig,
    ) -> Self {
        Self {
            mode,
            direct: Arc::new(DirectExecutor),
            container: Arc::new(ContainerExecutor::new(container_config)),
            wasi: Arc::new(WasiExecutor::new(wasi_config)),
        }
    }

    /// Current execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }
}

#[async_trait]
impl ToolExecutor for ExecutorRouter {
    async fn run_command(&self, request: CommandRequest) -> Result<ExecutionOutcome, McpError> {
        match self.mode {
            ExecutionMode::Hybrid | ExecutionMode::Containerized => {
                self.container.run_command(request).await
            }
            ExecutionMode::Direct => self.direct.run_command(request).await,
        }
    }

    async fn apply_patch(&self, request: PatchRequest) -> Result<ExecutionOutcome, McpError> {
        match self.mode {
            ExecutionMode::Hybrid => self.wasi.apply_patch(request).await,
            ExecutionMode::Containerized => self.container.apply_patch(request).await,
            ExecutionMode::Direct => self.direct.apply_patch(request).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn patch_request(temp_dir: &TempDir) -> PatchRequest {
        let scope = temp_dir.path().join("demo.rs");
        std::fs::write(&scope, "fn before() {}\n").unwrap();
        PatchRequest {
            workspace_root: temp_dir.path().to_path_buf(),
            scope,
            current: "fn before() {}\n".to_string(),
            patch: "--- demo.rs\n+++ demo.rs\n@@ -1,1 +1,1 @@\n-fn before() {}\n+fn after() {}\n"
                .to_string(),
        }
    }

    #[tokio::test]
    async fn hybrid_apply_patch_routes_to_wasi() {
        let temp_dir = TempDir::new().unwrap();
        let router = ExecutorRouter::new(
            ExecutionMode::Hybrid,
            ContainerExecutorConfig::default(),
            WasiExecutorConfig::default(),
        );

        let outcome = router.apply_patch(patch_request(&temp_dir)).await.unwrap();

        assert!(outcome.success);
        assert_eq!(
            outcome.metadata.unwrap()["executor"],
            Value::String("wasi".to_string())
        );
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("demo.rs")).unwrap(),
            "fn after() {}\n"
        );
    }

    #[tokio::test]
    async fn direct_apply_patch_routes_to_direct() {
        let temp_dir = TempDir::new().unwrap();
        let router = ExecutorRouter::new(
            ExecutionMode::Direct,
            ContainerExecutorConfig::default(),
            WasiExecutorConfig::default(),
        );

        let outcome = router.apply_patch(patch_request(&temp_dir)).await.unwrap();

        assert!(outcome.success);
        assert_eq!(
            outcome.metadata.unwrap()["executor"],
            Value::String("direct".to_string())
        );
    }

    #[tokio::test]
    async fn hybrid_run_command_routes_to_container_backend() {
        let temp_dir = TempDir::new().unwrap();
        let router = ExecutorRouter::new(
            ExecutionMode::Hybrid,
            ContainerExecutorConfig {
                runtime_binary: "definitely-missing-container-runtime".to_string(),
                ..ContainerExecutorConfig::default()
            },
            WasiExecutorConfig::default(),
        );

        let result = router
            .run_command(CommandRequest {
                program: "echo".to_string(),
                args: vec!["hello".to_string()],
                workspace_root: temp_dir.path().to_path_buf(),
                timeout_secs: 1,
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(router.mode(), ExecutionMode::Hybrid));
    }

    #[tokio::test]
    async fn direct_run_command_routes_to_host_backend() {
        let temp_dir = TempDir::new().unwrap();
        let router = ExecutorRouter::new(
            ExecutionMode::Direct,
            ContainerExecutorConfig::default(),
            WasiExecutorConfig::default(),
        );

        let outcome = router
            .run_command(CommandRequest {
                program: "rustc".to_string(),
                args: vec!["--version".to_string()],
                workspace_root: temp_dir.path().to_path_buf(),
                timeout_secs: 5,
            })
            .await
            .unwrap();

        assert!(outcome.success);
        assert_eq!(
            outcome.metadata.unwrap()["executor"],
            Value::String("direct".to_string())
        );
    }
}
