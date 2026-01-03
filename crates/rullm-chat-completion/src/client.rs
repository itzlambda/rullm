use std::sync::Arc;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};

use crate::config::{ApiResponse, AuthConfig, ClientConfig, ResponseMeta};
use crate::error::{ApiError, ApiErrorBody, ClientError, DeserializeError, HttpError};
use crate::streaming::ChatCompletionStream;
use crate::types::{
    ChatCompletion, ChatCompletionRequest, Message, ModelId, ResponseFormat, Stop, StreamOptions,
    ToolChoice, ToolDefinition,
};

/// The main client for the Chat Completions API.
#[derive(Clone)]
pub struct ChatCompletionsClient {
    http: reqwest::Client,
    config: Arc<ClientConfig>,
}

impl ChatCompletionsClient {
    /// Create a new client with the given configuration.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let mut builder = reqwest::Client::builder().timeout(config.timeout);

        // Add default headers
        let mut headers = config.default_headers.clone();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        builder = builder.default_headers(headers);

        let http = builder.build().map_err(|e| HttpError {
            message: format!("Failed to build HTTP client: {}", e),
            source: Some(e),
        })?;

        Ok(Self {
            http,
            config: Arc::new(config),
        })
    }

    /// Create a chat completion (non-streaming).
    pub async fn create(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ApiResponse<ChatCompletion>, ClientError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut request = self.http.post(&url);
        request = self.apply_auth(request);

        let body = serde_json::to_string(&req)?;
        request = request.body(body);

        let response = request.send().await?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let meta = ResponseMeta::from_headers(&headers);

        let raw_body = response.text().await?;

        if status >= 400 {
            return Err(self.parse_error_response(status, &raw_body));
        }

        let completion: ChatCompletion =
            serde_json::from_str(&raw_body).map_err(|e| DeserializeError {
                message: format!("Failed to parse response: {}", e),
                source: Some(e),
                raw_body: Some(Arc::from(raw_body.as_str())),
            })?;

        Ok(ApiResponse::with_raw_json(
            completion,
            meta,
            Arc::from(raw_body),
        ))
    }

    /// Create a streaming chat completion.
    pub async fn stream(
        &self,
        mut req: ChatCompletionRequest,
    ) -> Result<ChatCompletionStream, ClientError> {
        // Ensure streaming is enabled
        req.stream = Some(true);

        let url = format!("{}/chat/completions", self.config.base_url);

        let mut request = self.http.post(&url);
        request = self.apply_auth(request);

        let body = serde_json::to_string(&req)?;
        request = request.body(body);

        let response = request.send().await?;
        let status = response.status().as_u16();

        if status >= 400 {
            let raw_body = response.text().await?;
            return Err(self.parse_error_response(status, &raw_body));
        }

        Ok(ChatCompletionStream::new(response.bytes_stream()))
    }

    /// Get a convenience builder for chat requests.
    pub fn chat(&self) -> ChatRequestBuilder {
        ChatRequestBuilder::new(self.clone())
    }

    /// Apply authentication to a request.
    fn apply_auth(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth {
            AuthConfig::None => {}
            AuthConfig::BearerToken(token) => {
                let auth_value = format!("Bearer {}", token);
                if let Ok(header) = HeaderValue::from_str(&auth_value) {
                    request = request.header(AUTHORIZATION, header);
                }
            }
            AuthConfig::Header { name, value } => {
                request = request.header(name.clone(), value.clone());
            }
            AuthConfig::QueryParam { name, value } => {
                request = request.query(&[(name.as_ref(), value.as_ref())]);
            }
        }
        request
    }

    /// Parse an error response.
    fn parse_error_response(&self, status: u16, raw_body: &str) -> ClientError {
        // Try to parse as an API error
        if let Ok(wrapper) = serde_json::from_str::<ErrorWrapper>(raw_body) {
            return ClientError::Api(ApiError {
                status,
                error: wrapper.error,
                raw_body: Some(Arc::from(raw_body)),
            });
        }

        // Fallback to a generic error
        ClientError::Api(ApiError {
            status,
            error: ApiErrorBody {
                message: Arc::from(raw_body),
                error_type: None,
                param: None,
                code: None,
            },
            raw_body: Some(Arc::from(raw_body)),
        })
    }
}

