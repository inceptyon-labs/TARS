//! Plugin and marketplace types and scanner

use crate::error::{ScanError, ScanResult};
use crate::types::Scope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Plugin inventory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginInventory {
    /// Configured marketplaces
    #[serde(default)]
    pub marketplaces: Vec<Marketplace>,
    /// Installed plugins
    #[serde(default)]
    pub installed: Vec<InstalledPlugin>,
}

impl PluginInventory {
    /// Scan the Claude Code plugins directory for installed plugins and marketplaces
    ///
    /// # Errors
    /// Returns an error if reading plugin files fails
    pub fn scan() -> ScanResult<Self> {
        let home = dirs::home_dir().ok_or(ScanError::HomeNotFound)?;
        let plugins_dir = home.join(".claude").join("plugins");

        if !plugins_dir.exists() {
            return Ok(Self::default());
        }

        let marketplaces = Self::scan_marketplaces(&plugins_dir)?;
        let installed = Self::scan_installed_plugins(&plugins_dir)?;

        Ok(Self {
            marketplaces,
            installed,
        })
    }

    fn scan_marketplaces(plugins_dir: &Path) -> ScanResult<Vec<Marketplace>> {
        let known_file = plugins_dir.join("known_marketplaces.json");
        if !known_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&known_file)?;
        let raw: HashMap<String, RawMarketplaceEntry> =
            serde_json::from_str(&content).map_err(ScanError::JsonParse)?;

        // Get list of installed plugin IDs for this marketplace
        let installed_plugins = Self::get_installed_plugin_ids(plugins_dir);

