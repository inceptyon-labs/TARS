//! Profile storage operations (CRUD)

use crate::profile::Profile;
use crate::storage::db::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

/// Profile storage operations
pub struct ProfileStore<'a> {
    conn: &'a Connection,
}

impl<'a> ProfileStore<'a> {
    /// Create a new profile store
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new profile
    ///
    /// # Errors
    /// Returns an error if the profile cannot be created
    pub fn create(&self, profile: &Profile) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(profile)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize profile: {e}")))?;

        self.conn.execute(
            r"
            INSERT INTO profiles (id, name, description, data, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                profile.id.to_string(),
                profile.name,
                profile.description,
                json,
                profile.created_at.to_rfc3339(),
                profile.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get a profile by ID
    ///
    /// # Errors
    /// Returns an error if the profile cannot be retrieved
    pub fn get(&self, id: Uuid) -> Result<Option<Profile>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM profiles WHERE id = ?1
            ",
        )?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let profile: Profile = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse profile: {e}"))
                })?;
                Ok(Some(profile))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a profile by name
    ///
    /// # Errors
    /// Returns an error if the profile cannot be retrieved
    pub fn get_by_name(&self, name: &str) -> Result<Option<Profile>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT data FROM profiles WHERE name = ?1
            ",
        )?;

        let result = stmt.query_row(params![name], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let profile: Profile = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse profile: {e}"))
                })?;
                Ok(Some(profile))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all profiles
    ///
    /// # Errors
    /// Returns an error if the profiles cannot be listed
    pub fn list(&self) -> Result<Vec<ProfileSummary>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, name, description, created_at, updated_at,
                   COALESCE(json_array_length(data, '$.tool_refs'), 0) as tool_count
            FROM profiles
            ORDER BY name
            ",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;
            let tool_count: i64 = row.get(5)?;

            Ok((
                id_str,
                name,
                description,
                created_at,
                updated_at,
                tool_count,
            ))
        })?;

        let mut profiles = Vec::new();
        for row in rows {
            let (id_str, name, description, created_at_str, updated_at_str, tool_count) = row?;
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid UUID: {e}")))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| DatabaseError::Migration(format!("Invalid datetime: {e}")))?
                .with_timezone(&Utc);

            profiles.push(ProfileSummary {
                id,
                name,
                description,
                tool_count: usize::try_from(tool_count).unwrap_or(0),
                created_at,
                updated_at,
            });
        }

        Ok(profiles)
    }

    /// Update a profile
    ///
    /// # Errors
    /// Returns an error if the profile cannot be updated
    pub fn update(&self, profile: &Profile) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(profile)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize profile: {e}")))?;

        let updated = self.conn.execute(
            r"
            UPDATE profiles
            SET name = ?1, description = ?2, data = ?3, updated_at = ?4
            WHERE id = ?5
            ",
            params![
                profile.name,
                profile.description,
                json,
                profile.updated_at.to_rfc3339(),
                profile.id.to_string(),
            ],
        )?;

        if updated == 0 {
            return Err(DatabaseError::Migration(format!(
                "Profile not found: {}",
                profile.id
            )));
        }

        Ok(())
    }

    /// Delete a profile
    ///
    /// # Errors
    /// Returns an error if the profile cannot be deleted
    pub fn delete(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            r"
            DELETE FROM profiles WHERE id = ?1
            ",
            params![id.to_string()],
        )?;

        Ok(deleted > 0)
    }
}

/// Profile summary (without full data)
#[derive(Debug, Clone)]
pub struct ProfileSummary {
    /// Unique identifier
    pub id: Uuid,
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Number of tool references in this profile
    pub tool_count: usize,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
}
