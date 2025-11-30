//! CLI Client wrapper for LLM providers
//!
//! This module provides a simple enum wrapper for CLI usage that supports
//! basic chat operations without exposing the full complexity of each provider's API.

use futures::StreamExt;
use rullm_core::config::{AnthropicConfig, GoogleAiConfig, OpenAICompatibleConfig, OpenAIConfig};
use rullm_core::error::LlmError;
use rullm_core::providers::openai_compatible::{OpenAICompatibleProvider, identities};
use rullm_core::providers::{AnthropicClient, GoogleClient, OpenAIClient};
use std::pin::Pin;

/// Simple configuration for CLI adapter
#[derive(Debug, Clone, Default)]
pub struct CliConfig {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// CLI adapter enum that wraps concrete provider clients
pub enum CliClient {
    OpenAI {
        client: OpenAIClient,
        model: String,
        config: CliConfig,
    },
    Anthropic {
        client: AnthropicClient,
        model: String,
        config: CliConfig,
    },
    Google {
        client: GoogleClient,
        model: String,
        config: CliConfig,
    },
    Groq {
        client: OpenAICompatibleProvider,
        model: String,
        config: CliConfig,
    },
    OpenRouter {
        client: OpenAICompatibleProvider,
        model: String,
        config: CliConfig,
    },
}

impl CliClient {
    /// Create OpenAI client
    pub fn openai(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, LlmError> {
        let client_config = OpenAIConfig::new(api_key);
        let client = OpenAIClient::new(client_config)?;
        Ok(Self::OpenAI {
            client,
            model: model.into(),
            config,
        })
    }

    /// Create Anthropic client
    pub fn anthropic(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, LlmError> {
        let client_config = AnthropicConfig::new(api_key);
        let client = AnthropicClient::new(client_config)?;
        Ok(Self::Anthropic {
            client,
            model: model.into(),
            config,
        })
    }

    /// Create Google client
    pub fn google(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, LlmError> {
        let client_config = GoogleAiConfig::new(api_key);
        let client = GoogleClient::new(client_config)?;
        Ok(Self::Google {
            client,
            model: model.into(),
            config,
        })
    }

    /// Create Groq client
    pub fn groq(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, LlmError> {
        let client_config = OpenAICompatibleConfig::groq(api_key);
        let client = OpenAICompatibleProvider::new(client_config, identities::GROQ)?;
        Ok(Self::Groq {
            client,
            model: model.into(),
            config,
        })
    }

    /// Create OpenRouter client
    pub fn openrouter(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, LlmError> {
        let client_config = OpenAICompatibleConfig::openrouter(api_key);
        let client = OpenAICompatibleProvider::new(client_config, identities::OPENROUTER)?;
        Ok(Self::OpenRouter {
            client,
            model: model.into(),
            config,
        })
    }

    /// Simple chat - send a message and get a response
    pub async fn chat(&self, message: &str) -> Result<String, LlmError> {
        match self {
            Self::OpenAI {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::openai::{ChatCompletionRequest, ChatMessage};

                let mut request =
                    ChatCompletionRequest::new(model, vec![ChatMessage::user(message)]);

                if let Some(temp) = config.temperature {
                    request.temperature = Some(temp);
                }
                if let Some(max) = config.max_tokens {
                    request.max_tokens = Some(max);
                }

                let response = client.chat_completion(request).await?;
                let content = response
                    .choices
                    .first()
                    .and_then(|c| c.message.content.as_ref())
                    .and_then(|c| match c {
                        rullm_core::providers::openai::MessageContent::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| LlmError::model("No content in response"))?;

                Ok(content)
            }
            Self::Anthropic {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::anthropic::{Message, MessagesRequest};

                let max_tokens = config.max_tokens.unwrap_or(1024);
                let mut request =
                    MessagesRequest::new(model, vec![Message::user(message)], max_tokens);

                if let Some(temp) = config.temperature {
                    request.temperature = Some(temp);
                }

                let response = client.messages(request).await?;
                let content = response
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        rullm_core::providers::anthropic::ContentBlock::Text { text } => {
                            Some(text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                Ok(content)
            }
            Self::Google {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::google::{
                    Content, GenerateContentRequest, GenerationConfig,
                };

                let mut request = GenerateContentRequest::new(vec![Content::user(message)]);

                if config.temperature.is_some() || config.max_tokens.is_some() {
                    let gen_config = GenerationConfig {
                        temperature: config.temperature,
                        max_output_tokens: config.max_tokens,
                        stop_sequences: None,
                        top_p: None,
                        top_k: None,
                        response_mime_type: None,
                        response_schema: None,
                    };
                    request.generation_config = Some(gen_config);
                }

                let response = client.generate_content(model, request).await?;
                let content = response
                    .candidates
                    .first()
                    .map(|c| {
                        c.content
                            .parts
                            .iter()
                            .filter_map(|part| match part {
                                rullm_core::providers::google::Part::Text { text } => {
                                    Some(text.clone())
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .ok_or_else(|| LlmError::model("No content in response"))?;

                Ok(content)
            }
            Self::Groq {
                client,
                model,
                config,
            }
            | Self::OpenRouter {
                client,
                model,
                config,
            } => {
                use rullm_core::{ChatRequestBuilder, ChatRole};

                let mut request = ChatRequestBuilder::new().add_message(ChatRole::User, message);

                if let Some(temp) = config.temperature {
                    request = request.temperature(temp);
                }
                if let Some(max) = config.max_tokens {
                    request = request.max_tokens(max);
                }

                let response = client.chat_completion(request.build(), model).await?;
                Ok(response.message.content)
            }
        }
    }

    /// Stream chat - for interactive chat mode
    pub async fn stream_chat_raw(
        &self,
        messages: Vec<(String, String)>, // (role, content) pairs
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<String, LlmError>> + Send>>, LlmError>
    {
        match self {
            Self::OpenAI {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::openai::{ChatCompletionRequest, ChatMessage, Role};

                let msgs: Vec<ChatMessage> = messages
                    .iter()
                    .map(|(role, content)| {
                        let r = match role.as_str() {
                            "system" => Role::System,
                            "user" => Role::User,
                            "assistant" => Role::Assistant,
                            _ => Role::User,
                        };
                        ChatMessage {
                            role: r,
                            content: Some(rullm_core::providers::openai::MessageContent::Text(
                                content.clone(),
                            )),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        }
                    })
                    .collect();

                let mut request = ChatCompletionRequest::new(model, msgs);
                if let Some(temp) = config.temperature {
                    request.temperature = Some(temp);
                }
                if let Some(max) = config.max_tokens {
                    request.max_tokens = Some(max);
                }

                let stream = client.chat_completion_stream(request).await?;
                Ok(Box::pin(stream.filter_map(|chunk_result| async move {
                    match chunk_result {
                        Ok(chunk) => chunk
                            .choices
                            .first()
                            .and_then(|choice| choice.delta.content.clone().map(Ok)),
                        Err(e) => Some(Err(e)),
                    }
                })))
            }
            Self::Anthropic {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::anthropic::{Message, MessagesRequest};

                let msgs: Vec<Message> = messages
                    .iter()
                    .filter_map(|(role, content)| {
                        match role.as_str() {
                            "user" => Some(Message::user(content)),
                            "assistant" => Some(Message::assistant(content)),
                            _ => None, // Skip system messages for now
                        }
                    })
                    .collect();

                let max_tokens = config.max_tokens.unwrap_or(1024);
                let mut request = MessagesRequest::new(model, msgs, max_tokens);
                if let Some(temp) = config.temperature {
                    request.temperature = Some(temp);
                }

                let stream = client.messages_stream(request).await?;
                Ok(Box::pin(stream.filter_map(|event_result| async move {
                    match event_result {
                        Ok(rullm_core::providers::anthropic::StreamEvent::ContentBlockDelta {
                            delta: rullm_core::providers::anthropic::Delta::TextDelta { text },
                            ..
                        }) => Some(Ok(text)),
                        Ok(_) => None,
                        Err(e) => Some(Err(e)),
                    }
                })))
            }
            Self::Google {
                client,
                model,
                config,
            } => {
                use rullm_core::providers::google::{
                    Content, GenerateContentRequest, GenerationConfig,
                };

                let contents: Vec<Content> = messages
                    .iter()
                    .map(|(role, content)| match role.as_str() {
                        "user" => Content::user(content),
                        _ => Content::model(content),
                    })
                    .collect();

                let mut request = GenerateContentRequest::new(contents);
                if config.temperature.is_some() || config.max_tokens.is_some() {
                    request.generation_config = Some(GenerationConfig {
                        temperature: config.temperature,
                        max_output_tokens: config.max_tokens,
                        stop_sequences: None,
                        top_p: None,
                        top_k: None,
                        response_mime_type: None,
                        response_schema: None,
                    });
                }

                let stream = client.stream_generate_content(model, request).await?;
                Ok(Box::pin(stream.filter_map(|response_result| async move {
                    match response_result {
                        Ok(response) => response
                            .candidates
                            .first()
                            .map(|candidate| {
                                let text = candidate
                                    .content
                                    .parts
                                    .iter()
                                    .filter_map(|part| match part {
                                        rullm_core::providers::google::Part::Text { text } => {
                                            Some(text.clone())
                                        }
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("");
                                Ok(text)
                            })
                            .filter(|s| matches!(s, Ok(t) if !t.is_empty())),
                        Err(e) => Some(Err(e)),
                    }
                })))
            }
            Self::Groq {
                client,
                model,
                config,
            }
            | Self::OpenRouter {
                client,
                model,
                config,
            } => {
                use rullm_core::{ChatRequestBuilder, ChatRole, ChatStreamEvent};

                let mut builder = ChatRequestBuilder::new();
                for (role, content) in messages {
                    let r = match role.as_str() {
                        "system" => ChatRole::System,
                        "user" => ChatRole::User,
                        "assistant" => ChatRole::Assistant,
                        _ => ChatRole::User,
                    };
                    builder = builder.add_message(r, content);
                }

                if let Some(temp) = config.temperature {
                    builder = builder.temperature(temp);
                }
                if let Some(max) = config.max_tokens {
                    builder = builder.max_tokens(max);
                }

                let stream = client
                    .chat_completion_stream(builder.build(), model, None)
                    .await;
                Ok(Box::pin(stream.filter_map(|event_result| async move {
                    match event_result {
                        Ok(ChatStreamEvent::Token(token)) => Some(Ok(token)),
                        Ok(_) => None,
                        Err(e) => Some(Err(e)),
                    }
                })))
            }
        }
    }

    /// Get provider name
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::OpenAI { .. } => "openai",
            Self::Anthropic { .. } => "anthropic",
            Self::Google { .. } => "google",
            Self::Groq { .. } => "groq",
            Self::OpenRouter { .. } => "openrouter",
        }
    }

    /// Get model name
    pub fn model_name(&self) -> &str {
        match self {
            Self::OpenAI { model, .. }
            | Self::Anthropic { model, .. }
            | Self::Google { model, .. }
            | Self::Groq { model, .. }
            | Self::OpenRouter { model, .. } => model,
        }
    }
}
