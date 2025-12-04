use futures::StreamExt;
use rullm_core::providers::openai::{ChatCompletionRequest, ChatMessage, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 OpenAI Streaming Chat Example");
    println!("================================\n");

    // 1. Configuration from environment
    // Set OPENAI_API_KEY environment variable before running
    let client = OpenAIClient::from_env()?;

    // 2. Simple streaming chat
    println!("💬 Simple streaming chat:");
    let mut request = ChatCompletionRequest::new(
        "gpt-3.5-turbo",
        vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Tell me a short joke about programming."),
        ],
    );
    request.temperature = Some(0.7);
    request.max_tokens = Some(100);
    request.stream = Some(true); // Enable streaming

    let mut stream = client.chat_completion_stream(request).await?;

    print!("🤖 Assistant: ");
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(choice) = chunk.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{content}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
            }
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Stream completed successfully");

    // 3. Multi-turn conversation streaming
    println!("\n\n🗨️ Multi-turn conversation streaming:");
    let mut conversation_request = ChatCompletionRequest::new(
        "gpt-4o-mini",
        vec![
            ChatMessage::system("You are a coding tutor. Give concise explanations."),
            ChatMessage::user("What is async/await?"),
            ChatMessage::assistant(
                "Async/await is a pattern for writing asynchronous code that looks synchronous.",
            ),
            ChatMessage::user("Can you give a simple example in Rust?"),
        ],
    );
    conversation_request.temperature = Some(0.5);
    conversation_request.max_tokens = Some(150);
    conversation_request.stream = Some(true);

    let mut conversation_stream = client.chat_completion_stream(conversation_request).await?;

    print!("🤖 Tutor: ");
    while let Some(chunk_result) = conversation_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(choice) = chunk.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{content}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
            }
            Err(e) => {
                println!("\n❌ Conversation stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Conversation stream completed");

    // 4. Creative writing with higher temperature
    println!("\n\n🎨 Creative writing stream (high temperature):");
    let mut creative_request = ChatCompletionRequest::new(
        "gpt-4",
        vec![
            ChatMessage::system("You are a creative writer."),
            ChatMessage::user("Write a very short story about a robot learning to dream."),
        ],
    );
    creative_request.temperature = Some(1.0); // Higher creativity
    creative_request.top_p = Some(0.9);
    creative_request.max_tokens = Some(200);
    creative_request.stream = Some(true);

    let mut creative_stream = client.chat_completion_stream(creative_request).await?;

    print!("✍️ Story: ");
    let mut token_count = 0;
    while let Some(chunk_result) = creative_stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(choice) = chunk.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{content}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                        token_count += 1;
                    }
                }
            }
            Err(e) => {
                // Stream complete is returned as an error
                if e.to_string().contains("Stream complete") {
                    break;
                }
                println!("\n❌ Creative stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Creative stream completed ({token_count} chunks received)");

    // 5. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let mut invalid_request = ChatCompletionRequest::new(
        "invalid-model-name",
        vec![ChatMessage::user("This request has an invalid model test.")],
    );
    invalid_request.temperature = Some(0.7);
    invalid_request.stream = Some(true);

    match client.chat_completion_stream(invalid_request).await {
        Ok(mut error_stream) => {
            while let Some(chunk_result) = error_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                print!("{content}");
                            }
                        }
                    }
                    Err(error) => {
                        println!("📡 Stream error (as expected): {error}");
                        break;
                    }
                }
            }
        }
        Err(error) => {
            println!("🔴 Request error (as expected): {error}");
        }
    }

    println!("\n\n🎯 Tips for using OpenAI streaming:");
    println!("• Set OPENAI_API_KEY environment variable");
    println!("• Set request.stream = Some(true) to enable streaming");
    println!("• Process ChatCompletionChunk deltas as they arrive");
    println!("• Flush stdout for real-time output");
    println!("• Consider using lower max_tokens for faster streaming");

    Ok(())
}
