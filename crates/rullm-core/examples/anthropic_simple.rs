use rullm_core::config::ConfigBuilder;
use rullm_core::{AnthropicProvider, ChatCompletion, ChatRequestBuilder, LlmProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Basic Configuration using ConfigBuilder
    let config = ConfigBuilder::anthropic_from_env()?;

    let provider = AnthropicProvider::new(config)?;

    // 2. Simple Chat Completion
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant.")
        .user("What is 2 + 2?")
        .temperature(0.7)
        .build();

    let response = provider
        .chat_completion(request, "claude-3-haiku-20240307")
        .await?;

    println!("🤖 Claude: {}", response.message.content);
    println!("📊 Tokens used: {}", response.usage.total_tokens);

    // 3. Multi-message conversation
    let conversation_request = ChatRequestBuilder::new()
        .system("You are a helpful math tutor.")
        .user("What is 5 * 7?")
        .assistant("5 * 7 = 35")
        .user("What about 6 * 8?")
        .max_tokens(100)
        .build();

    let conversation_response = provider
        .chat_completion(conversation_request, "claude-3-sonnet-20240229")
        .await?;

    println!("\n💬 Conversation:");
    println!("Claude: {}", conversation_response.message.content);

    // 4. Different Claude models comparison
    let models = [
        "claude-3-haiku-20240307",
        "claude-3-sonnet-20240229",
        "claude-3-opus-20240229",
    ];
    let question = "Explain async/await in one sentence.";

    for model in &models {
        let request = ChatRequestBuilder::new()
            .user(question)
            .temperature(0.5)
            .max_tokens(50)
            .build();

        match provider.chat_completion(request, model).await {
            Ok(response) => {
                println!("\n🔬 {model} says:");
                println!("{}", response.message.content);
            }
            Err(e) => {
                println!("❌ Error with {model}: {e}");
                // Note: Some models might not be available depending on your API access
            }
        }
    }

    // 5. Advanced parameters with Anthropic-specific features
    let creative_request = ChatRequestBuilder::new()
        .system("You are a creative writer.")
        .user("Write a haiku about programming.")
        .temperature(1.0) // Higher creativity
        .top_p(0.9) // Nucleus sampling
        // .stop_sequences(vec!["END".to_string(), "STOP".to_string()]) // Stop sequences
        .build();

    let creative_response = provider
        .chat_completion(creative_request, "claude-3-5-sonnet-20241022")
        .await?;

    println!("\n🎨 Creative Response:");
    println!("{}", creative_response.message.content);
    println!("Model: {}", creative_response.model);
    if let Some(reason) = creative_response.finish_reason {
        println!("Finish reason: {reason}");
    }

    // 6. Token estimation
    let text = "This is a sample text for token estimation.";
    let estimated_tokens = provider
        .estimate_tokens(text, "claude-3-haiku-20240307")
        .await?;
    println!("\n📏 Estimated tokens for '{text}': {estimated_tokens}");

    // 7. Health check
    match provider.health_check().await {
        Ok(_) => println!("\n✅ Anthropic API is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
