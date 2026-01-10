//! Agent CLI commands
//!
//! Handles: tars agent add/remove/move/list

use clap::Subcommand;
use std::path::PathBuf;

/// Agent commands
#[derive(Subcommand)]
pub enum AgentCommands {
    /// List all custom agents
    List {
        /// Filter by scope
        #[arg(long)]
        scope: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a new custom agent
    Add {
        /// Agent name
        name: String,
        /// Target scope
        #[arg(long, default_value = "project")]
        scope: String,
        /// Import from existing .md file
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Agent description
        #[arg(long)]
        description: Option<String>,
        /// Allowed tools (can specify multiple times)
        #[arg(long)]
        tools: Vec<String>,
        /// Preferred model
        #[arg(long)]
        model: Option<String>,
        /// Permission mode (ask, auto, deny)
        #[arg(long)]
        permission_mode: Option<String>,
        /// Available skills (can specify multiple times)
        #[arg(long)]
        skills: Vec<String>,
        /// Agent body/instructions
        #[arg(long)]
        body: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a custom agent
    Remove {
        /// Agent name
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
    /// Move a custom agent between scopes
    Move {
        /// Agent name
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

/// Execute agent command
#[allow(dead_code)]
pub fn execute(
    cmd: AgentCommands,
    _project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AgentCommands::List { .. } => {
            println!("Agent list not yet implemented");
        }
        AgentCommands::Add { .. } => {
            println!("Agent add not yet implemented");
        }
        AgentCommands::Remove { .. } => {
            println!("Agent remove not yet implemented");
        }
        AgentCommands::Move { .. } => {
            println!("Agent move not yet implemented");
        }
    }
    Ok(())
}
