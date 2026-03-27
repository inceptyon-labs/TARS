//! Project metadata storage operations

use super::db::DatabaseError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Structured project metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// Hosting & Deployment
    pub deploy_target: Option<String>,
    pub domain: Option<String>,
    pub production_url: Option<String>,
    pub staging_url: Option<String>,
    pub deploy_command: Option<String>,

    /// Data & Storage
    pub database_provider: Option<String>,
    pub database_name: Option<String>,
    pub object_storage: Option<String>,
    pub object_storage_bucket: Option<String>,

    /// Local Development
    pub start_command: Option<String>,
    pub requires_tunnel: bool,
    pub tunnel_provider: Option<String>,
    pub tunnel_id: Option<String>,

    /// Source & Distribution
    pub github_url: Option<String>,
    pub app_store_url: Option<String>,
    pub app_store_connect_url: Option<String>,
    pub play_store_url: Option<String>,
    pub package_registry_url: Option<String>,

    /// Infrastructure
    pub ci_cd: Option<String>,
    pub monitoring: Option<String>,

    /// Custom key-value pairs
    #[serde(default)]
    pub custom_fields: Vec<CustomField>,
}

/// A user-defined key-value field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub key: String,
    pub value: String,
}

/// Metadata storage operations
pub struct MetadataStore<'a> {
    conn: &'a Connection,
}

impl<'a> MetadataStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get metadata for a project
    pub fn get(&self, project_id: Uuid) -> Result<Option<ProjectMetadata>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM project_metadata WHERE project_id = ?1")?;

        let result = stmt.query_row(params![project_id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let metadata: ProjectMetadata = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse metadata: {e}"))
                })?;
                Ok(Some(metadata))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save metadata for a project (upsert)
    pub fn save(&self, project_id: Uuid, metadata: &ProjectMetadata) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO project_metadata (project_id, data, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(project_id) DO UPDATE SET
                data = excluded.data,
                updated_at = excluded.updated_at
            ",
            params![project_id.to_string(), json, now],
        )?;

        Ok(())
    }

    /// Delete metadata for a project
    pub fn delete(&self, project_id: Uuid) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            "DELETE FROM project_metadata WHERE project_id = ?1",
            params![project_id.to_string()],
        )?;
        Ok(deleted > 0)
    }
}
