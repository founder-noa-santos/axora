//! SDK containment boundary.
//!
//! This module owns all `async-openai` types and provides a clean interface
//! to the rest of the `openai_family` module.

pub mod client;
pub mod request;
pub mod response;
pub mod stream;

pub use client::SdkClient;
pub use request::build_sdk_request;
pub use response::{parse_raw_chat_completion_response, parse_sdk_response};
pub use stream::NormalizedStream;
