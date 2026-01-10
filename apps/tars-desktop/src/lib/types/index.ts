/**
 * TypeScript types matching Rust backend types
 */

// Scanner types
export interface Inventory {
  host: HostInfo;
  user_scope: UserScope;
  managed_scope: ManagedScope | null;
  projects: ProjectScope[];
  plugins: PluginInventory;
  collisions: CollisionReport;
  scanned_at: string;
}

export interface HostInfo {
  os: string;
  arch: string;
  hostname: string;
}

// Plugin types
export interface PluginInventory {
  marketplaces: Marketplace[];
  installed: InstalledPlugin[];
}

export interface Marketplace {
  name: string;
  source_type: MarketplaceSource;
  location: string;
  auto_update: boolean;
  available_plugins: AvailablePlugin[];
}

export type MarketplaceSource =
  | { type: 'GitHub'; owner: string; repo: string }
  | { type: 'Url'; url: string }
  | { type: 'Local'; path: string };

export interface AvailablePlugin {
  id: string;
  name: string;
  description: string;
  version?: string;
  author?: { name: string; email?: string };
  installed: boolean;
}

export interface InstalledPlugin {
  id: string;
  marketplace: string | null;
  version: string;
  scope: PluginScope;
  enabled: boolean;
  path: string;
  manifest: PluginManifest;
  installed_at: string | null;
  last_updated: string | null;
}

export type PluginScope =
  | { type: 'User' }
  | { type: 'Project' }
  | { type: 'Local' }
  | { type: 'Managed' };

export interface PluginManifest {
  name: string;
  version: string;
  description: string;
  author?: { name: string; email?: string };
  commands: string[];
  agents?: string;
  skills?: string;
  hooks?: string;
  mcp_servers?: string;
  parsed_skills: PluginSkillInfo[];
}

export interface PluginSkillInfo {
  name: string;
  invocation: string;
  is_init: boolean;
  is_settings: boolean;
}

export interface UserScope {
  settings: SettingsFile | null;
  mcp: McpConfig | null;
  skills: SkillInfo[];
  commands: CommandInfo[];
  agents: AgentInfo[];
}

export interface ManagedScope {
  settings: SettingsFile | null;
  mcp: McpConfig | null;
}

export interface ProjectScope {
  path: string;
  name: string;
  claude_md: string | null;
  settings: SettingsFile | null;
  mcp: McpConfig | null;
  skills: SkillInfo[];
  commands: CommandInfo[];
  agents: AgentInfo[];
  hooks: HookInfo[];
  git: GitInfo | null;
}

export interface SettingsFile {
  path: string;
  permissions: Permissions | null;
}

export interface Permissions {
  allow_mcp_servers: string[];
  deny_mcp_servers: string[];
}

export interface McpConfig {
  path: string;
  servers: McpServer[];
}

export interface McpServer {
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
}

export interface SkillInfo {
  name: string;
  path: string;
  description: string;
  user_invocable: boolean;
  scope: SkillScope;
}

export type SkillScope =
  | { type: 'User' }
  | { type: 'Project' }
  | { type: 'Local' }
  | { type: 'Managed' }
  | { type: 'Plugin'; plugin_id: string };

export interface CommandInfo {
  name: string;
  path: string;
  description: string | null;
  allowed_tools: string[] | null;
  thinking: boolean;
  body: string;
  sha256: string;
  scope: { type: string };
}

export interface AgentInfo {
  name: string;
  path: string;
  description: string | null;
  tools: string[];
  model: string | null;
  permission_mode: string;
  skills: string[];
  sha256: string;
  scope: { type: string };
}

export interface HookInfo {
  source: string;
  trigger: string;
  matcher: string | null;
  definition: HookDefinition;
}

export interface HookDefinition {
  command: string;
  timeout: number | null;
}

export interface GitInfo {
  remote: string | null;
  branch: string;
  is_dirty: boolean;
}

