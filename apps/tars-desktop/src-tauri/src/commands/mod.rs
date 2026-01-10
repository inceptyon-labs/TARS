//! Tauri command modules
//!
//! All IPC commands exposed to the frontend.

pub mod agents;
pub mod apply;
pub mod commands;
pub mod config;
pub mod hooks;
pub mod plugins;
pub mod profiles;
pub mod projects;
pub mod prompts;
pub mod scanner;
pub mod skills;
pub mod updates;
pub mod utils;

// Re-export all commands for easy registration
pub use agents::*;
pub use apply::*;
pub use commands::*;
pub use config::*;
pub use hooks::*;
pub use plugins::*;
pub use profiles::*;
pub use projects::*;
pub use prompts::*;
pub use scanner::*;
pub use skills::*;
pub use updates::*;
pub use utils::*;
