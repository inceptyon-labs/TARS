//! TARS Core - Profile engine, storage, and rollback
//!
//! This crate provides profile management, diff generation,
//! backup/rollback, and `SQLite` storage.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::option_if_let_else,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::ref_option,
    clippy::return_self_not_must_use,
    clippy::needless_pass_by_value,
    clippy::ptr_arg,
    clippy::items_after_statements,
    clippy::similar_names
)]

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
