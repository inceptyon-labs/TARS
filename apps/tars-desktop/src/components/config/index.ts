/**
 * Config management components
 *
 * UI components for managing MCP servers, skills, hooks, commands, and agents.
 */

// MCP Server components
export { McpPanel } from './McpPanel';
export { McpForm } from './McpForm';

// Shared components
export { ScopeSelector } from './ScopeSelector';
export { ConfirmDialog } from './ConfirmDialog';
export { DiffPreview, InlineDiff } from './DiffPreview';

// Types
export type {
  McpServer,
  Scope,
  Skill,
  Hook,
  Command,
  Agent,
  OperationResult,
  MoveResult,
} from './types';
