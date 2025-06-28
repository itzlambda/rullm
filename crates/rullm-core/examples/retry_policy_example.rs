use rullm_core::config::{ConfigBuilder, ProviderConfig, RetryPolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM Retry Policy Examples ===\n");

    // Example 1: Simple fixed retry
    let config = ConfigBuilder::openai_from_env()?.with_fixed_retry(3, 2000); // 3 retries, 2s delay

    println!("1. Fixed Retry Policy:");
    println!("   Max retries: {}", config.max_retries());
    println!("   Policy: {:?}\n", config.retry_policy());

    // Example 2: Exponential backoff
    let config =
        ConfigBuilder::openai_from_env()?.with_exponential_backoff(5, 1000, 30000, 2.0, true);

    println!("2. Exponential Backoff:");
    println!("   Max retries: {}", config.max_retries());
    println!("   Policy: {:?}\n", config.retry_policy());

    // Example 3: API-guided retry (respects rate limit headers)
    let fallback = RetryPolicy::ExponentialBackoff {
        initial_delay_ms: 1000,
        max_delay_ms: 15000,
        multiplier: 2.0,
        jitter: true,
    };

    let config = ConfigBuilder::openai_from_env()?.with_api_guided_retry(3, fallback, 60000); // Max 60s from API

    println!("3. API-Guided Retry (Smart):");
    println!("   Max retries: {}", config.max_retries());
    println!("   Policy: {:?}\n", config.retry_policy());

    // Example 4: Smart retry (default - API-guided with good fallback)
    let config = ConfigBuilder::openai_from_env()?.with_smart_retry(5);

    println!("4. Smart Retry (Default):");
    println!("   Max retries: {}", config.max_retries());
    println!("   Policy: {:?}\n", config.retry_policy());

    println!("=== How API-Guided Retry Works ===");
    println!("When a 429 (rate limit) response is received:");
    println!("1. Check for 'Retry-After' header -> use that delay");
    println!("2. Check for 'X-RateLimit-Reset' header -> calculate delay");
    println!("3. If no headers found -> use fallback policy");
    println!("4. Respect max_api_delay_ms limit (prevents infinite waits)");

    Ok(())
}
