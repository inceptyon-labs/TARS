//! Concrete `Provider` implementations, one per supported vendor.
//!
//! Each submodule is self-contained: its struct owns a `reqwest::Client` and
//! a configurable `base_url` (so tests can point at a `wiremock::MockServer`).
//! Default base URLs match each vendor's production API.

pub mod anthropic;
pub mod brave_search;
pub mod deepseek;
pub mod elevenlabs;
pub mod gemini;
pub mod groq;
pub mod mistral;
pub mod openai;
pub mod openrouter;
pub mod perplexity;
pub mod xai;

pub use anthropic::AnthropicProvider;
pub use brave_search::BraveSearchProvider;
pub use deepseek::DeepseekProvider;
pub use elevenlabs::ElevenLabsProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use mistral::MistralProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use perplexity::PerplexityProvider;
pub use xai::XAiProvider;
