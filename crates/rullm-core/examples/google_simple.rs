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
    // 1. Basic Configuration using from_env
    let client = GoogleClient::from_env()?;

    // 2. Simple Chat Completion
    let request = GenerateContentRequest::new(vec![Content::user("What is 2 + 2?")])
        .with_system("You are a helpful assistant.".to_string())
        .with_generation_config(GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(1024),
            stop_sequences: None,
            top_p: None,
            top_k: None,
            response_mime_type: None,
            response_schema: None,
        });

    let response = client.generate_content("gemini-1.5-flash", request).await?;

    println!("🤖 Assistant: {}", extract_text(&response));
    if let Some(usage) = &response.usage_metadata {
        println!("📊 Tokens used: {}", usage.total_token_count);
    }

    // 3. Multi-message conversation
    let conversation_request = GenerateContentRequest::new(vec![
        Content::user("What is 5 * 7?"),
        Content::model("5 * 7 = 35"),
        Content::user("What about 6 * 8?"),
    ])
    .with_system("You are a helpful math tutor.".to_string())
    .with_generation_config(GenerationConfig {
        max_output_tokens: Some(100),
        stop_sequences: None,
        temperature: None,
        top_p: None,
        top_k: None,
        response_mime_type: None,
        response_schema: None,
    });

    let conversation_response = client
        .generate_content("gemini-1.5-pro", conversation_request)
        .await?;

    println!("\n💬 Conversation:");
    println!("Assistant: {}", extract_text(&conversation_response));

    // 4. Different models comparison
    let models = ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash-exp"];
    let question = "Explain async/await in one sentence.";

    for model in &models {
        let request = GenerateContentRequest::new(vec![Content::user(question)])
            .with_generation_config(GenerationConfig {
                temperature: Some(0.5),
                max_output_tokens: Some(50),
                stop_sequences: None,
                top_p: None,
                top_k: None,
                response_mime_type: None,
                response_schema: None,
            });

        match client.generate_content(model, request).await {
            Ok(response) => {
                println!("\n🔬 {model} says:");
                println!("{}", extract_text(&response));
            }
            Err(e) => {
                println!("❌ Error with {model}: {e}");
            }
        }
    }

    // 5. Advanced parameters with Google-specific features
    let creative_request =
        GenerateContentRequest::new(vec![Content::user("Write a haiku about programming.")])
            .with_system("You are a creative writer.".to_string())
            .with_generation_config(GenerationConfig {
                temperature: Some(1.0), // Higher creativity
                top_p: Some(0.9),
                max_output_tokens: Some(200),
                stop_sequences: None,
                top_k: None,
                response_mime_type: None,
                response_schema: None,
            });

    let creative_response = client
        .generate_content("gemini-1.5-pro", creative_request)
        .await?;

    println!("\n🎨 Creative Response:");
    println!("{}", extract_text(&creative_response));
    if let Some(candidate) = creative_response.candidates.first() {
        if let Some(reason) = &candidate.finish_reason {
            println!("Finish reason: {:?}", reason);
        }
    }

    // 6. Display safety ratings if available (Google AI specific)
    if let Some(candidate) = creative_response.candidates.first() {
        if let Some(safety_ratings) = &candidate.safety_ratings {
            println!("🛡️ Safety ratings: {} checks", safety_ratings.len());
        }
    }

    // 7. List models
    println!("\n📋 Available models:");
    let models = client.list_models().await?;
    for (i, model) in models.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, model);
    }
    if models.len() > 5 {
        println!("  ... and {} more", models.len() - 5);
    }

    // 8. Health check
    match client.health_check().await {
        Ok(_) => println!("\n✅ Google AI is healthy"),
        Err(e) => println!("\n❌ Health check failed: {e}"),
    }

    Ok(())
}
