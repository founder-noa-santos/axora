//! OpenAI-family transport layer.
//!
//! This module provides a strongly-typed integration with OpenAI and OpenAI-compatible providers
//! using the `async-openai` SDK.
//!
//! ## Architecture
//!
//! - `adapter`: SDK containment boundary (owns async-openai types)
//! - `types`: Conversion between SDK types and internal types
//! - `capabilities`: Provider capability matrix
//! - `config`: Configuration types
//! - `error`: Error types and mapping
//! - `transport`: Transport implementation
//!
//! ## SDK Containment Boundary
//!
//! **Only the `adapter` submodule may import `async_openai` types.**
//! All other code must use normalized types from this module.
//! This prevents SDK leakage throughout the codebase and maintains clean abstraction boundaries.
//!
//! ## Provider Support
//!
//! - ✅ OpenAI (official API)
//! - ✅ OpenAI-compatible providers (Qwen, DeepSeek, Moonshot, OpenRouter, etc.)
//! - ❌ Anthropic (intentionally removed - may re-enter via openakta-api)

pub mod adapter;
pub mod capabilities;
pub mod config;
pub mod error;
pub mod transport;
pub mod types;
pub mod validation;
pub mod wrapper;

pub use capabilities::{ModelCapabilities, ProviderCapabilities, ResolvedCapabilities};
pub use config::{CompatibleProviderConfig, OfficialOpenAiConfig, OpenAiFamilyConfig};
pub use error::TransportError;
pub use transport::OpenAiFamilyTransport;
