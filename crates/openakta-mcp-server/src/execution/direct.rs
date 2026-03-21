//! Direct host execution backend.

use crate::execution::{CommandRequest, ExecutionOutcome, PatchRequest, ToolExecutor};
use crate::McpError;
use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

/// Direct executor with host ambient authority.
#[derive(Debug, Default)]
pub struct DirectExecutor;

#[async_trait]
impl ToolExecutor for DirectExecutor {
    async fn run_command(&self, request: CommandRequest) -> Result<ExecutionOutcome, McpError> {
        let mut process = TokioCommand::new(&request.program);
        process
            .args(&request.args)
            .current_dir(&request.workspace_root)
            .kill_on_drop(true);
        let output = match timeout(
            Duration::from_secs(request.timeout_secs as u64),
            process.output(),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(err)) => return Err(McpError::ToolExecution(err.to_string())),
            Err(_) => {
                return Ok(ExecutionOutcome {
                    success: false,
                    stdout: String::new(),
                    stderr: format!(
                        "command '{}' timed out after {}s",
                        request.program, request.timeout_secs
                    ),
                    exit_code: -1,
                    metadata: Some(json!({ "executor": "direct" })),
                });
            }
        };

        Ok(ExecutionOutcome {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            metadata: Some(json!({ "executor": "direct" })),
        })
    }

    async fn apply_patch(&self, request: PatchRequest) -> Result<ExecutionOutcome, McpError> {
        let result = openakta_cache::apply_patch(&request.current, &request.patch);
        if !result.success {
            return Ok(ExecutionOutcome {
                success: false,
                stdout: String::new(),
                stderr: result.error.unwrap_or_else(|| "patch failed".to_string()),
                exit_code: 1,
                metadata: Some(json!({ "executor": "direct" })),
            });
        }

        std::fs::write(&request.scope, &result.content)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(ExecutionOutcome {
            success: true,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            metadata: Some(json!({
                "executor": "direct",
                "path": request.scope,
                "applied": true,
            })),
        })
    }
}
