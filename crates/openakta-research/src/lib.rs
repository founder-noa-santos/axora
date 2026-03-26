//! Multi-provider web search with tiered fallback for OPENAKTA research mode.
//!
//! - **Primary:** Serper (Google search API).
//! - **Fallback:** Tavily (AI-oriented results).
//! - **Premium BYOK:** Brave (Web Search API), Exa (neural semantic search).
//! - **BYOK:** API keys via workspace-relative files (e.g. `.openakta/secrets/serper.key`).
//!
//! Use [`ResearchRuntime::from_workspace`] after deserializing [`ResearchConfig`] from `openakta.toml`.
//!
//! ## Optional live tests
//!
//! Integration tests against real APIs are not run by default. Set `OPENAKTA_RESEARCH_LIVE=1` and
//! valid key files when adding a dedicated live test in the future.

mod brave;
mod error;
mod exa;
mod http_util;
mod normalize;
mod provider;
mod router;
pub mod runtime;
mod serde_util;
mod serper;
#[cfg(feature = "local-memory")]
pub mod storage;
mod tavily;
mod types;
#[cfg(feature = "local-memory")]
pub mod vector_math;

pub use brave::{parse_brave_response_body, BraveClient};
pub use error::{is_retryable_http_status, SearchError};
pub use exa::{parse_exa_response_body, ExaClient, EXA_CATEGORY_GITHUB};
pub use normalize::normalize_results;
#[cfg(feature = "local-memory")]
pub use openakta_embeddings::{
    DeterministicTestEmbeddingProvider, EmbeddingProvider, ResearchMinilmConfig,
    ResearchMinilmEmbedder, RESEARCH_EMBED_BYTES, RESEARCH_EMBED_DIM,
};
pub use provider::SearchProvider;
pub use router::SearchRouter;
pub use runtime::{
    BraveProviderConfig, ExaProviderConfig, ResearchConfig, ResearchRuntime, SerperProviderConfig,
    TavilyProviderConfig,
};
pub use serper::{parse_serper_response_body, SerperClient};
#[cfg(feature = "local-memory")]
pub use storage::{
    embedding_to_blob, ResearchStorage, ResearchStorageError, Result as ResearchStorageResult,
};
pub use tavily::{parse_tavily_response_body, TavilyClient};
pub use types::{SearchOptions, SearchQuery, SearchResult};
