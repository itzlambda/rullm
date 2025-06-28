//! Test utilities for simulating Server-Sent Events (SSE) responses
//!
//! This module provides helpers for creating realistic SSE streams in unit tests,
//! allowing testing of streaming parsers with various edge cases and chunk boundaries.

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Configuration for fake SSE response generation
#[derive(Debug, Clone, Default)]
pub struct FakeSseConfig {
    /// Whether to append a final "[DONE]" event
    pub include_done: bool,
    /// Split events across multiple chunks to test partial frame handling
    pub chunk_size: Option<usize>,
}

/// Creates a fake SSE response stream for testing
///
/// # Arguments
/// * `events` - Array of event data (without "data: " prefix)
/// * `config` - Optional configuration for response behavior
///
/// # Returns
/// A stream that yields `Result<bytes::Bytes, reqwest::Error>` compatible with SSE parsers
///
/// # Examples
/// ```
/// use futures::StreamExt;
/// use rullm_core::utils::test_helpers::fake_sse_response;
///
/// #[tokio::test]
/// async fn test_basic_sse() {
///     let events = ["hello", "world"];
///     let stream = fake_sse_response(&events, None);
///     let chunks: Vec<_> = stream.collect().await;
///     // Verify chunks contain properly formatted SSE data
/// }
/// ```
pub fn fake_sse_response(
    events: &[&str],
    config: Option<FakeSseConfig>,
) -> impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> {
    let config = config.unwrap_or_default();

    // Build the complete SSE response
    let mut response = String::new();
    for event in events {
        response.push_str(&format!("data: {event}\n\n"));
    }

    if config.include_done {
        response.push_str("data: [DONE]\n\n");
    }

    FakeSseStream::new(response, config)
}

/// Internal stream implementation for fake SSE responses
pub struct FakeSseStream {
    data: Vec<u8>,
    position: usize,
    chunk_size: Option<usize>,
}

impl FakeSseStream {
    fn new(response: String, config: FakeSseConfig) -> Self {
        Self {
            data: response.into_bytes(),
            position: 0,
            chunk_size: config.chunk_size,
        }
    }
}

impl Stream for FakeSseStream {
    type Item = Result<bytes::Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if we've reached the end
        if self.position >= self.data.len() {
            return Poll::Ready(None);
        }

        // Determine chunk size (use configured size or remaining data)
        let chunk_size = self.chunk_size.unwrap_or(self.data.len() - self.position);
        let end_pos = std::cmp::min(self.position + chunk_size, self.data.len());

        // Extract the chunk
        let chunk = self.data[self.position..end_pos].to_vec();
        self.position = end_pos;

        Poll::Ready(Some(Ok(bytes::Bytes::from(chunk))))
    }
}

/// Creates a fake SSE response with events split across chunk boundaries
///
/// This is particularly useful for testing partial frame handling in SSE parsers.
pub fn fake_sse_response_chunked(
    events: &[&str],
    chunk_size: usize,
) -> impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> {
    fake_sse_response(
        events,
        Some(FakeSseConfig {
            chunk_size: Some(chunk_size),
            ..Default::default()
        }),
    )
}

/// Creates a fake SSE response that includes a [DONE] event at the end
pub fn fake_sse_response_with_done(
    events: &[&str],
) -> impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> {
    fake_sse_response(
        events,
        Some(FakeSseConfig {
            include_done: true,
            ..Default::default()
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_basic_fake_sse_response() {
        let events = ["hello", "world"];
        let stream = fake_sse_response(&events, None);
        let chunks: Vec<Result<bytes::Bytes, reqwest::Error>> = stream.collect().await;

        assert_eq!(chunks.len(), 1);
        let chunk = chunks[0].as_ref().unwrap();
        let data = String::from_utf8(chunk.to_vec()).unwrap();
        assert_eq!(data, "data: hello\n\ndata: world\n\n");
    }

    #[tokio::test]
    async fn test_fake_sse_response_with_done() {
        let events = ["test"];
        let stream = fake_sse_response_with_done(&events);
        let chunks: Vec<Result<bytes::Bytes, reqwest::Error>> = stream.collect().await;

        assert_eq!(chunks.len(), 1);
        let chunk = chunks[0].as_ref().unwrap();
        let data = String::from_utf8(chunk.to_vec()).unwrap();
        assert_eq!(data, "data: test\n\ndata: [DONE]\n\n");
    }

    #[tokio::test]
    async fn test_fake_sse_response_chunked() {
        let events = ["hello", "world"];
        let stream = fake_sse_response_chunked(&events, 5); // Small chunks
        let chunks: Vec<Result<bytes::Bytes, reqwest::Error>> = stream.collect().await;

        // Should have multiple chunks due to small chunk size
        assert!(chunks.len() > 1);

        // Reconstruct the full response
        let mut full_data = String::new();
        for chunk in chunks {
            let bytes = chunk.unwrap();
            full_data.push_str(core::str::from_utf8(&bytes).unwrap());
        }

        assert_eq!(full_data, "data: hello\n\ndata: world\n\n");
    }

    #[tokio::test]
    async fn test_empty_events() {
        let events: &[&str] = &[];
        let stream = fake_sse_response(events, None);
        let chunks: Vec<Result<bytes::Bytes, reqwest::Error>> = stream.collect().await;

        // Should produce no chunks for empty events
        assert!(chunks.is_empty());
    }
}
