//! Project secrets storage operations
//!
//! Secrets store credential information (API keys, passwords, etc.) with
//! structured fields: name, key, url, notes. The sensitive fields (key, url,
//! notes) are encrypted together as a JSON blob using AES-256-GCM.
//! The encryption key lives in the OS keychain, never on disk.

use super::db::DatabaseError;
use crate::crypto;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The sensitive fields stored as an encrypted JSON blob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretData {
    pub key: String,
    pub url: String,
    pub notes: String,
}

/// A fully decrypted secret (returned to the frontend on reveal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSecret {
    pub id: i64,
    pub project_id: String,
    pub name: String,
    pub key: String,
    pub url: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A secret summary (name only, no decrypted data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSummary {
    pub id: i64,
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating or updating a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInput {
    pub name: String,
    pub key: String,
    pub url: String,
    pub notes: String,
}

/// Secret storage operations
pub struct SecretStore<'a> {
    conn: &'a Connection,
}

impl<'a> SecretStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// List all secrets for a project (without decrypting)
    pub fn list(&self, project_id: Uuid) -> Result<Vec<SecretSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, name, created_at, updated_at
            FROM project_secrets
            WHERE project_id = ?1
            ORDER BY name
            ",
        )?;

        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            Ok(SecretSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
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

    /// Get a single decrypted secret by `project_id` and name
    pub fn get(
        &self,
        project_id: Uuid,
        name: &str,
    ) -> Result<Option<ProjectSecret>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, name, encrypted_data, nonce, created_at, updated_at
            FROM project_secrets
            WHERE project_id = ?1 AND name = ?2
            ",
        )?;

        let result = stmt.query_row(params![project_id.to_string(), name], |row| {
            let id: i64 = row.get(0)?;
            let pid: String = row.get(1)?;
            let n: String = row.get(2)?;
            let encrypted: String = row.get(3)?;
            let nonce: String = row.get(4)?;
            let created: String = row.get(5)?;
            let updated: String = row.get(6)?;
            Ok((id, pid, n, encrypted, nonce, created, updated))
        });

        match result {
            Ok((id, pid, n, encrypted, nonce, created, updated)) => {
                let json = crypto::decrypt(&nonce, &encrypted).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to decrypt secret: {e}"))
                })?;
                let data: SecretData = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse secret data: {e}"))
                })?;
                Ok(Some(ProjectSecret {
                    id,
                    project_id: pid,
                    name: n,
                    key: data.key,
                    url: data.url,
                    notes: data.notes,
                    created_at: created,
                    updated_at: updated,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save a new secret. Encrypts the sensitive fields before storing.
    pub fn save(&self, project_id: Uuid, input: &SecretInput) -> Result<(), DatabaseError> {
        let data = SecretData {
            key: input.key.clone(),
            url: input.url.clone(),
            notes: input.notes.clone(),
        };
        let json = serde_json::to_string(&data)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize secret: {e}")))?;
        let (nonce, encrypted) = crypto::encrypt(&json)
            .map_err(|e| DatabaseError::Migration(format!("Failed to encrypt secret: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO project_secrets (project_id, name, encrypted_data, nonce, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ON CONFLICT(project_id, name) DO UPDATE SET
                encrypted_data = excluded.encrypted_data,
                nonce = excluded.nonce,
                updated_at = excluded.updated_at
            ",
            params![project_id.to_string(), input.name, encrypted, nonce, now],
        )?;

        Ok(())
    }

    /// Update an existing secret by id. Allows changing name and sensitive fields.
    pub fn update(
        &self,
        project_id: Uuid,
        secret_id: i64,
        input: &SecretInput,
    ) -> Result<bool, DatabaseError> {
        let data = SecretData {
            key: input.key.clone(),
            url: input.url.clone(),
            notes: input.notes.clone(),
        };
        let json = serde_json::to_string(&data)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize secret: {e}")))?;
        let (nonce, encrypted) = crypto::encrypt(&json)
            .map_err(|e| DatabaseError::Migration(format!("Failed to encrypt secret: {e}")))?;
        let now = Utc::now().to_rfc3339();

        let updated = self.conn.execute(
            r"
            UPDATE project_secrets
            SET name = ?1, encrypted_data = ?2, nonce = ?3, updated_at = ?4
            WHERE id = ?5 AND project_id = ?6
            ",
            params![
                input.name,
                encrypted,
                nonce,
                now,
                secret_id,
                project_id.to_string()
            ],
        )?;

        Ok(updated > 0)
    }

    /// Delete a secret by name
    pub fn delete(&self, project_id: Uuid, name: &str) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            "DELETE FROM project_secrets WHERE project_id = ?1 AND name = ?2",
            params![project_id.to_string(), name],
        )?;
        Ok(deleted > 0)
    }
}