        Ok(raw
            .into_iter()
            .map(|(name, entry)| {
                let source_type = match &entry.source {
                    RawMarketplaceSource::GitHub { repo } => {
                        let parts: Vec<&str> = repo.split('/').collect();
                        if parts.len() == 2 {
                            MarketplaceSource::GitHub {
                                owner: parts[0].to_string(),
                                repo: parts[1].to_string(),
                            }
                        } else {
                            MarketplaceSource::Url { url: repo.clone() }
                        }
                    }
                    RawMarketplaceSource::Git { url } => {
                        MarketplaceSource::Url { url: url.clone() }
                    }
                    RawMarketplaceSource::Local { path } => MarketplaceSource::Local {
                        path: PathBuf::from(path),
                    },
                };

                let location = match &entry.source {
                    RawMarketplaceSource::GitHub { repo } => repo.clone(),
                    RawMarketplaceSource::Git { url } => url.clone(),
                    RawMarketplaceSource::Local { path } => path.clone(),
                };

                // Scan available plugins in this marketplace
                let marketplace_dir = plugins_dir.join("marketplaces").join(&name);
                let available_plugins =
                    Self::scan_available_plugins(&marketplace_dir, &name, &installed_plugins);

                Marketplace {
                    name,
                    source_type,
                    location,
                    auto_update: entry.auto_update.unwrap_or(false),
                    available_plugins,
                }
            })
            .collect())
    }

    /// Get set of installed plugin IDs per marketplace
    fn get_installed_plugin_ids(plugins_dir: &Path) -> HashMap<String, Vec<String>> {
        let installed_file = plugins_dir.join("installed_plugins.json");
        if !installed_file.exists() {
            return HashMap::new();
        }

        let content = match fs::read_to_string(&installed_file) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        let raw: RawInstalledPlugins = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => return HashMap::new(),
        };

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for plugin_key in raw.plugins.keys() {
            // Plugin key format: "plugin-name@marketplace"
            let parts: Vec<&str> = plugin_key.split('@').collect();
            let plugin_name = parts.first().copied().unwrap_or(plugin_key);
            let marketplace = parts.get(1).map(|s| (*s).to_string()).unwrap_or_default();

            result
                .entry(marketplace)
                .or_default()
                .push(plugin_name.to_string());
        }
        result
    }

    /// Scan a marketplace directory for available plugins
    fn scan_available_plugins(
        marketplace_dir: &Path,
        marketplace_name: &str,
        installed_plugins: &HashMap<String, Vec<String>>,
    ) -> Vec<AvailablePlugin> {
        // First, check if marketplace.json index exists (used by Claude CLI)
        // This is the authoritative source for what plugins are actually available
        let marketplace_json_path = marketplace_dir
            .join(".claude-plugin")
            .join("marketplace.json");
        if marketplace_json_path.exists() {
            if let Some(plugins) = Self::parse_marketplace_json(
                &marketplace_json_path,
                marketplace_name,
                installed_plugins,
            ) {
                return plugins;
            }
        }

        // Fall back to filesystem scanning for marketplaces without marketplace.json
        Self::scan_available_plugins_from_filesystem(
            marketplace_dir,
            marketplace_name,
            installed_plugins,
        )
    }

    /// Parse marketplace.json index file to get available plugins
    fn parse_marketplace_json(
        marketplace_json_path: &Path,
        marketplace_name: &str,
        installed_plugins: &HashMap<String, Vec<String>>,
    ) -> Option<Vec<AvailablePlugin>> {
        let content = fs::read_to_string(marketplace_json_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let plugins_array = json.get("plugins")?.as_array()?;
        let mut available = Vec::new();

        for plugin in plugins_array {
            let name = plugin.get("name")?.as_str()?.to_string();
            let description = plugin
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let version = plugin
                .get("version")
                .and_then(|v| v.as_str())
                .map(String::from);
            let author = plugin.get("author").and_then(|a| {
                a.get("name").and_then(|n| n.as_str()).map(|n| Author {
                    name: n.to_string(),
                    email: a.get("email").and_then(|e| e.as_str()).map(String::from),
                })
            });

            let is_installed = installed_plugins
                .get(marketplace_name)
                .is_some_and(|ids| ids.contains(&name));

            available.push(AvailablePlugin {
                id: name.clone(),
                name: name.clone(),
                description,
                version,
                author,
                installed: is_installed,
            });
        }

        // Also check external_plugins array if present
        if let Some(external_array) = json.get("external_plugins").and_then(|e| e.as_array()) {
            for plugin in external_array {
                let name = match plugin.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let description = plugin
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = plugin
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let author = plugin.get("author").and_then(|a| {
                    a.get("name").and_then(|n| n.as_str()).map(|n| Author {
                        name: n.to_string(),
                        email: a.get("email").and_then(|e| e.as_str()).map(String::from),
                    })
                });

                let is_installed = installed_plugins
                    .get(marketplace_name)
                    .is_some_and(|ids| ids.contains(&name));

                available.push(AvailablePlugin {
                    id: name.clone(),
                    name: name.clone(),
                    description,
                    version,
                    author,
                    installed: is_installed,
                });
            }
        }

        // Sort by name
        available.sort_by(|a, b| a.name.cmp(&b.name));
        Some(available)
    }

    /// Scan filesystem for available plugins (fallback when no marketplace.json)
    fn scan_available_plugins_from_filesystem(
        marketplace_dir: &Path,
        marketplace_name: &str,
        installed_plugins: &HashMap<String, Vec<String>>,
    ) -> Vec<AvailablePlugin> {
        let mut available = Vec::new();

        // First, check if the marketplace root itself is a single-plugin marketplace
        // (i.e., the marketplace directory IS the plugin, with .claude-plugin/plugin.json at root)
        let root_manifest_path = marketplace_dir.join(".claude-plugin").join("plugin.json");
        if root_manifest_path.exists() {
            let manifest: Option<PluginManifest> = fs::read_to_string(&root_manifest_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok());

            // For single-plugin marketplaces, the plugin ID is the marketplace name
            let plugin_id = marketplace_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(marketplace_name)
                .to_string();

            let is_installed = installed_plugins
                .get(marketplace_name)
                .is_some_and(|ids| ids.contains(&plugin_id));

            available.push(AvailablePlugin {
                id: plugin_id.clone(),
                name: manifest
                    .as_ref()
                    .map_or_else(|| plugin_id.clone(), |m| m.name.clone()),
                description: manifest
                    .as_ref()
                    .map(|m| m.description.clone())
                    .unwrap_or_default(),
                version: manifest.as_ref().map(|m| m.version.clone()),
                author: manifest.and_then(|m| m.author),
                installed: is_installed,
            });
        }

        // Then check "plugins/" and "external_plugins/" directories for multi-plugin marketplaces
        for subdir in &["plugins", "external_plugins"] {
            let plugins_path = marketplace_dir.join(subdir);
            if !plugins_path.exists() {
                continue;
            }

            let entries = match fs::read_dir(&plugins_path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let plugin_id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // Skip hidden directories
                if plugin_id.starts_with('.') {
                    continue;
                }

                // Read plugin manifest
                let manifest_path = path.join(".claude-plugin").join("plugin.json");
                let manifest: Option<PluginManifest> = if manifest_path.exists() {
                    fs::read_to_string(&manifest_path)
                        .ok()
                        .and_then(|c| serde_json::from_str(&c).ok())
                } else {
                    None
                };

                let is_installed = installed_plugins
                    .get(marketplace_name)
                    .is_some_and(|ids| ids.contains(&plugin_id));

                available.push(AvailablePlugin {
                    id: plugin_id.clone(),
                    name: manifest
                        .as_ref()
                        .map_or_else(|| plugin_id.clone(), |m| m.name.clone()),
                    description: manifest
                        .as_ref()
                        .map(|m| m.description.clone())
                        .unwrap_or_default(),
                    version: manifest.as_ref().map(|m| m.version.clone()),
                    author: manifest.and_then(|m| m.author),
                    installed: is_installed,
                });
            }
        }

        // Sort by name
        available.sort_by(|a, b| a.name.cmp(&b.name));
        available
    }

    fn scan_installed_plugins(plugins_dir: &Path) -> ScanResult<Vec<InstalledPlugin>> {
        let installed_file = plugins_dir.join("installed_plugins.json");
        if !installed_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&installed_file)?;
        let raw: RawInstalledPlugins =
            serde_json::from_str(&content).map_err(ScanError::JsonParse)?;

        // Read enabled state from settings.json
        let enabled_map = Self::read_plugins_enabled_state(plugins_dir);

        let mut plugins = Vec::new();

        for (plugin_key, installs) in raw.plugins {
            // Plugin key format: "plugin-name@marketplace"
            let parts: Vec<&str> = plugin_key.split('@').collect();
            let plugin_name = parts.first().copied().unwrap_or(&plugin_key);
            let marketplace = parts.get(1).map(|s| (*s).to_string());

            for install in installs {
                // Try multiple locations for plugin.json:
                // 1. install_path/plugin.json (legacy)
                // 2. install_path/.claude-plugin/plugin.json
                // 3. marketplaces/{marketplace}/plugins/{plugin}/.claude-plugin/plugin.json
                // 4. marketplaces/{marketplace}/.claude-plugin/plugin.json (single-plugin marketplace)
                let manifest = Self::find_and_read_manifest(
                    plugins_dir,
                    &install.install_path,
                    plugin_name,
                    marketplace.as_deref(),
                );

                let scope = match install.scope.as_str() {
                    "user" => Scope::User,
                    "project" => Scope::Project,
                    "local" => Scope::Local,
                    "managed" => Scope::Managed,
                    _ => Scope::User,
                };

                // Check enabled state from settings, default to true if not specified
                let enabled = enabled_map.get(&plugin_key).copied().unwrap_or(true);

                plugins.push(InstalledPlugin {
                    id: plugin_name.to_string(),
                    marketplace: marketplace.clone(),
                    version: install.version.clone(),
                    scope,
                    enabled,
                    path: PathBuf::from(&install.install_path),
                    manifest: manifest.unwrap_or_else(|| PluginManifest {
                        name: plugin_name.to_string(),
                        version: "unknown".to_string(),
                        description: String::new(),
                        author: None,
                        commands: Vec::new(),
                        agents: None,
                        skills: None,
                        hooks: None,
                        mcp_servers: None,
                        parsed_skills: Vec::new(),
                    }),
                    installed_at: install.installed_at,
                    last_updated: install.last_updated,
                    project_path: install.project_path,
                });
            }
        }

        Ok(plugins)
    }

    /// Read the plugins enabled state from ~/.claude/settings.json
    fn read_plugins_enabled_state(plugins_dir: &Path) -> HashMap<String, bool> {
        // settings.json is in the parent directory (.claude)
        let settings_path = plugins_dir
            .parent()
            .map(|p| p.join("settings.json"))
            .unwrap_or_default();

        if !settings_path.exists() {
            return HashMap::new();
        }

        let content = match fs::read_to_string(&settings_path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        // Parse settings.json and extract the "enabledPlugins" key
        let settings: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return HashMap::new(),
        };

        let Some(plugins_obj) = settings.get("enabledPlugins").and_then(|p| p.as_object()) else {
            return HashMap::new();
        };

        plugins_obj
            .iter()
            .filter_map(|(key, value)| value.as_bool().map(|enabled| (key.clone(), enabled)))
            .collect()
    }

    /// Find and read manifest from multiple possible locations
    fn find_and_read_manifest(
        plugins_dir: &Path,
        install_path: &str,
        plugin_name: &str,
        marketplace: Option<&str>,
    ) -> Option<PluginManifest> {
        let install_path = PathBuf::from(install_path);

        // Try locations in order of likelihood
        let possible_paths = [
            // 1. install_path/.claude-plugin/plugin.json
            install_path.join(".claude-plugin").join("plugin.json"),
            // 2. install_path/plugin.json (legacy)
            install_path.join("plugin.json"),
        ];

        for path in &possible_paths {
            if let Some(manifest) = Self::read_manifest(path, plugin_name) {
                return Some(manifest);
            }
        }

        // 3. Try marketplace directory if we have marketplace info
        if let Some(mkt) = marketplace {
            // Multi-plugin marketplace: marketplaces/{marketplace}/plugins/{plugin}/.claude-plugin/plugin.json
            let marketplace_plugin_path = plugins_dir
                .join("marketplaces")
                .join(mkt)
                .join("plugins")
                .join(plugin_name)
                .join(".claude-plugin")
                .join("plugin.json");

            if let Some(manifest) = Self::read_manifest(&marketplace_plugin_path, plugin_name) {
                return Some(manifest);
            }

            // Single-plugin marketplace: marketplaces/{marketplace}/.claude-plugin/plugin.json
            let single_plugin_path = plugins_dir
                .join("marketplaces")
                .join(mkt)
                .join(".claude-plugin")
                .join("plugin.json");

            if let Some(manifest) = Self::read_manifest(&single_plugin_path, plugin_name) {
                return Some(manifest);
            }

            // Multi-plugin marketplace with marketplace.json
            let marketplace_json_path = plugins_dir
                .join("marketplaces")
                .join(mkt)
                .join(".claude-plugin")
                .join("marketplace.json");

            if let Some(manifest) =
                Self::read_manifest_from_marketplace_json(&marketplace_json_path, plugin_name)
            {
                return Some(manifest);
            }
        }

        None
    }

    /// Read manifest from marketplace.json (for multi-plugin marketplaces)
    fn read_manifest_from_marketplace_json(
        path: &Path,
        plugin_name: &str,
    ) -> Option<PluginManifest> {
        let content = fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        // Find the plugin in the plugins array
        let plugins = json.get("plugins")?.as_array()?;

        for plugin in plugins {
            let name = plugin.get("name")?.as_str()?;
            if name == plugin_name {
                // Extract commands array
                let commands: Vec<PathBuf> = plugin
                    .get("commands")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(PathBuf::from))
                            .collect()
                    })
                    .unwrap_or_default();

                // Extract description
                let description = plugin
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                // Get owner info from marketplace root
                let author = json.get("owner").and_then(|o| {
                    Some(Author {
                        name: o.get("name")?.as_str()?.to_string(),
                        email: o.get("email").and_then(|e| e.as_str()).map(String::from),
                    })
                });

                // Parse skills from commands
                let parsed_skills = Self::parse_skills_from_commands(&commands, plugin_name);

                return Some(PluginManifest {
                    name: name.to_string(),
                    version: json
                        .get("metadata")
                        .and_then(|m| m.get("version"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    description,
                    author,
                    commands,
                    agents: None,
                    skills: plugin.get("skills").and_then(|s| {
                        s.as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|v| v.as_str())
                            .map(PathBuf::from)
                    }),
                    hooks: None,
                    mcp_servers: None,
                    parsed_skills,
                });
            }
        }

        None
    }

    fn read_manifest(path: &Path, plugin_id: &str) -> Option<PluginManifest> {
        let content = fs::read_to_string(path).ok()?;
        let mut manifest: PluginManifest = serde_json::from_str(&content).ok()?;

        // Parse skills from command paths
        manifest.parsed_skills = Self::parse_skills_from_commands(&manifest.commands, plugin_id);

        Some(manifest)
    }

    /// Parse skill information from command file paths
    fn parse_skills_from_commands(commands: &[PathBuf], plugin_id: &str) -> Vec<PluginSkillInfo> {
        commands
            .iter()
            .filter_map(|cmd_path| {
                // Extract skill name from path like "./commands/notifications-init.md"
                let file_name = cmd_path.file_stem()?.to_str()?;
                let name = file_name.to_string();
                let invocation = format!("/{plugin_id}:{name}");

                // Detect init/settings skills by name patterns
                let name_lower = name.to_lowercase();
                let is_init = name_lower.contains("init")
                    || name_lower.contains("setup")
                    || name_lower.contains("install");
                let is_settings = name_lower.contains("setting")
                    || name_lower.contains("config")
                    || name_lower.contains("preference");

                Some(PluginSkillInfo {
                    name,
                    invocation,
                    is_init,
                    is_settings,
                })
            })
            .collect()
    }
}

