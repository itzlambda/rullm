//! Main client implementation for the Anthropic API
//!
//! Provides `Client` and sub-clients like `MessagesClient`.

use crate::config::{ClientBuilder, ClientConfig, RequestOptions};
use crate::error::Result;
use crate::messages::{
    CountTokensRequest, CountTokensResponse, MessageStream, MessagesRequest, MessagesResponse,
    StreamEvent, parse_sse_stream,
};
use crate::transport::HttpTransport;
use futures::Stream;
use std::sync::Arc;

/// Main Anthropic API client
///
/// # Example
///
/// ```no_run
/// use rullm_anthropic::{Client, Message, MessagesRequest, RequestOptions};
///
/// # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
/// let client = Client::from_env()?;
///
/// let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
///     .system("You are a helpful assistant.")
///     .message(Message::user("Hello!"))
///     .build();
///
/// let response = client.messages().create(request, RequestOptions::default()).await?;
/// println!("{}", response.text());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    transport: Arc<HttpTransport>,
}

impl Client {
    /// Create a new client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a client from environment variables
    ///
    /// Reads from:
    /// - `ANTHROPIC_API_KEY` - API key
    /// - `ANTHROPIC_AUTH_TOKEN` - OAuth token (takes precedence)
    /// - `ANTHROPIC_BASE_URL` - Base URL override
    pub fn from_env() -> Result<Self> {
        let config = ClientBuilder::from_env().build()?;
        Self::new(config)
    }

    /// Create a client with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        let transport = HttpTransport::new(config)?;
        Ok(Self {
            transport: Arc::new(transport),
        })
    }

    /// Get the messages sub-client
    pub fn messages(&self) -> MessagesClient {
        MessagesClient {
            transport: self.transport.clone(),
        }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        self.transport.base_url()
    }
}

/// Messages API sub-client
#[derive(Clone)]
pub struct MessagesClient {
    transport: Arc<HttpTransport>,
}

impl MessagesClient {
    const MESSAGES_PATH: &'static str = "/v1/messages";
    const COUNT_TOKENS_PATH: &'static str = "/v1/messages/count_tokens";

    /// Create a message (non-streaming)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rullm_anthropic::{Client, Message, MessagesRequest, RequestOptions};
    ///
    /// # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
    /// let client = Client::from_env()?;
    ///
    /// let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
    ///     .message(Message::user("What is 2+2?"))
    ///     .temperature(0.0)
    ///     .build();
    ///
    /// let response = client.messages().create(request, RequestOptions::default()).await?;
    /// println!("Answer: {}", response.text());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<MessagesResponse> {
        let (response, _meta) = self
            .transport
            .post_json(Self::MESSAGES_PATH, &request, &options)
            .await?;
        Ok(response)
    }

    /// Create a streaming message
    ///
    /// Returns a `MessageStream` that provides:
    /// - Raw event stream via `Stream` trait
    /// - Text-only stream via `text_stream()`
    /// - Final message via `final_message()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rullm_anthropic::{Client, Message, MessagesRequest, RequestOptions};
    /// use futures::StreamExt;
    /// use std::pin::pin;
    ///
    /// # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
    /// let client = Client::from_env()?;
    /// let messages = client.messages();
    ///
    /// let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
    ///     .message(Message::user("Tell me a story"))
    ///     .build();
    ///
    /// let stream = messages.stream(request, RequestOptions::default()).await?;
    /// let mut text_stream = pin!(stream.text_stream());
    ///
    /// while let Some(chunk) = text_stream.next().await {
    ///     print!("{}", chunk?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stream(
        &self,
        mut request: MessagesRequest,
        options: RequestOptions,
    ) -> Result<MessageStream<impl Stream<Item = Result<StreamEvent>> + Unpin>> {
        // Force streaming
        request.stream = Some(true);

        let (byte_stream, _meta) = self
            .transport
            .post_stream(Self::MESSAGES_PATH, &request, &options)
            .await?;

        let event_stream = parse_sse_stream(byte_stream);
        Ok(MessageStream::new(Box::pin(event_stream)))
    }

    /// Count tokens for a request without sending it
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rullm_anthropic::{Client, CountTokensRequest, Message, RequestOptions};
    ///
    /// # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
    /// let client = Client::from_env()?;
    ///
    /// let request = CountTokensRequest::new(
    ///     "claude-3-5-sonnet-20241022",
    ///     vec![Message::user("Hello, how are you?")],
    /// );
    ///
    /// let count = client.messages().count_tokens(request, RequestOptions::default()).await?;
    /// println!("Input tokens: {}", count.input_tokens);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count_tokens(
        &self,
        request: CountTokensRequest,
        options: RequestOptions,
    ) -> Result<CountTokensResponse> {
        let (response, _meta) = self
            .transport
            .post_json(Self::COUNT_TOKENS_PATH, &request, &options)
            .await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, Role};

    #[test]
    fn test_client_builder() {
        let result = Client::builder().api_key("test-key").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_messages_request_builder() {
        let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
            .system("You are helpful")
            .message(Message::user("Hello"))
            .temperature(0.7)
            .build();

        assert_eq!(request.model.as_ref(), "claude-3-5-sonnet-20241022");
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.messages.len(), 1);
        assert!(request.temperature.is_some());
    }

    #[test]
    fn test_message_helpers() {
        let user_msg = Message::user("Hello");
        assert!(matches!(user_msg.role, Role::User));

        let assistant_msg = Message::assistant("Hi there!");
        assert!(matches!(assistant_msg.role, Role::Assistant));
    }
}
