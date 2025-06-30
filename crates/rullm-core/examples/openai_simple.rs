use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, LlmProvider, OpenAIProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Basic Configuration using ConfigBuilder
    let config = ConfigBuilder::openai_from_env()?;

    let provider = OpenAIProvider::new(config)?;

    // 2. Simple Chat Completion
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant.")
        .user("What is 2 + 2?")
        .temperature(0.7)
        .build();

    let response = provider.chat_completion(request, "gpt-3.5-turbo").await?;

    println!("🤖 Assistant: {}", response.message.content);
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
        .chat_completion(conversation_request, "gpt-4o-mini")
        .await?;

    println!("\n💬 Conversation:");
    println!("Assistant: {}", conversation_response.message.content);

    // 4. Different models comparison
    let models = ["gpt-3.5-turbo", "gpt-4o-mini"];
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
            }
        }
    }

    // 5. Advanced parameters
    let creative_request = ChatRequestBuilder::new()
        .system("You are a creative writer.")
        .user("Write a haiku about programming.")
        .temperature(1.0) // Higher creativity
        .top_p(0.9) // Nucleus sampling
        // .frequency_penalty(0.2) // Reduce repetition
        // .presence_penalty(0.2) // Encourage diverse topics
        // .stop_sequences(vec!["END".to_string(), "STOP".to_string()]) // Stop sequences
        .build();

    let creative_response = provider.chat_completion(creative_request, "gpt-4").await?;

    println!("\n🎨 Creative Response:");
    println!("{}", creative_response.message.content);
    println!("Model: {}", creative_response.model);
    if let Some(reason) = creative_response.finish_reason {
        println!("Finish reason: {reason}");
    }

    // 6. Token estimation
    let text = "This is a sample text for token estimation.";
    let estimated_tokens = provider.estimate_tokens(text, "gpt-3.5-turbo").await?;
    println!("\n📏 Estimated tokens for '{text}': {estimated_tokens}");

    // 7. Health check
    match provider.health_check().await {
        Ok(_) => println!("\n✅ OpenAI API is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
