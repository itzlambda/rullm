//! CLI Client wrapper for LLM providers
//!
//! This module provides a simple enum wrapper for CLI usage that supports
//! basic chat operations without exposing the full complexity of each provider's API.

use crate::error::CliError;
use futures::StreamExt;
use rullm_anthropic::{
    Client as AnthropicClient, Message as AnthropicMessage, MessagesRequest, RequestOptions,
    SystemBlock, SystemContent,
};
use rullm_chat_completion::{ChatCompletionsClient, ClientConfig, Message as ChatMessage};
use std::pin::Pin;

/// Claude Code identification text for OAuth requests
const CLAUDE_CODE_SPOOF_TEXT: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// Prepend Claude Code system block to an existing system prompt (for OAuth requests)
fn prepend_claude_code_system(existing: Option<SystemContent>) -> SystemContent {
    let spoof_block = SystemBlock::text_with_cache(CLAUDE_CODE_SPOOF_TEXT);

    match existing {
        None => SystemContent::Blocks(vec![spoof_block]),
        Some(SystemContent::Text(text)) => {
            SystemContent::Blocks(vec![spoof_block, SystemBlock::text(text)])
        }
        Some(SystemContent::Blocks(mut blocks)) => {
            blocks.insert(0, spoof_block);
            SystemContent::Blocks(blocks)
        }
    }
}

/// Simple configuration for CLI adapter
#[derive(Debug, Clone, Default)]
pub struct CliConfig {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// CLI adapter enum that wraps concrete provider clients
pub enum CliClient {
    OpenAI {
        client: ChatCompletionsClient,
        model: String,
        config: CliConfig,
    },
    Anthropic {
        client: AnthropicClient,
        model: String,
        config: CliConfig,
        is_oauth: bool,
    },
    Groq {
        client: ChatCompletionsClient,
        model: String,
        config: CliConfig,
    },
    OpenRouter {
        client: ChatCompletionsClient,
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
    ) -> Result<Self, CliError> {
        let client_config = ClientConfig::builder()
            .bearer_token(api_key.into())
            .build()
            .map_err(|e| CliError::Other(e.to_string()))?;
        let client = ChatCompletionsClient::new(client_config)?;
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
        use_oauth: bool,
    ) -> Result<Self, CliError> {
        let api_key_str = api_key.into();
        let client_config = if use_oauth {
            AnthropicClient::builder()
                .auth_token(api_key_str)
                .betas([
                    "oauth-2025-04-20",
                    "claude-code-20250219",
                    "interleaved-thinking-2025-05-14",
                    "fine-grained-tool-streaming-2025-05-14",
                ])
                .build()?
        } else {
            AnthropicClient::builder().api_key(api_key_str).build()?
        };
        let anthropic_client = AnthropicClient::new(client_config)?;
        Ok(Self::Anthropic {
            client: anthropic_client,
            model: model.into(),
            config,
            is_oauth: use_oauth,
        })
    }

    /// Create Groq client
    pub fn groq(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: CliConfig,
    ) -> Result<Self, CliError> {
        let client_config = ClientConfig::builder()
            .base_url("https://api.groq.com/openai/v1")
            .bearer_token(api_key.into())
            .build()
            .map_err(|e| CliError::Other(e.to_string()))?;
        let client = ChatCompletionsClient::new(client_config)?;
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
    ) -> Result<Self, CliError> {
        let client_config = ClientConfig::builder()
            .base_url("https://openrouter.ai/api/v1")
            .bearer_token(api_key.into())
            .build()
            .map_err(|e| CliError::Other(e.to_string()))?;
        let client = ChatCompletionsClient::new(client_config)?;
        Ok(Self::OpenRouter {
            client,
            model: model.into(),
            config,
        })
    }

    /// Simple chat - send a message and get a response
    pub async fn chat(&self, message: &str) -> Result<String, CliError> {
        match self {
            Self::OpenAI {
                client,
                model,
                config,
            }
            | Self::Groq {
                client,
                model,
                config,
            }
            | Self::OpenRouter {
                client,
                model,
                config,
            } => {
                let mut builder = client.chat().model(model.as_str()).user(message);

                if let Some(temp) = config.temperature {
                    builder = builder.temperature(temp);
                }
                if let Some(max) = config.max_tokens {
                    builder = builder.max_completion_tokens(max);
                }

                let response = builder.send().await?;
                response
                    .data
                    .first_text()
                    .map(|s| s.to_string())
                    .ok_or_else(|| CliError::Other("No content in response".to_string()))
            }
            Self::Anthropic {
                client,
                model,
                config,
                is_oauth,
            } => {
                let max_tokens = config.max_tokens.unwrap_or(1024);
                let mut builder = MessagesRequest::builder(model.as_str(), max_tokens)
                    .message(AnthropicMessage::user(message));

                if let Some(temp) = config.temperature {
                    builder = builder.temperature(temp);
                }

                if *is_oauth {
                    builder = builder.system_blocks(prepend_claude_code_system(None).into_blocks());
                }

                let request = builder.build();
                let response = client
                    .messages()
                    .create(request, RequestOptions::default())
                    .await?;
                Ok(response.text())
            }
        }
    }

