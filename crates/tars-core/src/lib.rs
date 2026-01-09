//! TARS Core - Profile engine, storage, and rollback
//!
//! This crate provides profile management, diff generation,
//! backup/rollback, and SQLite storage.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod apply;
pub mod backup;
pub mod config;
pub mod diff;
pub mod export;
pub mod profile;
pub mod project;
pub mod storage;
pub mod util;

pub use tars_scanner;

pub use backup::Backup;
pub use diff::DiffPlan;
pub use profile::Profile;
pub use project::Project;
