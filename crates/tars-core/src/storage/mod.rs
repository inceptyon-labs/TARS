//! Storage layer (`SQLite` + file bundles)

pub mod backups;
pub mod db;
pub mod migrations;
pub mod plugin_versions;
pub mod profiles;
pub mod projects;

pub use backups::BackupStore;
pub use db::Database;
pub use plugin_versions::PluginVersionStore;
pub use profiles::ProfileStore;
pub use projects::ProjectStore;
