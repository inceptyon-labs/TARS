//! TARS CLI - Command-line interface for TARS
//!
//! Provides `tars scan`, `tars profile`, `tars mcp`, and other commands.

mod commands;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use tars_core::backup::restore::restore_from_backup;
use tars_core::diff::display::{format_plan_terminal, DiffSummary};
use tars_core::diff::plan::generate_plan;
use tars_core::export::export_as_plugin;
use tars_core::profile::snapshot::snapshot_from_project;
use tars_core::storage::{BackupStore, Database, ProfileStore, ProjectStore};
use tars_core::{Backup, Project};
use tars_scanner::output::{json::to_json, markdown::to_markdown};
use tars_scanner::{CacheCleanupReport, Scanner};
use uuid::Uuid;

use commands::mcp::McpCommands;

#[derive(Parser)]
#[command(name = "tars")]
#[command(about = "TARS - Claude Code configuration manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan Claude Code configuration
    Scan {
        /// Project directories to scan (in addition to user scope)
        #[arg(value_name = "PROJECT")]
        projects: Vec<PathBuf>,

        /// Output directory for inventory files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "both")]
        format: OutputFormat,

        /// Include managed scope (/etc/claude/)
        #[arg(long)]
        include_managed: bool,
    },
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
    },
    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        action: McpCommands,
        /// Project directory (defaults to current directory)
        #[arg(short, long, global = true)]
        project: Option<PathBuf>,
    },
    /// Manage plugin cache
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    Markdown,
    Both,
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List all profiles
    List,
    /// Create a new profile from current state
    Create {
        /// Profile name
        name: String,
        /// Source project path (defaults to current directory)
        #[arg(short, long)]
        source: Option<String>,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Apply a profile to a project
    Apply {
        /// Profile name or ID
        profile: String,
        /// Target project path
        target: String,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
    /// Rollback to a previous state
    Rollback {
        /// Backup ID to restore
        backup_id: String,
        /// Target project path
        target: String,
    },
    /// Show profile details
    Show {
        /// Profile name or ID
        profile: String,
    },
    /// Delete a profile
    Delete {
        /// Profile name or ID
        profile: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Export a profile as a Claude Code plugin
    Export {
        /// Profile name or ID
        profile: String,
        /// Output directory (defaults to current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Plugin name (defaults to profile name)
        #[arg(short, long)]
        name: Option<String>,
        /// Plugin version
        #[arg(short, long, default_value = "1.0.0")]
        version: String,
    },
    /// List all backups
    Backups {
        /// Filter by project path
        #[arg(short, long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show stale plugin cache that can be cleaned
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Clean stale plugin cache
    Clean {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Preview what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            projects,
            output,
            format,
            include_managed,
        } => {
            if let Err(e) = run_scan(&projects, &output, format, include_managed) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Profile { action } => {
            if let Err(e) = run_profile_command(action) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Mcp { action, project } => {
            if let Err(e) = commands::mcp::execute(action, project.as_ref()) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Cache { action } => {
            if let Err(e) = run_cache_command(action) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn get_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try HOME first (Unix), then USERPROFILE (Windows)
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| "HOME or USERPROFILE environment variable not set")?;
    Ok(home.join(".tars"))
}

/// Validate a name for use in file paths (profile names, plugin names, etc.)
fn validate_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Name cannot contain path separators or '..'".into());
    }
    if name.starts_with('.') {
        return Err("Name cannot start with '.'".into());
    }
    if name.contains('\0') {
        return Err("Name cannot contain null bytes".into());
    }
    // Check for common reserved names on Windows
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];
    if reserved.iter().any(|r| name.eq_ignore_ascii_case(r)) {
        return Err("Name uses a reserved system name".into());
    }
    Ok(())
}

/// Look up a profile by name or UUID
fn find_profile(
    profiles: &ProfileStore,
    identifier: &str,
) -> Result<tars_core::Profile, Box<dyn std::error::Error>> {
    let profile = if let Ok(id) = Uuid::parse_str(identifier) {
        profiles.get(id)?
    } else {
        profiles.get_by_name(identifier)?
    }
    .ok_or_else(|| format!("Profile not found: {identifier}"))?;
    Ok(profile)
}

fn run_profile_command(action: ProfileCommands) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = get_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("tars.db");
    let db = Database::open(&db_path)?;
    let profiles = ProfileStore::new(db.connection());
    let projects = ProjectStore::new(db.connection());
    let backups = BackupStore::new(db.connection());

    match action {
        ProfileCommands::List => {
            let profile_list = profiles.list()?;
            if profile_list.is_empty() {
                println!("No profiles found.");
            } else {
                println!("Profiles:");
                for p in profile_list {
                    let desc = p.description.as_deref().unwrap_or("No description");
                    println!("  {} - {} ({})", p.id, p.name, desc);
                }
            }
        }
        ProfileCommands::Create { name, source, description } => {
            // Validate profile name
            validate_name(&name)?;

            let source_path = source
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

            if !source_path.exists() {
                return Err(format!("Source path does not exist: {}", source_path.display()).into());
            }

            println!("Creating profile '{}' from: {}", name, source_path.display());

            let mut profile = snapshot_from_project(&source_path, name)?;
            if let Some(desc) = description {
                profile.description = Some(desc);
            }
            profiles.create(&profile)?;

            println!("Created profile: {}", profile.id);
        }
        ProfileCommands::Apply {
            profile,
            target,
            dry_run,
        } => {
            let target_path = PathBuf::from(&target);

            if !target_path.exists() {
                return Err(format!("Target path does not exist: {}", target_path.display()).into());
            }

            // Find profile by name or ID
            let prof = find_profile(&profiles, &profile)?;

            // Find or create project
            let proj = match projects.get_by_path(&target_path)? {
                Some(p) => p,
                None => {
                    let p = Project::new(target_path.clone());
                    projects.create(&p)?;
                    p
                }
            };

            // Generate diff plan
            let plan = generate_plan(proj.id, &target_path, &prof)?;

            if plan.is_empty() {
                println!("No changes needed - project already matches profile.");
                return Ok(());
            }

            // Show plan
            println!("{}", format_plan_terminal(&plan));
            let summary = DiffSummary::from_plan(&plan);
            println!("Summary: {}", summary.one_line());

            if dry_run {
                println!("\nDry run - no changes made.");
                return Ok(());
            }

            // Create backup and apply
            let backup_dir = data_dir.join("backups");
            std::fs::create_dir_all(&backup_dir)?;

            let archive_path = backup_dir.join(format!("backup-{}.json", chrono::Utc::now().format("%Y%m%d-%H%M%S")));
            let mut backup = Backup::new(proj.id, archive_path.clone())
                .with_profile(prof.id)
                .with_description(format!("Before applying profile '{}'", prof.name));

            tars_core::apply::apply_operations(&plan, &target_path, &mut backup)?;

            // Save backup
            let backup_json = serde_json::to_string_pretty(&backup)?;
            std::fs::write(&archive_path, backup_json)?;
            backups.create(&backup)?;

            println!("\nApplied {} operations.", plan.operations.len());
            println!("Backup created: {}", backup.id);
        }
        ProfileCommands::Rollback { backup_id, target } => {
            let target_path = PathBuf::from(&target);

            let id = Uuid::parse_str(&backup_id)?;
            let backup = backups.get(id)?.ok_or("Backup not found")?;

            // Verify backup integrity
            tars_core::backup::restore::verify_backup_integrity(&backup)?;

            // Restore
            restore_from_backup(&target_path, &backup)?;

            println!("Rolled back {} files.", backup.files.len());
        }
        ProfileCommands::Show { profile } => {
            let prof = find_profile(&profiles, &profile)?;

            println!("Profile: {}", prof.name);
            println!("ID: {}", prof.id);
            if let Some(desc) = &prof.description {
                println!("Description: {desc}");
            }
            println!("Created: {}", prof.created_at);
            println!("Updated: {}", prof.updated_at);
            println!("\nRepo Overlays:");
            println!("  Skills: {}", prof.repo_overlays.skills.len());
            println!("  Commands: {}", prof.repo_overlays.commands.len());
            println!("  Agents: {}", prof.repo_overlays.agents.len());
            println!("  CLAUDE.md: {}", prof.repo_overlays.claude_md.is_some());
            println!("\nUser Overlays:");
            println!("  Skills: {}", prof.user_overlays.skills.len());
            println!("  Commands: {}", prof.user_overlays.commands.len());
        }
        ProfileCommands::Delete { profile, force } => {
            let prof = find_profile(&profiles, &profile)?;

            if !force {
                print!("Delete profile '{}' (ID: {})? [y/N] ", prof.name, prof.id);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            profiles.delete(prof.id)?;
            println!("Deleted profile: {}", prof.name);
        }
        ProfileCommands::Export {
            profile,
            output,
            name,
            version,
        } => {
            let prof = find_profile(&profiles, &profile)?;

            let output_dir = output.unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
            let plugin_name = name.unwrap_or_else(|| prof.name.clone());

            // Validate plugin name and version for use in filenames
            validate_name(&plugin_name)?;
            if version.contains('/') || version.contains('\\') || version.contains("..") {
                return Err("Version cannot contain path separators".into());
            }

            if !output_dir.exists() {
                return Err(format!("Output directory does not exist: {}", output_dir.display()).into());
            }

            println!(
                "Exporting profile '{}' as plugin '{}' v{}...",
                prof.name, plugin_name, version
            );

            export_as_plugin(&prof, &output_dir, &plugin_name, &version)?;
            let output_path = output_dir.join(format!("{}-{}", plugin_name, version));
            println!("Created plugin: {}", output_path.display());
        }
        ProfileCommands::Backups { project } => {
            let backup_list = backups.list_all()?;
            let filtered: Vec<_> = if let Some(proj_path) = project {
                let proj_path = PathBuf::from(&proj_path);
                if let Some(proj) = projects.get_by_path(&proj_path)? {
                    backup_list
                        .into_iter()
                        .filter(|b| b.project_id == proj.id)
                        .collect()
                } else {
                    println!("Project not found: {}", proj_path.display());
                    return Ok(());
                }
            } else {
                backup_list
            };

            if filtered.is_empty() {
                println!("No backups found.");
            } else {
                println!("Backups:");
                for b in filtered {
                    let desc = b.description.as_deref().unwrap_or("No description");
                    println!("  {} - {} ({})", b.id, b.created_at.format("%Y-%m-%d %H:%M"), desc);
                }
            }
        }
    }

    Ok(())
}

fn run_cache_command(action: CacheCommands) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CacheCommands::Status { json } => {
            let report = CacheCleanupReport::scan()?;

            if json {
                let output = serde_json::to_string_pretty(&report)?;
                println!("{output}");
            } else {
                println!("Plugin Cache Status");
                println!("===================\n");
                println!("Installed plugins: {}", report.installed_count);
                println!("Stale cache entries: {}", report.stale_entries.len());
                println!("Total stale size: {}\n", report.format_size());

                if report.stale_entries.is_empty() {
                    println!("No stale cache to clean up.");
                } else {
                    println!("Stale entries:");
                    for entry in &report.stale_entries {
                        println!(
                            "  {}@{} v{} ({:.2} KB)",
                            entry.plugin_name,
                            entry.marketplace,
                            entry.version,
                            entry.size_bytes as f64 / 1024.0
                        );
                    }
                    println!("\nRun 'tars cache clean' to remove stale cache.");
                }
            }
        }
        CacheCommands::Clean { force, dry_run } => {
            let report = CacheCleanupReport::scan()?;

            if report.stale_entries.is_empty() {
                println!("No stale cache to clean up.");
                return Ok(());
            }

            println!("Found {} stale cache entries ({})",
                report.stale_entries.len(),
                report.format_size()
            );

            if dry_run {
                println!("\nWould delete:");
                for entry in &report.stale_entries {
                    println!(
                        "  {} ({})",
                        entry.path.display(),
                        format_entry_size(entry.size_bytes)
                    );
                }
                println!("\nDry run - no changes made.");
                return Ok(());
            }

            if !force {
                print!("\nDelete these entries? [y/N] ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            let result = report.clean()?;

            println!("\nCleaned up {} entries, freed {}",
                result.deleted_count,
                result.format_size()
            );

            if !result.errors.is_empty() {
                println!("\nErrors encountered:");
                for err in &result.errors {
                    println!("  {err}");
                }
            }
        }
    }

    Ok(())
}

/// Format bytes as human-readable string
fn format_entry_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

fn run_scan(
    projects: &[PathBuf],
    output_dir: &Path,
    format: OutputFormat,
    include_managed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning Claude Code configuration...");

    let scanner = Scanner::new().with_managed(include_managed);

    // Convert PathBuf to &Path for the scanner
    let project_refs: Vec<&Path> = projects.iter().map(PathBuf::as_path).collect();

    let inventory = scanner.scan_all(&project_refs)?;

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    // Write output files
    match format {
        OutputFormat::Json => {
            write_json(&inventory, output_dir)?;
        }
        OutputFormat::Markdown => {
            write_markdown(&inventory, output_dir)?;
        }
        OutputFormat::Both => {
            write_json(&inventory, output_dir)?;
            write_markdown(&inventory, output_dir)?;
        }
    }

    // Print summary
    println!("\nScan complete!");
    println!("  User skills: {}", inventory.user_scope.skills.len());
    println!("  User commands: {}", inventory.user_scope.commands.len());
    println!("  User agents: {}", inventory.user_scope.agents.len());
    println!("  Projects scanned: {}", inventory.projects.len());

    if inventory.collisions.has_collisions() {
        println!(
            "\n  Collisions detected: {}",
            inventory.collisions.total_count()
        );
    }

    Ok(())
}

fn write_json(
    inventory: &tars_scanner::Inventory,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = to_json(inventory)?;
    let path = output_dir.join("inventory.json");
    std::fs::write(&path, json)?;
    println!("Wrote JSON inventory to: {}", path.display());
    Ok(())
}

fn write_markdown(
    inventory: &tars_scanner::Inventory,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let md = to_markdown(inventory);
    let path = output_dir.join("inventory.md");
    std::fs::write(&path, md)?;
    println!("Wrote Markdown inventory to: {}", path.display());
    Ok(())
}
