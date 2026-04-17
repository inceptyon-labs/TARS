//! Cache of provider-reported model catalogs.
//!
//! Backs the `provider_models` table created in migration v5. Reads are served
//! directly from `SQLite`; writes atomically replace the cached rows for a given
//! provider. The TTL check (`is_stale`) is the sole staleness authority — 24h
//! is the policy, but the duration is passed in by the caller so tests can
//! pin it.

use super::db::DatabaseError;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};

/// Input row for cache upserts (kept local so `tars-core` does not take a
/// dependency on `tars-providers`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRow {
    pub model_id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
}

/// A model record as stored in the cache.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedModel {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub fetched_at: DateTime<Utc>,
}

pub struct ModelCache<'a> {
    conn: &'a Connection,
}

impl<'a> ModelCache<'a> {
    #[must_use]
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Atomically replace cached rows for a provider with `models`, preserving
    /// any user-set `price_override_json` values across the refresh.
    ///
    /// The transaction:
    /// 1. Deletes provider rows whose `model_id` is **not** in the new set
    ///    (these models are no longer offered upstream).
    /// 2. Upserts the new set via `ON CONFLICT DO UPDATE`, leaving
    ///    `price_override_json` untouched for rows that already exist.
    ///
    /// All rows are stamped with the supplied `fetched_at`. The whole sequence
    /// runs in a single transaction so a partial failure leaves the prior
    /// cache intact.
    ///
    /// # Errors
    /// Returns an error if the transaction or any statement fails.
    pub fn upsert_all(
        &self,
        provider_id: &str,
        models: &[ModelRow],
        fetched_at: DateTime<Utc>,
    ) -> Result<usize, DatabaseError> {
        let tx = self.conn.unchecked_transaction()?;

        // Remove rows no longer offered by the provider. Only model_ids not
        // present in `models` are dropped — surviving rows keep their
        // `price_override_json`.
        let keep: Vec<&str> = models.iter().map(|m| m.model_id.as_str()).collect();
        if keep.is_empty() {
            tx.execute(
                "DELETE FROM provider_models WHERE provider_id = ?1",
                params![provider_id],
            )?;
        } else {
            let placeholders = (0..keep.len())
                .map(|i| format!("?{}", i + 2))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "DELETE FROM provider_models
                 WHERE provider_id = ?1 AND model_id NOT IN ({placeholders})"
            );
            let mut stmt = tx.prepare(&sql)?;
            let mut args: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(keep.len() + 1);
            args.push(&provider_id);
            for id in &keep {
                args.push(id);
            }
            stmt.execute(rusqlite::params_from_iter(args.iter().copied()))?;
        }

        let stamp = fetched_at.to_rfc3339();
        let mut inserted = 0_usize;
        for row in models {
            tx.execute(
                r"
                INSERT INTO provider_models
                    (provider_id, model_id, display_name, context_window,
                     input_price, output_price, price_override_json, fetched_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)
                ON CONFLICT(provider_id, model_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    context_window = excluded.context_window,
                    input_price = excluded.input_price,
                    output_price = excluded.output_price,
                    fetched_at = excluded.fetched_at
                ",
                params![
                    provider_id,
                    row.model_id,
                    row.display_name,
                    row.context_window,
                    row.input_price,
                    row.output_price,
                    stamp,
                ],
            )?;
            inserted += 1;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// List cached rows for a provider, ordered by `model_id`.
    ///
    /// # Errors
    /// Returns an error if the query or row parsing fails.
    pub fn list_for_provider(&self, provider_id: &str) -> Result<Vec<CachedModel>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT provider_id, model_id, display_name, context_window,
                   input_price, output_price, fetched_at
            FROM provider_models
            WHERE provider_id = ?1
            ORDER BY model_id
            ",
        )?;
        let rows = stmt.query_map(params![provider_id], |row| {
            let fetched_at_raw: String = row.get(6)?;
            let fetched_at = DateTime::parse_from_rfc3339(&fetched_at_raw)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&Utc);
            let context_window: Option<i64> = row.get(3)?;
            Ok(CachedModel {
                provider_id: row.get(0)?,
                model_id: row.get(1)?,
                display_name: row.get(2)?,
                context_window: context_window.and_then(|v| u32::try_from(v).ok()),
                input_price: row.get(4)?,
                output_price: row.get(5)?,
                fetched_at,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Oldest `fetched_at` among rows for `provider_id`, or `None` if empty.
    ///
    /// # Errors
    /// Returns an error if the query fails or a timestamp is malformed.
    pub fn oldest_fetched_at(
        &self,
        provider_id: &str,
    ) -> Result<Option<DateTime<Utc>>, DatabaseError> {
        let row: Option<String> = self
            .conn
            .query_row(
                "SELECT MIN(fetched_at) FROM provider_models WHERE provider_id = ?1",
                params![provider_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        row.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| DatabaseError::Migration(format!("Bad fetched_at: {e}")))
        })
        .transpose()
    }

    /// Return `true` if there are no cached rows for `provider_id`, or if the
    /// oldest row is older than `ttl` relative to `now`.
    ///
    /// # Errors
    /// Returns an error if the underlying query fails.
    pub fn is_stale(
        &self,
        provider_id: &str,
        ttl: Duration,
        now: DateTime<Utc>,
    ) -> Result<bool, DatabaseError> {
        match self.oldest_fetched_at(provider_id)? {
            None => Ok(true),
            Some(oldest) => Ok(now.signed_duration_since(oldest) > ttl),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    fn sample_rows() -> Vec<ModelRow> {
        vec![
            ModelRow {
                model_id: "gpt-4o".into(),
                display_name: Some("GPT-4o".into()),
                context_window: Some(128_000),
                input_price: None,
                output_price: None,
            },
            ModelRow {
                model_id: "gpt-3.5-turbo".into(),
                display_name: None,
                context_window: None,
                input_price: Some(0.5),
                output_price: Some(1.5),
            },
        ]
    }

    #[test]
    fn is_stale_true_when_empty() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();
        assert!(cache.is_stale("openai", Duration::hours(24), now).unwrap());
    }

    #[test]
    fn upsert_and_list_roundtrip() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();

        let inserted = cache.upsert_all("openai", &sample_rows(), now).unwrap();
        assert_eq!(inserted, 2);

        let listed = cache.list_for_provider("openai").unwrap();
        assert_eq!(listed.len(), 2);
        // Ordered by model_id ascending
        assert_eq!(listed[0].model_id, "gpt-3.5-turbo");
        assert_eq!(listed[1].model_id, "gpt-4o");
        assert_eq!(listed[1].context_window, Some(128_000));
        assert_eq!(listed[0].input_price, Some(0.5));
    }

    #[test]
    fn upsert_replaces_prior_rows_for_provider() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();

        cache.upsert_all("openai", &sample_rows(), now).unwrap();

        let replacement = vec![ModelRow {
            model_id: "gpt-4o-mini".into(),
            display_name: None,
            context_window: None,
            input_price: None,
            output_price: None,
        }];
        cache.upsert_all("openai", &replacement, now).unwrap();

        let listed = cache.list_for_provider("openai").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].model_id, "gpt-4o-mini");
    }

