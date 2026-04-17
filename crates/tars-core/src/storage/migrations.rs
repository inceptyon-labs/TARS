//! Database migrations

use rusqlite::Connection;

use super::db::DatabaseError;

const CURRENT_VERSION: i32 = 6;

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

    if version < 5 {
        migrate_v5(conn)?;
    }

    if version < 6 {
        migrate_v6(conn)?;
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

fn migrate_v5(conn: &Connection) -> Result<(), DatabaseError> {
    // API keys vault: encrypted AI provider API keys, not tied to a project.
    // `provider_id` is the stable string form of tars_providers::ProviderId
    // (e.g. "openai", "anthropic"). `label` is free-form user text used to
    // distinguish multiple keys for the same provider.
    //
    // `provider_models` caches the model list fetched from each provider.
    // Pricing columns populate from the Phase-4 LiteLLM import; they are
    // nullable until first refresh.
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL,
            label TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            nonce TEXT NOT NULL,
            last_validated_at TEXT,
            last_valid INTEGER,
            balance_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(provider_id, label)
        );

        CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider_id);

        CREATE TABLE IF NOT EXISTS provider_models (
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            display_name TEXT,
            context_window INTEGER,
            input_price REAL,
            output_price REAL,
            price_override_json TEXT,
            fetched_at TEXT NOT NULL,
            PRIMARY KEY(provider_id, model_id)
        );
        ",
    )?;

    Ok(())
}

fn migrate_v6(conn: &Connection) -> Result<(), DatabaseError> {
    // Pricing metadata: tracks LiteLLM refresh state. Keys include
    // "last_refresh" (ISO timestamp of last successful fetch) and
    // "last_error" (error string from most recent failure). Values are
    // free-form text and interpreted by callers.
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS pricing_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        run_migrations(&conn).expect("migrations");
        conn
    }

    fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn migrations_reach_current_version() {
        let conn = fresh_conn();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_VERSION);
    }

    #[test]
    fn v5_creates_api_keys_table() {
        let conn = fresh_conn();
        let cols = table_columns(&conn, "api_keys");
        for expected in [
            "id",
            "provider_id",
            "label",
            "encrypted_key",
            "nonce",
            "last_validated_at",
            "last_valid",
            "balance_json",
            "created_at",
            "updated_at",
        ] {
            assert!(
                cols.contains(&expected.to_string()),
                "missing col {expected}"
            );
        }
    }

    #[test]
    fn v5_creates_provider_models_table() {
        let conn = fresh_conn();
        let cols = table_columns(&conn, "provider_models");
        for expected in [
            "provider_id",
            "model_id",
            "display_name",
            "context_window",
            "input_price",
            "output_price",
            "price_override_json",
            "fetched_at",
        ] {
            assert!(
                cols.contains(&expected.to_string()),
                "missing col {expected}"
            );
        }
    }

    #[test]
    fn api_keys_unique_on_provider_and_label() {
        let conn = fresh_conn();
        let now = "2026-04-17T00:00:00Z";
        conn.execute(
            "INSERT INTO api_keys (provider_id, label, encrypted_key, nonce, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params!["openai", "work", "enc1", "nonce1", now],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO api_keys (provider_id, label, encrypted_key, nonce, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params!["openai", "work", "enc2", "nonce2", now],
        );
        assert!(dup.is_err(), "duplicate should violate UNIQUE");
        // Same label different provider is allowed
        conn.execute(
            "INSERT INTO api_keys (provider_id, label, encrypted_key, nonce, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params!["anthropic", "work", "enc3", "nonce3", now],
        )
        .unwrap();
    }

    #[test]
    fn provider_models_pk_on_provider_and_model() {
        let conn = fresh_conn();
        let now = "2026-04-17T00:00:00Z";
        conn.execute(
            "INSERT INTO provider_models (provider_id, model_id, fetched_at) VALUES (?1, ?2, ?3)",
            params!["openai", "gpt-4", now],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO provider_models (provider_id, model_id, fetched_at) VALUES (?1, ?2, ?3)",
            params!["openai", "gpt-4", now],
        );
        assert!(dup.is_err(), "duplicate (provider,model) should violate PK");
    }

    #[test]
    fn v6_creates_pricing_metadata_table() {
        let conn = fresh_conn();
        let cols = table_columns(&conn, "pricing_metadata");
        for expected in ["key", "value", "updated_at"] {
            assert!(
                cols.contains(&expected.to_string()),
                "missing col {expected}"
            );
        }
    }

    #[test]
    fn pricing_metadata_key_is_primary() {
        let conn = fresh_conn();
        let now = "2026-04-17T00:00:00Z";
        conn.execute(
            "INSERT INTO pricing_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params!["last_refresh", "2026-04-17T00:00:00Z", now],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO pricing_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params!["last_refresh", "other", now],
        );
        assert!(dup.is_err(), "duplicate key should violate PK");
    }

    #[test]
    fn migrations_are_idempotent_from_v5_to_v6() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Second call should not error (version already at CURRENT_VERSION).
        run_migrations(&conn).unwrap();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_VERSION);
    }
}
