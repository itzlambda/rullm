use futures::StreamExt;
use rullm_core::providers::google::{
    Content, GenerateContentRequest, GenerationConfig, GoogleClient, Part,
};

// Helper to extract text from response
fn extract_text(response: &rullm_core::providers::google::GenerateContentResponse) -> String {
    response
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.content.parts)
        .filter_map(|part| match part {
            Part::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Google Gemini Streaming Chat Example");
    println!("=======================================\n");

    // 1. Configuration from environment
    // Set GOOGLE_API_KEY environment variable before running
    let client = GoogleClient::from_env()?;

    // 2. Simple streaming chat with Gemini Flash
    println!("💬 Simple streaming chat:");
    let request = GenerateContentRequest::new(vec![Content::user(
        "Explain machine learning in simple terms.",
    )])
    .with_system("You are a helpful AI assistant built by Google.".to_string())
    .with_generation_config(GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(150),
        stop_sequences: None,
        top_p: None,
        top_k: None,
        response_mime_type: None,
        response_schema: None,
    });

    let mut stream = client
        .stream_generate_content("gemini-1.5-flash", request)
        .await?;

    print!("🤖 Gemini: ");
    while let Some(response_result) = stream.next().await {
        match response_result {
            Ok(response) => {
                let text = extract_text(&response);
                if !text.is_empty() {
                    print!("{text}");
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Stream completed successfully");

    // 3. Multi-turn technical conversation
    println!("\n\n🗨️ Multi-turn technical conversation:");
    let conversation_request = GenerateContentRequest::new(vec![
        Content::user("What are the differences between Rust and Go?"),
        Content::model("Rust focuses on memory safety and zero-cost abstractions, while Go emphasizes simplicity and built-in concurrency."),
        Content::user("Which would you recommend for a web API?"),
    ])
    .with_system("You are a technical expert who gives precise, helpful answers.".to_string())
    .with_generation_config(GenerationConfig {
        temperature: Some(0.5),
        max_output_tokens: Some(200),
        stop_sequences: None,
        top_p: None,
        top_k: None,
        response_mime_type: None,
        response_schema: None,
    });

    let mut conversation_stream = client
        .stream_generate_content("gemini-1.5-pro", conversation_request)
        .await?;

    print!("🤖 Expert Gemini: ");
    while let Some(response_result) = conversation_stream.next().await {
        match response_result {
            Ok(response) => {
                let text = extract_text(&response);
                if !text.is_empty() {
                    print!("{text}");
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Technical conversation completed");

    // 4. Creative writing with experimental Gemini 2.0
    println!("\n\n🎨 Creative writing stream (Gemini 2.0 experimental):");
    let creative_request = GenerateContentRequest::new(vec![Content::user(
        "Write a short story about an AI that discovers it can paint digital masterpieces.",
    )])
    .with_system("You are a creative writer who crafts engaging, vivid stories.".to_string())
    .with_generation_config(GenerationConfig {
        temperature: Some(0.9), // Higher creativity
        top_p: Some(0.95),
        max_output_tokens: Some(250),
        stop_sequences: None,
        top_k: None,
        response_mime_type: None,
        response_schema: None,
    });

    let mut creative_stream = client
        .stream_generate_content("gemini-2.0-flash-exp", creative_request)
        .await?;

    print!("✍️ Creative Story: ");
    let mut char_count = 0;
    while let Some(response_result) = creative_stream.next().await {
        match response_result {
            Ok(response) => {
                let text = extract_text(&response);
                if !text.is_empty() {
                    print!("{text}");
                    std::io::Write::flush(&mut std::io::stdout())?;
                    char_count += text.len();
                }
            }
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Creative stream completed (~{char_count} characters)");

    // 5. Code analysis with streaming
    println!("\n\n💻 Code analysis stream:");
    let code_request = GenerateContentRequest::new(vec![Content::user(
        "Review this Rust function and suggest improvements:\n\nfn fibonacci(n: u32) -> u32 {\n    if n <= 1 {\n        n\n    } else {\n        fibonacci(n - 1) + fibonacci(n - 2)\n    }\n}",
    )])
    .with_system("You are a code reviewer who provides detailed, constructive feedback.".to_string())
    .with_generation_config(GenerationConfig {
        temperature: Some(0.3), // Lower temperature for technical accuracy
        max_output_tokens: Some(300),
        stop_sequences: None,
        top_p: None,
        top_k: None,
        response_mime_type: None,
        response_schema: None,
    });

    let mut code_stream = client
        .stream_generate_content("gemini-1.5-pro", code_request)
        .await?;

    print!("🔍 Code Reviewer: ");
    while let Some(response_result) = code_stream.next().await {
        match response_result {
            Ok(response) => {
                let text = extract_text(&response);
                if !text.is_empty() {
                    print!("{text}");
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            Err(e) => {
                println!("\n❌ Stream error: {e}");
                break;
            }
        }
    }
    println!("\n✅ Code review completed");

    // 6. Model comparison streaming
    println!("\n\n⚖️ Model comparison streaming:");
    let models = ["gemini-1.5-flash", "gemini-1.5-pro"];
    let question = "What makes quantum computing different from classical computing?";

    for model in &models {
        println!("\n📋 Streaming with {model}:");
        let request = GenerateContentRequest::new(vec![Content::user(question)])
            .with_generation_config(GenerationConfig {
                temperature: Some(0.6),
                max_output_tokens: Some(120),
                stop_sequences: None,
                top_p: None,
                top_k: None,
                response_mime_type: None,
                response_schema: None,
            });

        let mut stream = client.stream_generate_content(model, request).await?;

        print!("🤖 {model}: ");
        while let Some(response_result) = stream.next().await {
            match response_result {
                Ok(response) => {
                    let text = extract_text(&response);
                    if !text.is_empty() {
                        print!("{text}");
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                }
                Err(e) => {
                    println!("\n❌ Stream error: {e}");
                    break;
                }
            }
        }
        println!("\n✅ {model} completed");
    }

    // 7. Error handling demonstration
    println!("\n\n⚠️ Error handling demonstration:");
    let invalid_request =
        GenerateContentRequest::new(vec![Content::user("Test with invalid model.")])
            .with_generation_config(GenerationConfig {
                temperature: Some(0.7),
                stop_sequences: None,
                max_output_tokens: None,
                top_p: None,
                top_k: None,
                response_mime_type: None,
                response_schema: None,
            });

    match client
        .stream_generate_content("gemini-invalid-model", invalid_request)
        .await
    {
        Ok(mut error_stream) => {
            while let Some(response_result) = error_stream.next().await {
                match response_result {
                    Ok(response) => {
                        let text = extract_text(&response);
                        if !text.is_empty() {
                            print!("{text}");
                        }
                    }
                    Err(error) => {
                        println!("🔴 Request error (as expected): {error}");
                        break;
                    }
                }
            }
        }
        Err(error) => {
            println!("🔴 Request error (as expected): {error}");
        }
    }

    println!("\n\n🎯 Tips for using Google Gemini streaming:");
    println!("• Set GOOGLE_API_KEY environment variable");
    println!("• Use stream_generate_content() for streaming responses");
    println!("• Process GenerateContentResponse chunks as they arrive");
    println!(
        "• Models: gemini-1.5-flash (fast), gemini-1.5-pro (balanced), gemini-2.0-flash-exp (experimental)"
    );
    println!("• Gemini supports reasoning, code analysis, and creative tasks");
    println!("• Lower temperature (0.1-0.4) for factual/technical content");
    println!("• Higher temperature (0.7-1.0) for creative content");
    println!("• Use top_p for more controlled randomness");

    Ok(())
}
