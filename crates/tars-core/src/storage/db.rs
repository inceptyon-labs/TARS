//! Database connection management

use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

use super::migrations;

/// Database errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    Migration(String),
}

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened
    pub fn open(path: &Path) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;

        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // Performance optimizations
        // WAL mode for better concurrent read/write performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // NORMAL synchronous is safe with WAL and faster than FULL
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        // 10MB cache size for better query performance
        conn.pragma_update(None, "cache_size", "-10000")?;
        // Store temp tables in memory
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Create an in-memory database (for testing)
    ///
    /// # Errors
    /// Returns an error if the database cannot be created
    pub fn in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Get a reference to the connection
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
