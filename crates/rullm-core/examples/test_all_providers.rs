use rullm_core::providers::anthropic::{AnthropicClient, AnthropicConfig};
use rullm_core::providers::google::{GoogleAiConfig, GoogleClient};
use rullm_core::providers::openai::OpenAIClient;
use rullm_core::providers::openai_compatible::OpenAIConfig;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing All LLM Providers and Their Available Models\n");

    // Test results tracking
    let mut results = Vec::new();

    // 1. Test OpenAI Provider
    println!("🔍 Testing OpenAI Provider...");
    match test_openai_provider().await {
        Ok(models) => {
            println!("✅ OpenAI: Found {} models", models.len());
            println!(
                "   Models (first 5): {}",
                models
                    .iter()
                    .take(5)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            results.push(("OpenAI", true, models.len()));
        }
        Err(e) => {
            println!("❌ OpenAI: Failed - {e}");
            results.push(("OpenAI", false, 0));
        }
    }
    println!();

    // 2. Test Anthropic Provider
    println!("🔍 Testing Anthropic Provider...");
    match test_anthropic_provider().await {
        Ok(models) => {
            println!("✅ Anthropic: Found {} models", models.len());
            println!(
                "   Models (first 5): {}",
                models
                    .iter()
                    .take(5)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            results.push(("Anthropic", true, models.len()));
        }
        Err(e) => {
            println!("❌ Anthropic: Failed - {e}");
            results.push(("Anthropic", false, 0));
        }
    }
    println!();

    // 3. Test Google Provider
    println!("🔍 Testing Google Provider...");
    match test_google_provider().await {
        Ok(models) => {
            println!("✅ Google: Found {} models", models.len());
            println!(
                "   Models (first 5): {}",
                models
                    .iter()
                    .take(5)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            results.push(("Google", true, models.len()));
        }
        Err(e) => {
            println!("❌ Google: Failed - {e}");
            results.push(("Google", false, 0));
        }
    }
    println!();

    // Summary
    println!("📊 SUMMARY:");
    println!("┌─────────────┬────────┬─────────────┐");
    println!("│ Provider    │ Status │ Models      │");
    println!("├─────────────┼────────┼─────────────┤");
    for (provider, success, model_count) in &results {
        let status = if *success { "✅ Pass" } else { "❌ Fail" };
        let models = if *success {
            format!("{model_count} models")
        } else {
            "N/A".to_string()
        };
        println!("│ {provider:11} │ {status:6} │ {models:11} │");
    }
    println!("└─────────────┴────────┴─────────────┘");

    let successful_providers = results.iter().filter(|(_, success, _)| *success).count();
    let total_providers = results.len();

    if successful_providers == total_providers {
        println!("\n🎉 All providers are working correctly!");
    } else {
        println!(
            "\n⚠️  {successful_providers}/{total_providers} providers working. Check API keys and network connectivity."
        );
    }

    Ok(())
}

async fn test_openai_provider() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let api_key =
        env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY environment variable not set")?;

    let config = OpenAIConfig::new(api_key);
    let client = OpenAIClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    // Get available models
    let models = client.list_models().await?;

    // Verify we have expected models
    let expected_models = ["gpt-4", "gpt-3.5-turbo"];
    for expected in &expected_models {
        if models.iter().any(|m| m.contains(expected)) {
            println!("   Expected model '{expected}': ✅ Found");
        } else {
            println!("   Expected model '{expected}': ⚠️  Not found in list");
        }
    }

    Ok(models)
}

async fn test_anthropic_provider() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable not set")?;

    let config = AnthropicConfig::new(api_key);
    let client = AnthropicClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    // Get available models
    let models = client.list_models().await?;

    Ok(models)
}

async fn test_google_provider() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let api_key =
        env::var("GOOGLE_API_KEY").map_err(|_| "GOOGLE_API_KEY environment variable not set")?;

    let config = GoogleAiConfig::new(api_key);
    let client = GoogleClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    // Get available models
    let models = client.list_models().await?;

    // Verify we have expected models
    let expected_models = ["gemini", "flash", "pro"];
    for expected in &expected_models {
        if models.iter().any(|m| m.contains(expected)) {
            println!("   Expected model pattern '{expected}': ✅ Found");
        } else {
            println!("   Expected model pattern '{expected}': ⚠️  Not found in list");
        }
    }

    Ok(models)
}
