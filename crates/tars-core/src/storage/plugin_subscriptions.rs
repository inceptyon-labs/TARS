//! Managed plugin subscription storage.
//!
//! Tracks the user's intent for cross-runtime plugin installation so TARS can
//! reconcile Claude Code and Codex configuration from one source of truth.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::db::DatabaseError;

/// Persisted managed plugin subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSubscription {
    pub id: i64,
    pub plugin_name: String,
    pub source: String,
    pub source_kind: String,
    pub marketplace_source: Option<String>,
    pub marketplace_name: Option<String>,
    pub codex_source: Option<String>,
    pub scope: String,
    pub targets: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating or updating a managed plugin subscription.
#[derive(Debug, Clone)]
pub struct PluginSubscriptionInput {
    pub plugin_name: String,
    pub source: String,
    pub source_kind: String,
    pub marketplace_source: Option<String>,
    pub marketplace_name: Option<String>,
    pub codex_source: Option<String>,
    pub scope: String,
    pub targets: Vec<String>,
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, DatabaseError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DatabaseError::Migration(format!("Bad plugin subscription timestamp: {e}")))
}

fn row_to_subscription(row: &rusqlite::Row<'_>) -> Result<PluginSubscription, rusqlite::Error> {
    let targets_json: String = row.get(8)?;
    let targets: Vec<String> = serde_json::from_str(&targets_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let created_at: String = row.get(9)?;
    let updated_at: String = row.get(10)?;

    Ok(PluginSubscription {
        id: row.get(0)?,
        plugin_name: row.get(1)?,
        source: row.get(2)?,
        source_kind: row.get(3)?,
        marketplace_source: row.get(4)?,
        marketplace_name: row.get(5)?,
        codex_source: row.get(6)?,
        scope: row.get(7)?,
        targets,
        created_at: parse_datetime(&created_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
        updated_at: parse_datetime(&updated_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
    })
}

/// Store for managed plugin subscriptions.
pub struct PluginSubscriptionStore<'a> {
    conn: &'a Connection,
}

impl<'a> PluginSubscriptionStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list(&self) -> Result<Vec<PluginSubscription>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, plugin_name, source, source_kind, marketplace_source, marketplace_name,
                   codex_source, scope, targets_json, created_at, updated_at
            FROM plugin_subscriptions
            ORDER BY updated_at DESC, id DESC
            ",
        )?;

        let rows = stmt.query_map([], row_to_subscription)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    pub fn get(&self, id: i64) -> Result<Option<PluginSubscription>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, plugin_name, source, source_kind, marketplace_source, marketplace_name,
                   codex_source, scope, targets_json, created_at, updated_at
            FROM plugin_subscriptions
            WHERE id = ?1
            ",
        )?;

        stmt.query_row(params![id], row_to_subscription)
            .optional()
            .map_err(DatabaseError::from)
    }

    pub fn upsert(
        &self,
        input: &PluginSubscriptionInput,
    ) -> Result<PluginSubscription, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let targets_json = serde_json::to_string(&input.targets).map_err(|e| {
            DatabaseError::Migration(format!("Failed to serialize plugin targets: {e}"))
        })?;

        self.conn.execute(
            r"
            INSERT INTO plugin_subscriptions (
                plugin_name, source, source_kind, marketplace_source, marketplace_name,
                codex_source, scope, targets_json, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
            ON CONFLICT(source, scope) DO UPDATE SET
                plugin_name = excluded.plugin_name,
                source_kind = excluded.source_kind,
                marketplace_source = excluded.marketplace_source,
                marketplace_name = excluded.marketplace_name,
                codex_source = excluded.codex_source,
                targets_json = excluded.targets_json,
                updated_at = excluded.updated_at
            ",
            params![
                input.plugin_name,
                input.source,
                input.source_kind,
                input.marketplace_source,
                input.marketplace_name,
                input.codex_source,
                input.scope,
                targets_json,
                now
            ],
        )?;

        let mut stmt = self.conn.prepare(
            r"
            SELECT id, plugin_name, source, source_kind, marketplace_source, marketplace_name,
                   codex_source, scope, targets_json, created_at, updated_at
            FROM plugin_subscriptions
            WHERE source = ?1 AND scope = ?2
            ",
        )?;

        stmt.query_row(params![input.source, input.scope], row_to_subscription)
            .map_err(DatabaseError::from)
    }

    pub fn delete(&self, id: i64) -> Result<bool, DatabaseError> {
        let count = self.conn.execute(
            "DELETE FROM plugin_subscriptions WHERE id = ?1",
            params![id],
        )?;
        Ok(count > 0)
    }
}
