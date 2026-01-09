//! Database migrations

use rusqlite::Connection;

use super::db::DatabaseError;

const CURRENT_VERSION: i32 = 1;

/// Run all pending migrations
///
/// # Errors
/// Returns an error if migrations fail
pub fn run_migrations(conn: &Connection) -> Result<(), DatabaseError> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < 1 {
        migrate_v1(conn)?;
    }

    conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
    Ok(())
}

fn migrate_v1(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r"
        -- Projects table
        -- Stores full project state as JSON blob in data column
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Profiles table
        -- Stores full profile configuration as JSON blob in data column
        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Backups table
        -- Stores backup metadata and file contents as JSON blob in data column
        CREATE TABLE IF NOT EXISTS backups (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
            description TEXT,
            archive_path TEXT NOT NULL,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        -- Inventory cache (optional, for faster re-display)
        CREATE TABLE IF NOT EXISTS inventory_cache (
            id TEXT PRIMARY KEY,
            project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
            scope TEXT NOT NULL,
            inventory_json TEXT NOT NULL,
            scanned_at TEXT NOT NULL
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);
        CREATE INDEX IF NOT EXISTS idx_profiles_name ON profiles(name);
        CREATE INDEX IF NOT EXISTS idx_backups_project ON backups(project_id);
        CREATE INDEX IF NOT EXISTS idx_inventory_scope ON inventory_cache(scope);
        ",
    )?;

    Ok(())
}
