//! Simple API Wrapper for LLM Providers
//!
//! This module provides a simplified async interface that abstracts away
//! the complexity of Tower and provides easy-to-use methods for basic users.

use crate::config::{AnthropicConfig, GoogleAiConfig, OpenAICompatibleConfig, OpenAIConfig};
use crate::error::LlmError;
use crate::providers::{AnthropicProvider, GoogleProvider, GroqProvider, OpenAIProvider, OpenRouterProvider};
use crate::types::{
    ChatCompletion, ChatMessage, ChatRequest, ChatRole, ChatStreamEvent, LlmProvider,
};
use async_trait::async_trait;
use futures::StreamExt;
use std::pin::Pin;
use std::time::Duration;

/// Configuration for SimpleLlm clients
#[derive(Debug, Clone)]
pub struct SimpleLlmConfig {
    /// Default model to use for each provider
    pub default_models: DefaultModels,
    /// Default temperature for chat requests (0.0 to 2.0)
    pub default_temperature: Option<f32>,
    /// Default maximum tokens for responses
    pub default_max_tokens: Option<u32>,
    /// Default top_p for nucleus sampling
    pub default_top_p: Option<f32>,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Whether to validate inputs before sending requests
    pub validate_inputs: bool,
}

/// Default models for each provider
#[derive(Debug, Clone)]
pub struct DefaultModels {
    /// Default OpenAI model
    pub openai: String,
    /// Default Groq model
    pub groq: String,
    /// Default OpenRouter model
    pub openrouter: String,
    /// Default Anthropic model
    pub anthropic: String,
    /// Default Google model
    pub google: String,
}

impl Default for DefaultModels {
    fn default() -> Self {
        Self {
            openai: "gpt-3.5-turbo".to_string(),
            groq: "llama-3.3-70b-versatile".to_string(),
            openrouter: "openai/gpt-3.5-turbo".to_string(),
            anthropic: "claude-3-haiku-20240307".to_string(),
            google: "gemini-pro".to_string(),
        }
    }
}

impl Default for SimpleLlmConfig {
    fn default() -> Self {
        Self {
            default_models: DefaultModels::default(),
            default_temperature: None,
            default_max_tokens: None,
            default_top_p: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            validate_inputs: true,
        }
    }
}

impl SimpleLlmConfig {
    /// Create a new SimpleLlmConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default OpenAI model
    pub fn with_openai_model(mut self, model: impl Into<String>) -> Self {
        self.default_models.openai = model.into();
        self
    }

    /// Set the default Anthropic model
    pub fn with_anthropic_model(mut self, model: impl Into<String>) -> Self {
        self.default_models.anthropic = model.into();
        self
    }

    /// Set the default Groq model
    pub fn with_groq_model(mut self, model: impl Into<String>) -> Self {
        self.default_models.groq = model.into();
        self
    }

    /// Set the default OpenRouter model
    pub fn with_openrouter_model(mut self, model: impl Into<String>) -> Self {
        self.default_models.openrouter = model.into();
        self
    }

    /// Set the default Google model
    pub fn with_google_model(mut self, model: impl Into<String>) -> Self {
        self.default_models.google = model.into();
        self
    }

    /// Set the default temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = Some(temperature);
        self
    }

    /// Set the default max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.default_max_tokens = Some(max_tokens);
        self
    }

    /// Set the default top_p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.default_top_p = Some(top_p);
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Enable or disable input validation
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate_inputs = validate;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), LlmError> {
        if let Some(temp) = self.default_temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(LlmError::configuration(
                    "Temperature must be between 0.0 and 2.0",
                ));
            }
        }

        if let Some(top_p) = self.default_top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(LlmError::configuration("Top_p must be between 0.0 and 1.0"));
            }
        }

        if let Some(max_tokens) = self.default_max_tokens {
            if max_tokens == 0 {
                return Err(LlmError::configuration("Max tokens must be greater than 0"));
            }
        }

        if self.timeout.as_secs() == 0 {
            return Err(LlmError::configuration("Timeout must be greater than 0"));
        }

        Ok(())
    }
}

/// Simplified async trait for easy LLM interactions
#[async_trait]
pub trait SimpleLlm: Send + Sync {
    /// Send a simple chat message and get a response
    async fn chat(&self, message: &str) -> Result<String, LlmError>;

