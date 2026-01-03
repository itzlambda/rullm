use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::StreamExt;
use futures::stream::Stream;

use crate::error::{ClientError, StreamError};
use crate::types::{
    ChatChoice, ChatCompletion, ChatCompletionChunk, ChunkDelta, FunctionCall, Message,
    MessageContent, ModelId, Role, ToolCall, ToolCallDelta, Usage,
};

/// A stream of chat completion chunks.
pub struct ChatCompletionStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ClientError>> + Send>>,
}

impl ChatCompletionStream {
    /// Create a new stream from a byte stream.
    pub fn new<S>(byte_stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    {
        let chunk_stream = parse_sse_stream(byte_stream);
        Self {
            inner: Box::pin(chunk_stream),
        }
    }

    /// Consume this stream and return an accumulator that collects chunks into a final response.
    pub fn accumulator(self) -> ChatCompletionAccumulator {
        ChatCompletionAccumulator::new(self)
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk, ClientError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Parse an SSE byte stream into chat completion chunks.
fn parse_sse_stream<S>(
    byte_stream: S,
) -> impl Stream<Item = Result<ChatCompletionChunk, ClientError>>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    async_stream::try_stream! {
        let mut buffer = String::new();

        futures::pin_mut!(byte_stream);

        while let Some(result) = byte_stream.next().await {
            let bytes = result?;
            let text = String::from_utf8_lossy(&bytes);
            buffer.push_str(&text);

            // Process complete lines
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim_end_matches('\r').to_string();
                buffer = buffer[line_end + 1..].to_string();

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                // Parse SSE data lines
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();

                    // Check for stream end
                    if data == "[DONE]" {
                        return;
                    }

                    // Parse JSON
                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                        Ok(chunk) => yield chunk,
                        Err(e) => {
                            // Check if it's an error object
                            if let Ok(error_wrapper) = serde_json::from_str::<SseErrorWrapper>(data) {
                                Err(ClientError::Stream(StreamError::ApiError(
                                    crate::error::ApiError {
                                        status: 0,
                                        error: error_wrapper.error,
                                        raw_body: Some(Arc::from(data)),
                                    },
                                )))?;
                            } else {
                                Err(ClientError::Stream(StreamError::ParseError(
                                    format!("Failed to parse SSE data: {}", e),
                                )))?;
                            }
                        }
                    }
                }
            }
        }

        // Handle any remaining data in buffer
        if !buffer.trim().is_empty() {
            let data = buffer.trim();
            if let Some(data) = data.strip_prefix("data: ") {
                if data != "[DONE]" && !data.is_empty() {
                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                        Ok(chunk) => yield chunk,
                        Err(_) => {
                            // Ignore trailing incomplete data
                        }
                    }
                }
            }
        }
    }
}

/// Wrapper for error objects in SSE data.
#[derive(serde::Deserialize)]
struct SseErrorWrapper {
    error: crate::error::ApiErrorBody,
}

/// Accumulator for collecting streaming chunks into a final response.
pub struct ChatCompletionAccumulator {
    stream: ChatCompletionStream,
    id: Option<Arc<str>>,
    object: Option<Arc<str>>,
    created: Option<u64>,
    model: Option<ModelId>,
    service_tier: Option<Arc<str>>,
    system_fingerprint: Option<Arc<str>>,
    choices: HashMap<u32, AccumulatedChoice>,
    usage: Option<Usage>,
}

impl ChatCompletionAccumulator {
    /// Create a new accumulator from a stream.
    pub fn new(stream: ChatCompletionStream) -> Self {
        Self {
            stream,
            id: None,
            object: None,
            created: None,
            model: None,
            service_tier: None,
            system_fingerprint: None,
            choices: HashMap::new(),
            usage: None,
        }
    }

    /// Consume all chunks and return the final chat completion.
    pub async fn collect(mut self) -> Result<ChatCompletion, ClientError> {
        while let Some(chunk) = self.stream.next().await {
            self.process_chunk(chunk?);
        }
        Ok(self.into_completion())
    }

    /// Process a chunk and update the accumulated state.
    fn process_chunk(&mut self, chunk: ChatCompletionChunk) {
        // Update metadata from first chunk
        if self.id.is_none() {
            self.id = Some(chunk.id);
            self.object = Some(chunk.object);
            self.created = Some(chunk.created);
            self.model = Some(chunk.model);
        }

        // Update optional fields
        if chunk.service_tier.is_some() {
            self.service_tier = chunk.service_tier;
        }
        if chunk.system_fingerprint.is_some() {
            self.system_fingerprint = chunk.system_fingerprint;
        }
        if chunk.usage.is_some() {
            self.usage = chunk.usage;
        }

        // Process choices
        for choice in chunk.choices.iter() {
            let accumulated = self
                .choices
                .entry(choice.index)
                .or_insert_with(|| AccumulatedChoice::new(choice.index));
            accumulated.process_delta(&choice.delta);

            if choice.finish_reason.is_some() {
                accumulated.finish_reason = choice.finish_reason.clone();
            }
            if choice.logprobs.is_some() {
                accumulated.logprobs = choice.logprobs.clone();
            }
        }
    }

    /// Convert the accumulated state into a final ChatCompletion.
    fn into_completion(self) -> ChatCompletion {
        let mut choices: Vec<(u32, ChatChoice)> = self
            .choices
            .into_iter()
            .map(|(idx, acc)| (idx, acc.into_choice()))
            .collect();
        choices.sort_by_key(|(idx, _)| *idx);

        ChatCompletion {
            id: self.id.unwrap_or_else(|| Arc::from("")),
            object: self.object.unwrap_or_else(|| Arc::from("chat.completion")),
            created: self.created.unwrap_or(0),
            model: self.model.unwrap_or_else(|| ModelId::new("")),
            choices: choices.into_iter().map(|(_, c)| c).collect(),
            usage: self.usage,
            service_tier: self.service_tier,
            system_fingerprint: self.system_fingerprint,
            extra: serde_json::Map::new(),
        }
    }
}

