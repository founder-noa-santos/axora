//! Type conversion utilities for OpenAI-family transport.
//!
//! This module provides conversion functions between SDK types and internal types.
//! It serves as the boundary layer between the SDK containment (adapter) and
//! the rest of the system.

pub use crate::openai_family::adapter::{build_sdk_request, parse_sdk_response, NormalizedStream};