impl std::fmt::Debug for ChatCompletionsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatCompletionsClient")
            .field("base_url", &self.config.base_url)
            .finish()
    }
}

/// Wrapper for parsing API error responses.
#[derive(serde::Deserialize)]
struct ErrorWrapper {
    error: ApiErrorBody,
}

/// Builder for creating chat completion requests with a fluent API.
pub struct ChatRequestBuilder {
    client: ChatCompletionsClient,
    model: Option<ModelId>,
    messages: Vec<Message>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u32>,
    stop: Option<Stop>,
    max_completion_tokens: Option<u32>,
    tools: Option<Vec<ToolDefinition>>,
    tool_choice: Option<ToolChoice>,
    response_format: Option<ResponseFormat>,
    stream_options: Option<StreamOptions>,
    seed: Option<u64>,
}

impl ChatRequestBuilder {
    /// Create a new builder with the given client.
    fn new(client: ChatCompletionsClient) -> Self {
        Self {
            client,
            model: None,
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            n: None,
            stop: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            stream_options: None,
            seed: None,
        }
    }

    /// Set the model to use.
    pub fn model(mut self, model: impl Into<ModelId>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a system message.
    pub fn system(mut self, content: impl Into<Arc<str>>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// Add a developer message.
    pub fn developer(mut self, content: impl Into<Arc<str>>) -> Self {
        self.messages.push(Message::developer(content));
        self
    }

    /// Add a user message.
    pub fn user(mut self, content: impl Into<Arc<str>>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add an assistant message.
    pub fn assistant(mut self, content: impl Into<Arc<str>>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Add a custom message.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages.
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p sampling.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set the number of completions to generate.
    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, stop: Stop) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set the maximum number of completion tokens.
    pub fn max_completion_tokens(mut self, max: u32) -> Self {
        self.max_completion_tokens = Some(max);
        self
    }

    /// Add a tool definition.
    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set tool definitions.
    pub fn tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Enable usage reporting in stream.
    pub fn include_usage(mut self) -> Self {
        self.stream_options = Some(StreamOptions {
            include_usage: Some(true),
        });
        self
    }

    /// Set a seed for deterministic sampling.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Build the request.
    fn build_request(&self) -> Result<ChatCompletionRequest, ClientError> {
        let model = self.model.clone().ok_or_else(|| {
            ClientError::Http(HttpError {
                message: "Model is required".to_string(),
                source: None,
            })
        })?;

        Ok(ChatCompletionRequest {
            model,
            messages: self.messages.clone().into(),
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            stop: self.stop.clone(),
            presence_penalty: None,
            frequency_penalty: None,
            max_completion_tokens: self.max_completion_tokens,
            max_tokens: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            tools: self.tools.as_ref().map(|t| t.clone().into()),
            tool_choice: self.tool_choice.clone(),
            parallel_tool_calls: None,
            functions: None,
            response_format: self.response_format.clone(),
            modalities: None,
            audio: None,
            stream: None,
            stream_options: self.stream_options.clone(),
            prediction: None,
            web_search_options: None,
            reasoning_effort: None,
            service_tier: None,
            store: None,
            metadata: None,
            seed: self.seed,
            user: None,
            extra_body: serde_json::Map::new(),
        })
    }

    /// Send the request and get a non-streaming response.
    pub async fn send(self) -> Result<ApiResponse<ChatCompletion>, ClientError> {
        let req = self.build_request()?;
        self.client.create(req).await
    }

    /// Send the request and get a streaming response.
    pub async fn stream(self) -> Result<ChatCompletionStream, ClientError> {
        let req = self.build_request()?;
        self.client.stream(req).await
    }
}
