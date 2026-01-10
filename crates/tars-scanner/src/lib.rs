//! TARS Scanner - Claude Code configuration discovery
//!
//! This crate provides read-only scanning of Claude Code configuration
//! across user, project, and managed scopes.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::unnecessary_debug_formatting,
    clippy::ref_option,
    clippy::option_if_let_else,
    clippy::needless_pass_by_value,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::cast_precision_loss
)]

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