// Raw JSON structures for parsing Claude Code plugin files

#[derive(Debug, Deserialize)]
struct RawMarketplaceEntry {
    source: RawMarketplaceSource,
    #[serde(rename = "installLocation")]
    #[allow(dead_code)]
    install_location: String,
    #[serde(rename = "autoUpdate")]
    auto_update: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "source")]
enum RawMarketplaceSource {
    #[serde(rename = "github")]
    GitHub { repo: String },
    #[serde(rename = "git")]
    Git { url: String },
    #[serde(rename = "local")]
    Local { path: String },
}

#[derive(Debug, Deserialize)]
struct RawInstalledPlugins {
    #[allow(dead_code)]
    version: u32,
    plugins: HashMap<String, Vec<RawPluginInstall>>,
}

#[derive(Debug, Deserialize)]
struct RawPluginInstall {
    scope: String,
    #[serde(rename = "projectPath")]
    project_path: Option<String>,
    #[serde(rename = "installPath")]
    install_path: String,
    version: String,
    #[serde(rename = "installedAt")]
    installed_at: Option<String>,
    #[serde(rename = "lastUpdated")]
    last_updated: Option<String>,
}

/// Marketplace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marketplace {
    /// Marketplace name
    pub name: String,
    /// Source type
    pub source_type: MarketplaceSource,
    /// Location (URL, path, or owner/repo)
    pub location: String,
    /// Whether auto-update is enabled
    #[serde(default)]
    pub auto_update: bool,
    /// Available plugins in this marketplace
    #[serde(default)]
    pub available_plugins: Vec<AvailablePlugin>,
}

