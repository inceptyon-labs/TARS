/**
 * IPC wrapper functions for Tauri commands
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  Inventory,
  ProjectInfo,
  ProfileInfo,
  ProfileDetails,
  DiffPreview,
  BackupInfo,
  SkillDetails,
  AgentDetails,
  CommandDetails,
  DirectoryInfo,
  PluginExportOptions,
  SettingsHooksConfig,
  SettingsHookEvent,
} from '../types';

// Scanner commands
export async function scanProject(path: string): Promise<Inventory> {
  return invoke('scan_project', { path });
}

export async function scanUserScope(): Promise<Inventory> {
  return invoke('scan_user_scope');
}

export async function scanProjects(paths: string[]): Promise<Inventory> {
  return invoke('scan_projects', { paths });
}

// Project commands
export async function listProjects(): Promise<ProjectInfo[]> {
  return invoke('list_projects');
}

export async function addProject(path: string): Promise<ProjectInfo> {
  return invoke('add_project', { path });
}

export async function getProject(id: string): Promise<ProjectInfo> {
  return invoke('get_project', { id });
}

export async function removeProject(id: string): Promise<boolean> {
  return invoke('remove_project', { id });
}

export interface ClaudeMdInfo {
  path: string;
  content: string | null;
  exists: boolean;
}

export async function readClaudeMd(projectPath: string): Promise<ClaudeMdInfo> {
  return invoke('read_claude_md', { projectPath });
}

export async function saveClaudeMd(projectPath: string, content: string): Promise<void> {
  return invoke('save_claude_md', { projectPath, content });
}

export async function deleteClaudeMd(projectPath: string): Promise<void> {
  return invoke('delete_claude_md', { projectPath });
}

export interface ContextItem {
  name: string;
  path: string;
  chars: number;
  tokens: number;
  scope: string; // "user" or "project"
}

export interface McpComplexity {
  name: string;
  server_type: string; // "stdio", "http", "sse", "unknown"
  uses_wrapper: boolean;
  env_var_count: number;
  is_plugin: boolean;
  tool_count: number;
  complexity_score: number;
  status: string; // "connected", "disabled", "unknown"
}

export interface ContextStats {
  claude_md_chars: number;
  claude_md_tokens: number;
  skills_chars: number;
  skills_tokens: number;
  skills_count: number;
  skills_items: ContextItem[];
  commands_chars: number;
  commands_tokens: number;
  commands_count: number;
  commands_items: ContextItem[];
  agents_chars: number;
  agents_tokens: number;
  agents_count: number;
  agents_items: ContextItem[];
  settings_chars: number;
  settings_tokens: number;
  mcp_servers: McpComplexity[];
  total_chars: number;
  total_tokens: number;
}

export async function getContextStats(projectPath: string): Promise<ContextStats> {
  return invoke('get_context_stats', { projectPath });
}

// Profile commands
export async function listProfiles(): Promise<ProfileInfo[]> {
  return invoke('list_profiles');
}

export async function createProfile(
  name: string,
  sourcePath: string,
  description?: string
): Promise<ProfileInfo> {
  return invoke('create_profile', {
    name,
    source_path: sourcePath,
    description,
  });
}

export async function getProfile(id: string): Promise<ProfileDetails> {
  return invoke('get_profile', { id });
}

export async function deleteProfile(id: string): Promise<boolean> {
  return invoke('delete_profile', { id });
}

export async function exportProfileAsPlugin(
  profileId: string,
  outputPath: string,
  options: PluginExportOptions
): Promise<string> {
  return invoke('export_profile_as_plugin', {
    profile_id: profileId,
    output_path: outputPath,
    options,
  });
}

// Apply commands
export async function previewApply(profileId: string, projectPath: string): Promise<DiffPreview> {
  return invoke('preview_apply', {
    profile_id: profileId,
    project_path: projectPath,
  });
}

export async function applyProfile(profileId: string, projectPath: string): Promise<BackupInfo> {
  return invoke('apply_profile', {
    profile_id: profileId,
    project_path: projectPath,
  });
}

export async function listBackups(projectId: string): Promise<BackupInfo[]> {
  return invoke('list_backups', { project_id: projectId });
}

export async function rollback(backupId: string, projectPath: string): Promise<number> {
  return invoke('rollback', {
    backup_id: backupId,
    project_path: projectPath,
  });
}

// Skill commands
export async function readSkill(path: string): Promise<SkillDetails> {
  return invoke('read_skill', { path });
}

export async function saveSkill(path: string, content: string): Promise<void> {
  return invoke('save_skill', { path, content });
}

export async function createSkill(
  name: string,
  scope: 'user' | 'project',
  projectPath?: string
): Promise<SkillDetails> {
  return invoke('create_skill', {
    name,
    scope,
    project_path: projectPath,
  });
}

export async function deleteSkill(path: string): Promise<void> {
  return invoke('delete_skill', { path });
}

export async function readSupportingFile(path: string): Promise<string> {
  return invoke('read_supporting_file', { path });
}

import type { SupportingFile } from '../types';

export async function saveSupportingFile(
  skillPath: string,
  fileName: string,
  content: string
): Promise<SupportingFile> {
  return invoke('save_supporting_file', {
    skillPath,
    fileName,
    content,
  });
}

export async function deleteSupportingFile(path: string): Promise<void> {
  return invoke('delete_supporting_file', { path });
}

// Agent commands
export async function readAgent(path: string): Promise<AgentDetails> {
  return invoke('read_agent', { path });
}

export async function saveAgent(path: string, content: string): Promise<void> {
  return invoke('save_agent', { path, content });
}

export async function createAgent(
  name: string,
  scope: 'user' | 'project',
  projectPath?: string
): Promise<AgentDetails> {
  return invoke('create_agent', {
    name,
    scope,
    project_path: projectPath,
  });
}

export async function deleteAgent(path: string): Promise<void> {
  return invoke('delete_agent', { path });
}

export async function moveAgent(
  path: string,
  targetScope: 'user' | 'project',
  projectPaths?: string[]
): Promise<AgentDetails> {
  return invoke('move_agent', {
    path,
    targetScope,
    projectPaths,
  });
}

export async function disableAgent(path: string): Promise<string> {
  return invoke('disable_agent', { path });
}

export async function enableAgent(path: string): Promise<string> {
  return invoke('enable_agent', { path });
}

export async function listDisabledAgents(projectPath?: string): Promise<AgentDetails[]> {
  // Only include project_path if it's defined, otherwise Tauri may not deserialize None correctly
  if (projectPath) {
    return invoke('list_disabled_agents', { project_path: projectPath });
  }
  return invoke('list_disabled_agents', {});
}

// Command commands
export async function readCommand(path: string): Promise<CommandDetails> {
  return invoke('read_command', { path });
}

export async function saveCommand(path: string, content: string): Promise<void> {
  return invoke('save_command', { path, content });
}

export async function createCommand(
  name: string,
  scope: 'user' | 'project',
  projectPath?: string
): Promise<CommandDetails> {
  return invoke('create_command', {
    name,
    scope,
    project_path: projectPath,
  });
}

export async function deleteCommand(path: string): Promise<void> {
  return invoke('delete_command', { path });
}

export async function moveCommand(
  path: string,
  targetScope: 'user' | 'project',
  projectPaths?: string[]
): Promise<CommandDetails> {
  return invoke('move_command', {
    path,
    targetScope,
    projectPaths,
  });
}

// Utility commands
export async function directoryExists(path: string): Promise<boolean> {
  return invoke('directory_exists', { path });
}

export async function getDirectoryInfo(path: string): Promise<DirectoryInfo> {
  return invoke('get_directory_info', { path });
}

export async function getHomeDir(): Promise<string> {
  return invoke('get_home_dir');
}

export async function listSubdirectories(path: string): Promise<DirectoryInfo[]> {
  return invoke('list_subdirectories', { path });
}

export async function getAppVersion(): Promise<string> {
  return invoke('get_app_version');
}

// Hook commands
export async function getUserHooks(): Promise<SettingsHooksConfig> {
  return invoke('get_user_hooks');
}

export async function getProjectHooks(projectPath: string): Promise<SettingsHooksConfig> {
  return invoke('get_project_hooks', { project_path: projectPath });
}

export async function saveUserHooks(events: SettingsHookEvent[]): Promise<void> {
  return invoke('save_user_hooks', { events });
}

export async function saveProjectHooks(
  projectPath: string,
  events: SettingsHookEvent[]
): Promise<void> {
  return invoke('save_project_hooks', { project_path: projectPath, events });
}

export async function getHookEventTypes(): Promise<string[]> {
  return invoke('get_hook_event_types');
}

// Prompt commands (stored in ~/.tars/prompts/, not in Claude config)
import type { Prompt, PromptSummary } from '../types';

export async function listPrompts(): Promise<PromptSummary[]> {
  return invoke('list_prompts');
}

export async function readPrompt(id: string): Promise<Prompt> {
  return invoke('read_prompt', { id });
}

export async function createPrompt(title: string, content: string): Promise<Prompt> {
  return invoke('create_prompt', { title, content });
}

export async function updatePrompt(id: string, title: string, content: string): Promise<Prompt> {
  return invoke('update_prompt', { id, title, content });
}

export async function deletePrompt(id: string): Promise<void> {
  return invoke('delete_prompt', { id });
}

// Update commands
import type {
  ClaudeVersionInfo,
  ChangelogResponse,
  PluginUpdatesResponse,
  TarsUpdateInfo,
} from '../types';

export async function getInstalledClaudeVersion(): Promise<string | null> {
  return invoke('get_installed_claude_version');
}

export async function fetchClaudeChangelog(): Promise<ChangelogResponse> {
  return invoke('fetch_claude_changelog');
}

export async function getClaudeVersionInfo(): Promise<ClaudeVersionInfo> {
  return invoke('get_claude_version_info');
}

export async function checkPluginUpdates(): Promise<PluginUpdatesResponse> {
  return invoke('check_plugin_updates');
}

// TARS app update commands
export async function checkTarsUpdate(): Promise<TarsUpdateInfo> {
  return invoke('check_tars_update');
}

export async function installTarsUpdate(): Promise<void> {
  return invoke('install_tars_update');
}

export async function getTarsVersion(): Promise<string> {
  return invoke('get_tars_version');
}
