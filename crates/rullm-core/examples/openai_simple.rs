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
    // 1. Basic Configuration using from_env
    let client = OpenAIClient::from_env()?;

    // 2. Simple Chat Completion
    let request = ChatCompletionRequest::new(
        "gpt-3.5-turbo",
        vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("What is 2 + 2?"),
        ],
    );

    let response = client.chat_completion(request).await?;

    println!(
        "🤖 Assistant: {}",
        extract_text(response.choices[0].message.content.as_ref().unwrap())
    );
    println!("📊 Tokens used: {}", response.usage.total_tokens);

    // 3. Multi-message conversation
    let mut conversation_request = ChatCompletionRequest::new(
        "gpt-4o-mini",
        vec![
            ChatMessage::system("You are a helpful math tutor."),
            ChatMessage::user("What is 5 * 7?"),
            ChatMessage::assistant("5 * 7 = 35"),
            ChatMessage::user("What about 6 * 8?"),
        ],
    );
    conversation_request.max_tokens = Some(100);

    let conversation_response = client.chat_completion(conversation_request).await?;

    println!("\n💬 Conversation:");
    println!(
        "Assistant: {}",
        extract_text(
            conversation_response.choices[0]
                .message
                .content
                .as_ref()
                .unwrap()
        )
    );

    // 4. Different models comparison
    let models = ["gpt-3.5-turbo", "gpt-4o-mini"];
    let question = "Explain async/await in one sentence.";

    for model in &models {
        let mut request = ChatCompletionRequest::new(*model, vec![ChatMessage::user(question)]);
        request.temperature = Some(0.5);
        request.max_tokens = Some(50);

        match client.chat_completion(request).await {
            Ok(response) => {
                println!("\n🔬 {model} says:");
                println!(
                    "{}",
                    extract_text(response.choices[0].message.content.as_ref().unwrap())
                );
            }
            Err(e) => {
                println!("❌ Error with {model}: {e}");
            }
        }
    }

    // 5. Advanced parameters
    let mut creative_request = ChatCompletionRequest::new(
        "gpt-4",
        vec![
            ChatMessage::system("You are a creative writer."),
            ChatMessage::user("Write a haiku about programming."),
        ],
    );
    creative_request.temperature = Some(1.0); // Higher creativity
    creative_request.top_p = Some(0.9); // Nucleus sampling
    // creative_request.frequency_penalty = Some(0.2); // Reduce repetition
    // creative_request.presence_penalty = Some(0.2); // Encourage diverse topics
    // creative_request.stop = Some(vec!["END".to_string(), "STOP".to_string()]); // Stop sequences

    let creative_response = client.chat_completion(creative_request).await?;

    println!("\n🎨 Creative Response:");
    println!(
        "{}",
        extract_text(
            creative_response.choices[0]
                .message
                .content
                .as_ref()
                .unwrap()
        )
    );
    println!("Model: {}", creative_response.model);
    println!(
        "Finish reason: {}",
        creative_response.choices[0].finish_reason
    );

    // 6. List models
    println!("\n📋 Available models:");
    let models = client.list_models().await?;
    for (i, model) in models.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, model);
    }
    if models.len() > 5 {
        println!("  ... and {} more", models.len() - 5);
    }

    // 7. Health check
    match client.health_check().await {
        Ok(_) => println!("\n✅ OpenAI API is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
