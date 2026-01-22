//! Config Operations Module
//!
//! Provides granular CRUD operations for Claude Code configuration items:
//! - MCP servers
//! - Skills
//! - Hooks
//! - Commands
//! - Agents
//!
//! This module sits between the scanner (read) and profile engine (write),
//! enabling surgical add/remove/move/update operations on individual config
//! items without overwriting entire config files.

mod error;
mod item;
mod ops;
mod scope;

// Item-specific modules
mod agent;
mod command;
mod hook;
mod mcp;
mod mcp_ops;
mod skill;

// Re-exports
pub use error::{ConfigError, ConfigResult};
pub use item::{validate_name, ConfigItem, ConfigItemData, ConfigItemType};
pub use ops::{ConfigOps, OperationPlan, OperationResult, OperationType};
pub use scope::ConfigScope;

// Item-specific re-exports
pub use agent::AgentConfig;
pub use command::CommandConfig;
pub use hook::{HookConfig, HookDefinition, HookTrigger};
pub use mcp::{McpServerConfig, McpServerUpdate, McpTransport};
pub use mcp_ops::McpOps;
pub use skill::SkillConfig;
