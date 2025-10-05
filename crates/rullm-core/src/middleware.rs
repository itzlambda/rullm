use crate::error::LlmError;
use crate::types::{
    ChatCompletion, ChatRequest, ChatResponse, ChatStreamEvent, StreamConfig, StreamResult,
};
use futures::Stream;
use metrics::{counter, histogram};

use std::pin::Pin;
use std::time::{Duration, Instant};

/// Streaming metrics collector
#[derive(Debug, Clone)]
pub struct StreamingMetrics {
    pub start_time: Instant,
    pub first_byte_latency: Option<Duration>,
    pub total_bytes: usize,
    pub total_tokens: usize,
    pub provider_name: String,
}

impl StreamingMetrics {
    pub fn new(provider_name: String) -> Self {
        Self {
            start_time: Instant::now(),
            first_byte_latency: None,
            total_bytes: 0,
            total_tokens: 0,
            provider_name,
        }
    }

    pub fn record_first_byte(&mut self) {
        if self.first_byte_latency.is_none() {
            let latency = self.start_time.elapsed();
            self.first_byte_latency = Some(latency);

            // Record first-byte latency metric
            let provider = self.provider_name.clone();
            histogram!(
                "llm_streaming_first_byte_latency_ms",
                "provider" => provider
            )
            .record(latency.as_millis() as f64);

            log::debug!(
                "First byte received after {:?} for provider {}",
                latency,
                self.provider_name
            );
        }
    }

    pub fn record_token(&mut self, token: &str) {
        self.total_bytes += token.len();
        self.total_tokens += 1;
    }

    pub fn finalize(&self) {
        let total_duration = self.start_time.elapsed();

        // Record throughput metrics
        if total_duration.as_secs_f64() > 0.0 {
            let bytes_per_second = self.total_bytes as f64 / total_duration.as_secs_f64();
            let tokens_per_second = self.total_tokens as f64 / total_duration.as_secs_f64();

            let provider = self.provider_name.clone();
            histogram!(
                "llm_streaming_bytes_per_second",
                "provider" => provider.clone()
            )
            .record(bytes_per_second);
            histogram!(
                "llm_streaming_tokens_per_second",
                "provider" => provider
            )
            .record(tokens_per_second);

            log::debug!(
                "Stream completed: {} bytes, {} tokens in {:?} ({:.2} bytes/s, {:.2} tokens/s) for provider {}",
                self.total_bytes,
                self.total_tokens,
                total_duration,
                bytes_per_second,
                tokens_per_second,
                self.provider_name
            );
        }

        // Record stream completion
        let provider = self.provider_name.clone();
        counter!(
            "llm_streaming_completions_total",
            "provider" => provider
        )
        .increment(1);
    }
}

/// Wrapper stream that adds metrics tracking
pub struct MetricsStream<S> {
    inner: S,
    metrics: StreamingMetrics,
}

impl<S> MetricsStream<S> {
    pub fn new(inner: S, provider_name: String) -> Self {
        Self {
            inner,
            metrics: StreamingMetrics::new(provider_name),
        }
    }
}

impl<S> Stream for MetricsStream<S>
where
    S: Stream<Item = Result<ChatStreamEvent, LlmError>> + Unpin,
{
    type Item = Result<ChatStreamEvent, LlmError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Token(token)))) => {
                self.metrics.record_first_byte();
                self.metrics.record_token(&token);
                std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Token(token))))
            }
            std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Done))) => {
                self.metrics.finalize();
                std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Done)))
            }
            std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Error(msg)))) => {
                let provider = self.metrics.provider_name.clone();
                counter!(
                    "llm_streaming_errors_total",
                    "provider" => provider
                )
                .increment(1);
                std::task::Poll::Ready(Some(Ok(ChatStreamEvent::Error(msg))))
            }
            std::task::Poll::Ready(Some(Err(e))) => {
                let provider = self.metrics.provider_name.clone();
                counter!(
                    "llm_streaming_errors_total",
                    "provider" => provider
                )
                .increment(1);
                std::task::Poll::Ready(Some(Err(e)))
            }
            std::task::Poll::Ready(None) => {
                self.metrics.finalize();
                std::task::Poll::Ready(None)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Middleware configuration for LLM providers
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    pub timeout: Option<Duration>,
    pub rate_limit: Option<RateLimit>,
    pub enable_logging: bool,
    pub enable_metrics: bool,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            rate_limit: None,
            enable_logging: true,
            enable_metrics: false,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_period: u64,
    pub period: Duration,
}

