use rullm_core::{ChatRequestBuilder, ChatRole};

// This example demonstrates the unified interface (compat_types) for OpenAI-compatible providers
// It shows the builder pattern without requiring actual provider implementations

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LLM Library - Unified Interface Example (compat_types)");
    println!("=======================================================");
    println!();
    println!("This demonstrates the builder pattern for creating OpenAI-compatible requests.");
    println!(
        "These types are used by OpenAICompatibleProvider for providers like Groq/OpenRouter.\n"
    );

    // This demonstrates the builder pattern for creating requests
    let request = ChatRequestBuilder::new()
        .add_message(
            ChatRole::System,
            "You are a helpful assistant that provides concise answers",
        )
        .add_message(ChatRole::User, "What is the capital of France?")
        .temperature(0.7)
        .max_tokens(100)
        .build();

    println!("Created chat request:");
    println!("  Messages: {} total", request.messages.len());
    println!("  Temperature: {:?}", request.temperature);
    println!("  Max tokens: {:?}", request.max_tokens);
    println!("  Stream: {:?}", request.stream);

    for (i, message) in request.messages.iter().enumerate() {
        println!(
            "  Message {}: {:?} - {}",
            i + 1,
            message.role,
            message.content
        );
    }

    println!("\n🔍 Key Points:");
    println!("  • These compat_types are minimal types for OpenAI-compatible providers");
    println!("  • For full-featured OpenAI, use OpenAIClient with ChatCompletionRequest");
    println!("  • For Anthropic, use AnthropicClient with MessagesRequest");
    println!("  • For Google, use GoogleClient with GenerateContentRequest");
    println!(
        "\nSee provider-specific examples (openai_simple, anthropic_simple, google_simple) for details."
    );

    Ok(())
}
