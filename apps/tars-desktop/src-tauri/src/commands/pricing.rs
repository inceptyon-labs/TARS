//! Pricing commands.
//!
//! Wraps `tars_core::pricing` with HTTP fetch and Tauri IPC. The actual
//! parser and DB writer live in `tars-core` so this module stays thin.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::time::Duration;
use tars_core::pricing::{
    delete_metadata, get_metadata, parse_litellm_prices, set_metadata, update_prices, ParsedPrice,
    PriceUpdateRow, LITELLM_PRICES_URL, METADATA_KEY_LAST_ERROR, METADATA_KEY_LAST_REFRESH,
};
use tauri::State;

use crate::state::AppState;

const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Pricing metadata returned to the UI.
#[derive(Debug, Clone, Serialize)]
#[allow(clippy::struct_field_names)]
pub struct PricingMetadataResponse {
    pub last_refresh_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
}

/// Fetch the `LiteLLM` pricing manifest and write parsed prices into the
/// `provider_models` cache. Updates `pricing_metadata` with success or error
/// details so the UI can show staleness.
///
/// Returns the number of rows actually updated. Zero is a valid result when
/// no provider models have been fetched yet — the next model-list refresh
/// will populate the rows, and the cached `LiteLLM` data is reapplied on the
/// next pricing refresh.
#[tauri::command]
pub async fn refresh_pricing(state: State<'_, AppState>) -> Result<usize, String> {
    let client = build_client()?;
    let now = Utc::now();
    match fetch_and_apply(&client, &state, now).await {
        Ok(count) => Ok(count),
        Err(e) => {
            // Best-effort error recording — if even the metadata write fails
            // we just bubble the original error.
            let _ = state.with_db(|db| {
                set_metadata(db.connection(), METADATA_KEY_LAST_ERROR, &e, now)
                    .map_err(|err| err.to_string())
            });
            Err(e)
        }
    }
}

/// Read the latest pricing metadata for display. Returns `None` fields when
/// the corresponding metadata row does not exist.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_pricing_metadata(state: State<'_, AppState>) -> Result<PricingMetadataResponse, String> {
    state.with_db(|db| {
        let last = get_metadata(db.connection(), METADATA_KEY_LAST_REFRESH)
            .map_err(|e| format!("Failed to read pricing metadata: {e}"))?;
        let err = get_metadata(db.connection(), METADATA_KEY_LAST_ERROR)
            .map_err(|e| format!("Failed to read pricing metadata: {e}"))?;
        Ok(PricingMetadataResponse {
            last_refresh_at: last.map(|m| m.value),
            last_error: err.as_ref().map(|m| m.value.clone()),
            last_error_at: err.map(|m| m.updated_at.to_rfc3339()),
        })
    })
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .user_agent(format!("tars/{}", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))
}

async fn fetch_and_apply(
    client: &reqwest::Client,
    state: &AppState,
    now: DateTime<Utc>,
) -> Result<usize, String> {
    let raw = fetch_litellm_body(client, LITELLM_PRICES_URL).await?;
    let parsed = parse_litellm_prices(&raw).map_err(|e| format!("LiteLLM parse failed: {e}"))?;
    let rows: Vec<PriceUpdateRow> = parsed.into_iter().map(parsed_to_update).collect();

    let written = state.with_db(|db| {
        update_prices(db.connection(), &rows).map_err(|e| format!("DB write failed: {e}"))
    })?;

    // Only mark a successful refresh when at least one model row was priced.
    // If provider_models is empty (no keys added yet), skip recording so the
    // background loop retries on the next launch instead of sleeping 7 days.
    if written > 0 {
        state.with_db(|db| {
            set_metadata(
                db.connection(),
                METADATA_KEY_LAST_REFRESH,
                &now.to_rfc3339(),
                now,
            )
            .map_err(|e| format!("Failed to record pricing refresh: {e}"))
        })?;
    }

    // Clear any previous error — a successful price write supersedes it.
    let _ = state.with_db(|db| {
        delete_metadata(db.connection(), METADATA_KEY_LAST_ERROR).map_err(|e| e.to_string())
    });

    Ok(written)
}

async fn fetch_litellm_body(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("LiteLLM fetch failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("LiteLLM fetch returned HTTP {}", response.status()));
    }
    response
        .text()
        .await
        .map_err(|e| format!("LiteLLM body read failed: {e}"))
}

fn parsed_to_update(p: ParsedPrice) -> PriceUpdateRow {
    PriceUpdateRow {
        provider_id: p.provider_id,
        model_id: p.model_id,
        input_price: p.input_price,
        output_price: p.output_price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_to_update_passes_fields_through() {
        let p = ParsedPrice {
            provider_id: "openai".into(),
            model_id: "gpt-4o".into(),
            input_price: 2.5,
            output_price: 10.0,
        };
        let r = parsed_to_update(p);
        assert_eq!(r.provider_id, "openai");
        assert_eq!(r.model_id, "gpt-4o");
        assert!((r.input_price - 2.5).abs() < 1e-9);
        assert!((r.output_price - 10.0).abs() < 1e-9);
    }

    #[test]
    fn build_client_succeeds() {
        // Mostly a smoke test that the reqwest builder options are compatible.
        assert!(build_client().is_ok());
    }
}