    #[test]
    fn upsert_is_scoped_per_provider() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();

        cache.upsert_all("openai", &sample_rows(), now).unwrap();
        cache
            .upsert_all(
                "anthropic",
                &[ModelRow {
                    model_id: "claude-sonnet".into(),
                    display_name: None,
                    context_window: None,
                    input_price: None,
                    output_price: None,
                }],
                now,
            )
            .unwrap();

        // Refreshing openai must not wipe anthropic
        cache.upsert_all("openai", &[], now).unwrap();
        assert!(cache.list_for_provider("openai").unwrap().is_empty());
        let anthropic = cache.list_for_provider("anthropic").unwrap();
        assert_eq!(anthropic.len(), 1);
    }

    #[test]
    fn is_stale_respects_ttl_boundary() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();
        cache.upsert_all("openai", &sample_rows(), now).unwrap();

        // Fresh: now == fetched_at
        assert!(!cache.is_stale("openai", Duration::hours(24), now).unwrap());

        // 23h59m later: still fresh
        let almost = now + Duration::hours(23) + Duration::minutes(59);
        assert!(!cache
            .is_stale("openai", Duration::hours(24), almost)
            .unwrap());

        // 24h01m later: stale
        let past = now + Duration::hours(24) + Duration::minutes(1);
        assert!(cache.is_stale("openai", Duration::hours(24), past).unwrap());
    }

    #[test]
    fn oldest_fetched_at_uses_minimum() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());

        let early = Utc::now() - Duration::hours(10);
        let late = Utc::now();

        cache
            .upsert_all(
                "openai",
                &[ModelRow {
                    model_id: "a".into(),
                    display_name: None,
                    context_window: None,
                    input_price: None,
                    output_price: None,
                }],
                early,
            )
            .unwrap();

        // Insert a second row for the same provider with a later stamp using
        // raw SQL, to verify MIN() is applied.
        db.connection()
            .execute(
                "INSERT INTO provider_models (provider_id, model_id, fetched_at)
                 VALUES (?1, ?2, ?3)",
                params!["openai", "b", late.to_rfc3339()],
            )
            .unwrap();

        let oldest = cache.oldest_fetched_at("openai").unwrap().unwrap();
        assert_eq!(oldest.timestamp(), early.timestamp());
    }

    #[test]
    fn upsert_preserves_price_override_json() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();

        cache.upsert_all("openai", &sample_rows(), now).unwrap();

        // Simulate a user-set price override on the gpt-4o row.
        let override_json = r#"{"input":0.001,"output":0.002}"#;
        db.connection()
            .execute(
                "UPDATE provider_models
                 SET price_override_json = ?1
                 WHERE provider_id = 'openai' AND model_id = 'gpt-4o'",
                params![override_json],
            )
            .unwrap();

        // Refresh with the same model list; override must be preserved.
        let later = now + Duration::hours(1);
        cache.upsert_all("openai", &sample_rows(), later).unwrap();

        let saved: String = db
            .connection()
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
    fn upsert_drops_models_no_longer_offered() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        let now = Utc::now();

        cache.upsert_all("openai", &sample_rows(), now).unwrap();

        // Refresh with a smaller set — gpt-3.5-turbo must be dropped.
        let reduced = vec![ModelRow {
            model_id: "gpt-4o".into(),
            display_name: Some("GPT-4o".into()),
            context_window: Some(128_000),
            input_price: None,
            output_price: None,
        }];
        cache.upsert_all("openai", &reduced, now).unwrap();

        let listed = cache.list_for_provider("openai").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].model_id, "gpt-4o");
    }

    #[test]
    fn list_for_unknown_provider_is_empty() {
        let db = Database::in_memory().unwrap();
        let cache = ModelCache::new(db.connection());
        assert!(cache.list_for_provider("mystery").unwrap().is_empty());
    }
}
