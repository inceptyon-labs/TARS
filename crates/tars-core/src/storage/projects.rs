//! Project storage operations (CRUD)

use crate::project::Project;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

/// Project storage operations
pub struct ProjectStore<'a> {
    conn: &'a Connection,
}

impl<'a> ProjectStore<'a> {
    /// Create a new project store
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new project
    ///
    /// # Errors
    /// Returns an error if the project cannot be created
    pub fn create(&self, project: &Project) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(project)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize project: {e}")))?;

        self.conn.execute(
            r"
            INSERT INTO projects (id, name, path, data, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                project.id.to_string(),
                project.name,
                project.path.display().to_string(),
                json,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get a project by ID
    ///
    /// # Errors
    /// Returns an error if the project cannot be retrieved
    pub fn get(&self, id: Uuid) -> Result<Option<Project>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM projects WHERE id = ?1
            ",
        )?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let project: Project = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse project: {e}"))
                })?;
                Ok(Some(project))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a project by path
    ///
    /// # Errors
    /// Returns an error if the project cannot be retrieved
    pub fn get_by_path(&self, path: &PathBuf) -> Result<Option<Project>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM projects WHERE path = ?1
            ",
        )?;

        let result = stmt.query_row(params![path.display().to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let project: Project = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse project: {e}"))
                })?;
                Ok(Some(project))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all projects
    ///
    /// # Errors
    /// Returns an error if the projects cannot be listed
    pub fn list(&self) -> Result<Vec<ProjectSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, name, path, created_at, updated_at
            FROM projects
            ORDER BY name
            ",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            let path_str: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;

            Ok((id_str, name, path_str, created_at, updated_at))
        })?;

        let mut projects = Vec::new();
        for row in rows {
            let (id_str, name, path_str, created_at_str, updated_at_str) = row?;
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let path = PathBuf::from(path_str);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);

            projects.push(ProjectSummary {
                id,
                name,
                path,
                created_at,
                updated_at,
            });
        }

        Ok(projects)
    }

    /// Update a project
    ///
    /// # Errors
    /// Returns an error if the project cannot be updated
    pub fn update(&self, project: &Project) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(project)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize project: {e}")))?;

        let updated = self.conn.execute(
            r"
            UPDATE projects
            SET name = ?1, path = ?2, data = ?3, updated_at = ?4
            WHERE id = ?5
            ",
            params![
                project.name,
                project.path.display().to_string(),
                json,
                project.updated_at.to_rfc3339(),
                project.id.to_string(),
            ],
        )?;

        if updated == 0 {
            return Err(DatabaseError::Migration(format!(
                "Project not found: {}",
                project.id
            )));
        }

        Ok(())
    }

    /// Delete a project
    ///
    /// # Errors
    /// Returns an error if the project cannot be deleted
    pub fn delete(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            r"
            DELETE FROM projects WHERE id = ?1
            ",
            params![id.to_string()],
        )?;

        Ok(deleted > 0)
    }

    /// List all projects with a specific profile assigned
    ///
    /// # Errors
    /// Returns an error if the projects cannot be listed
    pub fn list_by_profile(&self, profile_id: Uuid) -> Result<Vec<Project>, DatabaseError> {
        // Since assigned_profile_id is stored in the JSON data column,
        // we need to load all projects and filter in Rust.
        // For MVP scale this is acceptable. Future optimization:
        // add assigned_profile_id as a separate column for SQL filtering.
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM projects
            ",
        )?;

        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut projects = Vec::new();
        for row in rows {
            let json = row?;
            let project: Project = serde_json::from_str(&json)
                .map_err(|e| DatabaseError::Migration(format!("Failed to parse project: {e}")))?;

            if project.assigned_profile_id == Some(profile_id) {
                projects.push(project);
            }
        }

        Ok(projects)
    }

    /// Count projects with a specific profile assigned
    ///
    /// # Errors
    /// Returns an error if the count cannot be performed
    pub fn count_by_profile(&self, profile_id: Uuid) -> Result<usize, DatabaseError> {
        // For now, use list_by_profile and count
        // Could be optimized if needed
        Ok(self.list_by_profile(profile_id)?.len())
    }
}

/// Project summary (without full data)
#[derive(Debug, Clone)]
pub struct ProjectSummary {
    /// Unique identifier
    pub id: Uuid,
    /// Project name
    pub name: String,
    /// Project path
    pub path: PathBuf,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
}