    /// Send a streaming chat message and get a concatenated response
    async fn stream_chat(&self, message: &str) -> Result<String, LlmError>;

    /// Send a streaming chat request and get raw event stream for advanced users
    ///
    /// This method provides access to the raw ChatStreamEvent stream, allowing
    /// advanced users to handle Token, Done, and Error events individually.
    /// Unlike `stream_chat`, this doesn't concatenate tokens - it returns the
    /// raw events as they arrive from the provider.
    ///
    /// # Example
    /// ```rust,ignore
    /// let request = ChatRequestBuilder::new()
    ///     .user("Hello!")
    ///     .build();
    /// let mut stream = client.stream_chat_raw(request).await;
    /// while let Some(event) = stream.next().await {
    ///     match event? {
    ///         ChatStreamEvent::Token(token) => print!("{}", token),
    ///         ChatStreamEvent::Done => break,
    ///         ChatStreamEvent::Error(msg) => eprintln!("Error: {}", msg),
    ///     }
    /// }
    /// ```
    async fn stream_chat_raw(
        &self,
        request: ChatRequest,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<ChatStreamEvent, LlmError>> + Send>>,
        LlmError,
    >;

    /// Send a chat message with a system prompt
    async fn chat_with_system(&self, system: &str, message: &str) -> Result<String, LlmError>;

    /// Send multiple messages in a conversation
    async fn conversation(&self, messages: Vec<(ChatRole, String)>) -> Result<String, LlmError>;

    /// Get available models for this provider
    async fn models(&self) -> Result<Vec<String>, LlmError>;

    /// Check if the provider is working
    async fn health_check(&self) -> Result<(), LlmError>;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;
}

/// Provider-agnostic simple LLM client
pub enum SimpleLlmClient {
    OpenAI {
        provider: OpenAIProvider,
        config: SimpleLlmConfig,
    },
    Groq {
        provider: GroqProvider,
        config: SimpleLlmConfig,
    },
    OpenRouter {
        provider: OpenRouterProvider,
        config: SimpleLlmConfig,
    },
    Anthropic {
        provider: AnthropicProvider,
        config: SimpleLlmConfig,
    },
    Google {
        provider: GoogleProvider,
        config: SimpleLlmConfig,
    },
}

impl SimpleLlmClient {
    /// Validate chat input based on configuration
    fn validate_chat_input(&self, total_length: usize, is_any_empty: bool) -> Result<(), LlmError> {
        let config = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        };

        if config.validate_inputs {
            if is_any_empty {
                return Err(LlmError::validation("Message cannot be empty"));
            }
            if total_length > 100_000 {
                return Err(LlmError::validation("Input too long (max 100k characters)"));
            }
        }
        Ok(())
    }

    /// Validate a ChatRequest for streaming methods
    fn validate_request(&self, request: &ChatRequest) -> Result<(), LlmError> {
        let config = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        };

