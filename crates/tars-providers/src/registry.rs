//! Static provider metadata registry.
//!
//! Real `Provider` implementations are added in issue #7.

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

/// Get static metadata for a provider
#[must_use]
pub const fn metadata_for(id: ProviderId) -> &'static ProviderMetadata {
    match id {
        ProviderId::OpenAi => &OPENAI,
        ProviderId::Anthropic => &ANTHROPIC,
        ProviderId::Gemini => &GEMINI,
        ProviderId::Deepseek => &DEEPSEEK,
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
    fn all_metadata_returns_four_entries() {
        let all = all_metadata();
        assert_eq!(all.len(), 4);
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
    fn all_providers_support_model_discovery() {
        for m in all_metadata() {
            assert!(m.supports_models, "{:?} should support models", m.id);
        }
    }
}
