//! API key vault storage.
//!
//! Stores AI provider API keys encrypted at rest (AES-256-GCM; master key in
//! OS keychain). Unlike [`super::secrets::SecretStore`], these keys are not
//! tied to a project — they are global references for the user's personal
//! key inventory.

use super::db::DatabaseError;
use crate::crypto;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Maximum allowed length for a human-facing label
pub const MAX_LABEL_LEN: usize = 100;

/// Maximum allowed length for an API key (generous; real keys are ~200 chars)
pub const MAX_KEY_LEN: usize = 4096;

/// Input validation errors specific to API key records
#[derive(Error, Debug)]
pub enum ApiKeyValidationError {
    #[error("Label is empty")]
    EmptyLabel,

    #[error("Label exceeds {MAX_LABEL_LEN} characters")]
    LabelTooLong,

    #[error("Label contains null byte")]
    LabelNullByte,

    #[error("Key is empty")]
    EmptyKey,

    #[error("Key exceeds {MAX_KEY_LEN} characters")]
    KeyTooLong,

    #[error("Key contains null byte")]
    KeyNullByte,

    #[error("Provider id is empty")]
    EmptyProviderId,
}

/// Validate a label for an API key entry.
///
/// Length is checked by Unicode character count (not bytes), since the limit
/// is user-facing.
///
/// # Errors
/// Returns an [`ApiKeyValidationError`] if the label is empty, too long, or
/// contains a null byte.
pub fn validate_label(label: &str) -> Result<(), ApiKeyValidationError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(ApiKeyValidationError::EmptyLabel);
    }
    if label.chars().count() > MAX_LABEL_LEN {
        return Err(ApiKeyValidationError::LabelTooLong);
    }
    if label.contains('\0') {
        return Err(ApiKeyValidationError::LabelNullByte);
    }
    Ok(())
}

/// Validate an API key value before encryption.
///
/// # Errors
/// Returns an [`ApiKeyValidationError`] if the key is empty, too long, or
/// contains a null byte.
pub fn validate_key(key: &str) -> Result<(), ApiKeyValidationError> {
    if key.is_empty() {
        return Err(ApiKeyValidationError::EmptyKey);
    }
    if key.len() > MAX_KEY_LEN {
        return Err(ApiKeyValidationError::KeyTooLong);
    }
    if key.contains('\0') {
        return Err(ApiKeyValidationError::KeyNullByte);
    }
    Ok(())
}

