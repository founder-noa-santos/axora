//! Deterministic helpers used by the MCP WASI executor.

use serde::{Deserialize, Serialize};

/// Patch request executed inside the sandbox boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchRequest {
    /// Current file content.
    pub current: String,
    /// Unified diff patch content.
    pub patch: String,
}

/// Patch execution result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchResponse {
    /// Whether the patch succeeded.
    pub success: bool,
    /// Patched file content.
    pub content: String,
    /// Optional failure detail.
    pub error: Option<String>,
}

/// Apply a unified diff deterministically.
pub fn apply_patch(request: &PatchRequest) -> PatchResponse {
    let result = openakta_cache::apply_patch(&request.current, &request.patch);
    PatchResponse {
        success: result.success,
        content: result.content,
        error: result.error,
    }
}
