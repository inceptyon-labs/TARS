//! Pricing cache writer and metadata reader.
//!
//! Writes parsed `LiteLLM` prices into the existing `provider_models` table
//! without disturbing the user-set `price_override_json` column. Tracks
//! refresh state in the `pricing_metadata` table created by migration v6.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::storage::db::DatabaseError;

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

#[derive(Debug, Deserialize)]
struct PriceOverride {
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
}

/// Update fetched prices for the supplied rows.
///
/// Only `input_price` and `output_price` are written; rows missing from
/// `provider_models` are skipped (model isn't currently cached for that
/// provider — likely not yet refreshed via the model-list discovery flow).
/// `price_override_json` is never touched, so user overrides survive.
///
/// Runs in a single transaction so a partial failure leaves prior values
/// intact. Returns the count of rows actually updated.
///
/// # Errors
/// Returns the underlying `DatabaseError` on transaction or statement failure.
pub fn update_prices(conn: &Connection, rows: &[PriceUpdateRow]) -> Result<usize, DatabaseError> {
    let tx = conn.unchecked_transaction()?;
    let mut updated = 0_usize;
    for row in rows {
        let n = tx.execute(
            "UPDATE provider_models
             SET input_price = ?1, output_price = ?2
             WHERE provider_id = ?3 AND model_id = ?4",
            params![
                row.input_price,
                row.output_price,
                row.provider_id,
                row.model_id
            ],
        )?;
        updated += n;
    }
    tx.commit()?;
    Ok(updated)
}

/// Fetch a single metadata row by key.
///
/// # Errors
/// Returns the underlying `DatabaseError` on query failure.
pub fn get_metadata(
    conn: &Connection,
    key: &str,
) -> Result<Option<PricingMetadata>, DatabaseError> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT key, value, updated_at FROM pricing_metadata WHERE key = ?1",
            params![key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;

    row.map(|(k, v, at)| {
        let updated_at = DateTime::parse_from_rfc3339(&at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::Migration(format!("Bad pricing_metadata.updated_at: {e}"))
            })?;
        Ok(PricingMetadata {
            key: k,
            value: v,
            updated_at,
        })
    })
    .transpose()
}

/// Upsert a metadata row.
///
/// # Errors
/// Returns the underlying `DatabaseError` on statement failure.
pub fn set_metadata(
    conn: &Connection,
    key: &str,
    value: &str,
    at: DateTime<Utc>,
) -> Result<(), DatabaseError> {
    let stamp = at.to_rfc3339();
    conn.execute(
        "INSERT INTO pricing_metadata (key, value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
        params![key, value, stamp],
    )?;
    Ok(())
}