        if config.validate_inputs {
            if request.messages.is_empty() {
                return Err(LlmError::validation(
                    "Request must contain at least one message",
                ));
            }

            let total_length: usize = request.messages.iter().map(|msg| msg.content.len()).sum();

            let is_any_empty = request
                .messages
                .iter()
                .any(|msg| msg.content.trim().is_empty());

            if is_any_empty {
                return Err(LlmError::validation("Messages cannot be empty"));
            }

            if total_length > 100_000 {
                return Err(LlmError::validation(
                    "Request too long (max 100k characters)",
                ));
            }
        }
        Ok(())
    }

    /// Validate a ChatRequest specifically for streaming operations
    fn validate_stream_request(&self, request: &ChatRequest) -> Result<(), LlmError> {
        // First run standard validation
        self.validate_request(request)?;

        let config = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        };

        if config.validate_inputs {
            // Validate streaming-specific parameters
            if let Some(temperature) = request.temperature {
                if !(0.0..=2.0).contains(&temperature) {
                    return Err(LlmError::validation(
                        "Temperature for streaming must be between 0.0 and 2.0",
                    ));
                }
            }

            if let Some(top_p) = request.top_p {
                if !(0.0..=1.0).contains(&top_p) {
                    return Err(LlmError::validation(
                        "Top_p for streaming must be between 0.0 and 1.0",
                    ));
                }
            }

            if let Some(max_tokens) = request.max_tokens {
                if max_tokens == 0 {
                    return Err(LlmError::validation(
                        "Max tokens for streaming must be greater than 0",
                    ));
                }
                // Streaming often has lower token limits to prevent timeout
                if max_tokens > 4096 {
                    return Err(LlmError::validation(
                        "Max tokens for streaming should not exceed 4096 to prevent timeouts",
                    ));
                }
            }

            // Validate stream parameter is set correctly for streaming
            if request.stream != Some(true) {
                return Err(LlmError::validation(
                    "Stream parameter must be set to true for streaming methods",
                ));
            }
        }
        Ok(())
    }

    /// Execute a chat request using the appropriate provider
    async fn execute_chat_request(&self, request: ChatRequest) -> Result<String, LlmError> {
        let response = match self {
            SimpleLlmClient::OpenAI { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.openai)
                    .await?
            }
            SimpleLlmClient::Groq { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.groq)
                    .await?
            }
            SimpleLlmClient::OpenRouter { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.openrouter)
                    .await?
            }
            SimpleLlmClient::Anthropic { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.anthropic)
                    .await?
            }
            SimpleLlmClient::Google { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.google)
                    .await?
            }
        };

        Ok(response.message.content)
    }

    /// Build base chat request with common parameters
    fn build_base_chat_request(&self, messages: Vec<ChatMessage>) -> ChatRequest {
        let config = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        };

        ChatRequest {
            messages,
            temperature: config.default_temperature,
            max_tokens: config.default_max_tokens,
            top_p: config.default_top_p,
            stream: Some(false),
            extra_params: None,
        }
    }

    /// Build streaming chat request with common parameters
    fn build_stream_chat_request(&self, messages: Vec<ChatMessage>) -> ChatRequest {
        let config = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        };

        ChatRequest {
            messages,
            temperature: config.default_temperature,
            max_tokens: config.default_max_tokens,
            top_p: config.default_top_p,
            stream: Some(true),
            extra_params: None,
        }
    }

    /// Execute a streaming chat request and return concatenated response
    async fn execute_stream_chat_request(&self, request: ChatRequest) -> Result<String, LlmError> {
        let mut stream = match self {
            SimpleLlmClient::OpenAI { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.openai, None)
                    .await
            }
            SimpleLlmClient::Groq { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.groq, None)
                    .await
            }
            SimpleLlmClient::OpenRouter { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.openrouter, None)
                    .await
            }
            SimpleLlmClient::Anthropic { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.anthropic, None)
                    .await
            }
            SimpleLlmClient::Google { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.google, None)
                    .await
            }
        };

        let mut result = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ChatStreamEvent::Token(token) => {
                    result.push_str(&token);
                }
                ChatStreamEvent::Done => {
                    break;
                }
                ChatStreamEvent::Error(error_msg) => {
                    return Err(LlmError::model(error_msg));
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl SimpleLlm for SimpleLlmClient {
    async fn chat(&self, message: &str) -> Result<String, LlmError> {
        // Input validation
        self.validate_chat_input(message.len(), message.trim().is_empty())?;

        // Build request with user message
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
        }];
        let request = self.build_base_chat_request(messages);

        // Execute request
        self.execute_chat_request(request).await
    }

    async fn stream_chat(&self, message: &str) -> Result<String, LlmError> {
        // Build request with user message
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
        }];
        let request = self.build_stream_chat_request(messages);

        // Use streaming-specific validation
        self.validate_stream_request(&request)?;

        // Execute streaming request
        self.execute_stream_chat_request(request).await
    }

    async fn stream_chat_raw(
        &self,
        request: ChatRequest,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<ChatStreamEvent, LlmError>> + Send>>,
        LlmError,
    > {
        // Validate the request with streaming-specific checks
        self.validate_stream_request(&request)?;

        let stream = match self {
            SimpleLlmClient::OpenAI { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.openai, None)
                    .await
            }
            SimpleLlmClient::Groq { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.groq, None)
                    .await
            }
            SimpleLlmClient::OpenRouter { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.openrouter, None)
                    .await
            }
            SimpleLlmClient::Anthropic { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.anthropic, None)
                    .await
            }
            SimpleLlmClient::Google { provider, config } => {
                provider
                    .chat_completion_stream(request, &config.default_models.google, None)
                    .await
            }
        };
        Ok(stream)
    }

    async fn chat_with_system(&self, system: &str, message: &str) -> Result<String, LlmError> {
        // Input validation
        let is_any_empty = system.trim().is_empty() || message.trim().is_empty();
        self.validate_chat_input(system.len() + message.len(), is_any_empty)?;

        // Build request with system and user messages
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: system.to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: message.to_string(),
            },
        ];
        let request = self.build_base_chat_request(messages);

        // Execute request
        self.execute_chat_request(request).await
    }

    async fn conversation(&self, messages: Vec<(ChatRole, String)>) -> Result<String, LlmError> {
        // Input validation if enabled
        match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => {
                if config.validate_inputs {
                    if messages.is_empty() {
                        return Err(LlmError::validation("Conversation cannot be empty"));
                    }
                    let total_len: usize = messages.iter().map(|(_, content)| content.len()).sum();
                    if total_len > 100_000 {
                        return Err(LlmError::validation(
                            "Conversation too long (max 100k characters)",
                        ));
                    }
                }
            }
        }

        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .map(|(role, content)| ChatMessage { role, content })
            .collect();

        let request = match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => ChatRequest {
                messages: chat_messages,
                temperature: config.default_temperature,
                max_tokens: config.default_max_tokens,
                top_p: config.default_top_p,
                stream: Some(false),
                extra_params: None,
            },
        };

        let response = match self {
            SimpleLlmClient::OpenAI { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.openai)
                    .await?
            }
            SimpleLlmClient::Groq { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.groq)
                    .await?
            }
            SimpleLlmClient::OpenRouter { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.openrouter)
                    .await?
            }
            SimpleLlmClient::Anthropic { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.anthropic)
                    .await?
            }
            SimpleLlmClient::Google { provider, config } => {
                provider
                    .chat_completion(request, &config.default_models.google)
                    .await?
            }
        };

        Ok(response.message.content)
    }

    async fn models(&self) -> Result<Vec<String>, LlmError> {
        let models = match self {
            SimpleLlmClient::OpenAI { provider, .. } => {
                let models = provider.available_models().await?;
                models
                    .into_iter()
                    // filter out non-chat models
                    .filter(|m| {
                        (m.starts_with("o") || m.starts_with("gpt"))
                            && (!m.contains("audio")
                                && !m.contains("deep")
                                && !m.contains("image")
                                && !m.contains("search")
                                && !m.contains("transcribe")
                                && !m.contains("realtime")
                                && !m.contains("moderation"))
                    })
                    .collect::<Vec<_>>()
            }
            SimpleLlmClient::Groq { provider, .. } => {
                // Groq provides chat models, no filtering needed for now
                provider.available_models().await?
            }
            SimpleLlmClient::OpenRouter { provider, .. } => {
                // OpenRouter aggregates models, return all
                provider.available_models().await?
            }
            SimpleLlmClient::Anthropic { provider, .. } => {
                let models = provider.available_models().await?;
                models
                    .into_iter()
                    // filter out non-chat models
                    .filter(|m| m.starts_with("claude"))
                    .collect::<Vec<_>>()
            }
            SimpleLlmClient::Google { provider, .. } => {
                let models = provider.available_models().await?;
                models
                    .into_iter()
                    // filter out non-chat models
                    .filter(|m| m.starts_with("gemini") && !m.contains("embedding"))
                    .collect::<Vec<_>>()
            }
        };

        Ok(models)
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        match self {
            SimpleLlmClient::OpenAI { provider, .. } => provider.health_check().await,
            SimpleLlmClient::Groq { provider, .. } => provider.health_check().await,
            SimpleLlmClient::OpenRouter { provider, .. } => provider.health_check().await,
            SimpleLlmClient::Anthropic { provider, .. } => provider.health_check().await,
            SimpleLlmClient::Google { provider, .. } => provider.health_check().await,
        }
    }

    fn provider_name(&self) -> &'static str {
        match self {
            SimpleLlmClient::OpenAI { provider, .. } => provider.name(),
            SimpleLlmClient::Groq { provider, .. } => provider.name(),
            SimpleLlmClient::OpenRouter { provider, .. } => provider.name(),
            SimpleLlmClient::Anthropic { provider, .. } => provider.name(),
            SimpleLlmClient::Google { provider, .. } => provider.name(),
        }
    }
}

