//! Wasmtime-backed patch executor.

use crate::execution::{ExecutionOutcome, PatchRequest, ToolExecutor, WasiExecutorConfig};
use crate::McpError;
use async_trait::async_trait;
use openakta_mcp_wasi_tools::{apply_patch, PatchRequest as SandboxPatchRequest, PatchResponse};
use serde_json::json;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

/// WASI executor for deterministic internal tools.
#[derive(Debug, Clone)]
pub struct WasiExecutor {
    config: WasiExecutorConfig,
}

impl WasiExecutor {
    /// Create a new executor.
    pub fn new(config: WasiExecutorConfig) -> Self {
        Self { config }
    }

    fn module_bytes(&self) -> Result<Vec<u8>, McpError> {
        if let Some(path) = &self.config.patch_module_path {
            return std::fs::read(path).map_err(|err| McpError::ToolExecution(err.to_string()));
        }

        wat::parse_str(
            r#"
            (module
              (import "host" "apply_patch" (func $apply_patch))
              (func (export "run")
                call $apply_patch))
            "#,
        )
        .map_err(|err| McpError::ToolExecution(err.to_string()))
    }
}

#[derive(Default)]
struct WasiPatchState {
    response: Option<PatchResponse>,
}

#[async_trait]
impl ToolExecutor for WasiExecutor {
    async fn run_command(
        &self,
        _request: crate::execution::CommandRequest,
    ) -> Result<ExecutionOutcome, McpError> {
        Err(McpError::ToolExecution(
            "WASI executor does not support arbitrary command execution".to_string(),
        ))
    }

    async fn apply_patch(&self, request: PatchRequest) -> Result<ExecutionOutcome, McpError> {
        let _wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        let engine = Engine::default();
        let module = Module::from_binary(&engine, &self.module_bytes()?)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let mut linker = Linker::new(&engine);
        let sandbox_request = SandboxPatchRequest {
            current: request.current.clone(),
            patch: request.patch.clone(),
        };
        linker
            .func_wrap(
                "host",
                "apply_patch",
                move |mut caller: wasmtime::Caller<'_, WasiPatchState>| {
                    caller.data_mut().response = Some(apply_patch(&sandbox_request));
                },
            )
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;

        let mut store = Store::new(&engine, WasiPatchState::default());
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let run = instance
            .get_typed_func::<(), ()>(&mut store, "run")
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        run.call(&mut store, ())
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;

        let response =
            store.data_mut().response.take().ok_or_else(|| {
                McpError::ToolExecution("missing WASI patch response".to_string())
            })?;
        if !response.success {
            return Ok(ExecutionOutcome {
                success: false,
                stdout: String::new(),
                stderr: response.error.unwrap_or_else(|| "patch failed".to_string()),
                exit_code: 1,
                metadata: Some(json!({ "executor": "wasi" })),
            });
        }

        std::fs::write(&request.scope, &response.content)
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        Ok(ExecutionOutcome {
            success: true,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            metadata: Some(json!({
                "executor": "wasi",
                "path": request.scope,
                "applied": true,
                "max_memory_bytes": self.config.max_memory_bytes,
            })),
        })
    }
}
