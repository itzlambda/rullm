use rullm_core::{
    ChatRequestBuilder, ConfigBuilder, LlmServiceBuilder, MiddlewareConfig, OpenAIProvider,
    RateLimit,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== LLM Middleware Usage Examples ===\n");

    // Example 1: Basic middleware stack with defaults
    basic_middleware_example().await?;

    // Example 2: Configuration with timeouts and rate limiting
    production_config_example().await?;

    // Example 3: Rate-limited and monitored configuration
    rate_limited_example().await?;

    // Example 4: Custom middleware configuration
    custom_middleware_config_example().await?;

    Ok(())
}

/// Example 1: Basic middleware stack with default settings
async fn basic_middleware_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Example 1: Basic Middleware Stack");

    // Configure the OpenAI provider
    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Create a basic middleware stack with default settings
    let mut middleware_stack = LlmServiceBuilder::new()
        .logging() // Enable request/response logging
        .metrics() // Enable performance metrics
        .build(provider, "gpt-3.5-turbo".to_string());

    // Create a simple request
    let request = ChatRequestBuilder::new()
        .user("What is the capital of France?")
        .build();

    // Make the request through the middleware stack
    let response = middleware_stack.call(request).await?;

    println!("✅ Response: {}", response.message.content);
    println!("📊 Tokens used: {}\n", response.usage.total_tokens);

    Ok(())
}

/// Example 2: Configuration with timeouts and rate limiting
async fn production_config_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏭 Example 2: Configuration with Timeouts and Rate Limiting");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Middleware configuration with timeouts and rate limiting
    let mut middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(30)) // Conservative timeout
        .rate_limit(100, Duration::from_secs(60)) // 100 requests per minute
        .logging()
        .metrics()
        .build(provider, "gpt-4".to_string());

    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant for a production application.")
        .user("How can I optimize my database queries?")
        .temperature(0.3) // Lower temperature for more deterministic output
        .max_tokens(300)
        .build();

    let response = middleware_stack.call(request).await?;

    println!("✅ Response received");
    println!("📊 Token usage: {}", response.usage.total_tokens);
    println!("🛡️ Configuration: Rate limited, logged and monitored\n");

    Ok(())
}

/// Example 3: Rate-limited and monitored configuration
async fn rate_limited_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️ Example 3: Rate Limited Configuration");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Configuration optimized for rate limiting and monitoring
    let mut middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(45))
        .rate_limit(50, Duration::from_secs(60)) // Conservative rate limit
        .logging()
        .metrics()
        .build(provider, "gpt-3.5-turbo".to_string());

    // Simulate multiple requests to show rate limiting behavior
    for i in 1..=3 {
        let request = ChatRequestBuilder::new()
            .user(format!(
                "What is the {} most important programming concept?",
                match i {
                    1 => "first",
                    2 => "second",
                    3 => "third",
                    _ => "unknown",
                }
            ))
            .build();

        let start = std::time::Instant::now();
        let response = middleware_stack.call(request).await?;
        let duration = start.elapsed();

        println!(
            "📝 Request {}: {} (took {:?})",
            i,
            response
                .message
                .content
                .chars()
                .take(50)
                .collect::<String>()
                + "...",
            duration
        );
    }

    println!("⏱️ Rate limiting applied successfully\n");

    Ok(())
}

/// Example 4: Custom middleware configuration from struct
async fn custom_middleware_config_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️ Example 4: Custom Middleware Configuration");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Define custom middleware configuration
    let middleware_config = MiddlewareConfig {
        timeout: Some(Duration::from_secs(20)),
        rate_limit: Some(RateLimit {
            requests_per_period: 25,
            period: Duration::from_secs(60),
        }),
        enable_logging: true,
        enable_metrics: true,
    };

    // Build middleware stack from custom configuration
    let mut middleware_stack = LlmServiceBuilder::with_config(middleware_config)
        .build(provider, "gpt-3.5-turbo".to_string());

    let request = ChatRequestBuilder::new()
        .system("You are an expert software architect.")
        .user("What are the key principles of microservices architecture?")
        .temperature(0.5)
        .build();

    let response = middleware_stack.call(request).await?;

    println!("✅ Custom configuration response received");
    println!(
        "📊 Response length: {} characters",
        response.message.content.len()
    );
    println!("⚙️ Configuration: Custom timeouts, 25 req/min limit\n");

    // Display the configuration details
    let config = middleware_stack.config();
    println!("📋 Middleware Configuration Details:");
    println!("   • Timeout: {:?}", config.timeout);
    println!("   • Logging: {}", config.enable_logging);
    println!("   • Metrics: {}", config.enable_metrics);
    if let Some(rate_limit) = &config.rate_limit {
        println!(
            "   • Rate limit: {} requests per {:?}",
            rate_limit.requests_per_period, rate_limit.period
        );
    }

    Ok(())
}
