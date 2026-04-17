//! Factory for constructing the concrete `Provider` impl for a given id.
//!
//! Callers typically want the production instance; tests construct impls
//! directly via `with_base_url` so they can point at a mock server.

use crate::{
    provider::Provider,
    providers::{AnthropicProvider, DeepseekProvider, GeminiProvider, OpenAiProvider},
    types::ProviderId,
};

/// Return a boxed `Provider` implementation for the given id.
///
/// Each call builds a fresh HTTP client; providers are cheap to construct.
#[must_use]
pub fn provider_for(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::OpenAi => Box::new(OpenAiProvider::new()),
        ProviderId::Anthropic => Box::new(AnthropicProvider::new()),
        ProviderId::Gemini => Box::new(GeminiProvider::new()),
        ProviderId::Deepseek => Box::new(DeepseekProvider::new()),
        ProviderId::BraveSearch
        | ProviderId::ElevenLabs
        | ProviderId::Groq
        | ProviderId::Mistral
        | ProviderId::XAi
        | ProviderId::OpenRouter
        | ProviderId::Perplexity => todo!("simple-storage providers wired in later commit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_covers_wired_providers() {
        let wired = [
            ProviderId::OpenAi,
            ProviderId::Anthropic,
            ProviderId::Gemini,
            ProviderId::Deepseek,
        ];
        for id in wired {
            let p = provider_for(id);
            assert_eq!(p.id(), id);
        }
    }
}
