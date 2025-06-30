use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, LlmProvider, OpenAIProvider};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OpenAI provider using ConfigBuilder
    let config = ConfigBuilder::openai_from_env()?;

    let provider = OpenAIProvider::new(config)?;

    // Check available models
    println!("Available models:");
    match provider.available_models().await {
        Ok(models) => {
            for model in models {
                println!("  - {model}");
            }
        }
        Err(e) => println!("Error getting models: {e}"),
    }

    // Health check
    match provider.health_check().await {
        Ok(_) => println!("✅ Provider is healthy"),
        Err(e) => {
            println!("❌ Health check failed: {e}");
            return Ok(());
        }
    }

    println!("\n=== Multi-turn Conversation Example ===");

    // Start with system message and context
    let mut conversation = vec![(
        "system".to_string(),
        "You are a helpful programming assistant. Keep responses concise but informative."
            .to_string(),
    )];

    // Interactive conversation loop
    loop {
        print!("\nYou: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let user_input = input.trim();

        if user_input.is_empty() || user_input == "quit" || user_input == "exit" {
            break;
        }

        conversation.push(("user".to_string(), user_input.to_string()));

        // Build request from conversation history
        let mut builder = ChatRequestBuilder::new() // Using faster model for demo
            .temperature(0.7)
            .max_tokens(500);

        // Add all messages from conversation
        for (role, content) in &conversation {
            match role.as_str() {
                "system" => builder = builder.system(content),
                "user" => builder = builder.user(content),
                "assistant" => builder = builder.assistant(content),
                "tool" => builder = builder.tool(content),
                _ => {}
            }
        }

        let request = builder.build();

        print!("Assistant: ");
        io::stdout().flush()?;

        match provider.chat_completion(request, "gpt-4o-mini").await {
            Ok(response) => {
                println!("{}", response.message.content);

                // Add assistant response to conversation history
                conversation.push(("assistant".to_string(), response.message.content.clone()));

                // Show token usage
                println!(
                    "\n📊 Tokens used: {} prompt + {} completion = {} total",
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                    response.usage.total_tokens
                );
            }
            Err(e) => {
                println!("Error: {e}");
            }
        }
    }

    println!("\n=== Advanced Configuration Example ===");

    // Example with different models and parameters
    let models_to_test = ["gpt-3.5-turbo", "gpt-4o-mini"];
    let question = "Explain the concept of ownership in Rust in one sentence.";

    for &model in &models_to_test {
        println!("\n🤖 Testing model: {model}");

        let request = ChatRequestBuilder::new()
            .system("You are a concise technical writer.")
            .user(question)
            .temperature(0.3) // Lower temperature for more consistent responses
            .max_tokens(100) // Limit response length
            .top_p(0.9) // Nucleus sampling
            // .frequency_penalty(0.1) // Reduce repetition
            // .presence_penalty(0.1) // Encourage diverse topics
            .build();

        match provider.chat_completion(request, model).await {
            Ok(response) => {
                println!("Response: {}", response.message.content);
                println!("Tokens: {}", response.usage.total_tokens);

                // Estimate tokens for the request (useful for cost estimation)
                match provider
                    .estimate_tokens(&response.message.content, model)
                    .await
                {
                    Ok(estimated) => println!("Estimated tokens for this text: {estimated}"),
                    Err(e) => println!("Error estimating tokens: {e}"),
                }
            }
            Err(e) => {
                println!("Error with {model}: {e}");
            }
        }
    }

    println!("\n=== Stop Sequences Example ===");

    // Example using stop sequences
    let request = ChatRequestBuilder::new()
        .system("You are a code generator. Always end code blocks with '// END'")
        .user("Write a simple hello world function in Rust")
        // .stop_sequences(vec!["// END".to_string()]) // Stop generation at this sequence
        .temperature(0.5)
        .build();

    match provider.chat_completion(request, "gpt-3.5-turbo").await {
        Ok(response) => {
            println!("Code generation (stopped at '// END'):");
            println!("{}", response.message.content);
            if let Some(reason) = response.finish_reason {
                println!("Finish reason: {reason}");
            }
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    Ok(())
}
