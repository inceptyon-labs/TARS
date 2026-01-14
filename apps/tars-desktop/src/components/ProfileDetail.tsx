import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import {
  Sparkles,
  Terminal,
  Bot,
  FileText,
  Calendar,
  Server,
  Webhook,
  FolderOpen,
  Users,
  Plus,
  Download,
  X,
  Pin,
  GitBranch,
  RefreshCw,
  AlertTriangle,
  ArrowDownToLine,
  Globe,
  Loader2,
} from 'lucide-react';
import { Button } from './ui/button';
import { ProfileToolPicker } from './ProfileToolPicker';
import { ToolPermissionsEditor } from './ToolPermissionsEditor';
import { ProjectSelectorDialog } from './ProjectSelectorDialog';
import {
  assignProfileAsPlugin,
  checkProfileUpdates,
  getProfileHooks,
  installProfileToUser,
  pullToolUpdate,
} from '../lib/ipc';
import type { ProfileDetails, ToolRef } from '../lib/types';

interface ProfileDetailProps {
  profile: ProfileDetails;
  onAddTools?: (tools: ToolRef[]) => void;
  onRemoveTool?: (toolIndex: number) => void;
  onExportProfile?: () => void;
  /** Called when tools are added via addToolsFromSource (to refresh profile) */
  onToolsAdded?: () => void;
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
      return Terminal;
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
      return 'Command';
    default:
      return toolType;
  }
}