/// A fully decrypted API key record.
///
/// Intentionally does not derive `Serialize`/`Deserialize`: the `key` field
/// contains decrypted secret material and this struct must never cross the
/// IPC boundary or be persisted to disk. `Debug` is hand-implemented so the
/// key is never printed via `{:?}`.
#[derive(Clone)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub provider_id: String,
    pub label: String,
    pub key: String,
    pub last_validated_at: Option<String>,
    pub last_valid: Option<bool>,
    pub balance: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl fmt::Debug for ApiKeyRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyRecord")
            .field("id", &self.id)
            .field("provider_id", &self.provider_id)
            .field("label", &self.label)
            .field("key", &"<redacted>")
            .field("last_validated_at", &self.last_validated_at)
            .field("last_valid", &self.last_valid)
            .field("balance", &self.balance)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Summary of an API key for list views (no decrypted key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySummary {
    pub id: i64,
    pub provider_id: String,
    pub label: String,
    pub last_validated_at: Option<String>,
    pub last_valid: Option<bool>,
    pub balance: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new API key entry.
///
/// Implements `Deserialize` so Tauri commands can accept it as a payload, but
/// intentionally does not derive `Serialize` — the `key` field holds plaintext
/// secret material. `Debug` is hand-implemented to redact the key.
#[derive(Clone, Deserialize)]
pub struct ApiKeyInput {
    pub provider_id: String,
    pub label: String,
    pub key: String,
}

impl fmt::Debug for ApiKeyInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyInput")
            .field("provider_id", &self.provider_id)
            .field("label", &self.label)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// API key storage operations
pub struct ApiKeyStore<'a> {
    conn: &'a Connection,
}

impl<'a> ApiKeyStore<'a> {
    #[must_use]
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all keys across all providers (without decrypting).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub fn list(&self) -> Result<Vec<ApiKeySummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, provider_id, label, last_validated_at, last_valid,
                   balance_json, created_at, updated_at
            FROM api_keys
            ORDER BY provider_id, label
            ",
        )?;
        let rows = stmt.query_map([], row_to_summary)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// List keys for a specific provider.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub fn list_by_provider(&self, provider_id: &str) -> Result<Vec<ApiKeySummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, provider_id, label, last_validated_at, last_valid,
                   balance_json, created_at, updated_at
            FROM api_keys
            WHERE provider_id = ?1
            ORDER BY label
            ",
        )?;
        let rows = stmt.query_map(params![provider_id], row_to_summary)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get a single decrypted record by id.
    ///
    /// # Errors
    /// Returns an error if the query fails, decryption fails, or the row is
    /// malformed.
    pub fn get(&self, id: i64) -> Result<Option<ApiKeyRecord>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, provider_id, label, encrypted_key, nonce,
                   last_validated_at, last_valid, balance_json,
                   created_at, updated_at
            FROM api_keys
            WHERE id = ?1
            ",
        )?;

        let row = stmt
            .query_row(params![id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .optional()?;

        let Some((
            id,
            provider_id,
            label,
            encrypted_key,
            nonce,
            last_validated_at,
            last_valid,
            balance_json,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        let key = crypto::decrypt(&nonce, &encrypted_key)
            .map_err(|e| DatabaseError::Migration(format!("Failed to decrypt api key: {e}")))?;
        let balance = parse_balance(balance_json.as_deref())?;

        Ok(Some(ApiKeyRecord {
            id,
            provider_id,
            label,
            key,
            last_validated_at,
            last_valid: last_valid.map(|v| v != 0),
            balance,
            created_at,
            updated_at,
        }))
    }

    /// Save a new API key. The key value is encrypted before storage.
    ///
    /// Returns the rowid of the inserted record.
    ///
    /// Note: `provider_id` is not validated against a provider allowlist at
    /// the storage layer — `tars-core` does not depend on `tars-providers`.
    /// Callers (typically the Tauri command layer) must ensure the value
    /// matches a known `tars_providers::ProviderId` before calling.
    ///
    /// # Errors
    /// Returns an error if the label/key fail validation, encryption fails,
    /// or the insert fails (e.g. duplicate `(provider_id, label)`).
    pub fn save(&self, input: &ApiKeyInput) -> Result<i64, DatabaseError> {
        if input.provider_id.trim().is_empty() {
            return Err(DatabaseError::Migration(
                ApiKeyValidationError::EmptyProviderId.to_string(),
            ));
        }
        validate_label(&input.label)
            .map_err(|e| DatabaseError::Migration(format!("Invalid label: {e}")))?;
        validate_key(&input.key)
            .map_err(|e| DatabaseError::Migration(format!("Invalid key: {e}")))?;

        // Normalize label: trim surrounding whitespace so "work" and " work "
        // collapse to the same UNIQUE key rather than two visually identical rows.
        let label = input.label.trim();

        let (nonce, encrypted) = crypto::encrypt(&input.key)
            .map_err(|e| DatabaseError::Migration(format!("Failed to encrypt api key: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO api_keys
                (provider_id, label, encrypted_key, nonce, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ",
            params![input.provider_id, label, encrypted, nonce, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Record the outcome of a validation attempt.
    ///
    /// `balance` is the raw JSON returned by the provider (if any); it is
    /// stored verbatim so the UI can display provider-specific fields.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub fn update_validation(
        &self,
        id: i64,
        valid: bool,
        balance: Option<&serde_json::Value>,
    ) -> Result<bool, DatabaseError> {
        let balance_json = balance
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize balance: {e}")))?;
        let now = Utc::now().to_rfc3339();

        let updated = self.conn.execute(
            r"
            UPDATE api_keys
            SET last_validated_at = ?1,
                last_valid = ?2,
                balance_json = ?3,
                updated_at = ?4
            WHERE id = ?5
            ",
            params![now, i64::from(valid), balance_json, now, id],
        )?;

        Ok(updated > 0)
    }

    /// Delete a key by id.
    ///
    /// # Errors
    /// Returns an error if the delete fails.
    pub fn delete(&self, id: i64) -> Result<bool, DatabaseError> {
        let deleted = self
            .conn
            .execute("DELETE FROM api_keys WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}

fn row_to_summary(row: &rusqlite::Row) -> rusqlite::Result<ApiKeySummary> {
    let balance_json: Option<String> = row.get(5)?;
    let balance = parse_balance(balance_json.as_deref()).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(ApiKeySummary {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        label: row.get(2)?,
        last_validated_at: row.get(3)?,
        last_valid: row.get::<_, Option<i64>>(4)?.map(|v| v != 0),
        balance,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn parse_balance(raw: Option<&str>) -> Result<Option<serde_json::Value>, DatabaseError> {
    match raw {
        None => Ok(None),
        Some(s) => serde_json::from_str(s)
            .map(Some)
            .map_err(|e| DatabaseError::Migration(format!("Failed to parse balance_json: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    fn is_keychain_error(e: &DatabaseError) -> bool {
        match e {
            DatabaseError::Migration(msg) => {
                msg.contains("Keyring") || msg.contains("keychain") || msg.contains("encrypt")
            }
            DatabaseError::Sqlite(_) => false,
        }
    }

    #[test]
    fn validate_label_rules() {
        assert!(validate_label("work").is_ok());
        assert!(validate_label("personal (2026)").is_ok());
        assert!(matches!(
            validate_label(""),
            Err(ApiKeyValidationError::EmptyLabel)
        ));
        assert!(matches!(
            validate_label("   "),
            Err(ApiKeyValidationError::EmptyLabel)
        ));
        assert!(matches!(
            validate_label("a\0b"),
            Err(ApiKeyValidationError::LabelNullByte)
        ));
        let long = "x".repeat(MAX_LABEL_LEN + 1);
        assert!(matches!(
            validate_label(&long),
            Err(ApiKeyValidationError::LabelTooLong)
        ));
    }

    #[test]
    fn validate_key_rules() {
        assert!(validate_key("sk-test-123").is_ok());
        assert!(matches!(
            validate_key(""),
            Err(ApiKeyValidationError::EmptyKey)
        ));
        assert!(matches!(
            validate_key("a\0b"),
            Err(ApiKeyValidationError::KeyNullByte)
        ));
        let long = "x".repeat(MAX_KEY_LEN + 1);
        assert!(matches!(
            validate_key(&long),
            Err(ApiKeyValidationError::KeyTooLong)
        ));
    }

    #[test]
    fn save_rejects_invalid_input() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());

        let bad = ApiKeyInput {
            provider_id: "openai".into(),
            label: String::new(),
            key: "sk-test".into(),
        };
        assert!(store.save(&bad).is_err());

        let bad = ApiKeyInput {
            provider_id: String::new(),
            label: "work".into(),
            key: "sk-test".into(),
        };
        assert!(store.save(&bad).is_err());
    }

    #[test]
    fn save_list_get_delete_roundtrip() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());

        let input = ApiKeyInput {
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-test-123".into(),
        };

        let id = match store.save(&input) {
            Ok(id) => id,
            Err(e) if is_keychain_error(&e) => {
                eprintln!("Skipping api_keys roundtrip: keychain unavailable ({e})");
                return;
            }
            Err(e) => panic!("unexpected save error: {e}"),
        };

        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].provider_id, "openai");
        assert_eq!(all[0].label, "work");

        let by_provider = store.list_by_provider("openai").unwrap();
        assert_eq!(by_provider.len(), 1);

        let empty = store.list_by_provider("anthropic").unwrap();
        assert!(empty.is_empty());

        let fetched = store.get(id).unwrap().expect("record exists");
        assert_eq!(fetched.key, "sk-test-123");
        assert_eq!(fetched.provider_id, "openai");
        assert_eq!(fetched.last_validated_at, None);
        assert_eq!(fetched.last_valid, None);

        assert!(store.delete(id).unwrap());
        assert!(store.list().unwrap().is_empty());
        assert!(store.get(id).unwrap().is_none());
    }

    #[test]
    fn duplicate_provider_and_label_rejected() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());

        let input = ApiKeyInput {
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-a".into(),
        };

        if store.save(&input).is_err() {
            return;
        }
        let dup = ApiKeyInput {
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-b".into(),
        };
        assert!(store.save(&dup).is_err());

        // Different label, same provider: allowed
        let ok = ApiKeyInput {
            provider_id: "openai".into(),
            label: "personal".into(),
            key: "sk-c".into(),
        };
        assert!(store.save(&ok).is_ok());

        // Same label, different provider: allowed
        let ok = ApiKeyInput {
            provider_id: "anthropic".into(),
            label: "work".into(),
            key: "sk-d".into(),
        };
        assert!(store.save(&ok).is_ok());
    }

    #[test]
    fn update_validation_persists() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());

        let input = ApiKeyInput {
            provider_id: "deepseek".into(),
            label: "work".into(),
            key: "sk-test".into(),
        };
        let id = match store.save(&input) {
            Ok(id) => id,
            Err(e) if is_keychain_error(&e) => return,
            Err(e) => panic!("unexpected: {e}"),
        };

        let balance = serde_json::json!({ "total_balance": "12.34", "currency": "USD" });
        assert!(store.update_validation(id, true, Some(&balance)).unwrap());

        let summary = &store.list().unwrap()[0];
        assert_eq!(summary.last_valid, Some(true));
        assert_eq!(summary.balance, Some(balance));
        assert!(summary.last_validated_at.is_some());

        assert!(store.update_validation(id, false, None).unwrap());
        let summary = &store.list().unwrap()[0];
        assert_eq!(summary.last_valid, Some(false));
        assert_eq!(summary.balance, None);
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());
        assert!(!store.delete(999).unwrap());
    }

    #[test]
    fn label_is_trimmed_on_save() {
        let db = Database::in_memory().unwrap();
        let store = ApiKeyStore::new(db.connection());

        let input = ApiKeyInput {
            provider_id: "openai".into(),
            label: "  work  ".into(),
            key: "sk-a".into(),
        };
        if store.save(&input).is_err() {
            return; // keychain unavailable
        }
        let all = store.list().unwrap();
        assert_eq!(
            all[0].label, "work",
            "leading/trailing whitespace should be stripped"
        );

        // A visually identical label with whitespace must collide with the first
        let dup = ApiKeyInput {
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-b".into(),
        };
        assert!(
            store.save(&dup).is_err(),
            "trimmed label should collide via UNIQUE"
        );
    }

    #[test]
    fn debug_redacts_secret_material() {
        let record = ApiKeyRecord {
            id: 1,
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-super-secret-12345".into(),
            last_validated_at: None,
            last_valid: None,
            balance: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        let formatted = format!("{record:?}");
        assert!(
            !formatted.contains("sk-super-secret-12345"),
            "plaintext key leaked via Debug: {formatted}"
        );
        assert!(formatted.contains("<redacted>"));

        let input = ApiKeyInput {
            provider_id: "openai".into(),
            label: "work".into(),
            key: "sk-another-secret".into(),
        };
        let formatted = format!("{input:?}");
        assert!(!formatted.contains("sk-another-secret"));
        assert!(formatted.contains("<redacted>"));
    }
}
