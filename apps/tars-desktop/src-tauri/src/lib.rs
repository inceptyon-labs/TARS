//! TARS Desktop Application
//!
//! Tauri-based desktop app for managing Claude Code configuration.

mod commands;
mod state;

use state::AppState;

/// Run the Tauri application
///
/// # Panics
/// Panics if the Tauri application fails to start
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    // Initialize database
    if let Err(e) = app_state.init_database() {
        eprintln!("Failed to initialize database: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Scanner commands
            commands::scan_project,
            commands::scan_user_scope,
            commands::scan_projects,
            // Project commands
            commands::list_projects,
            commands::add_project,
            commands::get_project,
            commands::remove_project,
            commands::read_claude_md,
            commands::save_claude_md,
            commands::delete_claude_md,
            commands::get_context_stats,
            // Profile commands
            commands::list_profiles,
            commands::create_profile,
            commands::get_profile,
            commands::delete_profile,
            commands::export_profile_as_plugin,
            // Apply commands
            commands::preview_apply,
            commands::apply_profile,
            commands::list_backups,
            commands::rollback,
            // Skill commands
            commands::read_skill,
            commands::read_supporting_file,
            commands::save_skill,
            commands::save_supporting_file,
            commands::create_skill,
            commands::delete_skill,
            commands::delete_supporting_file,
            // Agent commands
            commands::read_agent,
            commands::save_agent,
            commands::create_agent,
            commands::delete_agent,
            commands::move_agent,
            commands::disable_agent,
            commands::enable_agent,
            commands::list_disabled_agents,
            // Command commands
            commands::read_command,
            commands::save_command,
            commands::create_command,
            commands::delete_command,
            commands::move_command,
            // Hook commands
            commands::get_user_hooks,
            commands::get_project_hooks,
            commands::save_user_hooks,
            commands::save_project_hooks,
            commands::get_hook_event_types,
            // MCP config commands
            commands::mcp_list,
            commands::mcp_add,
            commands::mcp_remove,
            commands::mcp_update,
            commands::mcp_move,
            commands::config_rollback,
            // Plugin commands
            commands::plugin_marketplace_add,
            commands::plugin_marketplace_remove,
            commands::plugin_marketplace_update,
            commands::plugin_marketplace_set_auto_update,
            commands::plugin_install,
            commands::plugin_uninstall,
            commands::plugin_move_scope,
            commands::plugin_enable,
            commands::plugin_disable,
            // Cache commands
            commands::cache_status,
            commands::cache_clean,
            // Plugin skill commands
            commands::open_claude_with_skill,
            // Prompts commands
            commands::list_prompts,
            commands::read_prompt,
            commands::create_prompt,
            commands::update_prompt,
            commands::delete_prompt,
            // Utility commands
            commands::directory_exists,
            commands::get_directory_info,
            commands::get_home_dir,
            commands::list_subdirectories,
            commands::get_app_version,
            // Update commands
            commands::get_installed_claude_version,
            commands::fetch_claude_changelog,
            commands::get_claude_version_info,
            commands::check_plugin_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