export interface CollisionReport {
  skills: Collision[];
  commands: Collision[];
  agents: Collision[];
}

export interface Collision {
  name: string;
  occurrences: CollisionOccurrence[];
}

export interface CollisionOccurrence {
  scope: string;
  path: string;
}

// Project types
export interface ProjectInfo {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
}

// Profile types
export interface ProfileInfo {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProfileDetails {
  id: string;
  name: string;
  description: string | null;
  skills_count: number;
  commands_count: number;
  agents_count: number;
  has_claude_md: boolean;
  created_at: string;
  updated_at: string;
}

// Apply types
export interface DiffPreview {
  operations: OperationPreview[];
  summary: string;
  warnings: string[];
  terminal_output: string;
}

export interface OperationPreview {
  operation_type: 'create' | 'modify' | 'delete';
  path: string;
  diff: string | null;
  size: number | null;
}

export interface BackupInfo {
  id: string;
  project_id: string;
  profile_id: string | null;
  description: string | null;
  files_count: number;
  created_at: string;
}

// Skill types
export interface SupportingFile {
  name: string;
  path: string;
  file_type: 'markdown' | 'script' | 'config' | 'text' | 'other';
  is_referenced: boolean;
}

export interface SkillDetails {
  name: string;
  path: string;
  content: string;
  description: string | null;
  scope: SkillScope | string; // Can be SkillScope object or simple string from Tauri
  supporting_files: SupportingFile[];
}

// Agent types
export interface AgentDetails {
  name: string;
  path: string;
  content: string;
  description: string | null;
  scope: string;
}

// Command types
export interface CommandDetails {
  name: string;
  path: string;
  content: string;
  description: string | null;
  scope: string;
}

// Utility types
export interface DirectoryInfo {
  path: string;
  name: string;
  has_claude_config: boolean;
  is_git_repo: boolean;
}

// Plugin export options
export interface PluginExportOptions {
  name: string;
  version: string;
}

// Settings Hook types (for ~/.claude/settings.json hooks)
export interface SettingsHookAction {
  type: 'command' | 'prompt';
  command?: string;
  prompt?: string;
  timeout?: number;
}

export interface SettingsHookMatcher {
  matcher: string;
  hooks: SettingsHookAction[];
}

export interface SettingsHookEvent {
  event: string;
  matchers: SettingsHookMatcher[];
}

export interface SettingsHooksConfig {
  path: string;
  scope: string;
  events: SettingsHookEvent[];
}

// Cache cleanup types
export interface CacheEntry {
  path: string;
  plugin_name: string;
  marketplace: string;
  version: string;
  size_bytes: number;
}

export interface CacheStatusResponse {
  stale_entries: CacheEntry[];
  total_size_bytes: number;
  total_size_formatted: string;
  installed_count: number;
}

export interface CacheCleanResult {
  deleted_count: number;
  deleted_bytes: number;
  deleted_size_formatted: string;
  errors: string[];
}

// Prompt types (stored in ~/.tars/prompts/, not in Claude config)
export interface Prompt {
  id: string;
  title: string;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface PromptSummary {
  id: string;
  title: string;
  preview: string;
  created_at: string;
  updated_at: string;
}

// Update types
export interface ClaudeVersionInfo {
  installed_version: string | null;
  latest_version: string | null;
  update_available: boolean;
}

export interface ChangelogEntry {
  version: string;
  content: string;
}

export interface ChangelogResponse {
  entries: ChangelogEntry[];
  raw_content: string;
  fetched_at: string;
}

// Plugin update types
export interface PluginUpdateInfo {
  plugin_id: string;
  plugin_name: string;
  marketplace: string;
  installed_version: string;
  available_version: string;
  update_available: boolean;
}

export interface PluginUpdatesResponse {
  updates: PluginUpdateInfo[];
  total_plugins: number;
  plugins_with_updates: number;
}
