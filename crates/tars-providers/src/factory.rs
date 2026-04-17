//! Factory for constructing the concrete `Provider` impl for a given id.
//!
//! Callers typically want the production instance; tests construct impls
//! directly via `with_base_url` so they can point at a mock server.

use crate::{
    provider::Provider,
    providers::{
        AnthropicProvider, BraveSearchProvider, DeepseekProvider, ElevenLabsProvider,
        GeminiProvider, GroqProvider, MistralProvider, OpenAiProvider, OpenRouterProvider,
        PerplexityProvider, XAiProvider,
    },
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
        ProviderId::BraveSearch => Box::new(BraveSearchProvider::new()),
        ProviderId::ElevenLabs => Box::new(ElevenLabsProvider::new()),
        ProviderId::Groq => Box::new(GroqProvider::new()),
        ProviderId::Mistral => Box::new(MistralProvider::new()),
        ProviderId::XAi => Box::new(XAiProvider::new()),
        ProviderId::OpenRouter => Box::new(OpenRouterProvider::new()),
        ProviderId::Perplexity => Box::new(PerplexityProvider::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_covers_all_providers() {
        for &id in ProviderId::ALL {
            let p = provider_for(id);
            assert_eq!(p.id(), id);
        }
    }
}
