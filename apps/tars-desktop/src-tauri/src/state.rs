//! Application state management for Tauri
//!
//! Manages database connections and shared state across commands.

use std::path::PathBuf;
use std::sync::Mutex;
use tars_core::storage::Database;

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Database connection (wrapped in Mutex for thread safety)
    db: Mutex<Option<Database>>,
    /// Data directory path
    data_dir: PathBuf,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let data_dir = get_data_dir();
        Self {
            db: Mutex::new(None),
            data_dir,
        }
    }

    /// Initialize the database connection
    ///
    /// # Errors
    /// Returns an error if database initialization fails
    pub fn init_database(&self) -> Result<(), String> {
        let db_path = self.data_dir.join("tars.db");

        // Ensure data directory exists
        std::fs::create_dir_all(&self.data_dir)
            .map_err(|e| format!("Failed to create data directory: {e}"))?;

        let db = Database::open(&db_path)
            .map_err(|e| format!("Failed to open database: {e}"))?;

        // Handle mutex poisoning by recovering the lock
        let mut guard = self
            .db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(db);

        Ok(())
    }

    /// Check if the database is initialized
    pub fn is_initialized(&self) -> bool {
        let guard = self
            .db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.is_some()
    }

    /// Get a reference to the database
    ///
    /// # Errors
    /// Returns an error if database is not initialized
    pub fn with_db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Database) -> Result<T, String>,
    {
        // Handle mutex poisoning by recovering the lock
        let guard = self
            .db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let db = guard
            .as_ref()
            .ok_or_else(|| "Database not initialized. Please restart the application.".to_string())?;
        f(db)
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the application data directory
///
/// Priority:
/// 1. $HOME/.tars (Unix-style, most predictable)
/// 2. Standard XDG data directory on Linux
/// 3. App data directory on Windows/macOS
fn get_data_dir() -> PathBuf {
    // Prefer $HOME/.tars for consistency with CLI tools
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".tars");
    }

    // Windows fallback: use USERPROFILE
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        return PathBuf::from(userprofile).join(".tars");
    }

    // XDG fallback for Linux
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg_data).join("tars");
    }

    // Last resort: use a temp directory with a warning
    // This ensures the app can still function but data won't persist across reboots
    let temp = std::env::temp_dir().join("tars-data");
    eprintln!(
        "Warning: Could not determine home directory. Using temporary location: {}",
        temp.display()
    );
    temp
}
