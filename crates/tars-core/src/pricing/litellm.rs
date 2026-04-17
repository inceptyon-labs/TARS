//! `LiteLLM` `model_prices_and_context_window.json` parser.
//!
//! Source: <https://github.com/BerriAI/litellm>. The published JSON is a flat
//! object keyed by model name, e.g. `gpt-4o`. Each value carries pricing as
//! `input_cost_per_token` / `output_cost_per_token` (USD per token) plus a
//! `litellm_provider` discriminator. We translate that discriminator into our
//! `provider_id` strings (matching `tars_providers::ProviderId::as_str`) and
//! convert per-token prices to per-1M-token to match the existing UI columns.
//!
//! The parser is defensive: malformed entries, unknown providers, and the
//! `sample_spec` placeholder key are all silently skipped instead of failing
//! the whole import.

use std::collections::HashMap;

use serde::Deserialize;

/// Public location of `LiteLLM`'s pricing manifest. Used by the application
/// layer when issuing the HTTP fetch.
pub const LITELLM_PRICES_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

/// One parsed price entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPrice {
    pub provider_id: String,
    pub model_id: String,
    /// USD per 1M input tokens.
    pub input_price: f64,
    /// USD per 1M output tokens.
    pub output_price: f64,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(default)]
    litellm_provider: Option<String>,
    #[serde(default)]
    input_cost_per_token: Option<f64>,
    #[serde(default)]
    output_cost_per_token: Option<f64>,
}

/// Map a `LiteLLM` `litellm_provider` value to our internal provider id.
///
/// Returns `None` for providers we do not surface in the API-Keys vault, so
/// their pricing entries are dropped from the result. Mappings cover only
/// providers TARS currently integrates with for model discovery (issue #5).
fn map_provider(litellm_provider: &str) -> Option<&'static str> {
    match litellm_provider {
        "openai" => Some("openai"),
        "anthropic" => Some("anthropic"),
        // LiteLLM splits Gemini into "gemini" (AI Studio) and several
        // "vertex_ai-*" variants. We only list AI Studio under our `gemini`
        // provider id.
        "gemini" => Some("gemini"),
        "deepseek" => Some("deepseek"),
        _ => None,
    }
}

/// Strip provider prefixes that `LiteLLM` occasionally embeds in keys.
///
/// `LiteLLM` uses both bare model ids (e.g. `gpt-4o`) and prefixed forms
/// (`openai/gpt-4o`, `anthropic/claude-3-5-sonnet-latest`). The provider
/// model lists fetched by `tars_providers` always store the bare id, so we
/// normalise to that form here.
fn normalise_model_id<'a>(provider_id: &str, raw_key: &'a str) -> &'a str {
    let prefix_with_slash = match provider_id {
        "openai" => "openai/",
        "anthropic" => "anthropic/",
        "gemini" => "gemini/",
        "deepseek" => "deepseek/",
        _ => return raw_key,
    };
    raw_key.strip_prefix(prefix_with_slash).unwrap_or(raw_key)
}

