use rullm_core::config::{OpenAIConfig, ProviderConfig};
use rullm_core::{ChatCompletion, ChatRequestBuilder, LlmProvider, OpenAIProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenAI Provider Configuration Examples ===\n");

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

                    // Create provider and test
                    match OpenAIProvider::new(env_config) {
                        Ok(provider) => {
                            println!("   ✅ Provider created successfully");
                            println!("   Provider name: {}", provider.name());

                            // Test health check
                            match provider.health_check().await {
                                Ok(_) => println!("   ✅ Health check passed"),
                                Err(e) => println!("   ❌ Health check failed: {e}"),
                            }

                            // Get available models
                            match provider.available_models().await {
                                Ok(models) => {
                                    println!("   Available models: {}", models.join(", "));
                                }
                                Err(e) => println!("   ❌ Error getting models: {e}"),
                            }

                            // Make a simple request
                            println!("\n   Testing chat completion...");
                            let test_request = ChatRequestBuilder::new()
                                .user("Hello, test request")
                                .temperature(0.5)
                                .max_tokens(10)
                                .build();

                            match provider
                                .chat_completion(test_request, "gpt-3.5-turbo")
                                .await
                            {
                                Ok(response) => {
                                    println!("   ✅ Test response: {}", response.message.content);
                                    println!("   Tokens used: {}", response.usage.total_tokens);
                                }
                                Err(e) => println!("   ❌ Test request failed: {e}"),
                            }
                        }
                        Err(e) => println!("   ❌ Failed to create provider: {e}"),
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

    // 6. Request builder patterns
    println!("\n6. Request Builder Patterns:");

    // Minimal request
    let minimal = ChatRequestBuilder::new().user("Hello").build();
    println!("   Minimal request: {} message(s)", minimal.messages.len());

    // Full-featured request
    let full_request = ChatRequestBuilder::new()
        .system("You are a helpful assistant.")
        .user("What's the weather like?")
        .assistant("I don't have access to current weather data.")
        .user("That's okay, what can you help with?")
        .temperature(0.7)
        .max_tokens(150)
        .top_p(0.9)
        // .frequency_penalty(0.1)
        // .presence_penalty(0.1)
        // .stop_sequences(vec!["END".to_string()])
        .extra_param("custom_field", serde_json::json!("custom_value"))
        .build();

    println!(
        "   Full request: {} message(s)",
        full_request.messages.len()
    );
    println!("   Temperature: {:?}", full_request.temperature);
    println!("   Max tokens: {:?}", full_request.max_tokens);
    println!("   Top P: {:?}", full_request.top_p);
    println!("   Extra params: {:?}", full_request.extra_params);

    Ok(())
}