/// Plugin available for installation from a marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePlugin {
    /// Plugin identifier (directory name)
    pub id: String,
    /// Plugin name from manifest
    pub name: String,
    /// Plugin description
    pub description: String,
    /// Plugin version from manifest
    pub version: Option<String>,
    /// Author information
    pub author: Option<Author>,
    /// Whether this plugin is already installed
    pub installed: bool,
}

/// Marketplace source types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarketplaceSource {
    /// GitHub repository
    GitHub { owner: String, repo: String },
    /// URL
    Url { url: String },
    /// Local path
    Local { path: PathBuf },
}

/// Installed plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// Plugin identifier
    pub id: String,
    /// Marketplace it came from
    pub marketplace: Option<String>,
    /// Plugin version
    pub version: String,
    /// Installation scope
    pub scope: Scope,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Path to plugin directory
    pub path: PathBuf,
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// When the plugin was first installed (ISO 8601)
    pub installed_at: Option<String>,
    /// When the plugin was last updated (ISO 8601)
    pub last_updated: Option<String>,
    /// Project path (for project-scoped plugins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
}

/// Plugin manifest (plugin.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Author information
    pub author: Option<Author>,
    /// Paths to command files
    #[serde(default)]
    pub commands: Vec<PathBuf>,
    /// Path to agents directory
    pub agents: Option<PathBuf>,
    /// Path to skills directory
    pub skills: Option<PathBuf>,
    /// Path to hooks configuration
    pub hooks: Option<PathBuf>,
    /// Path to MCP servers configuration
    pub mcp_servers: Option<PathBuf>,
    /// Parsed skill/command info (not from JSON, computed during scan)
    #[serde(default)]
    pub parsed_skills: Vec<PluginSkillInfo>,
}

