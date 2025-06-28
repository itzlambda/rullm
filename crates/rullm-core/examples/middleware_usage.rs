use rullm_core::{
    ChatRequestBuilder, ConfigBuilder, LlmServiceBuilder, MiddlewareConfig, OpenAIProvider,
    RateLimit, config::RetryPolicy,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== LLM Middleware Usage Examples ===\n");

    // Example 1: Basic middleware stack with defaults
    basic_middleware_example().await?;

    // Example 2: Custom retry policy with exponential backoff
    custom_retry_example().await?;

    // Example 3: Production-ready configuration
    production_config_example().await?;

    // Example 4: Rate-limited and monitored configuration
    rate_limited_example().await?;

    // Example 5: Custom middleware configuration
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

/// Example 2: Custom retry policy with exponential backoff
async fn custom_retry_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Example 2: Custom Retry Policy");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Create middleware with custom exponential backoff retry policy
    let mut middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(60)) // 60 second timeout
        .retry(RetryPolicy::ExponentialBackoff {
            initial_delay_ms: 200, // Start with 200ms
            max_delay_ms: 10000,   // Cap at 10 seconds
            multiplier: 2.5,       // Aggressive backoff
            jitter: true,          // Add randomness
        })
        .logging()
        .build(provider, "gpt-3.5-turbo".to_string());

    let request = ChatRequestBuilder::new()
        .user("Explain quantum computing in simple terms")
        .temperature(0.7)
        .max_tokens(150)
        .build();

    let response = middleware_stack.call(request).await?;

    println!("✅ Response: {}", response.message.content);
    println!("🔄 Retry policy: Exponential backoff with jitter\n");

    Ok(())
}

/// Example 3: Production-ready configuration
async fn production_config_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏭 Example 3: Production Configuration");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Production-ready middleware configuration
    let mut middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(30)) // Conservative timeout
        .retry(RetryPolicy::ApiGuided {
            fallback: Box::new(RetryPolicy::ExponentialBackoff {
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                multiplier: 2.0,
                jitter: true,
            }),
            max_api_delay_ms: 30000, // Don't wait more than 30 seconds
            retry_headers: vec!["retry-after".to_string(), "x-ratelimit-reset".to_string()],
        })
        .rate_limit(100, Duration::from_secs(60)) // 100 requests per minute
        .logging() // Always log in production
        .metrics() // Always collect metrics
        .build(provider, "gpt-4".to_string());

    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant for a production application.")
        .user("How can I optimize my database queries?")
        .temperature(0.3) // More deterministic for production
        .max_tokens(300)
        .build();

    let response = middleware_stack.call(request).await?;

    println!("✅ Production response received");
    println!("📊 Token usage: {}", response.usage.total_tokens);
    println!("🛡️ Configuration: API-guided retry, rate limited, fully monitored\n");

    Ok(())
}

/// Example 4: Rate-limited and monitored configuration
async fn rate_limited_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️ Example 4: Rate Limited Configuration");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Configuration optimized for rate limiting and monitoring
    let mut middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(45))
        .retry(RetryPolicy::Fixed { delay_ms: 1000 }) // Simple fixed delay
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

/// Example 5: Custom middleware configuration from struct
async fn custom_middleware_config_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️ Example 5: Custom Middleware Configuration");

    let config = ConfigBuilder::openai_from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Define custom middleware configuration
    let middleware_config = MiddlewareConfig {
        timeout: Some(Duration::from_secs(20)),
        retry_policy: Some(RetryPolicy::ExponentialBackoff {
            initial_delay_ms: 500,
            max_delay_ms: 8000,
            multiplier: 1.8,
            jitter: false,
        }),
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
    println!(
        "⚙️ Configuration: Custom timeouts, exponential backoff (no jitter), 25 req/min limit\n"
    );

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