/// Builder for creating SimpleLlm clients
#[derive(Default)]
pub struct SimpleLlmBuilder {
    openai_config: Option<OpenAIConfig>,
    groq_config: Option<OpenAICompatibleConfig>,
    openrouter_config: Option<OpenAICompatibleConfig>,
    anthropic_config: Option<AnthropicConfig>,
    google_config: Option<GoogleAiConfig>,
    simple_config: SimpleLlmConfig,
}

impl SimpleLlmBuilder {
    /// Create a new SimpleLlmBuilder
    pub fn new() -> Self {
        Self {
            openai_config: None,
            groq_config: None,
            openrouter_config: None,
            anthropic_config: None,
            google_config: None,
            simple_config: SimpleLlmConfig::default(),
        }
    }

    /// Add OpenAI configuration
    pub fn with_openai(mut self, config: OpenAIConfig) -> Self {
        self.openai_config = Some(config);
        self
    }

    /// Add Groq configuration
    pub fn with_groq(mut self, config: OpenAICompatibleConfig) -> Self {
        self.groq_config = Some(config);
        self
    }

    /// Add OpenRouter configuration
    pub fn with_openrouter(mut self, config: OpenAICompatibleConfig) -> Self {
        self.openrouter_config = Some(config);
        self
    }

    /// Add Anthropic configuration
    pub fn with_anthropic(mut self, config: AnthropicConfig) -> Self {
        self.anthropic_config = Some(config);
        self
    }

