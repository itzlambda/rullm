//! Example demonstrating the simple API wrapper
//!
//! This example shows how to use the SimpleLlm trait for easy interactions
//! with different LLM providers without dealing with Tower complexity.

use rullm_core::simple::{SimpleLlm, SimpleLlmBuilder, SimpleLlmClient};
use rullm_core::{AnthropicConfig, ChatRole, LlmError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Simple LLM API Wrapper Demo ===\n");

    // Method 1: Quick setup with just API key
    demo_quick_setup().await?;

    // Method 2: Builder pattern with custom configurations
    demo_builder_pattern().await?;

    // Method 3: Working with conversations
    demo_conversations().await?;

    Ok(())
}

async fn demo_quick_setup() -> Result<(), LlmError> {
    println!("🚀 Demo 1: Quick Setup");

    // These would normally use real API keys from environment
    // For demo purposes, we'll show the pattern

    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        println!("Creating OpenAI client...");
        let client = SimpleLlmClient::openai(api_key)?;

        println!("Provider: {}", client.provider_name());

        // Check health
        match client.health_check().await {
            Ok(()) => println!("✅ OpenAI health check passed"),
            Err(e) => println!("❌ OpenAI health check failed: {e}"),
        }

        // List models
        match client.models().await {
            Ok(models) => {
                println!("Available OpenAI models: {}", models.len());
                for (i, model) in models.iter().take(3).enumerate() {
                    println!("  {}. {}", i + 1, model);
                }
                if models.len() > 3 {
                    println!("  ... and {} more", models.len() - 3);
                }
            }
            Err(e) => println!("❌ Failed to get models: {e}"),
        }

        // Simple chat
        match client
            .chat("Hello! Can you respond with just 'Hi there!' please?")
            .await
        {
            Ok(response) => println!("💬 OpenAI response: {response}"),
            Err(e) => println!("❌ Chat failed: {e}"),
        }
    } else {
        println!("⚠️  OPENAI_API_KEY not set, skipping OpenAI demo");
    }

    println!();
    Ok(())
}

async fn demo_builder_pattern() -> Result<(), LlmError> {
    println!("🔧 Demo 2: Builder Pattern");

    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        // Create a custom configuration
        let config = AnthropicConfig::new(api_key).with_base_url("https://api.anthropic.com");

        let client = SimpleLlmBuilder::new()
            .with_anthropic(config)
            .build_anthropic()?;

        println!("Provider: {}", client.provider_name());

        // Simple chat with system prompt
        match client
            .chat_with_system(
                "You are a helpful assistant that responds concisely.",
                "What is 2+2?",
            )
            .await
        {
            Ok(response) => println!("💬 Anthropic response: {response}"),
            Err(e) => println!("❌ Chat failed: {e}"),
        }
    } else {
        println!("⚠️  ANTHROPIC_API_KEY not set, skipping Anthropic demo");
    }

    println!();
    Ok(())
}

async fn demo_conversations() -> Result<(), LlmError> {
    println!("💭 Demo 3: Conversations");

    if let Ok(api_key) = std::env::var("GOOGLE_AI_API_KEY") {
        let client = SimpleLlmClient::google(api_key)?;

        // Multi-turn conversation

        let conversation = vec![
            (
                ChatRole::System,
                "You are a concise math tutor.".to_string(),
            ),
            (ChatRole::User, "What's 5 + 3?".to_string()),
            (ChatRole::Assistant, "5 + 3 = 8".to_string()),
            (ChatRole::User, "And what's 8 × 2?".to_string()),
        ];

        match client.conversation(conversation).await {
            Ok(response) => println!("💬 Google response: {response}"),
            Err(e) => println!("❌ Conversation failed: {e}"),
        }
    } else {
        println!("⚠️  GOOGLE_AI_API_KEY not set, skipping Google demo");
    }

    println!();
    Ok(())
}
