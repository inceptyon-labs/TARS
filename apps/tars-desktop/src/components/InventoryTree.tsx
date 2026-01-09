import { useState } from 'react';
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  Sparkles,
  Terminal,
  Bot,
  Globe,
  FileText,
  Settings,
  User,
  Shield,
} from 'lucide-react';
import type { Inventory, ProjectScope } from '../lib/types';

interface InventoryTreeProps {
  inventory: Inventory;
  onSelect?: (type: string, item: TreeItem) => void;
}

interface TreeItem {
  name: string;
  path: string;
  description?: string | null;
}

interface TreeNodeProps {
  label: string;
  icon: React.ElementType;
  count?: number;
  children?: React.ReactNode;
  defaultOpen?: boolean;
  level?: number;
}

function TreeNode({ label, icon: Icon, count, children, defaultOpen = false, level = 0 }: TreeNodeProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const hasChildren = Boolean(children);

  return (
    <div className="select-none">
      <button
        onClick={() => hasChildren && setIsOpen(!isOpen)}
        className={`flex items-center gap-1.5 w-full px-2 py-1 rounded text-sm hover:bg-muted transition-colors ${
          hasChildren ? 'cursor-pointer' : 'cursor-default'
        }`}
        style={{ paddingLeft: `${level * 12 + 8}px` }}
      >
        {hasChildren ? (
          isOpen ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )
        ) : (
          <span className="w-3.5 shrink-0" />
        )}
        <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
        <span className="truncate">{label}</span>
        {count !== undefined && count > 0 && (
          <span className="ml-auto text-xs text-muted-foreground bg-muted px-1.5 rounded-full">
            {count}
          </span>
        )}
      </button>
      {isOpen && children && <div className="mt-0.5">{children}</div>}
    </div>
  );
}

interface LeafNodeProps {
  label: string;
  icon: React.ElementType;
  level?: number;
  onClick?: () => void;
  selected?: boolean;
}

function LeafNode({ label, icon: Icon, level = 0, onClick, selected = false }: LeafNodeProps) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 w-full px-2 py-1 rounded text-sm hover:bg-muted transition-colors ${
        selected ? 'bg-muted' : ''
      }`}
      style={{ paddingLeft: `${level * 12 + 8}px` }}
    >
      <span className="w-3.5 shrink-0" />
      <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
      <span className="truncate">{label}</span>
    </button>
  );
}

function ScopeSection({
  title,
  icon: Icon,
  scope,
  level = 0,
  onSelect
}: {
  title: string;
  icon: React.ElementType;
  scope: ProjectScope | Inventory['user_scope'];
  level?: number;
  onSelect?: (type: string, item: TreeItem) => void;
}) {
  const hasContent =
    scope.skills.length > 0 ||
    scope.commands.length > 0 ||
    scope.agents.length > 0 ||
    scope.mcp?.servers.length;

  if (!hasContent) return null;

  return (
    <TreeNode label={title} icon={Icon} defaultOpen={true} level={level}>
      {scope.skills.length > 0 && (
        <TreeNode label="Skills" icon={Sparkles} count={scope.skills.length} level={level + 1}>
          {scope.skills.map((skill) => (
            <LeafNode
              key={skill.path}
              label={skill.name}
              icon={FileText}
              level={level + 2}
              onClick={() => onSelect?.('skill', { name: skill.name, path: skill.path, description: skill.description })}
            />
          ))}
        </TreeNode>
      )}

      {scope.commands.length > 0 && (
        <TreeNode label="Commands" icon={Terminal} count={scope.commands.length} level={level + 1}>
          {scope.commands.map((cmd) => (
            <LeafNode
              key={cmd.path}
              label={`/${cmd.name}`}
              icon={FileText}
              level={level + 2}
              onClick={() => onSelect?.('command', { name: cmd.name, path: cmd.path, description: cmd.description })}
            />
          ))}
        </TreeNode>
      )}

      {scope.agents.length > 0 && (
        <TreeNode label="Agents" icon={Bot} count={scope.agents.length} level={level + 1}>
          {scope.agents.map((agent) => (
            <LeafNode
              key={agent.path}
              label={agent.name}
              icon={FileText}
              level={level + 2}
              onClick={() => onSelect?.('agent', { name: agent.name, path: agent.path, description: agent.description })}
            />
          ))}
        </TreeNode>
      )}

      {scope.mcp && scope.mcp.servers.length > 0 && (
        <TreeNode label="MCP Servers" icon={Globe} count={scope.mcp.servers.length} level={level + 1}>
          {scope.mcp.servers.map((server) => (
            <LeafNode
              key={server.name}
              label={server.name}
              icon={Settings}
              level={level + 2}
              onClick={() => onSelect?.('mcp', { name: server.name, path: scope.mcp!.path })}
            />
          ))}
        </TreeNode>
      )}
    </TreeNode>
  );
}

export function InventoryTree({ inventory, onSelect }: InventoryTreeProps) {
  const hasUserContent =
    inventory.user_scope.skills.length > 0 ||
    inventory.user_scope.commands.length > 0 ||
    inventory.user_scope.agents.length > 0 ||
    inventory.user_scope.mcp?.servers.length;

  const hasManagedContent =
    inventory.managed_scope?.settings ||
    inventory.managed_scope?.mcp;

  return (
    <div className="py-2 overflow-auto h-full">
      {/* User Scope */}
      {hasUserContent && (
        <ScopeSection
          title="User"
          icon={User}
          scope={inventory.user_scope}
          onSelect={onSelect}
        />
      )}

      {/* Managed Scope */}
      {hasManagedContent && (
        <TreeNode label="Managed" icon={Shield} level={0}>
          {inventory.managed_scope?.mcp && (
            <TreeNode
              label="MCP Servers"
              icon={Globe}
              count={inventory.managed_scope.mcp.servers.length}
              level={1}
            >
              {inventory.managed_scope.mcp.servers.map((server) => (
                <LeafNode
                  key={server.name}
                  label={server.name}
                  icon={Settings}
                  level={2}
                />
              ))}
            </TreeNode>
          )}
        </TreeNode>
      )}

      {/* Projects */}
      {inventory.projects.length > 0 && (
        <TreeNode
          label="Projects"
          icon={Folder}
          count={inventory.projects.length}
          defaultOpen={true}
          level={0}
        >
          {inventory.projects.map((project) => (
            <TreeNode
              key={project.path}
              label={project.name}
              icon={FolderOpen}
              level={1}
            >
              <ScopeSection
                title="Configuration"
                icon={Settings}
                scope={project}
                level={2}
                onSelect={onSelect}
              />

              {'hooks' in project && project.hooks.length > 0 && (
                <TreeNode label="Hooks" icon={Terminal} count={project.hooks.length} level={2}>
                  {project.hooks.map((hook, idx) => (
                    <LeafNode
                      key={`${hook.trigger}-${idx}`}
                      label={`${hook.trigger}${hook.matcher ? `: ${hook.matcher}` : ''}`}
                      icon={FileText}
                      level={3}
                      onClick={() => onSelect?.('hook', {
                        name: hook.trigger,
                        path: hook.source,
                        description: hook.matcher
                      })}
                    />
                  ))}
                </TreeNode>
              )}
            </TreeNode>
          ))}
        </TreeNode>
      )}

      {/* Empty state */}
      {!hasUserContent && !hasManagedContent && inventory.projects.length === 0 && (
        <div className="px-4 py-8 text-center text-muted-foreground">
          <p className="text-sm">No Claude Code configuration found.</p>
          <p className="text-xs mt-1">Add a project or check ~/.claude directory.</p>
        </div>
      )}
    </div>
  );
}