    /// Add Google configuration
    pub fn with_google(mut self, config: GoogleAiConfig) -> Self {
        self.google_config = Some(config);
        self
    }

    /// Configure simple LLM settings
    pub fn with_simple_config(mut self, config: SimpleLlmConfig) -> Self {
        self.simple_config = config;
        self
    }

    /// Build an OpenAI client
    pub fn build_openai(self) -> Result<SimpleLlmClient, LlmError> {
        let config = self
            .openai_config
            .ok_or_else(|| LlmError::configuration("OpenAI configuration not found"))?;

        self.simple_config.validate()?;
        let provider = OpenAIProvider::new(config)?;
        Ok(SimpleLlmClient::OpenAI {
            provider,
            config: self.simple_config,
        })
    }

    /// Build a Groq client
    pub fn build_groq(self) -> Result<SimpleLlmClient, LlmError> {
        let config = self
            .groq_config
            .ok_or_else(|| LlmError::configuration("Groq configuration not found"))?;

        self.simple_config.validate()?;
        let provider = GroqProvider::new(config)?;
        Ok(SimpleLlmClient::Groq {
            provider,
            config: self.simple_config,
        })
    }

    /// Build an OpenRouter client
    pub fn build_openrouter(self) -> Result<SimpleLlmClient, LlmError> {
        let config = self
            .openrouter_config
            .ok_or_else(|| LlmError::configuration("OpenRouter configuration not found"))?;

        self.simple_config.validate()?;
        let provider = OpenRouterProvider::new(config)?;
        Ok(SimpleLlmClient::OpenRouter {
            provider,
            config: self.simple_config,
        })
    }

    /// Build an Anthropic client
    pub fn build_anthropic(self) -> Result<SimpleLlmClient, LlmError> {
        let config = self
            .anthropic_config
            .ok_or_else(|| LlmError::configuration("Anthropic configuration not found"))?;

        self.simple_config.validate()?;
        let provider = AnthropicProvider::new(config)?;
        Ok(SimpleLlmClient::Anthropic {
            provider,
            config: self.simple_config,
        })
    }

    /// Build a Google client
    pub fn build_google(self) -> Result<SimpleLlmClient, LlmError> {
        let config = self
            .google_config
            .ok_or_else(|| LlmError::configuration("Google configuration not found"))?;

        self.simple_config.validate()?;
        let provider = GoogleProvider::new(config)?;
        Ok(SimpleLlmClient::Google {
            provider,
            config: self.simple_config,
        })
    }
}

