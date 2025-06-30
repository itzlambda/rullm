use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, GoogleProvider, LlmProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Basic Configuration using ConfigBuilder
    let config = ConfigBuilder::google_ai_from_env()?;

    let provider = GoogleProvider::new(config)?;

    // 2. Simple Chat Completion
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant.")
        .user("What is 2 + 2?")
        .temperature(0.7)
        .build();

    let response = provider
        .chat_completion(request, "gemini-1.5-flash")
        .await?;

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
        .chat_completion(conversation_request, "gemini-1.5-pro")
        .await?;

    println!("\n💬 Conversation:");
    println!("Assistant: {}", conversation_response.message.content);

    // 4. Different models comparison
    let models = ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash-exp"];
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

    // 5. Advanced parameters with Google-specific features
    let creative_request = ChatRequestBuilder::new()
        .system("You are a creative writer.")
        .user("Write a haiku about programming.")
        .temperature(1.0)
        .top_p(0.9)
        .build();

    let creative_response = provider
        .chat_completion(creative_request, "gemini-1.5-pro")
        .await?;

    println!("\n🎨 Creative Response:");
    println!("{}", creative_response.message.content);
    println!("Model: {}", creative_response.model);
    if let Some(reason) = creative_response.finish_reason {
        println!("Finish reason: {reason}");
    }

    // 6. Display safety ratings if available (Google AI specific)
    if let Some(metadata) = &creative_response.provider_metadata {
        if let Some(safety_ratings) = metadata.get("safety_ratings") {
            println!("🛡️ Safety ratings: {safety_ratings}");
        }
        if let Some(version) = metadata.get("google_ai_version") {
            println!("🔧 API version: {version}");
        }
    }

    // 7. Token estimation
    let text = "This is a sample text for token estimation.";
    let estimated_tokens = provider.estimate_tokens(text, "gemini-1.5-pro").await?;
    println!("\n📏 Estimated tokens for '{text}': {estimated_tokens}");

    // 8. Health check
    match provider.health_check().await {
        Ok(_) => println!("\n✅ Google AI is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
