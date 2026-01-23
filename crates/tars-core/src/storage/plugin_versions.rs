//! Plugin version tracking storage
//!
//! Tracks when plugin versions actually changed (not just when checked).

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::db::DatabaseError;

/// Tracked plugin version info
#[derive(Debug, Clone)]
pub struct PluginVersionInfo {
    /// Plugin key (e.g., "plugin-name@marketplace")
    pub plugin_key: String,
    /// Current tracked version
    pub version: String,
    /// When the version actually changed
    pub version_changed_at: DateTime<Utc>,
    /// When we last checked the version
    pub last_checked_at: DateTime<Utc>,
}

/// Parse a datetime string, falling back to now if invalid
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc))
}

/// Plugin version tracking store
pub struct PluginVersionStore<'a> {
    conn: &'a Connection,
}

impl<'a> PluginVersionStore<'a> {
    /// Create a new plugin version store
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get tracked version info for a plugin
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub fn get(&self, plugin_key: &str) -> Result<Option<PluginVersionInfo>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT plugin_key, version, version_changed_at, last_checked_at
             FROM plugin_versions WHERE plugin_key = ?",
        )?;

        let result = stmt
            .query_row(params![plugin_key], |row| {
                let version_changed_at: String = row.get(2)?;
                let last_checked_at: String = row.get(3)?;
                Ok(PluginVersionInfo {
                    plugin_key: row.get(0)?,
                    version: row.get(1)?,
                    version_changed_at: parse_datetime(&version_changed_at),
                    last_checked_at: parse_datetime(&last_checked_at),
                })
            })
            .optional()?;

        Ok(result)
    }

    /// Update or insert version tracking for a plugin
    /// Returns the `version_changed_at` timestamp (which only updates if version changed)
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub fn track_version(
        &self,
        plugin_key: &str,
        current_version: &str,
    ) -> Result<DateTime<Utc>, DatabaseError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Check if we have existing tracking for this plugin
        if let Some(existing) = self.get(plugin_key)? {
            if existing.version == current_version {
                // Version hasn't changed, just update last_checked_at
                self.conn.execute(
                    "UPDATE plugin_versions SET last_checked_at = ? WHERE plugin_key = ?",
                    params![now_str, plugin_key],
                )?;
                Ok(existing.version_changed_at)
            } else {
                // Version changed! Update both timestamps and version
                self.conn.execute(
                    "UPDATE plugin_versions SET version = ?, version_changed_at = ?, last_checked_at = ? WHERE plugin_key = ?",
                    params![current_version, now_str, now_str, plugin_key],
                )?;
                Ok(now)
            }
        } else {
            // New plugin, insert tracking
            self.conn.execute(
                "INSERT INTO plugin_versions (plugin_key, version, version_changed_at, last_checked_at) VALUES (?, ?, ?, ?)",
                params![plugin_key, current_version, now_str, now_str],
            )?;
            Ok(now)
        }
    }

    /// Get all tracked plugin versions
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub fn list_all(&self) -> Result<Vec<PluginVersionInfo>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT plugin_key, version, version_changed_at, last_checked_at FROM plugin_versions",
        )?;

        let rows = stmt.query_map([], |row| {
            let version_changed_at: String = row.get(2)?;
            let last_checked_at: String = row.get(3)?;
            Ok(PluginVersionInfo {
                plugin_key: row.get(0)?,
                version: row.get(1)?,
                version_changed_at: parse_datetime(&version_changed_at),
                last_checked_at: parse_datetime(&last_checked_at),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    /// Delete tracking for a plugin
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub fn delete(&self, plugin_key: &str) -> Result<bool, DatabaseError> {
        let count = self.conn.execute(
            "DELETE FROM plugin_versions WHERE plugin_key = ?",
            params![plugin_key],
        )?;
        Ok(count > 0)
    }
}