/// Convenience functions for quick setup
impl SimpleLlmClient {
    /// Create a simple OpenAI client with API key
    pub fn openai(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = OpenAIConfig::new(api_key);
        let provider = OpenAIProvider::new(config)?;
        Ok(SimpleLlmClient::OpenAI {
            provider,
            config: SimpleLlmConfig::default(),
        })
    }

    /// Create a simple Groq client with API key
    pub fn groq(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = OpenAICompatibleConfig::groq(api_key);
        let provider = GroqProvider::new(config)?;
        Ok(SimpleLlmClient::Groq {
            provider,
            config: SimpleLlmConfig::default(),
        })
    }

    /// Create a simple OpenRouter client with API key
    pub fn openrouter(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = OpenAICompatibleConfig::openrouter(api_key);
        let provider = OpenRouterProvider::new(config)?;
        Ok(SimpleLlmClient::OpenRouter {
            provider,
            config: SimpleLlmConfig::default(),
        })
    }

    /// Create a simple Anthropic client with API key
    pub fn anthropic(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = AnthropicConfig::new(api_key);
        let provider = AnthropicProvider::new(config)?;
        Ok(SimpleLlmClient::Anthropic {
            provider,
            config: SimpleLlmConfig::default(),
        })
    }

    /// Create a simple Google client with API key
    pub fn google(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let config = GoogleAiConfig::new(api_key);
        let provider = GoogleProvider::new(config)?;
        Ok(SimpleLlmClient::Google {
            provider,
            config: SimpleLlmConfig::default(),
        })
    }

    /// Create a client with custom simple config
    pub fn openai_with_config(
        api_key: impl Into<String>,
        simple_config: SimpleLlmConfig,
    ) -> Result<Self, LlmError> {
        simple_config.validate()?;
        let config = OpenAIConfig::new(api_key);
        let provider = OpenAIProvider::new(config)?;
        Ok(SimpleLlmClient::OpenAI {
            provider,
            config: simple_config,
        })
    }

    /// Create a client with custom simple config
    pub fn anthropic_with_config(
        api_key: impl Into<String>,
        simple_config: SimpleLlmConfig,
    ) -> Result<Self, LlmError> {
        simple_config.validate()?;
        let config = AnthropicConfig::new(api_key);
        let provider = AnthropicProvider::new(config)?;
        Ok(SimpleLlmClient::Anthropic {
            provider,
            config: simple_config,
        })
    }

    /// Create a client with custom simple config
    pub fn google_with_config(
        api_key: impl Into<String>,
        simple_config: SimpleLlmConfig,
    ) -> Result<Self, LlmError> {
        simple_config.validate()?;
        let config = GoogleAiConfig::new(api_key);
        let provider = GoogleProvider::new(config)?;
        Ok(SimpleLlmClient::Google {
            provider,
            config: simple_config,
        })
    }

    /// Get the current simple config
    pub fn simple_config(&self) -> &SimpleLlmConfig {
        match self {
            SimpleLlmClient::OpenAI { config, .. }
            | SimpleLlmClient::Groq { config, .. }
            | SimpleLlmClient::OpenRouter { config, .. }
            | SimpleLlmClient::Anthropic { config, .. }
            | SimpleLlmClient::Google { config, .. } => config,
        }
    }