/// Accumulated state for a single choice.
struct AccumulatedChoice {
    index: u32,
    role: Option<Role>,
    content: String,
    refusal: Option<String>,
    tool_calls: HashMap<u32, AccumulatedToolCall>,
    finish_reason: Option<Arc<str>>,
    logprobs: Option<crate::types::Logprobs>,
}

impl AccumulatedChoice {
    fn new(index: u32) -> Self {
        Self {
            index,
            role: None,
            content: String::new(),
            refusal: None,
            tool_calls: HashMap::new(),
            finish_reason: None,
            logprobs: None,
        }
    }

    fn process_delta(&mut self, delta: &ChunkDelta) {
        if delta.role.is_some() {
            self.role = delta.role.clone();
        }

        if let Some(ref content) = delta.content {
            self.content.push_str(content);
        }

        if let Some(ref refusal) = delta.refusal {
            self.refusal
                .get_or_insert_with(String::new)
                .push_str(refusal);
        }

        if let Some(ref tool_calls) = delta.tool_calls {
            for tc_delta in tool_calls.iter() {
                let accumulated = self
                    .tool_calls
                    .entry(tc_delta.index)
                    .or_insert_with(|| AccumulatedToolCall::new(tc_delta.index));
                accumulated.process_delta(tc_delta);
            }
        }
    }

    fn into_choice(self) -> ChatChoice {
        // Build tool calls if any
        let tool_calls = if self.tool_calls.is_empty() {
            None
        } else {
            let mut calls: Vec<(u32, ToolCall)> = self
                .tool_calls
                .into_iter()
                .map(|(idx, acc)| (idx, acc.into_tool_call()))
                .collect();
            calls.sort_by_key(|(idx, _)| *idx);
            Some(calls.into_iter().map(|(_, tc)| tc).collect::<Arc<[_]>>())
        };

        // Build content
        let content = if self.content.is_empty() {
            None
        } else {
            Some(MessageContent::Text(Arc::from(self.content)))
        };

        // Build message
        let message = Some(Message {
            role: self.role.unwrap_or_else(Role::assistant),
            content,
            name: None,
            tool_calls,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: serde_json::Map::new(),
        });

        ChatChoice {
            index: self.index,
            message,
            finish_reason: self.finish_reason,
            logprobs: self.logprobs,
            extra: serde_json::Map::new(),
        }
    }
}

/// Accumulated state for a tool call.
struct AccumulatedToolCall {
    #[allow(dead_code)]
    index: u32,
    id: Option<Arc<str>>,
    call_type: Option<Arc<str>>,
    function_name: Option<Arc<str>>,
    function_arguments: String,
}

impl AccumulatedToolCall {
    fn new(index: u32) -> Self {
        Self {
            index,
            id: None,
            call_type: None,
            function_name: None,
            function_arguments: String::new(),
        }
    }

    fn process_delta(&mut self, delta: &ToolCallDelta) {
        if delta.id.is_some() {
            self.id = delta.id.clone();
        }
        if delta.call_type.is_some() {
            self.call_type = delta.call_type.clone();
        }
        if let Some(ref func) = delta.function {
            if func.name.is_some() {
                self.function_name = func.name.clone();
            }
            if let Some(ref args) = func.arguments {
                self.function_arguments.push_str(args);
            }
        }
    }

    fn into_tool_call(self) -> ToolCall {
        ToolCall {
            id: self.id.unwrap_or_else(|| Arc::from("")),
            call_type: self.call_type.unwrap_or_else(|| Arc::from("function")),
            function: FunctionCall {
                name: self.function_name.unwrap_or_else(|| Arc::from("")),
                arguments: Arc::from(self.function_arguments),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::stream;

    #[tokio::test]
    async fn test_parse_simple_sse() {
        let data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
"#;

        let byte_stream = stream::once(async move { Ok::<_, reqwest::Error>(Bytes::from(data)) });
        let mut stream = ChatCompletionStream::new(byte_stream);

        let mut chunks = Vec::new();
        while let Some(result) = stream.next().await {
            chunks.push(result.unwrap());
        }

        assert_eq!(chunks.len(), 4);
        assert_eq!(
            chunks[0].choices[0].delta.role.as_ref().unwrap().0.as_ref(),
            "assistant"
        );
        assert_eq!(
            chunks[1].choices[0]
                .delta
                .content
                .as_ref()
                .unwrap()
                .as_ref(),
            "Hello"
        );
        assert_eq!(
            chunks[2].choices[0]
                .delta
                .content
                .as_ref()
                .unwrap()
                .as_ref(),
            " world"
        );
        assert_eq!(
            chunks[3].choices[0]
                .finish_reason
                .as_ref()
                .unwrap()
                .as_ref(),
            "stop"
        );
    }

    #[tokio::test]
    async fn test_accumulator() {
        let data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" world!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
"#;

        let byte_stream = stream::once(async move { Ok::<_, reqwest::Error>(Bytes::from(data)) });
        let stream = ChatCompletionStream::new(byte_stream);
        let completion = stream.accumulator().collect().await.unwrap();

        assert_eq!(completion.id.as_ref(), "chatcmpl-123");
        assert_eq!(completion.first_text(), Some("Hello world!"));
        assert_eq!(completion.finish_reason(), Some("stop"));
    }
}
