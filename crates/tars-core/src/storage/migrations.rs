//! Database migrations

use rusqlite::Connection;

use super::db::DatabaseError;

const CURRENT_VERSION: i32 = 4;

/// Run all pending migrations
///
/// # Errors
/// Returns an error if migrations fail
pub fn run_migrations(conn: &Connection) -> Result<(), DatabaseError> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < 1 {
        migrate_v1(conn)?;
    }

    if version < 2 {
        migrate_v2(conn)?;
    }

    if version < 3 {
        migrate_v3(conn)?;
    }

    if version < 4 {
        migrate_v4(conn)?;
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

fn migrate_v3(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r"
        -- Project metadata table
        -- Stores structured project info (hosting, database, storage, etc.)
        CREATE TABLE IF NOT EXISTS project_metadata (
            project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            data TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Project secrets table
        -- Stores encrypted key-value secrets per project
        -- Values are AES-256-GCM encrypted, key stored in OS keychain
        CREATE TABLE IF NOT EXISTS project_secrets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            key TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            nonce TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(project_id, key)
        );

        CREATE INDEX IF NOT EXISTS idx_project_secrets_project ON project_secrets(project_id);
        ",
    )?;

    Ok(())
}

fn migrate_v4(conn: &Connection) -> Result<(), DatabaseError> {
    // Recreate project_secrets with richer schema:
    // - name: plaintext label (e.g. "OpenAI API Key")
    // - encrypted_data: AES-256-GCM encrypted JSON blob with key, url, notes
    //
    // Migrate existing rows: old `key` becomes `name`, old encrypted_value
    // is decrypted and re-encrypted as JSON { "key": "<old_value>" }
    conn.execute_batch(
        r"
        ALTER TABLE project_secrets RENAME TO project_secrets_old;
        DROP INDEX IF EXISTS idx_project_secrets_project;

        CREATE TABLE project_secrets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            encrypted_data TEXT NOT NULL,
            nonce TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(project_id, name)
        );

        CREATE INDEX idx_project_secrets_project ON project_secrets(project_id);
        ",
    )?;

    // Migrate existing secrets
    {
        let mut read_stmt = conn.prepare(
            "SELECT project_id, key, encrypted_value, nonce, created_at, updated_at FROM project_secrets_old",
        )?;
        let rows: Vec<(String, String, String, String, String, String)> = read_stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for (project_id, key_name, old_encrypted, old_nonce, created, updated) in rows {
            // Decrypt old value
            let old_value = crate::crypto::decrypt(&old_nonce, &old_encrypted)
                .map_err(|e| DatabaseError::Migration(format!("v4: decrypt failed: {e}")))?;

            // Re-encrypt as JSON blob with key field
            let json = serde_json::json!({
                "key": old_value,
                "url": "",
                "notes": ""
            })
            .to_string();

            let (new_nonce, new_encrypted) = crate::crypto::encrypt(&json)
                .map_err(|e| DatabaseError::Migration(format!("v4: encrypt failed: {e}")))?;

            conn.execute(
                r"INSERT INTO project_secrets (project_id, name, encrypted_data, nonce, created_at, updated_at)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![project_id, key_name, new_encrypted, new_nonce, created, updated],
            )?;
        }
    }

    conn.execute_batch("DROP TABLE project_secrets_old;")?;

    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r"
        -- Plugin version tracking table
        -- Tracks when plugin versions actually changed (not just checked)
        CREATE TABLE IF NOT EXISTS plugin_versions (
            plugin_key TEXT PRIMARY KEY,
            version TEXT NOT NULL,
            version_changed_at TEXT NOT NULL,
            last_checked_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_plugin_versions_key ON plugin_versions(plugin_key);
        ",
    )?;

    Ok(())
}
