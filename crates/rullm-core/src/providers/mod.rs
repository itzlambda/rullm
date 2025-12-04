// New feature-complete provider implementations
pub mod anthropic;
pub mod google;
pub mod openai;
pub mod openai_compatible; // Used for Groq/OpenRouter

// Export concrete clients
pub use anthropic::AnthropicClient;
pub use google::GoogleClient;
pub use openai::OpenAIClient;
pub use openai_compatible::{OpenAICompatibleProvider, ProviderIdentity, identities};
