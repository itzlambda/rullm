use rullm_core::providers::anthropic::{AnthropicClient, Message, MessagesRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Basic Configuration using from_env
    let client = AnthropicClient::from_env()?;

    // 2. Simple Chat Completion
    let request = MessagesRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("What is 2 + 2?")],
        1024,
    )
    .with_system("You are a helpful assistant.")
    .with_temperature(0.7);

    let response = client.messages(request).await?;

    // Extract text from content blocks
    let text = response
        .content
        .iter()
        .filter_map(|block| match block {
            rullm_core::providers::anthropic::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    println!("🤖 Claude: {}", text);
    println!(
        "📊 Tokens used: {} input + {} output",
        response.usage.input_tokens, response.usage.output_tokens
    );

    // 3. Multi-message conversation
    let conversation_request = MessagesRequest::new(
        "claude-3-sonnet-20240229",
        vec![
            Message::user("What is 5 * 7?"),
            Message::assistant("5 * 7 = 35"),
            Message::user("What about 6 * 8?"),
        ],
        100,
    )
    .with_system("You are a helpful math tutor.");

    let conversation_response = client.messages(conversation_request).await?;

    let conversation_text = conversation_response
        .content
        .iter()
        .filter_map(|block| match block {
            rullm_core::providers::anthropic::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    println!("\n💬 Conversation:");
    println!("Claude: {}", conversation_text);

    // 4. Different Claude models comparison
    let models = [
        "claude-3-haiku-20240307",
        "claude-3-sonnet-20240229",
        "claude-3-opus-20240229",
    ];
    let question = "Explain async/await in one sentence.";

    for model in &models {
        let request =
            MessagesRequest::new(*model, vec![Message::user(question)], 50).with_temperature(0.5);

        match client.messages(request).await {
            Ok(response) => {
                let text = response
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        rullm_core::providers::anthropic::ContentBlock::Text { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                println!("\n🔬 {model} says:");
                println!("{}", text);
            }
            Err(e) => {
                println!("❌ Error with {model}: {e}");
                // Note: Some models might not be available depending on your API access
            }
        }
    }

    // 5. Advanced parameters with Anthropic-specific features
    let creative_request = MessagesRequest::new(
        "claude-3-5-sonnet-20241022",
        vec![Message::user("Write a haiku about programming.")],
        200,
    )
    .with_system("You are a creative writer.")
    .with_temperature(1.0) // Higher creativity
    .with_top_p(0.9); // Nucleus sampling

    let creative_response = client.messages(creative_request).await?;

    let creative_text = creative_response
        .content
        .iter()
        .filter_map(|block| match block {
            rullm_core::providers::anthropic::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    println!("\n🎨 Creative Response:");
    println!("{}", creative_text);
    println!("Model: {}", creative_response.model);
    if let Some(reason) = creative_response.stop_reason {
        println!("Stop reason: {:?}", reason);
    }

    // 6. Token estimation
    let text = "This is a sample text for token estimation.";
    let estimated_tokens = client
        .count_tokens("claude-3-haiku-20240307", vec![Message::user(text)], None)
        .await?;
    println!("\n📏 Estimated tokens for '{text}': {estimated_tokens}");

    // 7. Health check
    match client.health_check().await {
        Ok(_) => println!("\n✅ Anthropic API is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