/// Information about a skill/command provided by a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSkillInfo {
    /// Skill name (e.g., "notifications-init")
    pub name: String,
    /// Full invocation command (e.g., "/plugin-name:skill-name")
    pub invocation: String,
    /// Whether this looks like an init/setup skill
    pub is_init: bool,
    /// Whether this looks like a settings/config skill
    pub is_settings: bool,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// Author name
    pub name: String,
    /// Author email
    pub email: Option<String>,
}

// ============================================================================
// Cache Cleanup
// ============================================================================

/// Information about a stale cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleCacheEntry {
    /// Full path to the stale cache directory
    pub path: PathBuf,
    /// Plugin name
    pub plugin_name: String,
    /// Marketplace name
    pub marketplace: String,
    /// Version identifier (commit hash or version number)
    pub version: String,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Report of stale cache that can be cleaned up
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheCleanupReport {
    /// List of stale cache entries
    pub stale_entries: Vec<StaleCacheEntry>,
    /// Total size of stale cache in bytes
    pub total_size_bytes: u64,
    /// Number of installed plugins (for reference)
    pub installed_count: usize,
}

impl CacheCleanupReport {
    /// Find all stale cache entries
    ///
    /// Compares the cache directory contents against `installed_plugins.json`
    /// to identify directories that are not currently installed.
    ///
    /// # Errors
    /// Returns an error if reading directories or `installed_plugins.json` fails
    pub fn scan() -> ScanResult<Self> {
        let home = dirs::home_dir().ok_or(ScanError::HomeNotFound)?;
        let plugins_dir = home.join(".claude").join("plugins");
        let cache_dir = plugins_dir.join("cache");

        if !cache_dir.exists() {
            return Ok(Self::default());
        }

        // Get installed plugin paths
        let installed = Self::get_installed_paths(&plugins_dir)?;
        let installed_count = installed.len();

        let mut stale_entries = Vec::new();
        let mut total_size_bytes = 0u64;

        // Iterate through cache: cache/{marketplace}/{plugin}/{version}/
        let marketplace_entries = fs::read_dir(&cache_dir).map_err(ScanError::Io)?;

        for marketplace_entry in marketplace_entries.flatten() {
            let marketplace_path = marketplace_entry.path();
            // Security: Skip symlinks and non-directories
            if !marketplace_path.is_dir() || marketplace_path.is_symlink() {
                continue;
            }

            let marketplace_name = marketplace_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let plugin_entries = match fs::read_dir(&marketplace_path) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for plugin_entry in plugin_entries.flatten() {
                let plugin_path = plugin_entry.path();
                // Security: Skip symlinks and non-directories
                if !plugin_path.is_dir() || plugin_path.is_symlink() {
                    continue;
                }

                let plugin_name = plugin_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let version_entries = match fs::read_dir(&plugin_path) {
                    Ok(entries) => entries,
                    Err(_) => continue,
                };

                for version_entry in version_entries.flatten() {
                    let version_path = version_entry.path();
                    // Security: Skip symlinks and non-directories
                    if !version_path.is_dir() || version_path.is_symlink() {
                        continue;
                    }

                    // Check if this version is installed
                    if !installed.contains(&version_path) {
                        let version = version_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let size_bytes = Self::dir_size(&version_path);
                        total_size_bytes = total_size_bytes.saturating_add(size_bytes);

                        stale_entries.push(StaleCacheEntry {
                            path: version_path,
                            plugin_name: plugin_name.clone(),
                            marketplace: marketplace_name.clone(),
                            version,
                            size_bytes,
                        });
                    }
                }
            }
        }

        // Sort by size descending
        stale_entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        Ok(Self {
            stale_entries,
            total_size_bytes,
            installed_count,
        })
    }

