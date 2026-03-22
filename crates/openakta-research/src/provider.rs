//! Pluggable search backends.

use async_trait::async_trait;

use crate::error::SearchError;
use crate::types::{SearchOptions, SearchQuery, SearchResult};

/// Async search backend (Serper, Tavily, Brave, Exa, …).
#[async_trait]
pub trait SearchProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn search(
        &self,
        query: &SearchQuery,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError>;
}
