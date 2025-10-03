use crate::error::LlmError;
use crate::providers::openai_compatible::{identities, OpenAICompatibleProvider};
use crate::types::{
    ChatCompletion, ChatRequest, ChatResponse, ChatStreamEvent, LlmProvider, StreamConfig,
    StreamResult,
};

/// Groq provider implementation (wrapper around OpenAICompatibleProvider)
#[derive(Clone)]
pub struct GroqProvider {
    inner: OpenAICompatibleProvider,
}

impl GroqProvider {
    pub fn new(config: crate::config::OpenAICompatibleConfig) -> Result<Self, LlmError> {
        let inner = OpenAICompatibleProvider::new(config, identities::GROQ)?;
        Ok(Self { inner })
    }
}

#[async_trait::async_trait]
impl LlmProvider for GroqProvider {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.inner.aliases()
    }

    fn env_key(&self) -> &'static str {
        self.inner.env_key()
    }

    fn default_base_url(&self) -> Option<&'static str> {
        self.inner.default_base_url()
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.available_models().await
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        self.inner.health_check().await
    }
}

#[async_trait::async_trait]
impl ChatCompletion for GroqProvider {
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        self.inner.chat_completion(request, model).await
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        model: &str,
        config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        self.inner.chat_completion_stream(request, model, config).await
    }

    async fn estimate_tokens(&self, text: &str, model: &str) -> Result<u32, LlmError> {
        self.inner.estimate_tokens(text, model).await
    }
}
