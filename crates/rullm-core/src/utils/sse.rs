use crate::error::LlmError;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Parses Server-Sent Events (SSE) from a byte stream, extracting data payloads
/// and filtering out [DONE] messages.
pub fn sse_lines<S>(stream: S) -> impl Stream<Item = Result<String, LlmError>>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    SseParser::new(stream)
}

struct SseParser<S> {
    stream: S,
    buffer: String,
    event_queue: Vec<String>,
}

impl<S> SseParser<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: String::new(),
            event_queue: Vec::new(),
        }
    }

    fn parse_events(&mut self) {
        // Split by SSE event delimiter "\n\n"
        while let Some(double_newline_pos) = self.buffer.find("\n\n") {
            let event_block = self.buffer[..double_newline_pos].to_string();
            self.buffer.drain(..double_newline_pos + 2);

            // Process the event block
            for line in event_block.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    // Skip [DONE] messages
                    if data.trim() != "[DONE]" {
                        self.event_queue.push(data.to_string());
                    }
                }
                // Ignore lines without "data: " prefix
            }
        }
    }
}

impl<S> Stream for SseParser<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<String, LlmError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // First, check if we have events in the queue
            if !self.event_queue.is_empty() {
                return Poll::Ready(Some(Ok(self.event_queue.remove(0))));
            }

            // Parse any complete events from the buffer
            self.parse_events();
            if !self.event_queue.is_empty() {
                return Poll::Ready(Some(Ok(self.event_queue.remove(0))));
            }

            // No complete events in buffer, try to get more data
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Add new bytes to buffer
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            // Normalize CRLF to LF to handle Windows-style/HTTP CRLF delimiters
                            let normalized = text.replace("\r\n", "\n");
                            self.buffer.push_str(&normalized);
                            // Continue loop to try parsing again
                        }
                        Err(e) => {
                            return Poll::Ready(Some(Err(LlmError::serialization(
                                "Invalid UTF-8 in SSE stream",
                                Box::new(e),
                            ))));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LlmError::network(format!("Stream error: {e}")))));
                }
                Poll::Ready(None) => {
                    // Stream ended, parse any remaining events
                    self.parse_events();
                    if !self.event_queue.is_empty() {
                        return Poll::Ready(Some(Ok(self.event_queue.remove(0))));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_helpers::{
        fake_sse_response, fake_sse_response_chunked, fake_sse_response_with_done,
    };
    use futures::{StreamExt, stream};

    fn bytes_from_str(s: &str) -> bytes::Bytes {
        bytes::Bytes::from(s.to_string())
    }

    #[tokio::test]
    async fn test_single_event() {
        let data = vec![Ok(bytes_from_str("data: hello\n\n"))];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, vec!["hello"]);
    }

    #[tokio::test]
    async fn test_multi_event() {
        let data = vec![Ok(bytes_from_str("data: foo\n\ndata: bar\n\n"))];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, vec!["foo", "bar"]);
    }

    #[tokio::test]
    async fn test_done_filter() {
        let data = vec![Ok(bytes_from_str(
            "data: baz\n\ndata: [DONE]\n\ndata: qux\n\n",
        ))];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, vec!["baz", "qux"]);
    }

    #[tokio::test]
    async fn test_partial_chunks() {
        let data = vec![
            Ok(bytes_from_str("data: split")),
            Ok(bytes_from_str("-me\n\n")),
        ];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, vec!["split-me"]);
    }

    #[tokio::test]
    async fn test_empty_stream() {
        let data: Vec<Result<bytes::Bytes, reqwest::Error>> = vec![];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, Vec::<String>::new());
    }

    #[tokio::test]
    async fn test_no_data_prefix() {
        let data = vec![Ok(bytes_from_str("event: test\nid: 123\ndata: valid\n\n"))];
        let stream = stream::iter(data);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let events: Vec<String> = results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(events, vec!["valid"]);
    }

    // New tests using the fake SSE helpers

    #[tokio::test]
    async fn test_fake_sse_helper_basic() {
        let events = ["message1", "message2", "message3"];
        let stream = fake_sse_response(&events, None);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let parsed_events: Vec<String> =
            results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(parsed_events, vec!["message1", "message2", "message3"]);
    }

    #[tokio::test]
    async fn test_fake_sse_helper_with_done() {
        let events = ["before_done"];
        let stream = fake_sse_response_with_done(&events);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let parsed_events: Vec<String> =
            results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        // Should only contain "before_done", [DONE] should be filtered out
        assert_eq!(parsed_events, vec!["before_done"]);
    }

    #[tokio::test]
    async fn test_fake_sse_helper_chunked_boundaries() {
        let events = ["chunk_test", "split_me"];
        let stream = fake_sse_response_chunked(&events, 7); // Small chunks to force splits

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let parsed_events: Vec<String> =
            results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(parsed_events, vec!["chunk_test", "split_me"]);
    }

    #[tokio::test]
    async fn test_realistic_openai_style_stream() {
        // Simulate a realistic OpenAI-style streaming response
        let events = [
            r#"{"choices": [{"delta": {"content": "Hello"}}]}"#,
            r#"{"choices": [{"delta": {"content": " there"}}]}"#,
            r#"{"choices": [{"delta": {"content": "!"}}]}"#,
        ];
        let stream = fake_sse_response_with_done(&events);

        let sse_stream = sse_lines(stream);
        let results: Vec<Result<String, LlmError>> = sse_stream.collect().await;
        let parsed_events: Vec<String> =
            results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(parsed_events.len(), 3);
        assert!(parsed_events[0].contains("Hello"));
        assert!(parsed_events[1].contains(" there"));
        assert!(parsed_events[2].contains("!"));
    }
}