export function ProfileDetail({
  profile,
  onAddTools,
  onRemoveTool,
  onExportProfile,
  onToolsAdded,
}: ProfileDetailProps) {
  const [isToolPickerOpen, setIsToolPickerOpen] = useState(false);
  const [isProjectSelectorOpen, setIsProjectSelectorOpen] = useState(false);
  const queryClient = useQueryClient();

  // Mutation for installing profile to a project
  const installToProjectMutation = useMutation({
    mutationFn: (projectId: string) => assignProfileAsPlugin(projectId, profile.id),
    onSuccess: (result) => {
      toast.success(`Installed to project: ${result.plugin_id}`);
      setIsProjectSelectorOpen(false);
    },
    onError: (err) => {
      toast.error(`Failed to install to project: ${err}`);
    },
  });

  // Mutation for installing profile globally
  const installToUserMutation = useMutation({
    mutationFn: () => installProfileToUser(profile.id),
    onSuccess: (result) => {
      toast.success(`Installed globally: ${result.plugin_id}`);
    },
    onError: (err) => {
      toast.error(`Failed to install globally: ${err}`);
    },
  });

  // Fetch update information for tracked tools
  const {
    data: updateCheck,
    isLoading: isCheckingUpdates,
    refetch: recheckUpdates,
  } = useQuery({
    queryKey: ['profile-updates', profile.id],
    queryFn: () => checkProfileUpdates(profile.id),
    staleTime: 60000, // 1 minute
  });

  // Mutation for pulling updates
  const pullUpdateMutation = useMutation({
    mutationFn: ({ toolName }: { toolName: string }) => pullToolUpdate(profile.id, toolName),
    onSuccess: (_, { toolName }) => {
      toast.success(`Updated: ${toolName}`);
      queryClient.invalidateQueries({ queryKey: ['profile-updates', profile.id] });
      queryClient.invalidateQueries({ queryKey: ['profile', profile.id] });
      onToolsAdded?.(); // Refresh profile
    },
    onError: (err, { toolName }) => {
      toast.error(`Failed to update ${toolName}: ${err}`);
    },
  });

  // Create a map of tool names that have updates
  const toolsWithUpdates = new Set(updateCheck?.updates?.map((u) => u.name) || []);

  const { data: hooksConfig } = useQuery({
    queryKey: ['profile-hooks', profile.id],
    queryFn: () => getProfileHooks(profile.id),
  });

  const hookEvents = hooksConfig?.events || [];
  const hooksCount = hookEvents.reduce(
    (sum, event) =>
      sum + event.matchers.reduce((matcherSum, matcher) => matcherSum + matcher.hooks.length, 0),
    0
  );

  const stats = [
    { label: 'MCP Servers', value: profile.mcp_count, icon: Server },
    { label: 'Skills', value: profile.skills_count, icon: Sparkles },
    { label: 'Commands', value: profile.commands_count, icon: Terminal },
    { label: 'Agents', value: profile.agents_count, icon: Bot },
    { label: 'Hooks', value: hooksCount, icon: Webhook },
  ];

  const handleAddTools = (tools: ToolRef[]) => {
    if (onAddTools) {
      onAddTools(tools);
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

      {/* Tools */}
      <div className="border-t pt-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <h4 className="text-sm font-medium">Tools ({profile.tool_refs?.length || 0})</h4>
            {/* Updates summary */}
            {updateCheck && updateCheck.updates && updateCheck.updates.length > 0 && (
              <span className="flex items-center gap-1 text-xs px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-400">
                <AlertTriangle className="h-3 w-3" />
                {updateCheck.updates.length} update{updateCheck.updates.length !== 1 ? 's' : ''}{' '}
                available
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {/* Check for updates button */}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => recheckUpdates()}
              disabled={isCheckingUpdates}
              title="Check for updates"
            >
              <RefreshCw className={`h-4 w-4 ${isCheckingUpdates ? 'animate-spin' : ''}`} />
            </Button>
            {onAddTools && (
              <Button variant="outline" size="sm" onClick={() => setIsToolPickerOpen(true)}>
                <Plus className="h-4 w-4 mr-1" />
                Add
              </Button>
            )}
          </div>
        </div>

        {/* Tools */}
        {profile.tool_refs && profile.tool_refs.length > 0 && (
          <div className="space-y-2 mb-4">
            {profile.tool_refs.map((tool, index) => {
              const Icon = getToolIcon(tool.tool_type);
              const hasUpdate = toolsWithUpdates.has(tool.name);
              const sourceMode = tool.source_ref?.mode || 'pin';
              const isTracking = sourceMode === 'track';

              return (
                <div
                  key={`${tool.tool_type}-${tool.name}-${index}`}
                  className={`flex items-center justify-between p-2 rounded-lg border bg-muted/30 group ${hasUpdate ? 'border-amber-500/50' : ''}`}
                >
                  <div className="flex items-center gap-2">
                    <Icon className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium">{tool.name}</span>
                    <span className="text-xs text-muted-foreground px-1.5 py-0.5 bg-muted rounded">
                      {getToolTypeLabel(tool.tool_type)}
                    </span>
                    {/* Source mode indicator */}
                    {tool.source_ref && (
                      <span
                        className={`flex items-center gap-1 text-xs px-1.5 py-0.5 rounded ${
                          isTracking
                            ? 'bg-blue-500/20 text-blue-400'
                            : 'bg-amber-500/20 text-amber-400'
                        }`}
                        title={isTracking ? 'Tracking source changes' : 'Pinned at current version'}
                      >
                        {isTracking ? (
                          <GitBranch className="h-3 w-3" />
                        ) : (
                          <Pin className="h-3 w-3" />
                        )}
                      </span>
                    )}
                    {/* Update available badge */}
                    {hasUpdate && (
                      <span className="flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-400">
                        <AlertTriangle className="h-3 w-3" />
                        Update
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {/* Pull update button */}
                    {hasUpdate && (
                      <button
                        type="button"
                        onClick={() => pullUpdateMutation.mutate({ toolName: tool.name })}
                        disabled={pullUpdateMutation.isPending}
                        className="flex items-center gap-1 px-2 py-1 rounded text-xs bg-amber-500/20 text-amber-400 hover:bg-amber-500/30 transition-colors disabled:opacity-50"
                        title="Pull latest changes from source"
                      >
                        {pullUpdateMutation.isPending ? (
                          <RefreshCw className="h-3 w-3 animate-spin" />
                        ) : (
                          <ArrowDownToLine className="h-3 w-3" />
                        )}
                        Pull
                      </button>
                    )}
                    <ToolPermissionsEditor
                      permissions={tool.permissions}
                      onChange={() => {}}
                      compact
                    />
                    {onRemoveTool && (
                      <button
                        type="button"
                        onClick={() => onRemoveTool(index)}
                        className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-all"
                        title="Remove tool"
                      >
                        <X className="h-4 w-4" />
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {(!profile.tool_refs || profile.tool_refs.length === 0) && (
          <div className="text-sm text-muted-foreground py-4 text-center border rounded-lg bg-muted/10">
            No tools added yet. Click "Add" to select from your inventory.
          </div>
        )}
      </div>

      {/* Hooks */}
      <div className="border-t pt-4">
        <h4 className="text-sm font-medium mb-3 flex items-center gap-2">
          <Webhook className="h-4 w-4" />
          Hooks ({hooksCount})
        </h4>
        {hookEvents.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center border rounded-lg bg-muted/10">
            No hooks configured yet.
          </div>
        ) : (
          <div className="space-y-3">
            {hookEvents.map((event) => (
              <div key={event.event} className="border rounded-lg p-3 bg-muted/20">
                <div className="text-sm font-medium">{event.event}</div>
                <div className="mt-2 space-y-2">
                  {event.matchers.map((matcher, matcherIndex) => (
                    <div key={`${event.event}-${matcherIndex}`} className="space-y-1">
                      <div className="text-xs text-muted-foreground">
                        Matcher: {matcher.matcher || '*'}
                      </div>
                      {matcher.hooks.map((hook, hookIndex) => (
                        <div
                          key={`${event.event}-${matcherIndex}-${hookIndex}`}
                          className="text-xs text-muted-foreground flex items-center gap-2"
                        >
                          <span className="text-[10px] uppercase tracking-wide bg-muted px-1.5 py-0.5 rounded">
                            {hook.type}
                          </span>
                          <span className="truncate">
                            {hook.type === 'command' ? hook.command : hook.prompt}
                          </span>
                          {hook.timeout != null && (
                            <span className="text-[10px] text-muted-foreground">
                              {hook.timeout}s
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            ))}
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
      <div className="border-t pt-4 flex flex-wrap gap-2">
        <Button
          variant="default"
          onClick={() => setIsProjectSelectorOpen(true)}
          disabled={installToProjectMutation.isPending}
        >
          {installToProjectMutation.isPending ? (
            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
          ) : (
            <FolderOpen className="h-4 w-4 mr-2" />
          )}
          Apply to Project
        </Button>
        <Button
          variant="outline"
          onClick={() => installToUserMutation.mutate()}
          disabled={installToUserMutation.isPending}
        >
          {installToUserMutation.isPending ? (
            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
          ) : (
            <Globe className="h-4 w-4 mr-2" />
          )}
          Apply to User
        </Button>
        <Button variant="outline" onClick={onExportProfile} disabled={!onExportProfile}>
          <Download className="h-4 w-4 mr-2" />
          Export Plugin
        </Button>
      </div>

      {/* Project Selector Dialog */}
      <ProjectSelectorDialog
        open={isProjectSelectorOpen}
        onOpenChange={setIsProjectSelectorOpen}
        onSelectProject={(project) => installToProjectMutation.mutate(project.id)}
        title="Apply Profile to Project"
        description={`Select a project to install the "${profile.name}" profile as a plugin.`}
      />

      {/* Tool Picker Dialog */}
      <ProfileToolPicker
        open={isToolPickerOpen}
        onOpenChange={setIsToolPickerOpen}
        onAddTools={handleAddTools}
        existingTools={profile.tool_refs || []}
        profileId={profile.id}
        onToolsAdded={onToolsAdded}
      />
    </div>
  );
}
