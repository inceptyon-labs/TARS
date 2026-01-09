//! TARS Scanner - Claude Code configuration discovery
//!
//! This crate provides read-only scanning of Claude Code configuration
//! across user, project, and managed scopes.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod artifacts;
pub mod collision;
pub mod error;
pub mod inventory;
pub mod output;
pub mod parser;
pub mod plugins;
pub mod scan;
pub mod scope;
pub mod settings;
pub mod types;

pub use error::{ScanError, ScanResult};
pub use inventory::Inventory;
pub use plugins::{CacheCleanupReport, CleanupResult, StaleCacheEntry};
pub use scan::Scanner;
