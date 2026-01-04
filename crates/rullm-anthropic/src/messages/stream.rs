//! Streaming support for the Messages API
//!
//! Provides SSE parsing, streaming event types, and message accumulation.

use crate::error::{AnthropicError, Result};
use crate::messages::types::{ContentBlock, MessagesResponse, Role, StopReason, Usage};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

// =============================================================================
// Stream Event Types
// =============================================================================

/// Streaming event from the Messages API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Start of message
    MessageStart { message: MessageStartData },

    /// Start of content block
    ContentBlockStart {
        index: u32,
        content_block: ContentBlockStartData,
    },

    /// Incremental content delta
    ContentBlockDelta { index: u32, delta: Delta },

    /// End of content block
    ContentBlockStop { index: u32 },

    /// Message delta (stop reason, usage)
    MessageDelta {
        delta: MessageDeltaData,
        usage: Usage,
    },

    /// End of stream
    MessageStop,

    /// Ping event (keep-alive)
    Ping,

    /// Error event
    Error { error: StreamErrorData },
}

/// Message start data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartData {
    pub id: Arc<str>,
    #[serde(rename = "type")]
    pub message_type: Arc<str>,
    pub role: Role,
    pub content: Vec<serde_json::Value>,
    pub model: Arc<str>,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<Arc<str>>,
    pub usage: Usage,
}

/// Content block start data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStartData {
    Text { text: Arc<str> },
    ToolUse { id: Arc<str>, name: Arc<str> },
    Thinking { thinking: Arc<str> },
    ServerToolUse { id: Arc<str>, name: Arc<str> },
}

/// Delta (incremental change)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    /// Text delta
    TextDelta { text: Arc<str> },
    /// Tool input delta
    InputJsonDelta { partial_json: Arc<str> },
    /// Thinking delta
    ThinkingDelta { thinking: Arc<str> },
}

impl Delta {
    /// Get text content if this is a text delta
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::TextDelta { text } => Some(text),
            _ => None,
        }
    }
}

/// Message delta data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<Arc<str>>,
}

/// Stream error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamErrorData {
    #[serde(rename = "type")]
    pub error_type: Arc<str>,
    pub message: Arc<str>,
}

// =============================================================================
// SSE Parser
// =============================================================================

/// Parses Server-Sent Events from a byte stream
pub struct SseParser<S> {
    stream: S,
    buffer: String,
    event_queue: Vec<String>,
}

impl<S> SseParser<S>
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    /// Create a new SSE parser
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: String::new(),
            event_queue: Vec::new(),
        }
    }

    fn parse_events(&mut self) {
        // Split by SSE event delimiter "\n\n"
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_block = self.buffer[..pos].to_string();
            self.buffer.drain(..pos + 2);

            // Extract data lines
            for line in event_block.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    // Skip [DONE] messages (though Anthropic doesn't use these)
                    let trimmed = data.trim();
                    if !trimmed.is_empty() && trimmed != "[DONE]" {
                        self.event_queue.push(data.to_string());
                    }
                }
            }
        }
    }
}

impl<S> Stream for SseParser<S>
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Return queued events first
            if !self.event_queue.is_empty() {
                return Poll::Ready(Some(Ok(self.event_queue.remove(0))));
            }

            // Parse any complete events from buffer
            self.parse_events();
            if !self.event_queue.is_empty() {
                return Poll::Ready(Some(Ok(self.event_queue.remove(0))));
            }

            // Get more data from stream
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            // Normalize CRLF to LF
                            let normalized = text.replace("\r\n", "\n");
                            self.buffer.push_str(&normalized);
                        }
                        Err(e) => {
                            return Poll::Ready(Some(Err(AnthropicError::serialization(
                                "Invalid UTF-8 in SSE stream",
                                Box::new(e),
                            ))));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(AnthropicError::Transport(e))));
                }
                Poll::Ready(None) => {
                    // Stream ended, parse remaining
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

/// Parse raw SSE data into stream events
pub fn parse_sse_stream<S>(stream: S) -> impl Stream<Item = Result<StreamEvent>>
where
    S: Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    SseParser::new(stream).map(|result| {
        result.and_then(|data| {
            serde_json::from_str::<StreamEvent>(&data).map_err(|e| {
                AnthropicError::serialization(format!("Failed to parse event: {data}"), Box::new(e))
            })
        })
    })
}

// =============================================================================
// Message Accumulator
// =============================================================================

