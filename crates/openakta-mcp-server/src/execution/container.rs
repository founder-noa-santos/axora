//! Containerized execution backend.

use crate::execution::{
    CommandRequest, ContainerExecutorConfig, ExecutionOutcome, PatchRequest, ToolExecutor,
};
use crate::McpError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

/// Container runtime executor.
#[derive(Debug, Clone)]
pub struct ContainerExecutor {
    config: ContainerExecutorConfig,
}

impl ContainerExecutor {
    /// Create a new container executor.
    pub fn new(config: ContainerExecutorConfig) -> Self {
        Self { config }
    }

    fn base_args(&self, workspace_root: &Path) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-w".to_string(),
            self.config.workspace_mount_path.clone(),
            "-v".to_string(),
            format!(
                "{}:{}",
                workspace_root.display(),
                self.config.workspace_mount_path
            ),
        ];
        args.extend(self.config.extra_args.clone());
        args.push(self.config.image.clone());
        args
    }

    async fn invoke(
        &self,
        workspace_root: &Path,
        timeout_secs: u32,
        runtime_args: Vec<String>,
    ) -> Result<ExecutionOutcome, McpError> {
        let mut process = TokioCommand::new(&self.config.runtime_binary);
        process.args(&runtime_args).kill_on_drop(true);
        let output = match timeout(Duration::from_secs(timeout_secs as u64), process.output()).await
        {
            Ok(Ok(output)) => output,
            Ok(Err(err)) => return Err(McpError::ToolExecution(err.to_string())),
            Err(_) => {
                return Ok(ExecutionOutcome {
                    success: false,
                    stdout: String::new(),
                    stderr: format!(
                        "container runtime '{}' timed out after {}s",
                        self.config.runtime_binary, timeout_secs
                    ),
                    exit_code: -1,
                    metadata: Some(json!({
                        "executor": "container",
                        "workspace_root": workspace_root,
                    })),
                });
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ExecutionOutcome {
            success: output.status.success(),
            stdout,
            stderr,
            exit_code: output.status.code().unwrap_or(-1),
            metadata: Some(json!({
                "executor": "container",
                "runtime_binary": self.config.runtime_binary,
                "image": self.config.image,
            })),
        })
    }
}

#[async_trait]
impl ToolExecutor for ContainerExecutor {
    async fn run_command(&self, request: CommandRequest) -> Result<ExecutionOutcome, McpError> {
        let mut args = self.base_args(&request.workspace_root);
        args.push(request.program);
        args.extend(request.args);
        self.invoke(&request.workspace_root, request.timeout_secs, args)
            .await
    }

    async fn apply_patch(&self, request: PatchRequest) -> Result<ExecutionOutcome, McpError> {
        let payload_dir =
            std::env::temp_dir().join(format!("openakta-container-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&payload_dir)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let payload_path = payload_dir.join("patch-request.json");
        let relative_scope = request
            .scope
            .strip_prefix(&request.workspace_root)
            .unwrap_or(&request.scope)
            .to_string_lossy()
            .to_string();
        std::fs::write(
            &payload_path,
            serde_json::to_vec(&json!({
                "scope": relative_scope,
                "current": request.current,
                "patch": request.patch,
            }))
            .map_err(|err| McpError::ToolExecution(err.to_string()))?,
        )
        .map_err(|err| McpError::ToolExecution(err.to_string()))?;

        let mut args = self.base_args(&request.workspace_root);
        args.extend([
            "-v".to_string(),
            format!("{}:/payload", payload_dir.display()),
            "openakta-apply-patch".to_string(),
            "/payload/patch-request.json".to_string(),
        ]);
        let outcome = self.invoke(&request.workspace_root, 30, args).await?;
        if !outcome.success {
            return Ok(outcome);
        }

        let response: Value = serde_json::from_str(&outcome.stdout)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let success = response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !success {
            return Ok(ExecutionOutcome {
                success: false,
                stdout: outcome.stdout,
                stderr: response
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("container patch failed")
                    .to_string(),
                exit_code: 1,
                metadata: outcome.metadata,
            });
        }

        let content = response
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::ToolExecution("missing patched content".to_string()))?;
        std::fs::write(&request.scope, content)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;

        Ok(ExecutionOutcome {
            success: true,
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            exit_code: 0,
            metadata: Some(json!({
                "executor": "container",
                "runtime_binary": self.config.runtime_binary,
                "image": self.config.image,
                "path": request.scope,
                "applied": true,
            })),
        })
    }
}
