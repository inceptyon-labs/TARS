//! Profile types and operations

pub mod export;
pub mod snapshot;
pub mod storage;
pub mod sync;
mod types;
pub mod updates;

pub use export::{ExportError, ExportedTool, ImportPreview, ProfileExport};
pub use storage::{PluginManifest, ProfileTools, ProjectProfileState, StorageError};
pub use sync::{
    assign_profile_as_plugin, install_profile_plugin_to_project, install_profile_plugin_to_user,
    regenerate_profile_plugin, reinstall_profile_plugin, remove_profile_from_marketplace,
    sync_profile_marketplace, unassign_profile_plugin, uninstall_profile_plugin_from_project,
    uninstall_profile_plugin_from_user, ApplyError, ApplyResult, MarketplaceSyncResult,
    PluginAssignResult, SyncResult, PROFILE_MARKETPLACE,
};
pub use types::*;
pub use updates::{
    check_profile_updates, create_source_ref, migrate_legacy_profile, needs_migration,
    set_source_mode, update_source_hash, ProfileUpdateCheck, ToolUpdateInfo,
};
