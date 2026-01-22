//! TARS Desktop Application
//!
//! Tauri-based desktop app for managing Claude Code configuration.

#![allow(
    clippy::manual_let_else,
    clippy::needless_continue,
    clippy::ptr_arg,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::module_inception,
    clippy::cast_possible_truncation
)]

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Scanner commands
            commands::scan_project,
            commands::scan_user_scope,
            commands::scan_projects,
            commands::scan_profiles,
            commands::discover_claude_projects,
            // Project commands
            commands::list_projects,
            commands::add_project,
            commands::get_project,
            commands::remove_project,
            commands::read_claude_md,
            commands::save_claude_md,
            commands::delete_claude_md,
            commands::read_project_notes,
            commands::save_project_notes,
            commands::get_context_stats,
            commands::get_projects_git_status,
            commands::get_project_stats,
            // Profile commands
            commands::list_profiles,
            commands::create_profile,
            commands::create_empty_profile,
            commands::get_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::delete_profile_cleanup,
            commands::export_profile_as_plugin,
            // Profile assignment commands
            commands::assign_profile,
            commands::unassign_profile,
            commands::get_project_tools,
            // Local override commands
            commands::add_local_tool,
            commands::remove_local_tool,
            // Add tools from source
            commands::add_tools_from_source,
            commands::create_profile_mcp_server,
            // Plugin profile commands
            commands::add_plugin_to_profile,
            commands::remove_plugin_from_profile,
            commands::list_profile_plugins,
            // Profile export/import commands
            commands::export_profile_json,
            commands::preview_profile_import,
            commands::import_profile_json,
            // Profile update detection commands
            commands::check_profile_updates,
            commands::pull_tool_update,
            commands::set_tool_source_mode,
            commands::assign_profile_as_plugin,
            commands::unassign_profile_plugin,
            // Profile install commands
            commands::install_profile_to_project,
            commands::install_profile_to_user,
            commands::uninstall_profile_from_user,
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
            commands::get_profile_hooks,
            commands::save_profile_hooks,
            commands::get_hook_event_types,
            // Profile MCP commands
            commands::list_profile_mcp_servers,
            commands::remove_profile_mcp_server,
            // MCP config commands
            commands::mcp_list,
            commands::mcp_add,
            commands::mcp_remove,
            commands::mcp_update,
            commands::mcp_move,
            commands::config_rollback,
            // Plugin commands
            commands::plugin_list,
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
            // Beacons commands
            commands::list_beacons,
            commands::read_beacon,
            commands::create_beacon,
            commands::update_beacon,
            commands::delete_beacon,
            // Utility commands
            commands::directory_exists,
            commands::get_directory_info,
            commands::get_home_dir,
            commands::list_subdirectories,
            commands::get_app_version,
            commands::get_platform_info,
            commands::get_claude_usage_stats,
            // Settings commands
            commands::read_settings_file,
            commands::save_settings_file,
            // Update commands
            commands::get_installed_claude_version,
            commands::fetch_claude_changelog,
            commands::fetch_tars_changelog,
            commands::get_claude_version_info,
            commands::check_plugin_updates,
            // TARS app update commands
            commands::check_tars_update,
            commands::install_tars_update,
            commands::get_tars_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
