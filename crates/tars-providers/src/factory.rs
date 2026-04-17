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