// ProviderService removed - using direct provider integration instead

/// Enhanced service builder for LLM middleware stacks
#[derive(Default)]
pub struct LlmServiceBuilder {
    middleware_config: MiddlewareConfig,
}

impl LlmServiceBuilder {
    pub fn new() -> Self {
        Self {
            middleware_config: MiddlewareConfig::default(),
        }
    }

    pub fn with_config(config: MiddlewareConfig) -> Self {
        Self {
            middleware_config: config,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.middleware_config.timeout = Some(timeout);
        self
    }

    pub fn rate_limit(mut self, requests_per_period: u64, period: Duration) -> Self {
        let rate_limit = RateLimit {
            requests_per_period,
            period,
        };
        self.middleware_config.rate_limit = Some(rate_limit);
        self
    }

    pub fn logging(mut self) -> Self {
        self.middleware_config.enable_logging = true;
        self
    }

    pub fn metrics(mut self) -> Self {
        self.middleware_config.enable_metrics = true;
        self
    }

    pub fn build<P>(self, provider: P, model: String) -> MiddlewareStack<P>
    where
        P: ChatCompletion + Clone + Send + Sync + 'static,
    {
        MiddlewareStack {
            provider,
            model,
            config: self.middleware_config,
        }
    }
}

/// Wrapper for a complete middleware stack
pub struct MiddlewareStack<P> {
    provider: P,
    model: String,
    config: MiddlewareConfig,
}

impl<P> MiddlewareStack<P>
where
    P: ChatCompletion + Clone,
{
    pub async fn call(&mut self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let start = std::time::Instant::now();

        // Apply logging middleware
        if self.config.enable_logging {
            log::info!(
                "Processing chat request with {} messages",
                request.messages.len()
            );
        }

        // Make the request
        let result = self.provider.chat_completion(request, &self.model).await;

        // Apply metrics and logging
        match &result {
            Ok(response) => {
                if self.config.enable_logging {
                    log::info!(
                        "Request completed in {:?}, tokens: {}",
                        start.elapsed(),
                        response.usage.total_tokens
                    );
                }
                if self.config.enable_metrics {
                    log::debug!(
                        "Metrics: request_duration={:?}, tokens={}, model={}",
                        start.elapsed(),
                        response.usage.total_tokens,
                        response.model
                    );
                }
            }
            Err(error) => {
                if self.config.enable_logging {
                    log::error!("Request failed after {:?}: {}", start.elapsed(), error);
                }
                if self.config.enable_metrics {
                    log::debug!(
                        "Metrics: request_duration={:?}, status=error",
                        start.elapsed()
                    );
                }
            }
        }

        result
    }

    /// Streaming version with metrics tracking
    pub async fn call_stream(
        &self,
        request: ChatRequest,
        config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        let provider_name = self.provider.name().to_string();

        if self.config.enable_logging {
            log::info!(
                "Processing streaming chat request with {} messages",
                request.messages.len()
            );
        }

        // Make the streaming request
        let stream = self
            .provider
            .chat_completion_stream(request, &self.model, config)
            .await;

        let metrics_stream = MetricsStream::new(stream, provider_name);

        Box::pin(metrics_stream)
    }

    pub fn config(&self) -> &MiddlewareConfig {
        &self.config
    }
}
