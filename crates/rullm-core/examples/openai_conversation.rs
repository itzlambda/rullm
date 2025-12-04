use rullm_core::providers::openai::{
    ChatCompletionRequest, ChatMessage, ContentPart, MessageContent, OpenAIClient,
};
use std::io::{self, Write};

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
    // Configure OpenAI client from environment
    let client = OpenAIClient::from_env()?;

    // Check available models
    println!("Available models:");
    match client.list_models().await {
        Ok(models) => {
            for (i, model) in models.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, model);
            }
            if models.len() > 10 {
                println!("  ... and {} more", models.len() - 10);
            }
        }
        Err(e) => println!("Error getting models: {e}"),
    }

    // Health check
    match client.health_check().await {
        Ok(_) => println!("✅ Client is healthy\n"),
        Err(e) => {
            println!("❌ Health check failed: {e}");
            return Ok(());
        }
    }

    println!("=== Multi-turn Conversation Example ===");
    println!("Type 'quit' or 'exit' to end the conversation\n");

    // Start with system message and context
    let mut conversation: Vec<ChatMessage> = vec![ChatMessage::system(
        "You are a helpful programming assistant. Keep responses concise but informative.",
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

        conversation.push(ChatMessage::user(user_input));

        // Build request from conversation history
        let mut request = ChatCompletionRequest::new("gpt-4o-mini", conversation.clone());
        request.temperature = Some(0.7);
        request.max_tokens = Some(500);

        print!("Assistant: ");
        io::stdout().flush()?;

        match client.chat_completion(request).await {
            Ok(response) => {
                let assistant_content = response.choices[0].message.content.as_ref().unwrap();

                // Extract text from MessageContent
                let assistant_text = match assistant_content {
                    rullm_core::providers::openai::MessageContent::Text(text) => text.clone(),
                    rullm_core::providers::openai::MessageContent::Parts(parts) => parts
                        .iter()
                        .filter_map(|part| match part {
                            rullm_core::providers::openai::ContentPart::Text { text } => {
                                Some(text.as_str())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                };

                println!("{}", assistant_text);

                // Add assistant response to conversation history
                conversation.push(ChatMessage::assistant(assistant_text));

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

        let mut request = ChatCompletionRequest::new(
            model,
            vec![
                ChatMessage::system("You are a concise technical writer."),
                ChatMessage::user(question),
            ],
        );
        request.temperature = Some(0.3); // Lower temperature for more consistent responses
        request.max_tokens = Some(100); // Limit response length
        request.top_p = Some(0.9); // Nucleus sampling
        request.frequency_penalty = Some(0.1); // Reduce repetition
        request.presence_penalty = Some(0.1); // Encourage diverse topics

        match client.chat_completion(request).await {
            Ok(response) => {
                println!(
                    "Response: {}",
                    extract_text(response.choices[0].message.content.as_ref().unwrap())
                );
                println!("Tokens: {}", response.usage.total_tokens);
            }
            Err(e) => {
                println!("Error with {model}: {e}");
            }
        }
    }

    println!("\n=== Stop Sequences Example ===");

    // Example using stop sequences
    let mut request = ChatCompletionRequest::new(
        "gpt-3.5-turbo",
        vec![
            ChatMessage::system("You are a code generator. Always end code blocks with '// END'"),
            ChatMessage::user("Write a simple hello world function in Rust"),
        ],
    );
    request.stop = Some(vec!["// END".to_string()]); // Stop generation at this sequence
    request.temperature = Some(0.5);

    match client.chat_completion(request).await {
        Ok(response) => {
            println!("Code generation (stopped at '// END'):");
            println!(
                "{}",
                extract_text(response.choices[0].message.content.as_ref().unwrap())
            );
            println!("Finish reason: {}", response.choices[0].finish_reason);
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    Ok(())
}