    /// Get the set of installed plugin paths from `installed_plugins.json`
    fn get_installed_paths(plugins_dir: &Path) -> ScanResult<std::collections::HashSet<PathBuf>> {
        use std::collections::HashSet;

        let installed_file = plugins_dir.join("installed_plugins.json");
        if !installed_file.exists() {
            return Ok(HashSet::new());
        }

        let content = fs::read_to_string(&installed_file)?;
        let raw: RawInstalledPlugins =
            serde_json::from_str(&content).map_err(ScanError::JsonParse)?;

        let mut paths = HashSet::new();
        for installs in raw.plugins.values() {
            for install in installs {
                paths.insert(PathBuf::from(&install.install_path));
            }
        }

        Ok(paths)
    }

    /// Calculate the total size of a directory recursively
    ///
    /// Security: Skips symlinks to prevent following links outside the directory.
    fn dir_size(path: &Path) -> u64 {
        // Security: Don't follow symlinks
        if path.is_symlink() {
            return 0;
        }

        let mut size = 0u64;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                // Security: Skip symlinks
                if entry_path.is_symlink() {
                    continue;
                }
                if entry_path.is_dir() {
                    size = size.saturating_add(Self::dir_size(&entry_path));
                } else if let Ok(metadata) = entry_path.metadata() {
                    size = size.saturating_add(metadata.len());
                }
            }
        }

        size
    }

    /// Format total size as human-readable string
    #[must_use]
    pub fn format_size(&self) -> String {
        format_bytes(self.total_size_bytes)
    }

    /// Clean up stale cache entries
    ///
    /// Attempts to delete all stale entries identified by `scan()`.
    /// If deletion of an entry fails, the error is recorded and
    /// cleanup continues with remaining entries.
    ///
    /// Security: Validates all paths are within the cache directory
    /// before deletion to prevent path traversal attacks.
    ///
    /// # Errors
    /// Returns an error if the cache directory cannot be determined
    pub fn clean(&self) -> ScanResult<CleanupResult> {
        let home = dirs::home_dir().ok_or(ScanError::HomeNotFound)?;
        let cache_dir = home.join(".claude").join("plugins").join("cache");

        // Get canonical path for security validation
        let canonical_cache = match cache_dir.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Cache dir doesn't exist, nothing to clean
                return Ok(CleanupResult {
                    deleted_count: 0,
                    deleted_bytes: 0,
                    errors: vec![],
                });
            }
        };

        let mut deleted_count = 0usize;
        let mut deleted_bytes = 0u64;
        let mut errors = Vec::new();

        for entry in &self.stale_entries {
            // Security: Validate path is within cache directory
            let canonical_path = match entry.path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("{}: {}", entry.path.display(), e));
                    continue;
                }
            };

            if !canonical_path.starts_with(&canonical_cache) {
                errors.push(format!(
                    "{}: path outside cache directory, skipping",
                    entry.path.display()
                ));
                continue;
            }

            // Security: Skip symlinks
            if entry.path.is_symlink() {
                errors.push(format!(
                    "{}: is a symlink, skipping for safety",
                    entry.path.display()
                ));
                continue;
            }

            match fs::remove_dir_all(&entry.path) {
                Ok(()) => {
                    deleted_count += 1;
                    deleted_bytes = deleted_bytes.saturating_add(entry.size_bytes);
                }
                Err(e) => {
                    errors.push(format!("{}: {}", entry.path.display(), e));
                }
            }
        }

        // Clean up empty parent directories
        Self::cleanup_empty_dirs();

        Ok(CleanupResult {
            deleted_count,
            deleted_bytes,
            errors,
        })
    }

    /// Remove empty directories in the cache hierarchy
    fn cleanup_empty_dirs() {
        let Some(home) = dirs::home_dir() else {
            return;
        };

        let cache_dir = home.join(".claude").join("plugins").join("cache");

        // Iterate marketplaces
        let Ok(marketplace_entries) = fs::read_dir(&cache_dir) else {
            return;
        };

        for marketplace_entry in marketplace_entries.flatten() {
            let marketplace_path = marketplace_entry.path();
            if !marketplace_path.is_dir() {
                continue;
            }

            // Check plugins within marketplace
            let Ok(plugin_entries) = fs::read_dir(&marketplace_path) else {
                continue;
            };

            for plugin_entry in plugin_entries.flatten() {
                let plugin_path = plugin_entry.path();
                if !plugin_path.is_dir() {
                    continue;
                }

                // If plugin directory is empty, remove it
                if Self::is_dir_empty(&plugin_path) {
                    let _ = fs::remove_dir(&plugin_path);
                }
            }

            // If marketplace directory is now empty, remove it
            if Self::is_dir_empty(&marketplace_path) {
                let _ = fs::remove_dir(&marketplace_path);
            }
        }
    }

    /// Check if a directory is empty
    fn is_dir_empty(path: &Path) -> bool {
        fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
    }
}

/// Result of cache cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    /// Number of entries deleted
    pub deleted_count: usize,
    /// Total bytes freed
    pub deleted_bytes: u64,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl CleanupResult {
    /// Format deleted bytes as human-readable string
    #[must_use]
    pub fn format_size(&self) -> String {
        format_bytes(self.deleted_bytes)
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
