use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, OpenAIProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OpenAI provider using ConfigBuilder
    let config = ConfigBuilder::openai_from_env()?;

    // Create provider instance
    let provider = OpenAIProvider::new(config)?;

    // Build a simple chat request
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant that explains concepts clearly.")
        .user("What is the difference between async and sync programming?")
        .temperature(0.7)
        .max_tokens(300)
        .build();

    // Make the request
    let response = provider.chat_completion(request, "gpt-4").await?;

    println!("Model: {}", response.model);
    println!("Response: {}", response.message.content);
    println!(
        "Token usage - Prompt: {}, Completion: {}, Total: {}",
        response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens
    );

    Ok(())
}
