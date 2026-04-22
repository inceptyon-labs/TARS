//! Tauri command modules
//!
//! All IPC commands exposed to the frontend.

pub mod agents;
pub mod api_keys;
pub mod app_data_backup;
pub mod apply;
pub mod beacons;
pub mod commands;
pub mod config;
pub mod developer;
pub mod hooks;
pub mod metadata;
pub mod plugins;
pub mod pricing;
pub mod profiles;
pub mod projects;
pub mod prompts;
pub mod scanner;
pub mod settings;
pub mod skills;
pub mod stats;
pub mod updates;
pub mod utils;

// Re-export all commands for easy registration
pub use agents::*;
pub use api_keys::*;
pub use app_data_backup::*;
pub use apply::*;
pub use beacons::*;
pub use commands::*;
pub use config::*;
pub use developer::*;
pub use hooks::*;
pub use metadata::*;
pub use plugins::*;
pub use pricing::*;
pub use profiles::*;
pub use projects::*;
pub use prompts::*;
pub use scanner::*;
pub use settings::*;
pub use skills::*;
pub use stats::*;
pub use updates::*;
pub use utils::*;
