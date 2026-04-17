//! Pricing cache writer and metadata reader (stub — populated in Step 3).
//!
//! Writes parsed LiteLLM prices into the existing `provider_models` table
//! without disturbing the user-set `price_override_json` column. Tracks
//! refresh state in the `pricing_metadata` table created by migration v6.

use chrono::{DateTime, Utc};

use crate::storage::db::DatabaseError;
use rusqlite::Connection;

/// Sentinel keys used in the `pricing_metadata` table.
pub const METADATA_KEY_LAST_REFRESH: &str = "last_refresh";
pub const METADATA_KEY_LAST_ERROR: &str = "last_error";

/// One row to upsert into `provider_models` price columns.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceUpdateRow {
    pub provider_id: String,
    pub model_id: String,
    pub input_price: f64,
    pub output_price: f64,
}

/// Effective per-1M-token price displayed to the user.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectivePrice {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub is_overridden: bool,
}

/// One row from `pricing_metadata`.
#[derive(Debug, Clone, PartialEq)]
pub struct PricingMetadata {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// Update fetched prices for the supplied rows. Stub — implemented in Step 3.
///
/// # Errors
/// Returns the underlying `DatabaseError` on transaction or statement failure.
pub fn update_prices(_conn: &Connection, _rows: &[PriceUpdateRow]) -> Result<usize, DatabaseError> {
    unimplemented!("populated in Step 3")
}

/// Fetch a single metadata row by key. Stub — implemented in Step 3.
///
/// # Errors
/// Returns the underlying `DatabaseError` on query failure.
pub fn get_metadata(
    _conn: &Connection,
    _key: &str,
) -> Result<Option<PricingMetadata>, DatabaseError> {
    unimplemented!("populated in Step 3")
}

/// Upsert a metadata row. Stub — implemented in Step 3.
///
/// # Errors
/// Returns the underlying `DatabaseError` on statement failure.
pub fn set_metadata(
    _conn: &Connection,
    _key: &str,
    _value: &str,
    _at: DateTime<Utc>,
) -> Result<(), DatabaseError> {
    unimplemented!("populated in Step 3")
}

/// Resolve the price the UI should display for one cached row, preferring an
/// override JSON blob when present. Stub — implemented in Step 3.
pub fn effective_price_for(
    _input_price: Option<f64>,
    _output_price: Option<f64>,
    _override_json: Option<&str>,
) -> EffectivePrice {
    unimplemented!("populated in Step 3")
}
