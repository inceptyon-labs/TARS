//! Skill CLI commands
//!
//! Handles: tars skill add/remove/move/list

use clap::Subcommand;
use std::path::PathBuf;

/// Skill commands
#[derive(Subcommand)]
pub enum SkillCommands {
    /// List all skills
    List {
        /// Filter by scope
        #[arg(long)]
        scope: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a new skill
    Add {
        /// Skill name
        name: String,
        /// Target scope
        #[arg(long, default_value = "project")]
        scope: String,
        /// Import from existing SKILL.md file
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Skill description
        #[arg(long)]
        description: Option<String>,
        /// Allow direct user invocation (/skill-name)
        #[arg(long)]
        user_invocable: bool,
        /// Allowed tools (can specify multiple times)
        #[arg(long = "allowed-tools")]
        allowed_tools: Vec<String>,
        /// Preferred model
        #[arg(long)]
        model: Option<String>,
        /// Skill body/instructions
        #[arg(long)]
        body: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Remove a skill
    Remove {
        /// Skill name
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
    /// Move a skill between scopes
    Move {
        /// Skill name
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

/// Execute skill command
#[allow(dead_code)]
pub fn execute(
    cmd: SkillCommands,
    _project_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        SkillCommands::List { .. } => {
            println!("Skill list not yet implemented");
        }
        SkillCommands::Add { .. } => {
            println!("Skill add not yet implemented");
        }
        SkillCommands::Remove { .. } => {
            println!("Skill remove not yet implemented");
        }
        SkillCommands::Move { .. } => {
            println!("Skill move not yet implemented");
        }
    }
    Ok(())
}
