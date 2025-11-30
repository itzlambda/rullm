use futures::StreamExt;
use rullm_core::providers::anthropic::{
    AnthropicClient, Delta, Message, MessagesRequest, StreamEvent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Anthropic Claude Streaming Chat Example");
    println!("==========================================\n");

    // 1. Configuration from environment
    // Set ANTHROPIC_API_KEY environment variable before running
    let client = AnthropicClient::from_env()?;

    // 2. Simple streaming chat with Claude
    println!("💬 Simple streaming chat:");
    let request = MessagesRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("Explain quantum computing in simple terms.")],
        150,
    )
    .with_system("You are Claude, a helpful and thoughtful AI assistant.")
    .with_temperature(0.7);

    let mut stream = client.messages_stream(request).await?;

    print!("🤖 Claude: ");
    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Delta::TextDelta { text } = delta {
                        print!("{text}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
                StreamEvent::MessageStop => {
                    println!("\n✅ Stream completed successfully");
                    break;
                }
                StreamEvent::Error { error } => {
                    println!("\n❌ Stream error: {}", error.message);
                    break;
                }
                _ => {} // Other events like MessageStart, ContentBlockStart, etc.
            },
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }

    // 3. Multi-turn philosophical conversation
    println!("\n\n🗨️ Multi-turn philosophical conversation:");
    let conversation_request = MessagesRequest::new(
        "claude-3-sonnet-20240229",
        vec![
            Message::user("What is consciousness?"),
            Message::assistant(
                "Consciousness is the subjective experience of being aware - the 'what it's like' quality of experience.",
            ),
            Message::user("Could an AI ever be truly conscious?"),
        ],
        200,
    )
    .with_system("You are a philosopher who enjoys exploring deep questions.")
    .with_temperature(0.6);

    let mut conversation_stream = client.messages_stream(conversation_request).await?;

    print!("🤖 Philosopher Claude: ");
    while let Some(event_result) = conversation_stream.next().await {
        match event_result {
            Ok(event) => match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Delta::TextDelta { text } = delta {
                        print!("{text}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
                StreamEvent::MessageStop => {
                    println!("\n✅ Philosophical stream completed");
                    break;
                }
                StreamEvent::Error { error } => {
                    println!("\n❌ Stream error: {}", error.message);
                    break;
                }
                _ => {}
            },
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }

    // 4. Creative writing with Claude's storytelling capabilities
    println!("\n\n🎨 Creative story stream:");
    let creative_request = MessagesRequest::new(
        "claude-3-5-sonnet-20241022",
        vec![Message::user(
            "Write a short story about a lighthouse keeper who discovers something extraordinary.",
        )],
        300,
    )
    .with_system("You are a master storyteller with a gift for vivid imagery.")
    .with_temperature(0.9) // Higher creativity
    .with_top_p(0.95);

    let mut creative_stream = client.messages_stream(creative_request).await?;

    print!("✍️ Story: ");
    let mut char_count = 0;
    while let Some(event_result) = creative_stream.next().await {
        match event_result {
            Ok(event) => match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Delta::TextDelta { text } = delta {
                        print!("{text}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                        char_count += text.len();
                    }
                }
                StreamEvent::MessageStop => {
                    println!("\n✅ Story completed (~{char_count} characters)");
                    break;
                }
                StreamEvent::Error { error } => {
                    println!("\n❌ Stream error: {}", error.message);
                    break;
                }
                _ => {}
            },
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }

    // 5. Code explanation with streaming
    println!("\n\n💻 Code explanation stream:");
    let code_request = MessagesRequest::new(
        "claude-3-opus-20240229",
        vec![Message::user(
            "Explain this Rust code step by step:\n\nlet mut v = vec![1, 2, 3];\nv.iter().map(|x| x * 2).collect::<Vec<_>>()",
        )],
        250,
    )
    .with_system("You are a programming mentor who explains code clearly and concisely.")
    .with_temperature(0.3); // Lower temperature for technical accuracy

    let mut code_stream = client.messages_stream(code_request).await?;

    print!("🧑‍💻 Mentor: ");
    while let Some(event_result) = code_stream.next().await {
        match event_result {
            Ok(event) => match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Delta::TextDelta { text } = delta {
                        print!("{text}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
                StreamEvent::MessageStop => {
                    println!("\n✅ Code explanation completed");
                    break;
                }
                StreamEvent::Error { error } => {
                    println!("\n❌ Stream error: {}", error.message);
                    break;
                }
                _ => {}
            },
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }

    // 6. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let invalid_request = MessagesRequest::new(
        "claude-invalid-model",
        vec![Message::user("Test with invalid model.")],
        100,
    )
    .with_temperature(0.7);

    match client.messages_stream(invalid_request).await {
        Ok(mut error_stream) => {
            while let Some(event_result) = error_stream.next().await {
                match event_result {
                    Ok(event) => match event {
                        StreamEvent::ContentBlockDelta { delta, .. } => {
                            if let Delta::TextDelta { text } = delta {
                                print!("{text}");
                            }
                        }
                        StreamEvent::Error { error } => {
                            println!("📡 Stream error event (as expected): {}", error.message);
                            break;
                        }
                        StreamEvent::MessageStop => {
                            println!("Unexpectedly completed");
                            break;
                        }
                        _ => {}
                    },
                    Err(error) => {
                        println!("🔴 Request error: {error}");
                        break;
                    }
                }
            }
        }
        Err(error) => {
            println!("🔴 Request error (as expected): {error}");
        }
    }

    println!("\n\n🎯 Tips for using Anthropic Claude streaming:");
    println!("• Set ANTHROPIC_API_KEY environment variable");
    println!("• Process StreamEvent variants: ContentBlockDelta, MessageStop, Error");
    println!("• Extract text from Delta::TextDelta events");
    println!("• Claude models: haiku (fast), sonnet (balanced), opus (largest)");
    println!("• Claude supports reasoning, analysis, and creative writing");
    println!("• Lower temperature (0.1-0.4) for factual content");
    println!("• Higher temperature (0.7-1.0) for creative content");

    Ok(())
}