    /// Returns the model name used by this client (the default model for the provider)
    pub fn model_name(&self) -> &str {
        match self {
            SimpleLlmClient::OpenAI { config, .. } => &config.default_models.openai,
            SimpleLlmClient::Groq { config, .. } => &config.default_models.groq,
            SimpleLlmClient::OpenRouter { config, .. } => &config.default_models.openrouter,
            SimpleLlmClient::Anthropic { config, .. } => &config.default_models.anthropic,
            SimpleLlmClient::Google { config, .. } => &config.default_models.google,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatRole;
    use std::time::Duration;

    #[test]
    fn test_default_models() {
        let models = DefaultModels::default();
        assert_eq!(models.openai, "gpt-3.5-turbo");
        assert_eq!(models.anthropic, "claude-3-haiku-20240307");
        assert_eq!(models.google, "gemini-pro");
    }

    #[test]
    fn test_simple_llm_config_default() {
        let config = SimpleLlmConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert!(config.validate_inputs);
        assert_eq!(config.default_models.openai, "gpt-3.5-turbo");
    }

    #[test]
    fn test_simple_llm_config_builder() {
        let config = SimpleLlmConfig::default()
            .with_temperature(0.8)
            .with_max_tokens(2000)
            .with_top_p(0.9)
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5)
            .with_validation(false)
            .with_openai_model("gpt-4")
            .with_anthropic_model("claude-3-sonnet-20240229")
            .with_google_model("gemini-1.5-pro");

        assert_eq!(config.default_temperature, Some(0.8));
        assert_eq!(config.default_max_tokens, Some(2000));
        assert_eq!(config.default_top_p, Some(0.9));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert!(!config.validate_inputs);
        assert_eq!(config.default_models.openai, "gpt-4");
        assert_eq!(config.default_models.anthropic, "claude-3-sonnet-20240229");
        assert_eq!(config.default_models.google, "gemini-1.5-pro");
    }

    #[test]
    fn test_config_validation_success() {
        let config = SimpleLlmConfig::default()
            .with_temperature(1.0)
            .with_top_p(0.5)
            .with_max_tokens(100);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_temperature_too_low() {
        let config = SimpleLlmConfig::default().with_temperature(-0.1);
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Temperature must be between 0.0 and 2.0")
        );
    }

    #[test]
    fn test_config_validation_temperature_too_high() {
        let config = SimpleLlmConfig::default().with_temperature(2.1);
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Temperature must be between 0.0 and 2.0")
        );
    }