/// Resolve the price the UI should display for one cached row.
///
/// When `override_json` is `Some` and parses to a `{"input":?, "output":?}`
/// shape, those values win — partial overrides (only one side set) fall back
/// to the fetched value for the other side. When parsing fails or both
/// override values are missing, the fetched values pass through unchanged.
pub fn effective_price_for(
    input_price: Option<f64>,
    output_price: Option<f64>,
    override_json: Option<&str>,
) -> EffectivePrice {
    let override_parsed = override_json.and_then(|s| serde_json::from_str::<PriceOverride>(s).ok());
    match override_parsed {
        Some(o) if o.input.is_some() || o.output.is_some() => EffectivePrice {
            input: o.input.or(input_price),
            output: o.output.or(output_price),
            is_overridden: true,
        },
        _ => EffectivePrice {
            input: input_price,
            output: output_price,
            is_overridden: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use rusqlite::params;

    fn seed_model(conn: &Connection, provider: &str, model: &str) {
        conn.execute(
            "INSERT INTO provider_models (provider_id, model_id, fetched_at)
             VALUES (?1, ?2, ?3)",
            params![provider, model, "2026-04-17T00:00:00Z"],
        )
        .unwrap();
    }

    fn read_prices(conn: &Connection, provider: &str, model: &str) -> (Option<f64>, Option<f64>) {
        conn.query_row(
            "SELECT input_price, output_price FROM provider_models
             WHERE provider_id = ?1 AND model_id = ?2",
            params![provider, model],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    #[test]
    fn update_prices_writes_to_existing_rows() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        seed_model(conn, "openai", "gpt-4o");

        let updated = update_prices(
            conn,
            &[PriceUpdateRow {
                provider_id: "openai".into(),
                model_id: "gpt-4o".into(),
                input_price: 2.5,
                output_price: 10.0,
            }],
        )
        .unwrap();
        assert_eq!(updated, 1);

        let (i, o) = read_prices(conn, "openai", "gpt-4o");
        assert_eq!(i, Some(2.5));
        assert_eq!(o, Some(10.0));
    }

    #[test]
    fn update_prices_skips_unknown_models() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        seed_model(conn, "openai", "gpt-4o");

        let updated = update_prices(
            conn,
            &[
                PriceUpdateRow {
                    provider_id: "openai".into(),
                    model_id: "gpt-4o".into(),
                    input_price: 2.5,
                    output_price: 10.0,
                },
                PriceUpdateRow {
                    provider_id: "openai".into(),
                    model_id: "not-cached".into(),
                    input_price: 1.0,
                    output_price: 2.0,
                },
            ],
        )
        .unwrap();
        // Only one row matched.
        assert_eq!(updated, 1);
    }

    #[test]
    fn update_prices_preserves_override_json() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        seed_model(conn, "openai", "gpt-4o");
        let override_json = r#"{"input":99.0,"output":42.0}"#;
        conn.execute(
            "UPDATE provider_models SET price_override_json = ?1
             WHERE provider_id = 'openai' AND model_id = 'gpt-4o'",
            params![override_json],
        )
        .unwrap();

        update_prices(
            conn,
            &[PriceUpdateRow {
                provider_id: "openai".into(),
                model_id: "gpt-4o".into(),
                input_price: 2.5,
                output_price: 10.0,
            }],
        )
        .unwrap();

        let saved: String = conn
            .query_row(
                "SELECT price_override_json FROM provider_models
                 WHERE provider_id = 'openai' AND model_id = 'gpt-4o'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(saved, override_json);
    }

    #[test]
    fn metadata_set_and_get_roundtrip() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        let at: DateTime<Utc> = "2026-04-17T12:34:56Z".parse::<DateTime<Utc>>().unwrap();
        set_metadata(conn, METADATA_KEY_LAST_REFRESH, "2026-04-17T12:00:00Z", at).unwrap();

        let row = get_metadata(conn, METADATA_KEY_LAST_REFRESH)
            .unwrap()
            .unwrap();
        assert_eq!(row.key, METADATA_KEY_LAST_REFRESH);
        assert_eq!(row.value, "2026-04-17T12:00:00Z");
        assert_eq!(row.updated_at.timestamp(), at.timestamp());
    }

    #[test]
    fn metadata_get_missing_returns_none() {
        let db = Database::in_memory().unwrap();
        assert!(get_metadata(db.connection(), "nope").unwrap().is_none());
    }

    #[test]
    fn metadata_set_upserts_on_conflict() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();
        let t1 = "2026-04-17T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t2 = "2026-04-17T13:00:00Z".parse::<DateTime<Utc>>().unwrap();
        set_metadata(conn, METADATA_KEY_LAST_ERROR, "fetch failed", t1).unwrap();
        set_metadata(conn, METADATA_KEY_LAST_ERROR, "different error", t2).unwrap();

        let row = get_metadata(conn, METADATA_KEY_LAST_ERROR)
            .unwrap()
            .unwrap();
        assert_eq!(row.value, "different error");
        assert_eq!(row.updated_at.timestamp(), t2.timestamp());
    }

    #[test]
    fn effective_price_returns_fetched_when_no_override() {
        let r = effective_price_for(Some(2.5), Some(10.0), None);
        assert_eq!(r.input, Some(2.5));
        assert_eq!(r.output, Some(10.0));
        assert!(!r.is_overridden);
    }

    #[test]
    fn effective_price_full_override_wins() {
        let r = effective_price_for(Some(2.5), Some(10.0), Some(r#"{"input":1.0,"output":3.0}"#));
        assert_eq!(r.input, Some(1.0));
        assert_eq!(r.output, Some(3.0));
        assert!(r.is_overridden);
    }

    #[test]
    fn effective_price_partial_override_falls_back_to_fetched() {
        let r = effective_price_for(Some(2.5), Some(10.0), Some(r#"{"input":1.0}"#));
        assert_eq!(r.input, Some(1.0));
        assert_eq!(r.output, Some(10.0));
        assert!(r.is_overridden);
    }

    #[test]
    fn effective_price_invalid_json_falls_back() {
        let r = effective_price_for(Some(2.5), Some(10.0), Some("not json"));
        assert_eq!(r.input, Some(2.5));
        assert_eq!(r.output, Some(10.0));
        assert!(!r.is_overridden);
    }

    #[test]
    fn effective_price_empty_override_object_is_not_overridden() {
        let r = effective_price_for(Some(2.5), Some(10.0), Some("{}"));
        assert_eq!(r.input, Some(2.5));
        assert_eq!(r.output, Some(10.0));
        assert!(!r.is_overridden);
    }
}
