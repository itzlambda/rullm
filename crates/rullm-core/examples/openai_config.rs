use rullm_core::config::{OpenAIConfig, ProviderConfig};
use rullm_core::providers::openai::{
    ChatCompletionRequest, ChatMessage, ContentPart, MessageContent, OpenAIClient,
};

// Helper to extract text from MessageContent
fn extract_text(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenAI Client Configuration Examples ===\n");

    // 1. Basic configuration
    println!("1. Basic Configuration:");
    let basic_config = OpenAIConfig::new("your-api-key-here");
    println!("   Base URL: {}", basic_config.base_url());
    println!("   Timeout: {:?}", basic_config.timeout());
    println!("   Headers: {:?}", basic_config.headers());

    // 2. Configuration with organization and project
    println!("\n2. Configuration with Organization & Project:");
    let org_config = OpenAIConfig::new("your-api-key-here")
        .with_organization("org-123")
        .with_project("proj-456");

    let headers = org_config.headers();
    if let Some(org) = headers.get("OpenAI-Organization") {
        println!("   Organization: {org}");
    }
    if let Some(project) = headers.get("OpenAI-Project") {
        println!("   Project: {project}");
    }

    // 3. Configuration with custom base URL (for proxies, Azure OpenAI, etc.)
    println!("\n3. Custom Base URL:");
    let custom_config =
        OpenAIConfig::new("your-api-key-here").with_base_url("https://your-custom-endpoint.com/v1");
    println!("   Custom Base URL: {}", custom_config.base_url());

    // 4. Configuration from environment (real example)
    println!("\n4. Configuration from Environment:");
    match std::env::var("OPENAI_API_KEY") {
        Ok(api_key) => {
            let env_config = OpenAIConfig::new(api_key);

            // Validate configuration
            match env_config.validate() {
                Ok(_) => {
                    println!("   ✅ Configuration is valid");

                    // Create client and test
                    match OpenAIClient::new(env_config) {
                        Ok(client) => {
                            println!("   ✅ Client created successfully");

                            // Test health check
                            match client.health_check().await {
                                Ok(_) => println!("   ✅ Health check passed"),
                                Err(e) => println!("   ❌ Health check failed: {e}"),
                            }

                            // Get available models
                            match client.list_models().await {
                                Ok(models) => {
                                    println!("   Available models (first 5):");
                                    for (i, model) in models.iter().take(5).enumerate() {
                                        println!("     {}. {}", i + 1, model);
                                    }
                                    if models.len() > 5 {
                                        println!("     ... and {} more", models.len() - 5);
                                    }
                                }
                                Err(e) => println!("   ❌ Error getting models: {e}"),
                            }

                            // Make a simple request
                            println!("\n   Testing chat completion...");
                            let mut test_request = ChatCompletionRequest::new(
                                "gpt-3.5-turbo",
                                vec![ChatMessage::user("Hello, test request")],
                            );
                            test_request.temperature = Some(0.5);
                            test_request.max_tokens = Some(10);

                            match client.chat_completion(test_request).await {
                                Ok(response) => {
                                    println!(
                                        "   ✅ Test response: {}",
                                        extract_text(
                                            response.choices[0].message.content.as_ref().unwrap()
                                        )
                                    );
                                    println!("   Tokens used: {}", response.usage.total_tokens);
                                }
                                Err(e) => println!("   ❌ Test request failed: {e}"),
                            }
                        }
                        Err(e) => println!("   ❌ Failed to create client: {e}"),
                    }
                }
                Err(e) => println!("   ❌ Invalid configuration: {e}"),
            }
        }
        Err(_) => {
            println!("   ⚠️  OPENAI_API_KEY not set - skipping real API test");
            println!("   Set OPENAI_API_KEY environment variable to test with real API");
        }
    }

    // 5. Error handling examples
    println!("\n5. Error Handling Examples:");

    // Invalid API key format
    let invalid_config = OpenAIConfig::new("invalid-key");
    match invalid_config.validate() {
        Ok(_) => println!("   Unexpected: validation passed"),
        Err(e) => println!("   ✅ Caught invalid API key: {e}"),
    }

    // Empty API key
    let empty_config = OpenAIConfig::new("");
    match empty_config.validate() {
        Ok(_) => println!("   Unexpected: validation passed"),
        Err(e) => println!("   ✅ Caught empty API key: {e}"),
    }

    // 6. Request construction patterns
    println!("\n6. Request Construction Patterns:");

    // Minimal request
    let minimal = ChatCompletionRequest::new("gpt-3.5-turbo", vec![ChatMessage::user("Hello")]);
    println!("   Minimal request: {} message(s)", minimal.messages.len());

    // Full-featured request
    let mut full_request = ChatCompletionRequest::new(
        "gpt-3.5-turbo",
        vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("What's the weather like?"),
            ChatMessage::assistant("I don't have access to current weather data."),
            ChatMessage::user("That's okay, what can you help with?"),
        ],
    );
    full_request.temperature = Some(0.7);
    full_request.max_tokens = Some(150);
    full_request.top_p = Some(0.9);
    full_request.frequency_penalty = Some(0.1);
    full_request.presence_penalty = Some(0.1);
    full_request.stop = Some(vec!["END".to_string()]);

    println!(
        "   Full request: {} message(s)",
        full_request.messages.len()
    );
    println!("   Temperature: {:?}", full_request.temperature);
    println!("   Max tokens: {:?}", full_request.max_tokens);
    println!("   Top P: {:?}", full_request.top_p);
    println!("   Frequency penalty: {:?}", full_request.frequency_penalty);
    println!("   Presence penalty: {:?}", full_request.presence_penalty);
    println!("   Stop sequences: {:?}", full_request.stop);

    Ok(())
}
