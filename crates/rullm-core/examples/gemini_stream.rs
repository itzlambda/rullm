use futures::StreamExt;
use rullm_core::config::ConfigBuilder;
use rullm_core::{ChatCompletion, ChatRequestBuilder, ChatStreamEvent, GoogleProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Google Gemini Streaming Chat Example");
    println!("=======================================\n");

    // 1. Configuration from environment
    // Set GOOGLE_API_KEY environment variable before running
    let config = ConfigBuilder::google_ai_from_env()?;
    let provider = GoogleProvider::new(config)?;

    // 2. Simple streaming chat with Gemini Flash
    println!("💬 Simple streaming chat:");
    let request = ChatRequestBuilder::new()
        .system("You are a helpful AI assistant built by Google.")
        .user("Explain machine learning in simple terms.")
        .temperature(0.7)
        .max_tokens(150)
        .stream(true) // Enable streaming
        .build();

    let mut stream = provider
        .chat_completion_stream(request, "gemini-1.5-flash", None)
        .await;

    print!("🤖 Gemini: ");
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

    // 3. Multi-turn conversation streaming with Gemini Pro
    println!("\n\n🗨️ Multi-turn technical conversation:");
    let conversation_request = ChatRequestBuilder::new()
        .system("You are a technical expert who gives precise, helpful answers.")
        .user("What are the differences between Rust and Go?")
        .assistant("Rust focuses on memory safety and zero-cost abstractions, while Go emphasizes simplicity and built-in concurrency.")
        .user("Which would you recommend for a web API?")
        .temperature(0.5)
        .max_tokens(200)
        .stream(true)
        .build();

    let mut conversation_stream = provider
        .chat_completion_stream(conversation_request, "gemini-1.5-pro", None)
        .await;

    print!("🤖 Expert Gemini: ");
    while let Some(event) = conversation_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Technical conversation completed");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Technical conversation error: {error}");
                break;
            }
        }
    }

    // 4. Creative writing with experimental Gemini 2.0
    println!("\n\n🎨 Creative writing stream (Gemini 2.0 experimental):");
    let creative_request = ChatRequestBuilder::new()
        .system("You are a creative writer who crafts engaging, vivid stories.")
        .user("Write a short story about an AI that discovers it can paint digital masterpieces.")
        .temperature(0.9) // Higher creativity
        .top_p(0.95)
        .max_tokens(250)
        .stream(true)
        .build();

    let mut creative_stream = provider
        .chat_completion_stream(creative_request, "gemini-2.0-flash-exp", None)
        .await;

    print!("✍️ Creative Story: ");
    let mut sentence_count = 0;
    while let Some(event) = creative_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
                // Count sentences
                if token.contains('.') || token.contains('!') || token.contains('?') {
                    sentence_count += token.matches(&['.', '!', '?'][..]).count();
                }
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Creative stream completed (~{sentence_count} sentences)");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Creative stream error: {error}");
                break;
            }
        }
    }

    // 5. Code analysis with streaming
    println!("\n\n💻 Code analysis stream:");
    let code_request = ChatRequestBuilder::new()
        .system("You are a code reviewer who provides detailed, constructive feedback.")
        .user("Review this Rust function and suggest improvements:\n\nfn fibonacci(n: u32) -> u32 {\n    if n <= 1 {\n        n\n    } else {\n        fibonacci(n - 1) + fibonacci(n - 2)\n    }\n}")
        .temperature(0.3) // Lower temperature for technical accuracy
        .max_tokens(300)
        .stream(true)
        .build();

    let mut code_stream = provider
        .chat_completion_stream(code_request, "gemini-1.5-pro", None)
        .await;

    print!("🔍 Code Reviewer: ");
    while let Some(event) = code_stream.next().await {
        match event? {
            ChatStreamEvent::Token(token) => {
                print!("{token}");
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            ChatStreamEvent::Done => {
                println!("\n✅ Code review completed");
                break;
            }
            ChatStreamEvent::Error(error) => {
                println!("\n❌ Code review error: {error}");
                break;
            }
        }
    }

    // 6. Model comparison streaming
    println!("\n\n⚖️ Model comparison streaming:");
    let models = ["gemini-1.5-flash", "gemini-1.5-pro"];
    let question = "What makes quantum computing different from classical computing?";

    for model in &models {
        println!("\n📋 Streaming with {model}:");
        let request = ChatRequestBuilder::new()
            .user(question)
            .temperature(0.6)
            .max_tokens(120)
            .stream(true)
            .build();

        let mut stream = provider.chat_completion_stream(request, model, None).await;

        print!("🤖 {model}: ");
        while let Some(event) = stream.next().await {
            match event? {
                ChatStreamEvent::Token(token) => {
                    print!("{token}");
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
                ChatStreamEvent::Done => {
                    println!("\n✅ {model} completed");
                    break;
                }
                ChatStreamEvent::Error(error) => {
                    println!("\n❌ {model} stream error: {error}");
                    break;
                }
            }
        }
    }

    // 7. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let invalid_request = ChatRequestBuilder::new()
        .user("Test with invalid model.")
        .temperature(0.7)
        .stream(true)
        .build();

    let mut error_stream = provider
        .chat_completion_stream(invalid_request, "gemini-invalid-model", None)
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

    println!("\n\n🎯 Tips for using Google Gemini streaming:");
    println!("• Set GOOGLE_API_KEY environment variable");
    println!("• Use .stream(true) in ChatRequestBuilder");
    println!(
        "• Models: gemini-1.5-flash (fast), gemini-1.5-pro (balanced), gemini-2.0-flash-exp (experimental)"
    );
    println!("• Gemini supports reasoning, code analysis, and creative tasks");
    println!("• Lower temperature (0.1-0.4) for factual/technical content");
    println!("• Higher temperature (0.7-1.0) for creative content");
    println!("• Use top_p for more controlled randomness");

    Ok(())
}
