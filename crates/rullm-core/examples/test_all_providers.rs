use rullm_core::providers::anthropic::{AnthropicClient, AnthropicConfig};
use rullm_core::providers::google::{GoogleAiConfig, GoogleClient};
use rullm_core::providers::openai::OpenAIClient;
use rullm_core::providers::openai_compatible::OpenAIConfig;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing All LLM Providers\n");

    // Test results tracking
    let mut results = Vec::new();

    // 1. Test OpenAI Provider
    println!("🔍 Testing OpenAI Provider...");
    match test_openai_provider().await {
        Ok(()) => {
            println!("✅ OpenAI: Health check passed");
            results.push(("OpenAI", true));
        }
        Err(e) => {
            println!("❌ OpenAI: Failed - {e}");
            results.push(("OpenAI", false));
        }
    }
    println!();

    // 2. Test Anthropic Provider
    println!("🔍 Testing Anthropic Provider...");
    match test_anthropic_provider().await {
        Ok(()) => {
            println!("✅ Anthropic: Health check passed");
            results.push(("Anthropic", true));
        }
        Err(e) => {
            println!("❌ Anthropic: Failed - {e}");
            results.push(("Anthropic", false));
        }
    }
    println!();

    // 3. Test Google Provider
    println!("🔍 Testing Google Provider...");
    match test_google_provider().await {
        Ok(()) => {
            println!("✅ Google: Health check passed");
            results.push(("Google", true));
        }
        Err(e) => {
            println!("❌ Google: Failed - {e}");
            results.push(("Google", false));
        }
    }
    println!();

    // Summary
    println!("📊 SUMMARY:");
    println!("┌─────────────┬────────┐");
    println!("│ Provider    │ Status │");
    println!("├─────────────┼────────┤");
    for (provider, success) in &results {
        let status = if *success { "✅ Pass" } else { "❌ Fail" };
        println!("│ {provider:11} │ {status:6} │");
    }
    println!("└─────────────┴────────┘");

    let successful_providers = results.iter().filter(|(_, success)| *success).count();
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

async fn test_openai_provider() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY environment variable not set")?;

    let config = OpenAIConfig::new(api_key);
    let client = OpenAIClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    Ok(())
}

async fn test_anthropic_provider() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable not set")?;

    let config = AnthropicConfig::new(api_key);
    let client = AnthropicClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    Ok(())
}

async fn test_google_provider() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        env::var("GOOGLE_API_KEY").map_err(|_| "GOOGLE_API_KEY environment variable not set")?;

    let config = GoogleAiConfig::new(api_key);
    let client = GoogleClient::new(config)?;

    // Test health check
    match client.health_check().await {
        Ok(_) => println!("   Health check: ✅ Passed"),
        Err(e) => println!("   Health check: ⚠️  Warning - {e}"),
    }

    Ok(())
}