    /// Stream chat - for interactive chat mode
    pub async fn stream_chat_raw(
        &self,
        messages: Vec<(String, String)>, // (role, content) pairs
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<String, CliError>> + Send>>, CliError>
    {
        match self {
            Self::OpenAI {
                client,
                model,
                config,
            }
            | Self::Groq {
                client,
                model,
                config,
            }
            | Self::OpenRouter {
                client,
                model,
                config,
            } => {
                let mut builder = client.chat().model(model.as_str());

                for (role, content) in &messages {
                    let msg = match role.as_str() {
                        "system" => ChatMessage::system(content.as_str()),
                        "user" => ChatMessage::user(content.as_str()),
                        "assistant" => ChatMessage::assistant(content.as_str()),
                        _ => ChatMessage::user(content.as_str()),
                    };
                    builder = builder.message(msg);
                }

                if let Some(temp) = config.temperature {
                    builder = builder.temperature(temp);
                }
                if let Some(max) = config.max_tokens {
                    builder = builder.max_completion_tokens(max);
                }

                let stream = builder.stream().await?;
                Ok(Box::pin(stream.filter_map(|chunk_result| async move {
                    match chunk_result {
                        Ok(chunk) => chunk
                            .choices
                            .first()
                            .and_then(|choice| choice.delta.content.as_ref())
                            .map(|content| Ok(content.to_string())),
                        Err(e) => Some(Err(CliError::ChatCompletion(e))),
                    }
                })))
            }
            Self::Anthropic {
                client,
                model,
                config,
                is_oauth,
            } => {
                let msgs: Vec<AnthropicMessage> = messages
                    .iter()
                    .filter_map(|(role, content)| match role.as_str() {
                        "user" => Some(AnthropicMessage::user(content.as_str())),
                        "assistant" => Some(AnthropicMessage::assistant(content.as_str())),
                        _ => None, // Skip system messages for now
                    })
                    .collect();

                let max_tokens = config.max_tokens.unwrap_or(1024);
                let mut builder =
                    MessagesRequest::builder(model.as_str(), max_tokens).messages(msgs.into_iter());

                if let Some(temp) = config.temperature {
                    builder = builder.temperature(temp);
                }

                if *is_oauth {
                    builder = builder.system_blocks(prepend_claude_code_system(None).into_blocks());
                }

                let request = builder.build();
                let messages_client = client.messages();
                let stream = messages_client
                    .stream(request, RequestOptions::default())
                    .await?;

                // Use text_stream() which provides a cleaner interface for text-only streaming
                let text_stream = stream.text_stream();

                Ok(Box::pin(async_stream::stream! {
                    use std::pin::pin;
                    let mut stream = pin!(text_stream);
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(text) => yield Ok(text.to_string()),
                            Err(e) => yield Err(CliError::Anthropic(e)),
                        }
                    }
                }))
            }
        }
    }

    /// Get provider name
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::OpenAI { .. } => "openai",
            Self::Anthropic { .. } => "anthropic",
            Self::Groq { .. } => "groq",
            Self::OpenRouter { .. } => "openrouter",
        }
    }

    /// Get model name
    pub fn model_name(&self) -> &str {
        match self {
            Self::OpenAI { model, .. }
            | Self::Anthropic { model, .. }
            | Self::Groq { model, .. }
            | Self::OpenRouter { model, .. } => model,
        }
    }
}

// Helper trait for SystemContent
trait SystemContentExt {
    fn into_blocks(self) -> Vec<SystemBlock>;
}

impl SystemContentExt for SystemContent {
    fn into_blocks(self) -> Vec<SystemBlock> {
        match self {
            SystemContent::Text(text) => vec![SystemBlock::text(text)],
            SystemContent::Blocks(blocks) => blocks,
        }
    }
}
