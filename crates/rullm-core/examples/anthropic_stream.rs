use futures::StreamExt;
use rullm_core::config::ConfigBuilder;
use rullm_core::{AnthropicProvider, ChatCompletion, ChatRequestBuilder, ChatStreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Anthropic Claude Streaming Chat Example");
    println!("==========================================\n");

    // 1. Configuration from environment
    // Set ANTHROPIC_API_KEY environment variable before running
    let config = ConfigBuilder::anthropic_from_env()?;
    let provider = AnthropicProvider::new(config)?;

    // 2. Simple streaming chat with Claude
    println!("💬 Simple streaming chat:");
    let request = ChatRequestBuilder::new()
        .system("You are Claude, a helpful and thoughtful AI assistant.")
        .user("Explain quantum computing in simple terms.")
        .temperature(0.7)
        .max_tokens(150)
        .stream(true) // Enable streaming
        .build();

    let mut stream = provider
        .chat_completion_stream(request, "claude-3-haiku-20240307", None)
        .await;

    print!("🤖 Claude: ");
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

    // 3. Multi-turn philosophical conversation
    println!("\n\n🗨️ Multi-turn philosophical conversation:");
    let conversation_request = ChatRequestBuilder::new()
        .system("You are a philosopher who enjoys exploring deep questions.")
        .user("What is consciousness?")
        .assistant("Consciousness is the subjective experience of being aware - the 'what it's like' quality of experience.")
        .user("Could an AI ever be truly conscious?")
        .temperature(0.6)
        .max_tokens(200)
        .stream(true)
        .build();

    let mut conversation_stream = provider
        .chat_completion_stream(conversation_request, "claude-3-sonnet-20240229", None)
        .await;

    print!("🤖 Philosopher Claude: ");
    while let Some(event) = conversation_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Philosophical stream completed");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Philosophical stream error: {error}");
                break;
            }
        }
    }

    // 4. Creative writing with Claude's storytelling capabilities
    println!("\n\n🎨 Creative story stream:");
    let creative_request = ChatRequestBuilder::new()
        .system("You are a master storyteller with a gift for vivid imagery.")
        .user(
            "Write a short story about a lighthouse keeper who discovers something extraordinary.",
        )
        .temperature(0.9) // Higher creativity
        .top_p(0.95)
        .max_tokens(300)
        .stream(true)
        .build();

    let mut creative_stream = provider
        .chat_completion_stream(creative_request, "claude-3-5-sonnet-20241022", None)
        .await;

    print!("✍️ Story: ");
    let mut word_count = 0;
    while let Some(event) = creative_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
                // Rough word counting
                if token.contains(' ') {
                    word_count += token.split_whitespace().count();
                }
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Story completed (~{word_count} words)");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Story stream error: {error}");
                break;
            }
        }
    }

    // 5. Code explanation with streaming
    println!("\n\n💻 Code explanation stream:");
    let code_request = ChatRequestBuilder::new()
        .system("You are a programming mentor who explains code clearly and concisely.")
        .user("Explain this Rust code step by step:\n\nlet mut v = vec![1, 2, 3];\nv.iter().map(|x| x * 2).collect::<Vec<_>>()")
        .temperature(0.3) // Lower temperature for technical accuracy
        .max_tokens(250)
        .stream(true)
        .build();

    let mut code_stream = provider
        .chat_completion_stream(code_request, "claude-3-opus-20240229", None)
        .await;

    print!("🧑‍💻 Mentor: ");
    while let Some(event) = code_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Code explanation completed");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Code explanation error: {error}");
                break;
            }
        }
    }

    // 6. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let invalid_request = ChatRequestBuilder::new()
        .user("Test with invalid model.")
        .temperature(0.7)
        .stream(true)
        .build();

    let mut error_stream = provider
        .chat_completion_stream(invalid_request, "claude-invalid-model", None)
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
                println!("🔴 Request error (as expected): {error}");
                break;
            }
        }
    }

    println!("\n\n🎯 Tips for using Anthropic Claude streaming:");
    println!("• Set ANTHROPIC_API_KEY environment variable");
    println!("• Use .stream(true) in ChatRequestBuilder");
    println!("• Claude models: haiku (fast), sonnet (balanced), opus (largest)");
    println!("• Claude supports reasoning, analysis, and creative writing");
    println!("• Lower temperature (0.1-0.4) for factual content");
    println!("• Higher temperature (0.7-1.0) for creative content");

    Ok(())
}
