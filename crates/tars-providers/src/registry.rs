//! Static provider metadata registry.

use crate::types::{ProviderId, ProviderMetadata};

const OPENAI: ProviderMetadata = ProviderMetadata {
    id: ProviderId::OpenAi,
    display_name: "OpenAI",
    docs_url: "https://platform.openai.com/api-keys",
    key_format_hint: "sk-...",
    supports_models: true,
    supports_balance: false,
};

const ANTHROPIC: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Anthropic,
    display_name: "Anthropic",
    docs_url: "https://console.anthropic.com/settings/keys",
    key_format_hint: "sk-ant-...",
    supports_models: true,
    supports_balance: false,
};

const GEMINI: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Gemini,
    display_name: "Google Gemini",
    docs_url: "https://aistudio.google.com/app/apikey",
    key_format_hint: "AIza...",
    supports_models: true,
    supports_balance: false,
};

const DEEPSEEK: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Deepseek,
    display_name: "DeepSeek",
    docs_url: "https://platform.deepseek.com/api_keys",
    key_format_hint: "sk-...",
    supports_models: true,
    supports_balance: true,
};

const BRAVE_SEARCH: ProviderMetadata = ProviderMetadata {
    id: ProviderId::BraveSearch,
    display_name: "Brave Search",
    docs_url: "https://api-dashboard.search.brave.com/app/keys",
    key_format_hint: "BSA...",
    supports_models: false,
    supports_balance: false,
};

const ELEVENLABS: ProviderMetadata = ProviderMetadata {
    id: ProviderId::ElevenLabs,
    display_name: "ElevenLabs",
    docs_url: "https://elevenlabs.io/app/settings/api-keys",
    key_format_hint: "sk_...",
    supports_models: false,
    supports_balance: false,
};

const GROQ: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Groq,
    display_name: "Groq",
    docs_url: "https://console.groq.com/keys",
    key_format_hint: "gsk_...",
    supports_models: false,
    supports_balance: false,
};

const MISTRAL: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Mistral,
    display_name: "Mistral",
    docs_url: "https://console.mistral.ai/api-keys",
    key_format_hint: "...",
    supports_models: false,
    supports_balance: false,
};

const XAI: ProviderMetadata = ProviderMetadata {
    id: ProviderId::XAi,
    display_name: "xAI",
    docs_url: "https://console.x.ai",
    key_format_hint: "xai-...",
    supports_models: false,
    supports_balance: false,
};

const OPENROUTER: ProviderMetadata = ProviderMetadata {
    id: ProviderId::OpenRouter,
    display_name: "OpenRouter",
    docs_url: "https://openrouter.ai/keys",
    key_format_hint: "sk-or-...",
    supports_models: false,
    supports_balance: false,
};

const PERPLEXITY: ProviderMetadata = ProviderMetadata {
    id: ProviderId::Perplexity,
    display_name: "Perplexity",
    docs_url: "https://www.perplexity.ai/settings/api",
    key_format_hint: "pplx-...",
    supports_models: false,
    supports_balance: false,
};

/// Get static metadata for a provider
#[must_use]
pub const fn metadata_for(id: ProviderId) -> &'static ProviderMetadata {
    match id {
        ProviderId::OpenAi => &OPENAI,
        ProviderId::Anthropic => &ANTHROPIC,
        ProviderId::Gemini => &GEMINI,
        ProviderId::Deepseek => &DEEPSEEK,
        ProviderId::BraveSearch => &BRAVE_SEARCH,
        ProviderId::ElevenLabs => &ELEVENLABS,
        ProviderId::Groq => &GROQ,
        ProviderId::Mistral => &MISTRAL,
        ProviderId::XAi => &XAI,
        ProviderId::OpenRouter => &OPENROUTER,
        ProviderId::Perplexity => &PERPLEXITY,
    }
}

/// Metadata for every registered provider
#[must_use]
pub fn all_metadata() -> Vec<&'static ProviderMetadata> {
    ProviderId::ALL.iter().map(|&id| metadata_for(id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_for_all_providers_has_nonempty_name() {
        for &id in ProviderId::ALL {
            let m = metadata_for(id);
            assert!(!m.display_name.is_empty(), "empty name for {id:?}");
            assert!(!m.docs_url.is_empty(), "empty docs for {id:?}");
            assert_eq!(m.id, id);
        }
    }

    #[test]
    fn all_metadata_returns_eleven_entries() {
        let all = all_metadata();
        assert_eq!(all.len(), 11);
    }

    #[test]
    fn deepseek_is_only_provider_with_balance() {
        let with_balance: Vec<_> = all_metadata()
            .iter()
            .filter(|m| m.supports_balance)
            .map(|m| m.id)
            .collect();
        assert_eq!(with_balance, vec![ProviderId::Deepseek]);
    }

    #[test]
    fn model_api_providers_support_model_discovery() {
        let model_providers = [
            ProviderId::OpenAi,
            ProviderId::Anthropic,
            ProviderId::Gemini,
            ProviderId::Deepseek,
        ];
        for id in model_providers {
            assert!(
                metadata_for(id).supports_models,
                "{id:?} should support models"
            );
        }
    }

    #[test]
    fn simple_storage_providers_skip_model_and_balance() {
        let simple = [
            ProviderId::BraveSearch,
            ProviderId::ElevenLabs,
            ProviderId::Groq,
            ProviderId::Mistral,
            ProviderId::XAi,
            ProviderId::OpenRouter,
            ProviderId::Perplexity,
        ];
        for id in simple {
            let m = metadata_for(id);
            assert!(
                !m.supports_models,
                "{id:?} simple-storage must not claim model discovery"
            );
            assert!(
                !m.supports_balance,
                "{id:?} simple-storage must not claim balance"
            );
            assert!(!m.display_name.is_empty());
            assert!(!m.docs_url.is_empty());
            assert!(!m.key_format_hint.is_empty());
        }
    }
}