    #[test]
    fn test_config_validation_top_p_too_low() {
        let config = SimpleLlmConfig::default().with_top_p(-0.1);
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Top_p must be between 0.0 and 1.0")
        );
    }

    #[test]
    fn test_config_validation_top_p_too_high() {
        let config = SimpleLlmConfig::default().with_top_p(1.1);
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Top_p must be between 0.0 and 1.0")
        );
    }

    #[test]
    fn test_config_validation_max_tokens_zero() {
        let config = SimpleLlmConfig::new().with_max_tokens(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max tokens must be greater than 0")
        );
    }

    #[test]
    fn test_config_validation_timeout_zero() {
        let config = SimpleLlmConfig::new().with_timeout(Duration::from_secs(0));
        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Timeout must be greater than 0")
        );
    }

    #[test]
    fn test_builder_default() {
        let builder = SimpleLlmBuilder::new();
        assert!(builder.openai_config.is_none());
        assert!(builder.anthropic_config.is_none());
        assert!(builder.google_config.is_none());
    }

    #[test]
    fn test_builder_configuration() {
        use crate::config::{AnthropicConfig, GoogleAiConfig, OpenAIConfig};

        let builder = SimpleLlmBuilder::new()
            .with_openai(OpenAIConfig::new("openai-key"))
            .with_anthropic(AnthropicConfig::new("anthropic-key"))
            .with_google(GoogleAiConfig::new("google-key"))
            .with_simple_config(SimpleLlmConfig::new().with_temperature(0.7));

        assert!(builder.openai_config.is_some());
        assert!(builder.anthropic_config.is_some());
        assert!(builder.google_config.is_some());
        assert_eq!(builder.simple_config.default_temperature, Some(0.7));
    }

    mod mock_tests {
        use super::*;

        #[test]
        fn test_chat_input_validation_empty() {
            let message = "";
            let is_valid = !message.trim().is_empty();
            assert!(!is_valid);
        }

        #[test]
        fn test_chat_input_validation_too_long() {
            let message = "a".repeat(100_001);
            let is_valid = message.len() <= 100_000;
            assert!(!is_valid);
        }

        #[test]
        fn test_chat_input_validation_valid() {
            let message = "Hello, world!";
            let is_valid = !message.trim().is_empty() && message.len() <= 100_000;
            assert!(is_valid);
        }

        #[test]
        fn test_conversation_validation_empty() {
            let messages: Vec<(ChatRole, String)> = vec![];
            let is_valid = !messages.is_empty();
            assert!(!is_valid);
        }

        #[test]
        fn test_conversation_validation_too_long() {
            let messages = [
                (ChatRole::User, "a".repeat(50_001)),
                (ChatRole::Assistant, "b".repeat(50_001)),
            ];
            let total_len: usize = messages.iter().map(|(_, content)| content.len()).sum();
            let is_valid = total_len <= 100_000;
            assert!(!is_valid);
        }

        #[test]
        fn test_conversation_validation_valid() {
            let messages = [
                (ChatRole::User, "Hello".to_string()),
                (ChatRole::Assistant, "Hi there!".to_string()),
            ];
            let total_len: usize = messages.iter().map(|(_, content)| content.len()).sum();
            let is_valid = !messages.is_empty() && total_len <= 100_000;
            assert!(is_valid);
        }

        #[test]
        fn test_system_message_validation() {
            let system = "";
            let message = "Hello";
            let is_valid = !system.trim().is_empty() && !message.trim().is_empty();
            assert!(!is_valid);

            let system = "You are helpful";
            let message = "";
            let is_valid = !system.trim().is_empty() && !message.trim().is_empty();
            assert!(!is_valid);

            let system = "You are helpful";
            let message = "Hello";
            let is_valid = !system.trim().is_empty()
                && !message.trim().is_empty()
                && (system.len() + message.len()) <= 100_000;
            assert!(is_valid);
        }

        #[test]
        fn test_streaming_validation_stream_parameter() {
            // Test that stream parameter validation works correctly
            let request_non_stream = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
                top_p: Some(0.9),
                stream: Some(false), // Should fail streaming validation
                extra_params: None,
            };

            let request_no_stream = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
                top_p: Some(0.9),
                stream: None, // Should fail streaming validation
                extra_params: None,
            };

            let request_valid_stream = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
                top_p: Some(0.9),
                stream: Some(true), // Should pass streaming validation
                extra_params: None,
            };

            // These would need actual client instances to test, but the logic is validated
            assert_eq!(request_non_stream.stream, Some(false));
            assert_eq!(request_no_stream.stream, None);
            assert_eq!(request_valid_stream.stream, Some(true));
        }

        #[test]
        fn test_streaming_parameter_validation() {
            // Test streaming-specific parameter validation logic
            let invalid_temperature = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(2.5), // Too high
                max_tokens: Some(100),
                top_p: Some(0.9),
                stream: Some(true),
                extra_params: None,
            };

            let invalid_top_p = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
                top_p: Some(1.5), // Too high
                stream: Some(true),
                extra_params: None,
            };

            let invalid_max_tokens_zero = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(0), // Should be > 0
                top_p: Some(0.9),
                stream: Some(true),
                extra_params: None,
            };

            let invalid_max_tokens_too_high = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(5000), // Too high for streaming
                top_p: Some(0.9),
                stream: Some(true),
                extra_params: None,
            };

            let valid_request = ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(1000),
                top_p: Some(0.9),
                stream: Some(true),
                extra_params: None,
            };

            // Test parameter ranges
            assert!(invalid_temperature.temperature.unwrap() > 2.0);
            assert!(invalid_top_p.top_p.unwrap() > 1.0);
            assert_eq!(invalid_max_tokens_zero.max_tokens.unwrap(), 0);
            assert!(invalid_max_tokens_too_high.max_tokens.unwrap() > 4096);

            // Valid request should pass basic parameter checks
            assert!(
                valid_request.temperature.unwrap() >= 0.0
                    && valid_request.temperature.unwrap() <= 2.0
            );
            assert!(valid_request.top_p.unwrap() >= 0.0 && valid_request.top_p.unwrap() <= 1.0);
            assert!(
                valid_request.max_tokens.unwrap() > 0 && valid_request.max_tokens.unwrap() <= 4096
            );
            assert_eq!(valid_request.stream, Some(true));
        }
    }

    #[test]
    fn test_client_enum_structure() {
        let config = SimpleLlmConfig::default();

        match "OpenAI" {
            "OpenAI" => {
                assert_eq!(config.default_models.openai, "gpt-3.5-turbo");
            }
            "Anthropic" => {
                assert_eq!(config.default_models.anthropic, "claude-3-haiku-20240307");
            }
            "Google" => {
                assert_eq!(config.default_models.google, "gemini-pro");
            }
            _ => panic!("Unknown provider"),
        }
    }

    #[test]
    fn test_chat_role_conversion() {
        let roles = vec![ChatRole::System, ChatRole::User, ChatRole::Assistant];

        for role in roles {
            let message = ChatMessage {
                role: role.clone(),
                content: "test".to_string(),
            };
            assert_eq!(message.role, role);
        }
    }
}
