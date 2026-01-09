//! Backup storage operations

use crate::backup::Backup;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

/// Backup storage operations
pub struct BackupStore<'a> {
    conn: &'a Connection,
}

impl<'a> BackupStore<'a> {
    /// Create a new backup store
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new backup record
    ///
    /// # Errors
    /// Returns an error if the backup cannot be created
    pub fn create(&self, backup: &Backup) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(backup)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize backup: {e}")))?;

        self.conn.execute(
            r"
            INSERT INTO backups (id, project_id, profile_id, description, archive_path, data, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                backup.id.to_string(),
                backup.project_id.to_string(),
                backup.profile_id.map(|id| id.to_string()),
                backup.description,
                backup.archive_path.display().to_string(),
                json,
                backup.created_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get a backup by ID
    ///
    /// # Errors
    /// Returns an error if the backup cannot be retrieved
    pub fn get(&self, id: Uuid) -> Result<Option<Backup>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM backups WHERE id = ?1
            ",
        )?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let backup: Backup = serde_json::from_str(&json)
                    .map_err(|e| DatabaseError::Migration(format!("Failed to parse backup: {e}")))?;
                Ok(Some(backup))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List backups for a project
    ///
    /// # Errors
    /// Returns an error if the backups cannot be listed
    pub fn list_for_project(&self, project_id: Uuid) -> Result<Vec<BackupSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, profile_id, description, archive_path, created_at
            FROM backups
            WHERE project_id = ?1
            ORDER BY created_at DESC
            ",
        )?;

        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: String = row.get(1)?;
            let profile_id_str: Option<String> = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let archive_path_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;

            Ok((
                id_str,
                project_id_str,
                profile_id_str,
                description,
                archive_path_str,
                created_at,
            ))
        })?;

        let mut backups = Vec::new();
        for row in rows {
            let (
                id_str,
                project_id_str,
                profile_id_str,
                description,
                archive_path_str,
                created_at_str,
            ) = row?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let project_id = Uuid::parse_str(&project_id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let profile_id = profile_id_str
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let archive_path = PathBuf::from(archive_path_str);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);

            backups.push(BackupSummary {
                id,
                project_id,
                profile_id,
                description,
                archive_path,
                created_at,
            });
        }

        Ok(backups)
    }

    /// List all backups
    ///
    /// # Errors
    /// Returns an error if the backups cannot be listed
    pub fn list_all(&self) -> Result<Vec<BackupSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, project_id, profile_id, description, archive_path, created_at
            FROM backups
            ORDER BY created_at DESC
            ",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: String = row.get(1)?;
            let profile_id_str: Option<String> = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let archive_path_str: String = row.get(4)?;
            let created_at: String = row.get(5)?;

            Ok((
                id_str,
                project_id_str,
                profile_id_str,
                description,
                archive_path_str,
                created_at,
            ))
        })?;

        let mut backups = Vec::new();
        for row in rows {
            let (
                id_str,
                project_id_str,
                profile_id_str,
                description,
                archive_path_str,
                created_at_str,
            ) = row?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let project_id = Uuid::parse_str(&project_id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let profile_id = profile_id_str
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let archive_path = PathBuf::from(archive_path_str);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);

            backups.push(BackupSummary {
                id,
                project_id,
                profile_id,
                description,
                archive_path,
                created_at,
            });
        }

        Ok(backups)
    }

    /// Delete a backup record (does not delete archive file)
    ///
    /// # Errors
    /// Returns an error if the backup cannot be deleted
    pub fn delete(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            r"
            DELETE FROM backups WHERE id = ?1
            ",
            params![id.to_string()],
        )?;

        Ok(deleted > 0)
    }
}

/// Backup summary (without full data)
#[derive(Debug, Clone)]
pub struct BackupSummary {
    /// Unique identifier
    pub id: Uuid,
    /// Project this backup is for
    pub project_id: Uuid,
    /// Profile that was applied (if any)
    pub profile_id: Option<Uuid>,
    /// Optional description
    pub description: Option<String>,
    /// Path to the backup archive
    pub archive_path: PathBuf,
    /// When the backup was created
    pub created_at: DateTime<Utc>,
}
