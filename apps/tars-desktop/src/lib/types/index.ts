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
  project_path?: string | null;
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
  tool_count: number;
  created_at: string;
  updated_at: string;
}

export interface ProfilePluginRef {
  id: string;
  marketplace: string | null;
  scope: string;
  enabled: boolean;
}

export interface ProfileDetails {
  id: string;
  name: string;
  description: string | null;
  tool_refs: ToolRef[];
  plugin_refs: ProfilePluginRef[];
  assigned_projects: ProjectRef[];
  mcp_count: number;
  skills_count: number;
  commands_count: number;
  agents_count: number;
  plugins_count: number;
  has_claude_md: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProfileSummary {
  id: string;
  name: string;
  description: string | null;
  tool_count: number;
  project_count: number;
  created_at: string;
  updated_at: string;
}

export interface ProjectRef {
  id: string;
  name: string;
  path: string;
}

// Tool reference types
export type ToolType = 'mcp' | 'skill' | 'agent' | 'hook';

export interface ToolPermissions {
  allowed_directories: string[];
  allowed_tools: string[];
  disallowed_tools: string[];
}

export interface ToolRef {
  name: string;
  tool_type: ToolType;
  source_scope: 'user' | 'project' | 'managed' | null;
  permissions: ToolPermissions | null;
}

export interface ToolRefWithSource extends ToolRef {
  source: 'profile' | 'local';
}

// Local overrides types
export interface LocalOverrides {
  mcp_servers: ToolRef[];
  skills: ToolRef[];
  agents: ToolRef[];
  hooks: ToolRef[];
}

// Profile sync types
export interface SyncResult {
  affected_projects: number;
  synced_at: string;
}

export interface UpdateProfileResponse {
  profile: ProfileSummary;
  sync_result: SyncResult;
}

export interface DeleteProfileResponse {
  deleted: boolean;
  converted_projects: number;
}

// Profile assignment types
export interface AssignProfileResponse {
  project_id: string;
  profile_id: string;
  assigned_at: string;
  /** Number of plugins that were installed by this assignment */
  plugins_installed: number;
  /** Plugins that failed to install (name, error message) */
  plugin_errors: [string, string][];
}

export interface UnassignProfileResponse {
  project_id: string;
  unassigned_at: string;
}

export interface ProjectToolsResponse {
  project_id: string;
  profile: { id: string; name: string } | null;
  profile_tools: ToolRefWithSource[];
  local_tools: ToolRefWithSource[];
}

// Local tool types
export interface AddLocalToolResponse {
  project_id: string;
  tool_name: string;
  added_at: string;
}

export interface RemoveLocalToolResponse {
  project_id: string;
  removed: boolean;
}

// Profile export/import types
export interface ExportProfileResponse {
  path: string;
  size_bytes: number;
  exported_at: string;
}

export interface ImportProfileResponse {
  profile: ProfileSummary;
  imported_from: string;
  collision_resolved: boolean;
}

export interface PreviewImportResponse {
  name: string;
  description: string | null;
  tool_count: number;
  has_name_collision: boolean;
  existing_profile_id: string | null;
  version: number;
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

// TARS app update types
export interface TarsUpdateInfo {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  release_notes: string | null;
  download_url: string | null;
}

// Claude Code usage stats types
export interface DailyActivity {
  date: string;
  messageCount: number;
  sessionCount: number;
  toolCallCount: number;
}

export interface DailyModelTokens {
  date: string;
  tokensByModel: Record<string, number>;
}

export interface ModelUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens: number;
  cacheCreationInputTokens: number;
}

export interface ClaudeUsageStats {
  totalSessions: number;
  totalMessages: number;
  firstSessionDate: string | null;
  lastComputedDate: string | null;
  dailyActivity: DailyActivity[];
  dailyModelTokens: DailyModelTokens[];
  modelUsage: Record<string, ModelUsage>;
  hourCounts: Record<string, number>;
}
