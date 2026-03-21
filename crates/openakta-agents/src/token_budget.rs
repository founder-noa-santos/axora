//! Token budget derivation helpers.

use crate::provider_transport::ModelRegistryEntry;

/// Derived token budget limits for a specific model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveTokenBudget {
    /// Effective retrieval token cap.
    pub retrieval_cap: usize,
    /// Effective task token cap.
    pub task_cap: u32,
}

/// Derive effective retrieval and task budgets from user caps and model metadata.
pub fn derive_effective_budget(
    entry: Option<&ModelRegistryEntry>,
    user_retrieval_cap: usize,
    user_task_cap: u32,
    context_use_ratio: f32,
    margin_tokens: u32,
    retrieval_share: f32,
) -> EffectiveTokenBudget {
    let Some(entry) = entry else {
        return EffectiveTokenBudget {
            retrieval_cap: user_retrieval_cap,
            task_cap: user_task_cap,
        };
    };

    let prompt_ceiling = ((entry.max_context_window as f32 * context_use_ratio).floor() as i64
        - entry.max_output_tokens as i64
        - margin_tokens as i64)
        .max(0) as u32;

    EffectiveTokenBudget {
        retrieval_cap: user_retrieval_cap.min((prompt_ceiling as f32 * retrieval_share) as usize),
        task_cap: user_task_cap.min(prompt_ceiling),
    }
}
