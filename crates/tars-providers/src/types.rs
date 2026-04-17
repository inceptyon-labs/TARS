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
}

impl ProviderId {
    /// All provider IDs supported in this phase
    pub const ALL: &'static [ProviderId] = &[
        ProviderId::OpenAi,
        ProviderId::Anthropic,
        ProviderId::Gemini,
        ProviderId::Deepseek,
    ];

    /// Stable string form used in DB rows and IPC payloads
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ProviderId::OpenAi => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Gemini => "gemini",
            ProviderId::Deepseek => "deepseek",
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
    fn provider_id_all_has_four() {
        assert_eq!(ProviderId::ALL.len(), 4);
    }

    #[test]
    fn provider_id_serialization_stable() {
        let json = serde_json::to_string(&ProviderId::OpenAi).unwrap();
        assert_eq!(json, "\"openai\"");
        let json = serde_json::to_string(&ProviderId::Deepseek).unwrap();
        assert_eq!(json, "\"deepseek\"");
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
