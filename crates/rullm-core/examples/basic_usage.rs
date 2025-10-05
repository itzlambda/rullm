use rullm_core::ChatRequestBuilder;

// This example demonstrates the unified interface without actual provider implementations
// It shows how the library would be used once provider modules are implemented

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LLM Library - Unified Interface Example");
    println!("=======================================");

    // This demonstrates the builder pattern for creating requests
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant that provides concise answers")
        .user("What is the capital of France?")
        .temperature(0.7)
        .max_tokens(100)
        .build();

    println!("Created chat request:");
    println!("  Messages: {} total", request.messages.len());
    println!("  Temperature: {:?}", request.temperature);
    println!("  Max tokens: {:?}", request.max_tokens);

    for (i, message) in request.messages.iter().enumerate() {
        println!(
            "  Message {}: {:?} - {}",
            i + 1,
            message.role,
            message.content
        );
    }

    println!("\nThis example shows the unified interface design.");
    println!("Actual provider implementations will be added in subsequent tasks.");

    Ok(())
}
