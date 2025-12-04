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
    // Configure OpenAI client from environment
    let client = OpenAIClient::from_env()?;

    // Build a simple chat request
    let mut request = ChatCompletionRequest::new(
        "gpt-4",
        vec![
            ChatMessage::system("You are a helpful assistant that explains concepts clearly."),
            ChatMessage::user("What is the difference between async and sync programming?"),
        ],
    );
    request.temperature = Some(0.7);
    request.max_tokens = Some(300);

    // Make the request
    let response = client.chat_completion(request).await?;

    println!("Model: {}", response.model);
    println!(
        "Response: {}",
        extract_text(response.choices[0].message.content.as_ref().unwrap())
    );
    println!(
        "Token usage - Prompt: {}, Completion: {}, Total: {}",
        response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens
    );

    Ok(())
}
