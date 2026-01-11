import { useState } from 'react';
import {
  Sparkles,
  Terminal,
  Bot,
  FileText,
  Calendar,
  Upload,
  Server,
  Webhook,
  FolderOpen,
  Users,
  Plus,
  Download,
  Puzzle,
} from 'lucide-react';
import { Button } from './ui/button';
import { ProfileToolPicker } from './ProfileToolPicker';
import { ToolPermissionsEditor } from './ToolPermissionsEditor';
import type { ProfileDetails, ToolRef, ProfilePluginRef } from '../lib/types';

interface ProfileDetailProps {
  profile: ProfileDetails;
  onAddTools?: (tools: ToolRef[]) => void;
  onAddPlugins?: (plugins: ProfilePluginRef[]) => void;
  onExportProfile?: () => void;
}

function getToolIcon(toolType: string) {
  switch (toolType) {
    case 'mcp':
      return Server;
    case 'skill':
      return Sparkles;
    case 'agent':
      return Bot;
    case 'hook':
      return Webhook;
    default:
      return Terminal;
  }
}

function getToolTypeLabel(toolType: string) {
  switch (toolType) {
    case 'mcp':
      return 'MCP Server';
    case 'skill':
      return 'Skill';
    case 'agent':
      return 'Agent';
    case 'hook':
      return 'Hook';
    default:
      return toolType;
  }
}

export function ProfileDetail({ profile, onAddTools, onAddPlugins, onExportProfile }: ProfileDetailProps) {
  const [isToolPickerOpen, setIsToolPickerOpen] = useState(false);

  const stats = [
    { label: 'MCP Servers', value: profile.mcp_count, icon: Server },
    { label: 'Skills', value: profile.skills_count, icon: Sparkles },
    { label: 'Commands', value: profile.commands_count, icon: Terminal },
    { label: 'Agents', value: profile.agents_count, icon: Bot },
    { label: 'Plugins', value: profile.plugins_count || 0, icon: Puzzle },
  ];

  const handleAddTools = (tools: ToolRef[]) => {
    if (onAddTools) {
      onAddTools(tools);
    }
    setIsToolPickerOpen(false);
  };

  const handleAddPlugins = (plugins: ProfilePluginRef[]) => {
    if (onAddPlugins) {
      onAddPlugins(plugins);
    }
    setIsToolPickerOpen(false);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h3 className="text-xl font-bold">{profile.name}</h3>
        {profile.description && <p className="text-muted-foreground mt-1">{profile.description}</p>}
      </div>

      {/* Stats */}
      <div className="grid grid-cols-5 gap-4">
        {stats.map((stat) => (
          <div key={stat.label} className="border rounded-lg p-4 text-center">
            <stat.icon className="h-6 w-6 mx-auto text-muted-foreground mb-2" />
            <div className="text-2xl font-bold">{stat.value}</div>
            <div className="text-xs text-muted-foreground">{stat.label}</div>
          </div>
        ))}
      </div>

      {/* CLAUDE.md indicator */}
      {profile.has_claude_md && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <FileText className="h-4 w-4" />
          <span>Includes CLAUDE.md</span>
        </div>
      )}

      {/* Tools and Plugins */}
      <div className="border-t pt-4">
        <div className="flex items-center justify-between mb-3">
          <h4 className="text-sm font-medium">
            Tools & Plugins ({(profile.tool_refs?.length || 0) + (profile.plugin_refs?.length || 0)})
          </h4>
          {(onAddTools || onAddPlugins) && (
            <Button variant="outline" size="sm" onClick={() => setIsToolPickerOpen(true)}>
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          )}
        </div>

        {/* Tools */}
        {profile.tool_refs && profile.tool_refs.length > 0 && (
          <div className="space-y-2 mb-4">
            {profile.tool_refs.map((tool, index) => {
              const Icon = getToolIcon(tool.tool_type);
              return (
                <div
                  key={`${tool.tool_type}-${tool.name}-${index}`}
                  className="flex items-center justify-between p-2 rounded-lg border bg-muted/30"
                >
                  <div className="flex items-center gap-2">
                    <Icon className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium">{tool.name}</span>
                    <span className="text-xs text-muted-foreground px-1.5 py-0.5 bg-muted rounded">
                      {getToolTypeLabel(tool.tool_type)}
                    </span>
                  </div>
                  <ToolPermissionsEditor
                    permissions={tool.permissions}
                    onChange={() => {}}
                    compact
                  />
                </div>
              );
            })}
          </div>
        )}

        {/* Plugins */}
        {profile.plugin_refs && profile.plugin_refs.length > 0 && (
          <div className="space-y-2 mb-4">
            {profile.plugin_refs.map((plugin, index) => (
              <div
                key={`plugin-${plugin.id}-${index}`}
                className="flex items-center justify-between p-2 rounded-lg border bg-muted/30"
              >
                <div className="flex items-center gap-2">
                  <Puzzle className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{plugin.id}</span>
                  <span className="text-xs text-muted-foreground px-1.5 py-0.5 bg-muted rounded">
                    Plugin
                  </span>
                  <span className="text-xs text-muted-foreground px-1.5 py-0.5 bg-muted rounded">
                    {plugin.scope}
                  </span>
                </div>
                <span className={`text-xs px-1.5 py-0.5 rounded ${plugin.enabled ? 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300' : 'bg-muted text-muted-foreground'}`}>
                  {plugin.enabled ? 'Enabled' : 'Disabled'}
                </span>
              </div>
            ))}
          </div>
        )}

        {(!profile.tool_refs || profile.tool_refs.length === 0) && (!profile.plugin_refs || profile.plugin_refs.length === 0) && (
          <div className="text-sm text-muted-foreground py-4 text-center border rounded-lg bg-muted/10">
            No tools or plugins added yet. Click "Add" to select from your inventory.
          </div>
        )}
      </div>

      {/* Assigned Projects */}
      {profile.assigned_projects && profile.assigned_projects.length > 0 && (
        <div className="border-t pt-4">
          <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
            <Users className="h-4 w-4" />
            Assigned Projects ({profile.assigned_projects.length})
          </h4>
          <div className="space-y-2">
            {profile.assigned_projects.map((project) => (
              <div
                key={project.id}
                className="flex items-center justify-between p-2 rounded-lg border bg-muted/30"
              >
                <div className="flex items-center gap-2">
                  <FolderOpen className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{project.name}</span>
                </div>
                <span className="text-xs text-muted-foreground truncate max-w-48">
                  {project.path}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Metadata */}
      <div className="border-t pt-4 space-y-2 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4" />
          <span>Created: {new Date(profile.created_at).toLocaleString()}</span>
        </div>
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4" />
          <span>Updated: {new Date(profile.updated_at).toLocaleString()}</span>
        </div>
      </div>

      {/* Actions */}
      <div className="border-t pt-4 flex gap-2">
        <Button variant="default" disabled title="Coming soon">
          Apply to Project
        </Button>
        <Button variant="outline" onClick={onExportProfile} disabled={!onExportProfile}>
          <Download className="h-4 w-4 mr-2" />
          Export Profile
        </Button>
        <Button variant="outline" disabled title="Coming soon">
          <Upload className="h-4 w-4 mr-2" />
          Export as Plugin
        </Button>
      </div>

      {/* Tool Picker Dialog */}
      <ProfileToolPicker
        open={isToolPickerOpen}
        onOpenChange={setIsToolPickerOpen}
        onAddTools={handleAddTools}
        onAddPlugins={handleAddPlugins}
        existingTools={profile.tool_refs || []}
        existingPlugins={profile.plugin_refs || []}
      />
    </div>
  );
}