/// Parse a `LiteLLM` JSON document into one [`ParsedPrice`] per supported model.
///
/// Entries are skipped when:
/// - the key is the `sample_spec` placeholder published in upstream JSON,
/// - the entry has no `litellm_provider` field,
/// - the provider is one we do not map (see [`map_provider`]),
/// - either of the cost fields is missing or non-finite.
///
/// Duplicate `(provider_id, model_id)` tuples (which happen when `LiteLLM`
/// publishes both bare and prefixed keys for the same model) collapse into
/// one entry — which of the two wins is unspecified, but both carry identical
/// prices so the choice is immaterial.
///
/// # Errors
/// Returns a `serde_json::Error` if the document is not a JSON object at the
/// top level or fails to decode. Per-entry decoding errors are swallowed so
/// a single bad row never poisons the whole refresh.
pub fn parse_litellm_prices(raw: &str) -> Result<Vec<ParsedPrice>, serde_json::Error> {
    let map: HashMap<String, serde_json::Value> = serde_json::from_str(raw)?;

    let mut out: HashMap<(String, String), ParsedPrice> = HashMap::new();
    for (key, value) in map {
        if key == "sample_spec" {
            continue;
        }
        let Ok(entry) = serde_json::from_value::<RawEntry>(value) else {
            continue;
        };
        let Some(litellm_provider) = entry.litellm_provider.as_deref() else {
            continue;
        };
        let Some(provider_id) = map_provider(litellm_provider) else {
            continue;
        };
        let Some(input_per_token) = entry.input_cost_per_token else {
            continue;
        };
        let Some(output_per_token) = entry.output_cost_per_token else {
            continue;
        };
        if !input_per_token.is_finite() || !output_per_token.is_finite() {
            continue;
        }

        let model_id = normalise_model_id(provider_id, &key).to_string();
        out.insert(
            (provider_id.to_string(), model_id.clone()),
            ParsedPrice {
                provider_id: provider_id.to_string(),
                model_id,
                input_price: input_per_token * 1_000_000.0,
                output_price: output_per_token * 1_000_000.0,
            },
        );
    }

    let mut prices: Vec<ParsedPrice> = out.into_values().collect();
    prices.sort_by(|a, b| {
        a.provider_id
            .cmp(&b.provider_id)
            .then_with(|| a.model_id.cmp(&b.model_id))
    });
    Ok(prices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        r#"{
            "sample_spec": { "input_cost_per_token": 0.0 },
            "gpt-4o": {
                "litellm_provider": "openai",
                "input_cost_per_token": 0.0000025,
                "output_cost_per_token": 0.00001
            },
            "openai/gpt-4o-mini": {
                "litellm_provider": "openai",
                "input_cost_per_token": 0.00000015,
                "output_cost_per_token": 0.0000006
            },
            "claude-3-5-sonnet-latest": {
                "litellm_provider": "anthropic",
                "input_cost_per_token": 0.000003,
                "output_cost_per_token": 0.000015
            },
            "gemini-1.5-pro": {
                "litellm_provider": "gemini",
                "input_cost_per_token": 0.00000125,
                "output_cost_per_token": 0.000005
            },
            "deepseek-chat": {
                "litellm_provider": "deepseek",
                "input_cost_per_token": 0.00000027,
                "output_cost_per_token": 0.0000011
            },
            "vertex_ai/gemini-1.5-pro": {
                "litellm_provider": "vertex_ai-gemini",
                "input_cost_per_token": 0.00000125,
                "output_cost_per_token": 0.000005
            },
            "weird-no-provider": {
                "input_cost_per_token": 0.000001,
                "output_cost_per_token": 0.000002
            },
            "weird-no-prices": {
                "litellm_provider": "openai"
            },
            "weird-non-finite": {
                "litellm_provider": "openai",
                "input_cost_per_token": "not-a-number",
                "output_cost_per_token": 0.000002
            }
        }"#
    }

    #[test]
    fn parses_supported_providers() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        let providers: Vec<_> = prices.iter().map(|p| p.provider_id.as_str()).collect();
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"anthropic"));
        assert!(providers.contains(&"gemini"));
        assert!(providers.contains(&"deepseek"));
    }

    #[test]
    fn skips_unmapped_providers() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        // vertex_ai-gemini must be dropped — we only map plain "gemini".
        assert!(prices.iter().all(|p| !p.provider_id.contains("vertex")));
        assert!(prices.iter().all(|p| p.provider_id != "vertex_ai-gemini"));
    }

    #[test]
    fn skips_sample_spec_key() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        assert!(prices.iter().all(|p| p.model_id != "sample_spec"));
    }

    #[test]
    fn skips_entries_missing_provider_or_prices() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        for bad in ["weird-no-provider", "weird-no-prices", "weird-non-finite"] {
            assert!(
                prices.iter().all(|p| p.model_id != bad),
                "{bad} should not parse"
            );
        }
    }

    #[test]
    fn converts_per_token_to_per_million() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        let gpt4o = prices
            .iter()
            .find(|p| p.provider_id == "openai" && p.model_id == "gpt-4o")
            .expect("gpt-4o present");
        // 0.0000025 * 1_000_000 = 2.5
        assert!((gpt4o.input_price - 2.5).abs() < 1e-9);
        // 0.00001  * 1_000_000 = 10.0
        assert!((gpt4o.output_price - 10.0).abs() < 1e-9);
    }

    #[test]
    fn strips_provider_prefix_in_keys() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        // "openai/gpt-4o-mini" must store as bare "gpt-4o-mini".
        assert!(prices
            .iter()
            .any(|p| p.provider_id == "openai" && p.model_id == "gpt-4o-mini"));
        assert!(prices.iter().all(|p| !p.model_id.starts_with("openai/")));
    }

    #[test]
    fn output_is_sorted_for_determinism() {
        let prices = parse_litellm_prices(fixture()).unwrap();
        let pairs: Vec<_> = prices
            .iter()
            .map(|p| (p.provider_id.as_str(), p.model_id.as_str()))
            .collect();
        let mut sorted = pairs.clone();
        sorted.sort_unstable();
        assert_eq!(pairs, sorted);
    }

    #[test]
    fn invalid_top_level_json_errors() {
        assert!(parse_litellm_prices("not json").is_err());
    }
}
