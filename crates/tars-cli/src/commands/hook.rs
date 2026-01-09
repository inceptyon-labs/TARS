//! Hook CLI commands
//!
//! Handles: tars hook add/remove/list

use clap::Subcommand;
use std::path::PathBuf;

/// Hook commands
#[derive(Subcommand)]
pub enum HookCommands {
    /// List all hooks
    List {
        /// Filter by scope
        #[arg(long)]
        scope: Option<String>,
        /// Filter by trigger type
        #[arg(long)]
        trigger: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a new hook
    Add {
        /// Target scope
        #[arg(long, default_value = "project")]
        scope: String,
        /// Hook trigger type
        #[arg(long)]
        trigger: String,
        /// Matcher pattern (for tool-specific hooks)
        #[arg(long)]
        matcher: Option<String>,
        /// Shell command to run
        #[arg(long)]
        command: Option<String>,
        /// Prompt to inject
        #[arg(long)]
        prompt: Option<String>,
        /// Agent to invoke
        #[arg(long)]
        agent: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a hook
    Remove {
        /// Scope to remove from
        #[arg(long)]
        scope: String,
        /// Hook trigger type
        #[arg(long)]
        trigger: String,
        /// Index of hook to remove (if multiple hooks for same trigger)
        #[arg(long)]
        index: Option<usize>,
        /// Remove all hooks of this trigger type
        #[arg(long)]
        all: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Execute hook command
pub fn execute(cmd: HookCommands, _project_path: Option<&PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        HookCommands::List { .. } => {
            println!("Hook list not yet implemented");
        }
        HookCommands::Add { .. } => {
            println!("Hook add not yet implemented");
        }
        HookCommands::Remove { .. } => {
            println!("Hook remove not yet implemented");
        }
    }
    Ok(())
}
