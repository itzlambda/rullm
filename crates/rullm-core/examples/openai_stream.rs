use futures::StreamExt;
use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, ChatStreamEvent, OpenAIProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 OpenAI Streaming Chat Example");
    println!("================================\n");

    // 1. Configuration from environment
    // Set OPENAI_API_KEY environment variable before running
    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // 2. Simple streaming chat
    println!("💬 Simple streaming chat:");
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant.")
        .user("Tell me a short joke about programming.")
        .temperature(0.7)
        .max_tokens(100)
        .stream(true) // Enable streaming
        .build();

    let mut stream = provider
        .chat_completion_stream(request, "gpt-3.5-turbo", None)
        .await;

    print!("🤖 Assistant: ");
    while let Some(event) = stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Stream completed successfully");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Stream error: {error}");
                break;
            }
        }
    }

    // 3. Multi-turn conversation streaming
    println!("\n\n🗨️ Multi-turn conversation streaming:");
    let conversation_request = ChatRequestBuilder::new()
        .system("You are a coding tutor. Give concise explanations.")
        .user("What is async/await?")
        .assistant("Async/await is a pattern for writing asynchronous code that looks synchronous.")
        .user("Can you give a simple example in Rust?")
        .temperature(0.5)
        .max_tokens(150)
        .stream(true)
        .build();

    let mut conversation_stream = provider
        .chat_completion_stream(conversation_request, "gpt-4o-mini", None)
        .await;

    print!("🤖 Tutor: ");
    while let Some(event) = conversation_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Conversation stream completed");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Conversation stream error: {error}");
                break;
            }
        }
    }

    // 4. Creative writing with higher temperature
    println!("\n\n🎨 Creative writing stream (high temperature):");
    let creative_request = ChatRequestBuilder::new()
        .system("You are a creative writer.")
        .user("Write a very short story about a robot learning to dream.")
        .temperature(1.0) // Higher creativity
        .top_p(0.9)
        .max_tokens(200)
        .stream(true)
        .build();

    let mut creative_stream = provider
        .chat_completion_stream(creative_request, "gpt-4", None)
        .await;

    print!("✍️ Story: ");
    let mut token_count = 0;
    while let Some(event) = creative_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
                token_count += 1;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Creative stream completed ({token_count} tokens received)");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Creative stream error: {error}");
                break;
            }
        }
    }

    // 5. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let invalid_request = ChatRequestBuilder::new()
        .user("This request has an invalid model test.")
        .temperature(0.7)
        .stream(true)
        .build();

    let mut error_stream = provider
        .chat_completion_stream(invalid_request, "invalid-model-name", None)
        .await;

    while let Some(event) = error_stream.next().await {
        match event {
            Ok(ChatStreamEvent::Token(token)) => print!("{token}"),
            Ok(ChatStreamEvent::Done) => {
                println!("Unexpectedly completed");
                break;
            }
            Ok(ChatStreamEvent::Error(error)) => {
                println!("📡 Stream error event: {error}");
                break;
            }
            Err(error) => {
                println!("🔴 Request error: {error}");
                break;
            }
        }
    }

    println!("\n\n🎯 Tips for using OpenAI streaming:");
    println!("• Set OPENAI_API_KEY environment variable");
    println!("• Use .stream(true) in ChatRequestBuilder");
    println!("• Handle Token, Done, and Error events");
    println!("• Flush stdout for real-time output");
    println!("• Consider using lower max_tokens for faster streaming");

    Ok(())
}
