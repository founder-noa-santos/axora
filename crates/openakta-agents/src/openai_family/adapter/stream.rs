//! Streaming handler: SDK ChatCompletionResponseStream → internal ModelResponseChunk.

use async_openai::types::ChatCompletionResponseStream;
use futures::StreamExt;

use crate::openai_family::error::TransportError;
use crate::provider::{ModelResponseChunk, ProviderKind};

/// Normalized stream wrapper.
pub struct NormalizedStream {
    inner: ChatCompletionResponseStream,
    done: bool,
}

impl NormalizedStream {
    /// Create a new normalized stream.
    pub fn new(inner: ChatCompletionResponseStream) -> Self {
        Self { inner, done: false }
    }

    /// Get the next chunk from the stream.
    pub async fn next_chunk(&mut self) -> Option<Result<ModelResponseChunk, TransportError>> {
        if self.done {
            return None;
        }

        match self.inner.next().await {
            Some(Ok(chunk)) => {
                let choice = chunk.choices.first()?;

                // Handle delta content
                let delta_text = choice
                    .delta
                    .content
                    .as_ref()
                    .map(|c| c.as_str())
                    .unwrap_or("");

                // Check for finish reason
                let done = choice.finish_reason.is_some();
                if done {
                    self.done = true;
                }

                Some(Ok(ModelResponseChunk {
                    provider: ProviderKind::OpenAi,
                    delta: delta_text.to_string(),
                    done,
                }))
            }
            Some(Err(err)) => {
                self.done = true;
                Some(Err(TransportError::SdkStream(err.to_string())))
            }
            None => {
                self.done = true;
                None
            }
        }
    }

    /// Check if the stream is done.
    pub fn is_done(&self) -> bool {
        self.done
    }
}

// Streaming behavior specification:
// - Partial text: emitted immediately as chunks arrive
// - Partial tool calls: accumulated until complete, then emitted (not implemented)
// - Finish reason: signaled via `done: true` on final chunk
// - Usage: arrives only in final chunk (or not at all for some providers)
// - Cancellation: drop stream, no cleanup required (HTTP connection closes)
// - Mid-stream errors: emit as Err, set done=true, no retry on stream
