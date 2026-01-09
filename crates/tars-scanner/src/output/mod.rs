//! Output formatters for inventory

pub mod json;
pub mod markdown;

pub use json::to_json;
pub use markdown::to_markdown;
