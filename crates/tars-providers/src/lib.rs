//! AI provider integrations for TARS.
//!
//! Provides a uniform `Provider` trait that each supported AI provider
//! implements, a static metadata registry, and concrete HTTP-backed
//! implementations for OpenAI, Anthropic, Gemini, and DeepSeek.

pub mod error;
pub mod factory;
pub mod provider;
pub mod providers;
pub mod registry;
pub mod types;

pub use error::ProviderError;
pub use factory::provider_for;
pub use provider::Provider;
pub use providers::{AnthropicProvider, DeepseekProvider, GeminiProvider, OpenAiProvider};
pub use registry::{all_metadata, metadata_for};
pub use types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult};
