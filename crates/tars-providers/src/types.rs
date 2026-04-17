//! Shared types for provider operations

use serde::{Deserialize, Serialize};

/// A model offered by a provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// Price per 1M input tokens (USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price_per_million: Option<f64>,
    /// Price per 1M output tokens (USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price_per_million: Option<f64>,
}

/// Account balance / credit info (providers that expose it)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    pub currency: String,
    pub amount: f64,
    /// Raw provider response preserved for display
    pub raw: serde_json::Value,
}

/// Result of a key validation check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Stable identifier for each supported provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    #[serde(rename = "openai")]
    OpenAi,
    Anthropic,
    Gemini,
    Deepseek,
    #[serde(rename = "brave-search")]
    BraveSearch,
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
    Groq,
    Mistral,
    #[serde(rename = "xai")]
    XAi,
    #[serde(rename = "openrouter")]
    OpenRouter,
    Perplexity,
}

impl ProviderId {
    /// All provider IDs supported in this phase
    pub const ALL: &'static [ProviderId] = &[
        ProviderId::OpenAi,
        ProviderId::Anthropic,
        ProviderId::Gemini,
        ProviderId::Deepseek,
        ProviderId::BraveSearch,
        ProviderId::ElevenLabs,
        ProviderId::Groq,
        ProviderId::Mistral,
        ProviderId::XAi,
        ProviderId::OpenRouter,
        ProviderId::Perplexity,
    ];

    /// Stable string form used in DB rows and IPC payloads
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Gemini => "gemini",
            ProviderId::Deepseek => "deepseek",
            ProviderId::BraveSearch => "brave-search",
            ProviderId::ElevenLabs => "elevenlabs",
            ProviderId::Groq => "groq",
            ProviderId::Mistral => "mistral",
            ProviderId::XAi => "xai",
            ProviderId::OpenRouter => "openrouter",
            ProviderId::Perplexity => "perplexity",
        }
    }

    /// Parse a stable string form back into a `ProviderId`
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "openai" => Some(ProviderId::OpenAi),
            "anthropic" => Some(ProviderId::Anthropic),
            "gemini" => Some(ProviderId::Gemini),
            "deepseek" => Some(ProviderId::Deepseek),
            "brave-search" => Some(ProviderId::BraveSearch),
            "elevenlabs" => Some(ProviderId::ElevenLabs),
            "groq" => Some(ProviderId::Groq),
            "mistral" => Some(ProviderId::Mistral),
            "xai" => Some(ProviderId::XAi),
            "openrouter" => Some(ProviderId::OpenRouter),
            "perplexity" => Some(ProviderId::Perplexity),
            _ => None,
        }
    }
}

/// Static metadata about a provider (display name, docs, capabilities)
#[derive(Debug, Clone, Serialize)]
pub struct ProviderMetadata {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub docs_url: &'static str,
    pub key_format_hint: &'static str,
    pub supports_models: bool,
    pub supports_balance: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_as_str_roundtrip() {
        for &id in ProviderId::ALL {
            let s = id.as_str();
            assert_eq!(ProviderId::parse(s), Some(id), "roundtrip failed for {s}");
        }
    }

    #[test]
    fn provider_id_parse_unknown() {
        assert_eq!(ProviderId::parse("unknown"), None);
        assert_eq!(ProviderId::parse(""), None);
        assert_eq!(ProviderId::parse("OPENAI"), None, "case-sensitive");
    }

    #[test]
    fn provider_id_all_has_eleven() {
        assert_eq!(ProviderId::ALL.len(), 11);
    }

    #[test]
    fn provider_id_serialization_stable() {
        let json = serde_json::to_string(&ProviderId::OpenAi).unwrap();
        assert_eq!(json, "\"openai\"");
        let json = serde_json::to_string(&ProviderId::Deepseek).unwrap();
        assert_eq!(json, "\"deepseek\"");
        let json = serde_json::to_string(&ProviderId::BraveSearch).unwrap();
        assert_eq!(json, "\"brave-search\"");
        let json = serde_json::to_string(&ProviderId::ElevenLabs).unwrap();
        assert_eq!(json, "\"elevenlabs\"");
        let json = serde_json::to_string(&ProviderId::Groq).unwrap();
        assert_eq!(json, "\"groq\"");
        let json = serde_json::to_string(&ProviderId::Mistral).unwrap();
        assert_eq!(json, "\"mistral\"");
        let json = serde_json::to_string(&ProviderId::XAi).unwrap();
        assert_eq!(json, "\"xai\"");
        let json = serde_json::to_string(&ProviderId::OpenRouter).unwrap();
        assert_eq!(json, "\"openrouter\"");
        let json = serde_json::to_string(&ProviderId::Perplexity).unwrap();
        assert_eq!(json, "\"perplexity\"");
    }

    #[test]
    fn simple_storage_providers_roundtrip() {
        let expected = [
            ("brave-search", ProviderId::BraveSearch),
            ("elevenlabs", ProviderId::ElevenLabs),
            ("groq", ProviderId::Groq),
            ("mistral", ProviderId::Mistral),
            ("xai", ProviderId::XAi),
            ("openrouter", ProviderId::OpenRouter),
            ("perplexity", ProviderId::Perplexity),
        ];
        for (s, id) in expected {
            assert_eq!(id.as_str(), s);
            assert_eq!(ProviderId::parse(s), Some(id));
        }
    }

    #[test]
    fn validation_result_omits_none_message() {
        let r = ValidationResult {
            valid: true,
            message: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"valid":true}"#);
    }
}
