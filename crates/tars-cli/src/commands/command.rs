//! Custom command CLI commands
//!
//! Handles: tars command add/remove/move/list

use clap::Subcommand;
use std::path::PathBuf;

/// Custom command commands
#[derive(Subcommand)]
pub enum CommandCommands {
    /// List all custom commands
    List {
        /// Filter by scope
        #[arg(long)]
        scope: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a new custom command
    Add {
        /// Command name
        name: String,
        /// Target scope
        #[arg(long, default_value = "project")]
        scope: String,
        /// Import from existing .md file
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Command description
        #[arg(long)]
        description: Option<String>,
        /// Enable thinking mode
        #[arg(long)]
        thinking: bool,
        /// Command body/template
        #[arg(long)]
        body: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a custom command
    Remove {
        /// Command name
        name: String,
        /// Scope to remove from
        #[arg(long)]
        scope: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Move a custom command between scopes
    Move {
        /// Command name
        name: String,
        /// Source scope
        #[arg(long)]
        from: Option<String>,
        /// Target scope
        #[arg(long)]
        to: String,
        /// Overwrite if exists
        #[arg(long)]
        force: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Execute command command
pub fn execute(
    cmd: CommandCommands,
    _project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        CommandCommands::List { .. } => {
            println!("Command list not yet implemented");
        }
        CommandCommands::Add { .. } => {
            println!("Command add not yet implemented");
        }
        CommandCommands::Remove { .. } => {
            println!("Command remove not yet implemented");
        }
        CommandCommands::Move { .. } => {
            println!("Command move not yet implemented");
        }
    }
    Ok(())
}