/// Accumulates streaming events into a complete message
#[derive(Debug, Clone)]
pub struct MessageAccumulator {
    id: Option<Arc<str>>,
    model: Option<Arc<str>>,
    role: Role,
    content_blocks: Vec<AccumulatingBlock>,
    stop_reason: Option<StopReason>,
    stop_sequence: Option<Arc<str>>,
    usage: Usage,
}

#[derive(Debug, Clone)]
enum AccumulatingBlock {
    Text(String),
    ToolUse {
        id: Arc<str>,
        name: Arc<str>,
        partial_json: String,
    },
    Thinking(String),
    ServerToolUse {
        id: Arc<str>,
        name: Arc<str>,
        partial_json: String,
    },
}

impl Default for MessageAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageAccumulator {
    /// Create a new accumulator
    pub fn new() -> Self {
        Self {
            id: None,
            model: None,
            role: Role::Assistant,
            content_blocks: Vec::new(),
            stop_reason: None,
            stop_sequence: None,
            usage: Usage::default(),
        }
    }

    /// Process a stream event
    pub fn process(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::MessageStart { message } => {
                self.id = Some(message.id.clone());
                self.model = Some(message.model.clone());
                self.role = message.role;
                self.usage = message.usage.clone();
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                let idx = *index as usize;
                // Ensure we have enough slots
                while self.content_blocks.len() <= idx {
                    self.content_blocks
                        .push(AccumulatingBlock::Text(String::new()));
                }

                self.content_blocks[idx] = match content_block {
                    ContentBlockStartData::Text { text } => {
                        AccumulatingBlock::Text(text.to_string())
                    }
                    ContentBlockStartData::ToolUse { id, name } => AccumulatingBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        partial_json: String::new(),
                    },
                    ContentBlockStartData::Thinking { thinking } => {
                        AccumulatingBlock::Thinking(thinking.to_string())
                    }
                    ContentBlockStartData::ServerToolUse { id, name } => {
                        AccumulatingBlock::ServerToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            partial_json: String::new(),
                        }
                    }
                };
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                let idx = *index as usize;
                if idx < self.content_blocks.len() {
                    match (&mut self.content_blocks[idx], delta) {
                        (AccumulatingBlock::Text(text), Delta::TextDelta { text: new_text }) => {
                            text.push_str(new_text);
                        }
                        (
                            AccumulatingBlock::ToolUse { partial_json, .. },
                            Delta::InputJsonDelta {
                                partial_json: new_json,
                            },
                        ) => {
                            partial_json.push_str(new_json);
                        }
                        (
                            AccumulatingBlock::Thinking(thinking),
                            Delta::ThinkingDelta {
                                thinking: new_thinking,
                            },
                        ) => {
                            thinking.push_str(new_thinking);
                        }
                        (
                            AccumulatingBlock::ServerToolUse { partial_json, .. },
                            Delta::InputJsonDelta {
                                partial_json: new_json,
                            },
                        ) => {
                            partial_json.push_str(new_json);
                        }
                        _ => {}
                    }
                }
            }
            StreamEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason;
                self.stop_sequence = delta.stop_sequence.clone();
                self.usage = usage.clone();
            }
            StreamEvent::ContentBlockStop { .. } | StreamEvent::MessageStop | StreamEvent::Ping => {
            }
            StreamEvent::Error { error } => {
                // Log error but don't fail accumulation
                eprintln!("Stream error: {}: {}", error.error_type, error.message);
            }
        }
    }

    /// Get current text content (for streaming display)
    pub fn current_text(&self) -> String {
        self.content_blocks
            .iter()
            .filter_map(|block| match block {
                AccumulatingBlock::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Build the final response
    pub fn build(self) -> Result<MessagesResponse> {
        let id = self.id.ok_or_else(|| {
            AnthropicError::serialization(
                "Missing message ID in stream",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "missing id",
                )),
            )
        })?;

        let model = self.model.ok_or_else(|| {
            AnthropicError::serialization(
                "Missing model in stream",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "missing model",
                )),
            )
        })?;

        let content: Vec<ContentBlock> = self
            .content_blocks
            .into_iter()
            .map(|block| match block {
                AccumulatingBlock::Text(text) => ContentBlock::Text {
                    text: Arc::from(text),
                },
                AccumulatingBlock::ToolUse {
                    id,
                    name,
                    partial_json,
                } => {
                    let input =
                        serde_json::from_str(&partial_json).unwrap_or(serde_json::Value::Null);
                    ContentBlock::ToolUse { id, name, input }
                }
                AccumulatingBlock::Thinking(thinking) => ContentBlock::Thinking {
                    thinking: Arc::from(thinking),
                },
                AccumulatingBlock::ServerToolUse {
                    id,
                    name,
                    partial_json,
                } => {
                    let input =
                        serde_json::from_str(&partial_json).unwrap_or(serde_json::Value::Null);
                    ContentBlock::ServerToolUse { id, name, input }
                }
            })
            .collect();

        Ok(MessagesResponse {
            id,
            response_type: Arc::from("message"),
            role: self.role,
            content,
            model,
            stop_reason: self.stop_reason,
            stop_sequence: self.stop_sequence,
            usage: self.usage,
        })
    }
}

