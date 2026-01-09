/**
 * Config component types
 */

/** Configuration scope */
export type Scope = 'user' | 'project' | 'local';

/** MCP server transport type */
export type McpTransport = 'stdio' | 'http' | 'sse';

/** MCP server configuration */
export interface McpServer {
  name: string;
  scope: Scope;
  transport: McpTransport;
  command?: string;
  args: string[];
  env: Record<string, string>;
  url?: string;
  filePath: string;
}

/** Skill configuration */
export interface Skill {
  name: string;
  scope: Scope;
  description: string;
  userInvocable: boolean;
  allowedTools: string[];
  model?: string;
  filePath: string;
}

/** Hook trigger types */
export type HookTrigger =
  | 'PreToolUse'
  | 'PostToolUse'
  | 'PermissionRequest'
  | 'UserPromptSubmit'
  | 'SessionStart'
  | 'SessionEnd'
  | 'Notification'
  | 'Stop'
  | 'SubagentStop'
  | 'PreCompact';

/** Hook definition */
export type HookDefinition =
  | { type: 'command'; command: string }
  | { type: 'prompt'; prompt: string }
  | { type: 'agent'; agent: string };

/** Hook configuration */
export interface Hook {
  trigger: HookTrigger;
  scope: Scope;
  index: number;
  matcher?: string;
  definition: HookDefinition;
  filePath: string;
}

/** Command configuration */
export interface Command {
  name: string;
  scope: Scope;
  description: string;
  thinking: boolean;
  filePath: string;
}

/** Agent configuration */
export interface Agent {
  name: string;
  scope: Scope;
  description: string;
  tools: string[];
  model?: string;
  permissionMode?: string;
  skills: string[];
  filePath: string;
}

/** Operation result from Tauri commands */
export interface OperationResult {
  success: boolean;
  backupId?: string;
  filePath: string;
  diff?: string;
  error?: string;
}

/** Move operation result */
export interface MoveResult extends OperationResult {
  removedFrom: string;
  addedTo: string;
}
