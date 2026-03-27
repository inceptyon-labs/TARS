//! Project secrets storage operations
//!
//! Secrets are encrypted with AES-256-GCM before storage.
//! The encryption key lives in the OS keychain, never on disk.

use super::db::DatabaseError;
use crate::crypto;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A decrypted secret (returned to the frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSecret {
    pub id: i64,
    pub project_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A secret summary (key only, no decrypted value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSummary {
    pub id: i64,
    pub project_id: String,
    pub key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Secret storage operations
pub struct SecretStore<'a> {
    conn: &'a Connection,
}

impl<'a> SecretStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all secret keys for a project (without decrypting values)
    pub fn list(&self, project_id: Uuid) -> Result<Vec<SecretSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, key, created_at, updated_at
            FROM project_secrets
            WHERE project_id = ?1
            ORDER BY key
            ",
        )?;

        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            Ok(SecretSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                key: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut secrets = Vec::new();
        for row in rows {
            secrets.push(row?);
        }
        Ok(secrets)
    }

    /// Get a single decrypted secret by `project_id` and key
    pub fn get(&self, project_id: Uuid, key: &str) -> Result<Option<ProjectSecret>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, key, encrypted_value, nonce, created_at, updated_at
            FROM project_secrets
            WHERE project_id = ?1 AND key = ?2
            ",
        )?;

        let result = stmt.query_row(params![project_id.to_string(), key], |row| {
            let id: i64 = row.get(0)?;
            let pid: String = row.get(1)?;
            let k: String = row.get(2)?;
            let encrypted: String = row.get(3)?;
            let nonce: String = row.get(4)?;
            let created: String = row.get(5)?;
            let updated: String = row.get(6)?;
            Ok((id, pid, k, encrypted, nonce, created, updated))
        });

        match result {
            Ok((id, pid, k, encrypted, nonce, created, updated)) => {
                let value = crypto::decrypt(&nonce, &encrypted).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to decrypt secret: {e}"))
                })?;
                Ok(Some(ProjectSecret {
                    id,
                    project_id: pid,
                    key: k,
                    value,
                    created_at: created,
                    updated_at: updated,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save a secret (upsert). Encrypts the value before storing.
    pub fn save(&self, project_id: Uuid, key: &str, value: &str) -> Result<(), DatabaseError> {
        let (nonce, encrypted) = crypto::encrypt(value)
            .map_err(|e| DatabaseError::Migration(format!("Failed to encrypt secret: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO project_secrets (project_id, key, encrypted_value, nonce, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ON CONFLICT(project_id, key) DO UPDATE SET
                encrypted_value = excluded.encrypted_value,
                nonce = excluded.nonce,
                updated_at = excluded.updated_at
            ",
            params![project_id.to_string(), key, encrypted, nonce, now],
        )?;

        Ok(())
    }

    /// Delete a secret
    pub fn delete(&self, project_id: Uuid, key: &str) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            "DELETE FROM project_secrets WHERE project_id = ?1 AND key = ?2",
            params![project_id.to_string(), key],
        )?;
        Ok(deleted > 0)
    }
}
