//! Global developer account, app target, and release command storage.
//!
//! Developer credentials are reusable across projects and app targets. Secret
//! payloads are AES-256-GCM encrypted with the shared TARS master key stored in
//! the OS keychain.

use super::db::DatabaseError;
use crate::crypto;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Credential list row. Does not include decrypted secret material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperCredentialSummary {
    pub id: i64,
    pub provider: String,
    pub credential_type: String,
    pub label: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// Decrypted credential record.
#[derive(Clone)]
pub struct DeveloperCredential {
    pub id: i64,
    pub provider: String,
    pub credential_type: String,
    pub label: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
}

impl fmt::Debug for DeveloperCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeveloperCredential")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("credential_type", &self.credential_type)
            .field("label", &self.label)
            .field("tags", &self.tags)
            .field("metadata", &self.metadata)
            .field("secret", &"<redacted>")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input for creating/updating a developer credential.
#[derive(Clone, Deserialize)]
pub struct DeveloperCredentialInput {
    pub provider: String,
    pub credential_type: String,
    pub label: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub secret: String,
}

impl fmt::Debug for DeveloperCredentialInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeveloperCredentialInput")
            .field("provider", &self.provider)
            .field("credential_type", &self.credential_type)
            .field("label", &self.label)
            .field("tags", &self.tags)
            .field("metadata", &self.metadata)
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTarget {
    pub id: i64,
    pub name: String,
    pub platform: String,
    pub project_id: Option<String>,
    pub bundle_id: Option<String>,
    pub package_name: Option<String>,
    pub store_app_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTargetInput {
    pub name: String,
    pub platform: String,
    pub project_id: Option<String>,
    pub bundle_id: Option<String>,
    pub package_name: Option<String>,
    pub store_app_id: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTargetCredential {
    pub app_target_id: i64,
    pub credential_id: i64,
    pub role: String,
    pub credential_label: String,
    pub provider: String,
    pub credential_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperCommandPreset {
    pub id: i64,
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub app_target_id: Option<i64>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperCommandInput {
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub app_target_id: Option<i64>,
    pub tags: Vec<String>,
}

pub struct DeveloperStore<'a> {
    conn: &'a Connection,
}

impl<'a> DeveloperStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list_credentials(&self) -> Result<Vec<DeveloperCredentialSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, provider, credential_type, label, tags_json, metadata_json, created_at, updated_at
            FROM developer_credentials
            ORDER BY provider, credential_type, label
            ",
        )?;
        let rows = stmt.query_map([], row_to_credential_summary)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_credential(&self, id: i64) -> Result<Option<DeveloperCredential>, DatabaseError> {
        let row = self
            .conn
            .query_row(
                r"
                SELECT id, provider, credential_type, label, tags_json, metadata_json,
                       encrypted_secret, nonce, created_at, updated_at
                FROM developer_credentials
                WHERE id = ?1
                ",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id,
            provider,
            credential_type,
            label,
            tags_json,
            metadata_json,
            encrypted_secret,
            nonce,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        let secret = crypto::decrypt(&nonce, &encrypted_secret).map_err(|e| {
            DatabaseError::Migration(format!("Failed to decrypt developer credential: {e}"))
        })?;

        Ok(Some(DeveloperCredential {
            id,
            provider,
            credential_type,
            label,
            tags: parse_tags(&tags_json)?,
            metadata: parse_metadata(&metadata_json)?,
            secret,
            created_at,
            updated_at,
        }))
    }

    pub fn save_credential(&self, input: &DeveloperCredentialInput) -> Result<i64, DatabaseError> {
        validate_nonempty("provider", &input.provider)?;
        validate_nonempty("credential type", &input.credential_type)?;
        validate_nonempty("label", &input.label)?;
        validate_nonempty("secret", &input.secret)?;

        let tags_json = serde_json::to_string(&normalize_tags(&input.tags))
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize tags: {e}")))?;
        let metadata_json = serde_json::to_string(&input.metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let (nonce, encrypted) = crypto::encrypt(&input.secret).map_err(|e| {
            DatabaseError::Migration(format!("Failed to encrypt developer credential: {e}"))
        })?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO developer_credentials
                (provider, credential_type, label, tags_json, metadata_json, encrypted_secret, nonce, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ",
            params![
                input.provider.trim(),
                input.credential_type.trim(),
                input.label.trim(),
                tags_json,
                metadata_json,
                encrypted,
                nonce,
                now
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_credential(
        &self,
        id: i64,
        input: &DeveloperCredentialInput,
    ) -> Result<bool, DatabaseError> {
        validate_nonempty("provider", &input.provider)?;
        validate_nonempty("credential type", &input.credential_type)?;
        validate_nonempty("label", &input.label)?;
        validate_nonempty("secret", &input.secret)?;

        let tags_json = serde_json::to_string(&normalize_tags(&input.tags))
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize tags: {e}")))?;
        let metadata_json = serde_json::to_string(&input.metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let (nonce, encrypted) = crypto::encrypt(&input.secret).map_err(|e| {
            DatabaseError::Migration(format!("Failed to encrypt developer credential: {e}"))
        })?;
        let now = Utc::now().to_rfc3339();

        let updated = self.conn.execute(
            r"
            UPDATE developer_credentials
            SET provider = ?1,
                credential_type = ?2,
                label = ?3,
                tags_json = ?4,
                metadata_json = ?5,
                encrypted_secret = ?6,
                nonce = ?7,
                updated_at = ?8
            WHERE id = ?9
            ",
            params![
                input.provider.trim(),
                input.credential_type.trim(),
                input.label.trim(),
                tags_json,
                metadata_json,
                encrypted,
                nonce,
                now,
                id
            ],
        )?;

        Ok(updated > 0)
    }

    pub fn delete_credential(&self, id: i64) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            "DELETE FROM developer_credentials WHERE id = ?1",
            params![id],
        )?;
        Ok(deleted > 0)
    }

    pub fn list_app_targets(&self) -> Result<Vec<AppTarget>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, name, platform, project_id, bundle_id, package_name, store_app_id,
                   metadata_json, created_at, updated_at
            FROM app_targets
            ORDER BY platform, name
            ",
        )?;
        let rows = stmt.query_map([], row_to_app_target)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_app_target(&self, input: &AppTargetInput) -> Result<i64, DatabaseError> {
        validate_nonempty("name", &input.name)?;
        validate_nonempty("platform", &input.platform)?;
        let metadata_json = serde_json::to_string(&input.metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO app_targets
                (name, platform, project_id, bundle_id, package_name, store_app_id, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ",
            params![
                input.name.trim(),
                input.platform.trim(),
                optional_trim(input.project_id.as_deref()),
                optional_trim(input.bundle_id.as_deref()),
                optional_trim(input.package_name.as_deref()),
                optional_trim(input.store_app_id.as_deref()),
                metadata_json,
                now
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_app_target(
        &self,
        id: i64,
        input: &AppTargetInput,
    ) -> Result<bool, DatabaseError> {
        validate_nonempty("name", &input.name)?;
        validate_nonempty("platform", &input.platform)?;
        let metadata_json = serde_json::to_string(&input.metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let now = Utc::now().to_rfc3339();

        let updated = self.conn.execute(
            r"
            UPDATE app_targets
            SET name = ?1,
                platform = ?2,
                project_id = ?3,
                bundle_id = ?4,
                package_name = ?5,
                store_app_id = ?6,
                metadata_json = ?7,
                updated_at = ?8
            WHERE id = ?9
            ",
            params![
                input.name.trim(),
                input.platform.trim(),
                optional_trim(input.project_id.as_deref()),
                optional_trim(input.bundle_id.as_deref()),
                optional_trim(input.package_name.as_deref()),
                optional_trim(input.store_app_id.as_deref()),
                metadata_json,
                now,
                id
            ],
        )?;

        Ok(updated > 0)
    }

    pub fn delete_app_target(&self, id: i64) -> Result<bool, DatabaseError> {
        let deleted = self
            .conn
            .execute("DELETE FROM app_targets WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    pub fn list_app_target_credentials(
        &self,
        app_target_id: i64,
    ) -> Result<Vec<AppTargetCredential>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT atc.app_target_id,
                   atc.credential_id,
                   atc.role,
                   dc.label,
                   dc.provider,
                   dc.credential_type,
                   atc.created_at
            FROM app_target_credentials atc
            JOIN developer_credentials dc ON dc.id = atc.credential_id
            WHERE atc.app_target_id = ?1
            ORDER BY atc.role, dc.label
            ",
        )?;
        let rows = stmt.query_map(params![app_target_id], |row| {
            Ok(AppTargetCredential {
                app_target_id: row.get(0)?,
                credential_id: row.get(1)?,
                role: row.get(2)?,
                credential_label: row.get(3)?,
                provider: row.get(4)?,
                credential_type: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn link_credential(
        &self,
        app_target_id: i64,
        credential_id: i64,
        role: &str,
    ) -> Result<(), DatabaseError> {
        validate_nonempty("role", role)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r"
            INSERT INTO app_target_credentials (app_target_id, credential_id, role, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(app_target_id, credential_id, role) DO NOTHING
            ",
            params![app_target_id, credential_id, role.trim(), now],
        )?;
        Ok(())
    }

    pub fn unlink_credential(
        &self,
        app_target_id: i64,
        credential_id: i64,
        role: &str,
    ) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            r"
            DELETE FROM app_target_credentials
            WHERE app_target_id = ?1 AND credential_id = ?2 AND role = ?3
            ",
            params![app_target_id, credential_id, role],
        )?;
        Ok(deleted > 0)
    }

    pub fn list_command_presets(&self) -> Result<Vec<DeveloperCommandPreset>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, name, command, working_dir, app_target_id, tags_json, created_at, updated_at
            FROM developer_commands
            ORDER BY name
            ",
        )?;
        let rows = stmt.query_map([], row_to_command_preset)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_command_preset(&self, input: &DeveloperCommandInput) -> Result<i64, DatabaseError> {
        validate_nonempty("name", &input.name)?;
        validate_nonempty("command", &input.command)?;
        let tags_json = serde_json::to_string(&normalize_tags(&input.tags))
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize tags: {e}")))?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r"
            INSERT INTO developer_commands
                (name, command, working_dir, app_target_id, tags_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ",
            params![
                input.name.trim(),
                input.command.trim(),
                optional_trim(input.working_dir.as_deref()),
                input.app_target_id,
                tags_json,
                now
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_command_preset(
        &self,
        id: i64,
        input: &DeveloperCommandInput,
    ) -> Result<bool, DatabaseError> {
        validate_nonempty("name", &input.name)?;
        validate_nonempty("command", &input.command)?;
        let tags_json = serde_json::to_string(&normalize_tags(&input.tags))
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize tags: {e}")))?;
        let now = Utc::now().to_rfc3339();
        let updated = self.conn.execute(
            r"
            UPDATE developer_commands
            SET name = ?1,
                command = ?2,
                working_dir = ?3,
                app_target_id = ?4,
                tags_json = ?5,
                updated_at = ?6
            WHERE id = ?7
            ",
            params![
                input.name.trim(),
                input.command.trim(),
                optional_trim(input.working_dir.as_deref()),
                input.app_target_id,
                tags_json,
                now,
                id
            ],
        )?;
        Ok(updated > 0)
    }

    pub fn delete_command_preset(&self, id: i64) -> Result<bool, DatabaseError> {
        let deleted = self
            .conn
            .execute("DELETE FROM developer_commands WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}

fn row_to_credential_summary(row: &rusqlite::Row) -> rusqlite::Result<DeveloperCredentialSummary> {
    let tags_json: String = row.get(4)?;
    let metadata_json: String = row.get(5)?;
    Ok(DeveloperCredentialSummary {
        id: row.get(0)?,
        provider: row.get(1)?,
        credential_type: row.get(2)?,
        label: row.get(3)?,
        tags: parse_tags(&tags_json).map_err(to_sql_err)?,
        metadata: parse_metadata(&metadata_json).map_err(to_sql_err)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_app_target(row: &rusqlite::Row) -> rusqlite::Result<AppTarget> {
    let metadata_json: String = row.get(7)?;
    Ok(AppTarget {
        id: row.get(0)?,
        name: row.get(1)?,
        platform: row.get(2)?,
        project_id: row.get(3)?,
        bundle_id: row.get(4)?,
        package_name: row.get(5)?,
        store_app_id: row.get(6)?,
        metadata: parse_metadata(&metadata_json).map_err(to_sql_err)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn row_to_command_preset(row: &rusqlite::Row) -> rusqlite::Result<DeveloperCommandPreset> {
    let tags_json: String = row.get(5)?;
    Ok(DeveloperCommandPreset {
        id: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        working_dir: row.get(3)?,
        app_target_id: row.get(4)?,
        tags: parse_tags(&tags_json).map_err(to_sql_err)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn parse_tags(json: &str) -> Result<Vec<String>, DatabaseError> {
    serde_json::from_str(json)
        .map_err(|e| DatabaseError::Migration(format!("Failed to parse tags: {e}")))
}

fn parse_metadata(json: &str) -> Result<serde_json::Value, DatabaseError> {
    serde_json::from_str(json)
        .map_err(|e| DatabaseError::Migration(format!("Failed to parse metadata: {e}")))
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = tags
        .iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn validate_nonempty(field: &str, value: &str) -> Result<(), DatabaseError> {
    if value.trim().is_empty() {
        return Err(DatabaseError::Migration(format!("{field} is required")));
    }
    if value.contains('\0') {
        return Err(DatabaseError::Migration(format!(
            "{field} contains null byte"
        )));
    }
    Ok(())
}

fn optional_trim(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn to_sql_err(e: DatabaseError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn command_presets_roundtrip_without_keychain() {
        let db = Database::in_memory().unwrap();
        let store = DeveloperStore::new(db.connection());
        let id = store
            .save_command_preset(&DeveloperCommandInput {
                name: "iOS beta".into(),
                command: "fastlane ios beta".into(),
                working_dir: Some("{project_path}".into()),
                app_target_id: None,
                tags: vec!["ios".into(), "beta".into(), "ios".into()],
            })
            .unwrap();

        let commands = store.list_command_presets().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, id);
        assert_eq!(commands[0].tags, vec!["beta", "ios"]);
    }
}