// =============================================================================
// Message Stream
// =============================================================================

/// High-level stream wrapper for consuming streaming responses
pub struct MessageStream<S>
where
    S: Stream<Item = Result<StreamEvent>> + Unpin,
{
    inner: S,
    accumulator: MessageAccumulator,
    finished: bool,
}

impl<S> MessageStream<S>
where
    S: Stream<Item = Result<StreamEvent>> + Unpin,
{
    /// Create a new message stream
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            accumulator: MessageAccumulator::new(),
            finished: false,
        }
    }

    /// Get a stream of text chunks only
    pub fn text_stream(self) -> impl Stream<Item = Result<Arc<str>>> {
        async_stream::stream! {
            let mut stream = self;
            while let Some(event) = stream.inner.next().await {
                match event {
                    Ok(StreamEvent::ContentBlockDelta { delta: Delta::TextDelta { text }, .. }) => {
                        yield Ok(text);
                    }
                    Ok(event) => {
                        stream.accumulator.process(&event);
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        }
    }

    /// Get the final accumulated message after consuming the stream
    pub async fn final_message(mut self) -> Result<MessagesResponse> {
        while let Some(event) = self.inner.next().await {
            let event = event?;
            self.accumulator.process(&event);
            if matches!(event, StreamEvent::MessageStop) {
                self.finished = true;
                break;
            }
        }
        self.accumulator.build()
    }
}

impl<S> Stream for MessageStream<S>
where
    S: Stream<Item = Result<StreamEvent>> + Unpin,
{
    type Item = Result<StreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                self.accumulator.process(&event);
                if matches!(event, StreamEvent::MessageStop) {
                    self.finished = true;
                }
                Poll::Ready(Some(Ok(event)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    fn bytes_from_str(s: &str) -> bytes::Bytes {
        bytes::Bytes::from(s.to_string())
    }

    #[tokio::test]
    async fn test_sse_parser_single_event() {
        let data = vec![Ok(bytes_from_str("data: {\"type\":\"ping\"}\n\n"))];
        let stream = stream::iter(data);
        let mut parser = SseParser::new(stream);

        let result = parser.next().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "{\"type\":\"ping\"}");
    }

    #[tokio::test]
    async fn test_sse_parser_multiple_events() {
        let data = vec![Ok(bytes_from_str(
            "data: {\"type\":\"ping\"}\n\ndata: {\"type\":\"message_stop\"}\n\n",
        ))];
        let stream = stream::iter(data);
        let parser = SseParser::new(stream);
        let results: Vec<_> = parser.collect().await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_sse_parser_chunked() {
        let data = vec![
            Ok(bytes_from_str("data: {\"type\":")),
            Ok(bytes_from_str("\"ping\"}\n\n")),
        ];
        let stream = stream::iter(data);
        let mut parser = SseParser::new(stream);

        let result = parser.next().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "{\"type\":\"ping\"}");
    }

    #[tokio::test]
    async fn test_accumulator_text() {
        let mut acc = MessageAccumulator::new();

        acc.process(&StreamEvent::MessageStart {
            message: MessageStartData {
                id: Arc::from("msg_123"),
                message_type: Arc::from("message"),
                role: Role::Assistant,
                content: vec![],
                model: Arc::from("claude-3-5-sonnet-20241022"),
                stop_reason: None,
                stop_sequence: None,
                usage: Usage::default(),
            },
        });

        acc.process(&StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStartData::Text {
                text: Arc::from(""),
            },
        });

        acc.process(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: Delta::TextDelta {
                text: Arc::from("Hello"),
            },
        });

        acc.process(&StreamEvent::ContentBlockDelta {
            index: 0,
            delta: Delta::TextDelta {
                text: Arc::from(" world!"),
            },
        });

        assert_eq!(acc.current_text(), "Hello world!");

        let response = acc.build().unwrap();
        assert_eq!(response.text(), "Hello world!");
    }
}
