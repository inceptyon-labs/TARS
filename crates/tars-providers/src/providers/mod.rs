//! Concrete `Provider` implementations, one per supported vendor.
//!
//! Each submodule is self-contained: its struct owns a `reqwest::Client` and
//! a configurable `base_url` (so tests can point at a `wiremock::MockServer`).
//! Default base URLs match each vendor's production API.

pub mod anthropic;
pub mod deepseek;
pub mod gemini;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepseekProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAiProvider;
