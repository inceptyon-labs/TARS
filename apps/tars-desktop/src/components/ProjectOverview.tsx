import { useState, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Sparkles,
  Terminal,
  Bot,
  Server,
  Webhook,
  FileText,
  ChevronRight,
  ChevronDown,
  Save,
  AlertTriangle,
  ExternalLink,
  Gauge,
  Layers,
  Plus,
  X,
  AlertCircle,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  MDXEditor,
  headingsPlugin,
  listsPlugin,
  quotePlugin,
  thematicBreakPlugin,
  markdownShortcutPlugin,
  toolbarPlugin,
  linkPlugin,
  linkDialogPlugin,
  tablePlugin,
  codeBlockPlugin,
  codeMirrorPlugin,
  UndoRedo,
  BoldItalicUnderlineToggles,
  CodeToggle,
  ListsToggle,
  BlockTypeSelect,
  CreateLink,
  InsertTable,
  InsertThematicBreak,
  Separator,
  type MDXEditorMethods,
} from '@mdxeditor/editor';
import '@mdxeditor/editor/style.css';
import { useUIStore } from '../stores/ui-store';
import type {
  Inventory,
  SkillInfo,
  CommandInfo,
  AgentInfo,
  HookInfo,
  McpServer,
  ProjectToolsResponse,
  ToolRefWithSource,
  CollisionOccurrence,
} from '../lib/types';
import {
  readClaudeMd,
  saveClaudeMd,
  getContextStats,
  addLocalTool,
  removeLocalTool,
} from '../lib/ipc';
import { Button } from './ui/button';
import { ProfileToolPicker } from './ProfileToolPicker';
import { ToolPermissionsEditor } from './ToolPermissionsEditor';
import type { ToolRef } from '../lib/types';

interface ProjectOverviewProps {
  inventory: Inventory;
  projectPath: string;
  projectTools?: ProjectToolsResponse | null;
  onAssignProfile?: () => void;
  onRefreshTools?: () => void;
}

type SectionId =
  | 'context'
  | 'claude-md'
  | 'tools'
  | 'project-skills'
  | 'project-commands'
  | 'project-agents'
  | 'project-hooks'
  | 'project-mcp'
  | 'global-tools'
  | 'global-skills'
  | 'global-commands'
  | 'global-agents'
  | 'global-mcp'
  | 'collisions';

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
      return Server;
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

function checkToolAvailability(
  tool: ToolRefWithSource,
  inventory: Inventory,
  projectPath: string
): { available: boolean; reason?: string } {
  if (tool.source === 'profile') {
    return { available: true };
  }

  const { name, tool_type, source_scope } = tool;
  const nameLower = name.toLowerCase();
  const userScope = inventory.user_scope;
  const projectData = inventory.projects.find((p) => p.path === projectPath);
  const managedScope = inventory.managed_scope;

  // Check based on tool type and source scope (case-insensitive matching)
  switch (tool_type) {
    case 'mcp': {
      // Check all relevant scopes based on source_scope
      const inUser = userScope.mcp?.servers.some((s) => s.name.toLowerCase() === nameLower);
      const inProject = projectData?.mcp?.servers.some((s) => s.name.toLowerCase() === nameLower);
      const inManaged = managedScope?.mcp?.servers.some((s) => s.name.toLowerCase() === nameLower);

      if (source_scope === 'user' && inUser) return { available: true };
      if (source_scope === 'project' && inProject) return { available: true };
      if (source_scope === 'managed' && inManaged) return { available: true };
      // If no source_scope specified, check all scopes
      if (!source_scope && (inUser || inProject || inManaged)) return { available: true };

      return { available: false, reason: `MCP server not found in ${source_scope || 'any'} scope` };
    }
    case 'skill': {
      const inUser = userScope.skills.some((s) => s.name.toLowerCase() === nameLower);
      const inProject = projectData?.skills.some((s) => s.name.toLowerCase() === nameLower);
      // Also check plugins
      const inPlugin = inventory.plugins.installed.some((p) =>
        p.manifest.parsed_skills?.some(
          (s) => s.name.toLowerCase() === nameLower || s.invocation?.toLowerCase() === nameLower
        )
      );

      if (source_scope === 'user' && inUser) return { available: true };
      if (source_scope === 'project' && inProject) return { available: true };
      if (!source_scope && (inUser || inProject || inPlugin)) return { available: true };

      return { available: false, reason: `Skill not found in ${source_scope || 'any'} scope` };
    }
    case 'agent': {
      const inUser = userScope.agents.some((a) => a.name.toLowerCase() === nameLower);
      const inProject = projectData?.agents.some((a) => a.name.toLowerCase() === nameLower);

      if (source_scope === 'user' && inUser) return { available: true };
      if (source_scope === 'project' && inProject) return { available: true };
      if (!source_scope && (inUser || inProject)) return { available: true };

      return { available: false, reason: `Agent not found in ${source_scope || 'any'} scope` };
    }
    case 'hook': {
      // Hooks are defined per-project, assume available if project has hooks
      return { available: true };
    }
    default:
      return { available: true };
  }
}

export function ProjectOverview({
  inventory,
  projectPath,
  projectTools,
  onAssignProfile,
  onRefreshTools,
}: ProjectOverviewProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const theme = useUIStore((state) => state.theme);
  const [expandedSections, setExpandedSections] = useState<Set<SectionId>>(
    new Set(['context', 'claude-md'])
  );
  const [claudeMdContent, setClaudeMdContent] = useState<string>('');
  const [claudeMdDirty, setClaudeMdDirty] = useState(false);
  const [editorKey, setEditorKey] = useState(0);
  const editorRef = useRef<MDXEditorMethods>(null);
  const [isToolPickerOpen, setIsToolPickerOpen] = useState(false);

  // Get project data from inventory
  const projectData = inventory.projects.find((p) => p.path === projectPath);
  const userScope = inventory.user_scope;
  const collisions = inventory.collisions;

  // Load CLAUDE.md content
  const {
    data: claudeMdInfo,
    isLoading: loadingClaudeMd,
    error: claudeMdError,
  } = useQuery({
    queryKey: ['claude-md', projectPath],
    queryFn: () => readClaudeMd(projectPath),
  });

  // Load context stats
  const { data: contextStats, isLoading: loadingStats } = useQuery({
    queryKey: ['context-stats', projectPath],
    queryFn: () => getContextStats(projectPath),
    refetchInterval: 30000, // Refresh every 30s
  });

  useEffect(() => {
    if (claudeMdInfo?.content !== undefined) {
      setClaudeMdContent(claudeMdInfo.content || '');
      setClaudeMdDirty(false);
      setEditorKey((k) => k + 1); // Force editor remount
    }
  }, [claudeMdInfo]);

  // Save CLAUDE.md mutation
  const saveMutation = useMutation({
    mutationFn: () => {
      const content = editorRef.current?.getMarkdown() || claudeMdContent;
      return saveClaudeMd(projectPath, content);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['claude-md', projectPath] });
      setClaudeMdDirty(false);
      toast.success('CLAUDE.md saved');
    },
    onError: (err) => {
      toast.error('Failed to save CLAUDE.md', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Add local tool mutation - handles partial success
  const addLocalToolMutation = useMutation({
    mutationFn: async (tools: ToolRef[]) => {
      if (!projectTools?.project_id) throw new Error('Project ID not available');
      const results = await Promise.allSettled(
        tools.map((tool) => addLocalTool(projectTools.project_id, tool))
      );
      const succeeded = results.filter((r) => r.status === 'fulfilled').length;
      const failed = results.filter((r) => r.status === 'rejected');
      return { succeeded, failed, total: tools.length };
    },
    onSuccess: ({ succeeded, failed, total }) => {
      // Always refresh to show any tools that succeeded
      onRefreshTools?.();
      if (failed.length === 0) {
        toast.success(succeeded === 1 ? 'Local tool added' : `${succeeded} local tools added`);
      } else if (succeeded > 0) {
        toast.warning(`Added ${succeeded} of ${total} tools`, {
          description: `${failed.length} failed to add`,
        });
      } else {
        toast.error('Failed to add local tools', {
          description:
            failed[0]?.reason instanceof Error
              ? failed[0].reason.message
              : String(failed[0]?.reason),
        });
      }
    },
    onError: (err) => {
      toast.error('Failed to add local tool', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Remove local tool mutation
  const removeLocalToolMutation = useMutation({
    mutationFn: ({ toolName, toolType }: { toolName: string; toolType: string }) => {
      if (!projectTools?.project_id) throw new Error('Project ID not available');
      return removeLocalTool(projectTools.project_id, toolName, toolType);
    },
    onSuccess: () => {
      toast.success('Local tool removed');
      onRefreshTools?.();
    },
    onError: (err) => {
      toast.error('Failed to remove local tool', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const handleAddLocalTools = (tools: ToolRef[]) => {
    addLocalToolMutation.mutate(tools);
    setIsToolPickerOpen(false);
  };

  // Separate project-scoped items from global items
  const projectSkills: SkillInfo[] = projectData?.skills || [];
  const projectCommands: CommandInfo[] = projectData?.commands || [];
  const projectAgents: AgentInfo[] = projectData?.agents || [];
  const projectHooks: HookInfo[] = projectData?.hooks || [];
  const projectMcpServers: McpServer[] = projectData?.mcp?.servers || [];

  // Global items (user scope, plugins, managed) - available but not project-specific
  const globalSkills: SkillInfo[] = [
    ...userScope.skills,
    ...inventory.plugins.installed.flatMap((p) =>
      p.manifest.skills
        ? [
            {
              name: `${p.id}:*`,
              path: p.path,
              description: `Skills from ${p.manifest.name}`,
              user_invocable: true,
              scope: { type: 'Plugin', plugin_id: p.id },
            } as SkillInfo,
          ]
        : []
    ),
  ];
  const globalCommands: CommandInfo[] = userScope.commands;
  const globalAgents: AgentInfo[] = userScope.agents;
  const globalMcpServers: { scope: string; servers: McpServer[] }[] = [
    ...(userScope.mcp?.servers.length ? [{ scope: 'User', servers: userScope.mcp.servers }] : []),
    ...(inventory.managed_scope?.mcp?.servers.length
      ? [{ scope: 'Managed', servers: inventory.managed_scope.mcp.servers }]
      : []),
  ];

  // Total counts for headers
  const totalGlobalTools =
    globalSkills.length +
    globalCommands.length +
    globalAgents.length +
    globalMcpServers.reduce((acc, g) => acc + g.servers.length, 0);
  const totalProjectTools =
    projectSkills.length +
    projectCommands.length +
    projectAgents.length +
    projectHooks.length +
    projectMcpServers.length;

  // Combine profile and local tools for display
  const combinedTools: ToolRefWithSource[] = [
    ...(projectTools?.profile_tools || []),
    ...(projectTools?.local_tools || []),
  ];

  const totalCollisions =
    collisions.skills.length + collisions.commands.length + collisions.agents.length;

  const formatCollisionScope = (scope: CollisionOccurrence['scope']) => {
    if (typeof scope === 'string') {
      return scope;
    }
    switch (scope.type) {
      case 'User':
        return 'User';
      case 'Project':
        return 'Project';
      case 'Local':
        return 'Local';
      case 'Managed':
        return 'Managed';
      case 'Plugin':
        return scope.plugin_id ? `Plugin (${scope.plugin_id})` : 'Plugin';
      default:
        return 'Unknown';
    }
  };

  const toggleSection = (id: SectionId) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const renderSectionHeader = (
    id: SectionId,
    icon: React.ElementType,
    title: string,
    count: number,
    modulePath?: string
  ) => {
    const Icon = icon;
    const isExpanded = expandedSections.has(id);

    return (
      <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border">
        <button
          onClick={() => toggleSection(id)}
          className="flex items-center gap-3 hover:text-primary transition-colors"
        >
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <Icon className="h-4 w-4 text-primary" />
          <span className="font-medium">{title}</span>
          <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
            {count}
          </span>
        </button>
        {modulePath && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => navigate(modulePath)}
            className="text-xs gap-1"
          >
            Configure
            <ExternalLink className="h-3 w-3" />
          </Button>
        )}
      </div>
    );
  };

  // Context usage from backend
  const contextLimit = 200000; // Claude's context window
  const totalTokens = contextStats?.total_tokens || 0;
  const contextUsagePercent = Math.min((totalTokens / contextLimit) * 100, 100);

  const formatNumber = (n: number) => n.toLocaleString();
  const formatSize = (chars: number) => {
    if (chars < 1000) return `${chars} chars`;
    if (chars < 1000000) return `${(chars / 1000).toFixed(1)}K chars`;
    return `${(chars / 1000000).toFixed(2)}M chars`;
  };

  return (
    <div className="h-full overflow-auto">
      <div className="max-w-4xl mx-auto p-6 space-y-4">
        {/* Project header */}
        <div className="tars-panel rounded-lg p-4 mb-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-xl font-semibold">{projectData?.name || 'Project'}</h2>
              <p className="text-sm text-muted-foreground font-mono mt-1">{projectPath}</p>
              {projectData?.git && (
                <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
                  <span className="bg-muted px-2 py-0.5 rounded">{projectData.git.branch}</span>
                  {projectData.git.is_dirty && (
                    <span className="text-amber-500">• uncommitted changes</span>
                  )}
                </div>
              )}
            </div>
            {onAssignProfile && (
              <Button variant="outline" size="sm" onClick={onAssignProfile}>
                <Layers className="h-4 w-4 mr-1" />
                {projectTools?.profile ? 'Change Profile' : 'Assign Profile'}
              </Button>
            )}
          </div>
          {projectTools?.profile && (
            <div className="flex items-center gap-2 mt-3 pt-3 border-t">
              <Layers className="h-4 w-4 text-primary" />
              <span className="text-sm font-medium">Profile:</span>
              <span className="text-sm text-primary">{projectTools.profile.name}</span>
              <span className="text-xs text-muted-foreground">
                ({projectTools.profile_tools.length} tool
                {projectTools.profile_tools.length === 1 ? '' : 's'})
              </span>
            </div>
          )}
        </div>

        {/* Project Tools Section - shows tools from profile and local */}
        {(combinedTools.length > 0 || projectTools?.project_id) && (
          <div className="tars-panel rounded-lg overflow-hidden">
            <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border">
              <button
                onClick={() => toggleSection('tools')}
                className="flex items-center gap-3 hover:text-primary transition-colors"
              >
                {expandedSections.has('tools') ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )}
                <Layers className="h-4 w-4 text-primary" />
                <span className="font-medium">Project Tools</span>
                <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
                  {combinedTools.length}
                </span>
              </button>
              {projectTools?.project_id && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setIsToolPickerOpen(true)}
                  className="text-xs gap-1"
                >
                  <Plus className="h-3 w-3" />
                  Add Local Tool
                </Button>
              )}
            </div>
            {expandedSections.has('tools') && (
              <div className="divide-y divide-border">
                {combinedTools.length === 0 ? (
                  <div className="px-4 py-6 text-center text-sm text-muted-foreground">
                    No tools configured. Add local tools or assign a profile.
                  </div>
                ) : (
                  combinedTools.map((tool, index) => {
                    const Icon = getToolIcon(tool.tool_type);
                    const availability = checkToolAvailability(tool, inventory, projectPath);
                    return (
                      <div
                        key={`${tool.tool_type}-${tool.name}-${index}`}
                        className={`flex items-center justify-between px-4 py-3 hover:bg-muted/30 group ${
                          !availability.available ? 'bg-destructive/5' : ''
                        }`}
                      >
                        <div className="flex items-center gap-3">
                          <Icon
                            className={`h-4 w-4 ${
                              availability.available
                                ? 'text-muted-foreground'
                                : 'text-destructive/70'
                            }`}
                          />
                          <div>
                            <div className="flex items-center gap-2">
                              <span
                                className={`font-medium text-sm ${
                                  !availability.available ? 'text-destructive' : ''
                                }`}
                              >
                                {tool.name}
                              </span>
                              {!availability.available && (
                                <span className="text-destructive" title={availability.reason}>
                                  <AlertCircle className="h-3.5 w-3.5" />
                                </span>
                              )}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {getToolTypeLabel(tool.tool_type)}
                              {!availability.available && (
                                <span className="text-destructive ml-2">• Not found</span>
                              )}
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <ToolPermissionsEditor
                            permissions={tool.permissions}
                            onChange={() => {}}
                            compact
                          />
                          {tool.source === 'profile' ? (
                            <span className="text-xs bg-primary/20 text-primary px-2 py-0.5 rounded">
                              From Profile
                            </span>
                          ) : (
                            <>
                              <span className="text-xs bg-amber-500/20 text-amber-600 px-2 py-0.5 rounded">
                                Local
                              </span>
                              <button
                                onClick={() =>
                                  removeLocalToolMutation.mutate({
                                    toolName: tool.name,
                                    toolType: tool.tool_type,
                                  })
                                }
                                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded text-destructive transition-opacity"
                                title={`Remove ${tool.name}`}
                                aria-label={`Remove local tool ${tool.name}`}
                              >
                                <X className="h-3.5 w-3.5" aria-hidden="true" />
                              </button>
                            </>
                          )}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            )}
          </div>
        )}

        {/* Context Usage Summary */}
        <div className="tars-panel rounded-lg overflow-hidden">
          <div
            onClick={() => toggleSection('context')}
            className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border cursor-pointer hover:bg-muted/50"
          >
            <div className="flex items-center gap-3">
              {expandedSections.has('context') ? (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              )}
              <Gauge className="h-4 w-4 text-primary" />
              <span className="font-medium">Context Usage</span>
              {!loadingStats && (
                <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
                  {contextUsagePercent.toFixed(1)}%
                </span>
              )}
            </div>
            {loadingStats ? (
              <span className="text-xs text-muted-foreground">Loading...</span>
            ) : (
              <span className="text-xs text-muted-foreground">
                ~{formatNumber(totalTokens)} / {formatNumber(contextLimit)} tokens
              </span>
            )}
          </div>

          {expandedSections.has('context') && (
            <div className="p-4">
              {/* Progress bar */}
              <div className="h-2 bg-muted rounded-full overflow-hidden mb-4">
                <div
                  className={`h-full transition-all ${
                    contextUsagePercent > 75
                      ? 'bg-destructive'
                      : contextUsagePercent > 50
                        ? 'bg-amber-500'
                        : 'bg-primary'
                  }`}
                  style={{ width: `${Math.max(contextUsagePercent, 0.5)}%` }}
                />
              </div>

              {/* Breakdown with expandable details */}
              {contextStats && (
                <div className="space-y-2">
                  {/* CLAUDE.md - not expandable */}
                  <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
                    <span className="text-sm">CLAUDE.md</span>
                    <span className="text-sm font-mono">
                      {formatNumber(contextStats.claude_md_tokens)} tokens
                      <span className="text-muted-foreground ml-2">
                        ({formatSize(contextStats.claude_md_chars)})
                      </span>
                    </span>
                  </div>

                  {/* Agents - expandable */}
                  <details className="group">
                    <summary className="flex items-center justify-between p-2 bg-muted/30 rounded cursor-pointer hover:bg-muted/50">
                      <span className="text-sm flex items-center gap-2">
                        <ChevronRight className="h-3 w-3 group-open:rotate-90 transition-transform" />
                        Agents ({contextStats.agents_count})
                        {contextStats.agents_tokens > 20000 && (
                          <span className="text-[10px] bg-amber-500/20 text-amber-600 px-1.5 py-0.5 rounded">
                            HIGH
                          </span>
                        )}
                      </span>
                      <span className="text-sm font-mono">
                        {formatNumber(contextStats.agents_tokens)} tokens
                        <span className="text-muted-foreground ml-2">
                          ({formatSize(contextStats.agents_chars)})
                        </span>
                      </span>
                    </summary>
                    {contextStats.agents_items.length > 0 && (
                      <div className="mt-1 ml-5 space-y-1 text-xs">
                        {contextStats.agents_items.slice(0, 10).map((item) => (
                          <div
                            key={item.path}
                            className="flex justify-between py-1 px-2 bg-muted/20 rounded"
                          >
                            <span className="truncate flex-1">
                              {item.name}
                              <span className="text-muted-foreground ml-1">({item.scope})</span>
                            </span>
                            <span className="font-mono text-muted-foreground ml-2">
                              {formatNumber(item.tokens)} tok
                            </span>
                          </div>
                        ))}
                        {contextStats.agents_items.length > 10 && (
                          <div className="text-muted-foreground py-1">
                            +{contextStats.agents_items.length - 10} more...
                          </div>
                        )}
                      </div>
                    )}
                  </details>

                  {/* Commands - expandable */}
                  <details className="group">
                    <summary className="flex items-center justify-between p-2 bg-muted/30 rounded cursor-pointer hover:bg-muted/50">
                      <span className="text-sm flex items-center gap-2">
                        <ChevronRight className="h-3 w-3 group-open:rotate-90 transition-transform" />
                        Commands ({contextStats.commands_count})
                      </span>
                      <span className="text-sm font-mono">
                        {formatNumber(contextStats.commands_tokens)} tokens
                        <span className="text-muted-foreground ml-2">
                          ({formatSize(contextStats.commands_chars)})
                        </span>
                      </span>
                    </summary>
                    {contextStats.commands_items.length > 0 && (
                      <div className="mt-1 ml-5 space-y-1 text-xs">
                        {contextStats.commands_items.slice(0, 10).map((item) => (
                          <div
                            key={item.path}
                            className="flex justify-between py-1 px-2 bg-muted/20 rounded"
                          >
                            <span className="truncate flex-1">
                              /{item.name}
                              <span className="text-muted-foreground ml-1">({item.scope})</span>
                            </span>
                            <span className="font-mono text-muted-foreground ml-2">
                              {formatNumber(item.tokens)} tok
                            </span>
                          </div>
                        ))}
                        {contextStats.commands_items.length > 10 && (
                          <div className="text-muted-foreground py-1">
                            +{contextStats.commands_items.length - 10} more...
                          </div>
                        )}
                      </div>
                    )}
                  </details>

                  {/* Skills - expandable */}
                  <details className="group">
                    <summary className="flex items-center justify-between p-2 bg-muted/30 rounded cursor-pointer hover:bg-muted/50">
                      <span className="text-sm flex items-center gap-2">
                        <ChevronRight className="h-3 w-3 group-open:rotate-90 transition-transform" />
                        Skills ({contextStats.skills_count})
                        <span className="text-[10px] text-muted-foreground">(on-demand)</span>
                      </span>
                      <span className="text-sm font-mono">
                        {formatNumber(contextStats.skills_tokens)} tokens
                        <span className="text-muted-foreground ml-2">
                          ({formatSize(contextStats.skills_chars)})
                        </span>
                      </span>
                    </summary>
                    {contextStats.skills_items.length > 0 ? (
                      <div className="mt-1 ml-5 space-y-1 text-xs">
                        {contextStats.skills_items.map((item) => (
                          <div
                            key={item.path}
                            className="flex justify-between py-1 px-2 bg-muted/20 rounded"
                          >
                            <span className="truncate flex-1">
                              {item.name}
                              <span className="text-muted-foreground ml-1">({item.scope})</span>
                            </span>
                            <span className="font-mono text-muted-foreground ml-2">
                              {formatNumber(item.tokens)} tok
                            </span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="mt-1 ml-5 text-xs text-muted-foreground py-1">
                        Plugin skills loaded on-demand when invoked
                      </div>
                    )}
                  </details>

                  {/* Settings - not expandable */}
                  <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
                    <span className="text-sm">Settings</span>
                    <span className="text-sm font-mono">
                      {formatNumber(contextStats.settings_tokens)} tokens
                      <span className="text-muted-foreground ml-2">
                        ({formatSize(contextStats.settings_chars)})
                      </span>
                    </span>
                  </div>

                  {/* MCP Servers - expandable with complexity */}
                  <details className="group">
                    <summary className="flex items-center justify-between p-2 bg-muted/30 rounded cursor-pointer hover:bg-muted/50">
                      <span className="text-sm flex items-center gap-2">
                        <ChevronRight className="h-3 w-3 group-open:rotate-90 transition-transform" />
                        MCP Servers ({contextStats.mcp_servers.length})
                      </span>
                      <span className="text-sm text-muted-foreground">complexity scores</span>
                    </summary>
                    {contextStats.mcp_servers.length > 0 ? (
                      <div className="mt-1 ml-5 space-y-1 text-xs">
                        {contextStats.mcp_servers.map((server) => (
                          <div
                            key={server.name}
                            className="flex justify-between items-center py-1.5 px-2 bg-muted/20 rounded"
                          >
                            <span className="truncate flex-1 flex items-center gap-2">
                              {server.name}
                              <span
                                className={`text-[10px] px-1.5 py-0.5 rounded ${
                                  server.server_type === 'http' || server.server_type === 'sse'
                                    ? 'bg-blue-500/20 text-blue-600'
                                    : 'bg-muted text-muted-foreground'
                                }`}
                              >
                                {server.server_type}
                              </span>
                              {server.is_plugin && (
                                <span className="text-[10px] bg-purple-500/20 text-purple-600 px-1.5 py-0.5 rounded">
                                  plugin
                                </span>
                              )}
                              {server.uses_wrapper && (
                                <span className="text-[10px] bg-amber-500/20 text-amber-600 px-1.5 py-0.5 rounded">
                                  wrapper
                                </span>
                              )}
                            </span>
                            <span
                              className={`font-mono ml-2 px-2 py-0.5 rounded ${
                                server.complexity_score >= 6
                                  ? 'bg-red-500/20 text-red-600'
                                  : server.complexity_score >= 4
                                    ? 'bg-amber-500/20 text-amber-600'
                                    : 'bg-green-500/20 text-green-600'
                              }`}
                            >
                              {server.complexity_score}
                            </span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="mt-1 ml-5 text-xs text-muted-foreground py-1">
                        No MCP servers configured
                      </div>
                    )}
                  </details>
                </div>
              )}

              <p className="text-xs text-muted-foreground mt-3 text-center">
                Token estimates based on actual file sizes (~3.5 chars/token). MCP complexity: 1-3
                low, 4-5 medium, 6+ high.
              </p>
            </div>
          )}
        </div>

        {/* CLAUDE.md Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader('claude-md', FileText, 'CLAUDE.md', claudeMdInfo?.exists ? 1 : 0)}
          {expandedSections.has('claude-md') && (
            <div className="p-4">
              {loadingClaudeMd ? (
                <p className="text-sm text-muted-foreground">Loading...</p>
              ) : claudeMdError ? (
                <div className="text-sm text-destructive">
                  Error loading CLAUDE.md:{' '}
                  {claudeMdError instanceof Error ? claudeMdError.message : String(claudeMdError)}
                </div>
              ) : (
                <>
                  <div className="flex items-center justify-between mb-3">
                    <p className="text-xs text-muted-foreground">
                      {claudeMdInfo?.exists
                        ? 'Project instructions for Claude'
                        : 'No CLAUDE.md file. Create one to add project instructions.'}
                    </p>
                    <Button
                      size="sm"
                      onClick={() => saveMutation.mutate()}
                      disabled={!claudeMdDirty || saveMutation.isPending}
                    >
                      <Save className="h-3 w-3 mr-1" />
                      {saveMutation.isPending ? 'Saving...' : 'Save'}
                    </Button>
                  </div>
                  <div className="mdx-editor-container h-80 border border-border rounded overflow-hidden">
                    <MDXEditor
                      key={`claude-md-${editorKey}`}
                      ref={editorRef}
                      markdown={claudeMdContent}
                      onChange={(markdown) => {
                        setClaudeMdContent(markdown);
                        setClaudeMdDirty(true);
                      }}
                      plugins={[
                        headingsPlugin(),
                        listsPlugin(),
                        quotePlugin(),
                        thematicBreakPlugin(),
                        markdownShortcutPlugin(),
                        linkPlugin(),
                        linkDialogPlugin(),
                        tablePlugin(),
                        codeBlockPlugin({ defaultCodeBlockLanguage: '' }),
                        codeMirrorPlugin({
                          codeBlockLanguages: {
                            js: 'JavaScript',
                            ts: 'TypeScript',
                            tsx: 'TypeScript (React)',
                            jsx: 'JavaScript (React)',
                            css: 'CSS',
                            html: 'HTML',
                            json: 'JSON',
                            python: 'Python',
                            rust: 'Rust',
                            bash: 'Bash',
                            sql: 'SQL',
                            markdown: 'Markdown',
                            '': 'Plain Text',
                          },
                        }),
                        toolbarPlugin({
                          toolbarContents: () => (
                            <>
                              <UndoRedo />
                              <Separator />
                              <BoldItalicUnderlineToggles />
                              <CodeToggle />
                              <Separator />
                              <ListsToggle />
                              <Separator />
                              <BlockTypeSelect />
                              <Separator />
                              <CreateLink />
                              <InsertTable />
                              <InsertThematicBreak />
                            </>
                          ),
                        }),
                      ]}
                      className={
                        theme === 'dark' ||
                        (theme === 'system' &&
                          window.matchMedia('(prefers-color-scheme: dark)').matches)
                          ? 'dark'
                          : ''
                      }
                      contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
                    />
                  </div>
                </>
              )}
            </div>
          )}
        </div>

        {/* Project-Scoped Tools - Items specific to this project */}
        {totalProjectTools > 0 && (
          <>
            {/* Project Skills */}
            {projectSkills.length > 0 && (
              <div className="tars-panel rounded-lg overflow-hidden">
                {renderSectionHeader(
                  'project-skills',
                  Sparkles,
                  'Skills',
                  projectSkills.length,
                  '/skills'
                )}
                {expandedSections.has('project-skills') && (
                  <div className="divide-y divide-border">
                    {projectSkills.map((skill) => (
                      <div key={skill.path} className="px-4 py-2.5 hover:bg-muted/30">
                        <div className="font-medium text-sm">{skill.name}</div>
                        {skill.description && (
                          <div className="text-xs text-muted-foreground mt-0.5">
                            {skill.description}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Project Commands */}
            {projectCommands.length > 0 && (
              <div className="tars-panel rounded-lg overflow-hidden">
                {renderSectionHeader(
                  'project-commands',
                  Terminal,
                  'Commands',
                  projectCommands.length,
                  '/commands'
                )}
                {expandedSections.has('project-commands') && (
                  <div className="divide-y divide-border">
                    {projectCommands.map((cmd) => (
                      <div key={cmd.path} className="px-4 py-2.5 hover:bg-muted/30">
                        <div className="font-medium text-sm font-mono">/{cmd.name}</div>
                        {cmd.description && (
                          <div className="text-xs text-muted-foreground mt-0.5">
                            {cmd.description}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Project Agents */}
            {projectAgents.length > 0 && (
              <div className="tars-panel rounded-lg overflow-hidden">
                {renderSectionHeader(
                  'project-agents',
                  Bot,
                  'Agents',
                  projectAgents.length,
                  '/agents'
                )}
                {expandedSections.has('project-agents') && (
                  <div className="divide-y divide-border">
                    {projectAgents.map((agent) => (
                      <div key={agent.path} className="px-4 py-2.5 hover:bg-muted/30">
                        <div className="font-medium text-sm">{agent.name}</div>
                        {agent.description && (
                          <div className="text-xs text-muted-foreground mt-0.5">
                            {agent.description}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Project Hooks */}
            {projectHooks.length > 0 && (
              <div className="tars-panel rounded-lg overflow-hidden">
                {renderSectionHeader(
                  'project-hooks',
                  Webhook,
                  'Hooks',
                  projectHooks.length,
                  '/hooks'
                )}
                {expandedSections.has('project-hooks') && (
                  <div className="divide-y divide-border">
                    {projectHooks.map((hook, i) => (
                      <div key={i} className="px-4 py-2.5 hover:bg-muted/30">
                        <div className="flex items-center gap-2">
                          <span className="font-medium text-sm">{hook.trigger}</span>
                          {hook.matcher && (
                            <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                              {hook.matcher}
                            </code>
                          )}
                        </div>
                        <div className="text-xs text-muted-foreground font-mono mt-0.5">
                          {hook.definition.command}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Project MCP Servers */}
            {projectMcpServers.length > 0 && (
              <div className="tars-panel rounded-lg overflow-hidden">
                {renderSectionHeader(
                  'project-mcp',
                  Server,
                  'MCP Servers',
                  projectMcpServers.length,
                  '/mcp'
                )}
                {expandedSections.has('project-mcp') && (
                  <div className="divide-y divide-border">
                    {projectMcpServers.map((server) => (
                      <div key={server.name} className="px-4 py-2.5 hover:bg-muted/30">
                        <div className="font-medium text-sm">{server.name}</div>
                        <div className="text-xs text-muted-foreground font-mono mt-0.5">
                          {server.command} {server.args.join(' ')}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </>
        )}

        {/* Global Tools Section - Collapsible, shows user-scoped items managed elsewhere */}
        {totalGlobalTools > 0 && (
          <div className="tars-panel rounded-lg overflow-hidden border-muted">
            <div
              onClick={() => toggleSection('global-tools')}
              className="flex items-center justify-between px-4 py-3 bg-muted/20 border-b border-border cursor-pointer hover:bg-muted/30"
            >
              <div className="flex items-center gap-3">
                {expandedSections.has('global-tools') ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )}
                <span className="font-medium text-muted-foreground">Global Tools</span>
                <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
                  {totalGlobalTools}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">Available from ~/.claude/</span>
            </div>

            {expandedSections.has('global-tools') && (
              <div className="divide-y divide-border/50">
                {/* Global Skills */}
                {globalSkills.length > 0 && (
                  <div>
                    <div
                      onClick={() => toggleSection('global-skills')}
                      className="flex items-center gap-2 px-4 py-2 bg-muted/10 cursor-pointer hover:bg-muted/20"
                    >
                      {expandedSections.has('global-skills') ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                      <Sparkles className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">Skills</span>
                      <span className="text-xs text-muted-foreground">({globalSkills.length})</span>
                      <div className="flex-1" />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          navigate('/skills');
                        }}
                        className="h-6 text-xs gap-1 text-muted-foreground"
                      >
                        Manage
                        <ExternalLink className="h-3 w-3" />
                      </Button>
                    </div>
                    {expandedSections.has('global-skills') && (
                      <div className="divide-y divide-border/30">
                        {globalSkills.map((skill) => (
                          <div key={skill.path} className="px-6 py-2 hover:bg-muted/20">
                            <div className="text-sm text-muted-foreground">{skill.name}</div>
                            {skill.description && (
                              <div className="text-xs text-muted-foreground/70 mt-0.5">
                                {skill.description}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Global Commands */}
                {globalCommands.length > 0 && (
                  <div>
                    <div
                      onClick={() => toggleSection('global-commands')}
                      className="flex items-center gap-2 px-4 py-2 bg-muted/10 cursor-pointer hover:bg-muted/20"
                    >
                      {expandedSections.has('global-commands') ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                      <Terminal className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">Commands</span>
                      <span className="text-xs text-muted-foreground">
                        ({globalCommands.length})
                      </span>
                      <div className="flex-1" />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          navigate('/commands');
                        }}
                        className="h-6 text-xs gap-1 text-muted-foreground"
                      >
                        Manage
                        <ExternalLink className="h-3 w-3" />
                      </Button>
                    </div>
                    {expandedSections.has('global-commands') && (
                      <div className="divide-y divide-border/30">
                        {globalCommands.map((cmd) => (
                          <div key={cmd.path} className="px-6 py-2 hover:bg-muted/20">
                            <div className="text-sm text-muted-foreground font-mono">
                              /{cmd.name}
                            </div>
                            {cmd.description && (
                              <div className="text-xs text-muted-foreground/70 mt-0.5">
                                {cmd.description}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Global Agents */}
                {globalAgents.length > 0 && (
                  <div>
                    <div
                      onClick={() => toggleSection('global-agents')}
                      className="flex items-center gap-2 px-4 py-2 bg-muted/10 cursor-pointer hover:bg-muted/20"
                    >
                      {expandedSections.has('global-agents') ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                      <Bot className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">Agents</span>
                      <span className="text-xs text-muted-foreground">({globalAgents.length})</span>
                      <div className="flex-1" />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          navigate('/agents');
                        }}
                        className="h-6 text-xs gap-1 text-muted-foreground"
                      >
                        Manage
                        <ExternalLink className="h-3 w-3" />
                      </Button>
                    </div>
                    {expandedSections.has('global-agents') && (
                      <div className="divide-y divide-border/30">
                        {globalAgents.map((agent) => (
                          <div key={agent.path} className="px-6 py-2 hover:bg-muted/20">
                            <div className="text-sm text-muted-foreground">{agent.name}</div>
                            {agent.description && (
                              <div className="text-xs text-muted-foreground/70 mt-0.5">
                                {agent.description}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Global MCP Servers */}
                {globalMcpServers.reduce((acc, g) => acc + g.servers.length, 0) > 0 && (
                  <div>
                    <div
                      onClick={() => toggleSection('global-mcp')}
                      className="flex items-center gap-2 px-4 py-2 bg-muted/10 cursor-pointer hover:bg-muted/20"
                    >
                      {expandedSections.has('global-mcp') ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                      <Server className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">MCP Servers</span>
                      <span className="text-xs text-muted-foreground">
                        ({globalMcpServers.reduce((acc, g) => acc + g.servers.length, 0)})
                      </span>
                      <div className="flex-1" />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          navigate('/mcp');
                        }}
                        className="h-6 text-xs gap-1 text-muted-foreground"
                      >
                        Manage
                        <ExternalLink className="h-3 w-3" />
                      </Button>
                    </div>
                    {expandedSections.has('global-mcp') && (
                      <div className="divide-y divide-border/30">
                        {globalMcpServers.map((group) =>
                          group.servers.map((server) => (
                            <div
                              key={`${group.scope}-${server.name}`}
                              className="px-6 py-2 hover:bg-muted/20"
                            >
                              <div className="flex items-center gap-2">
                                <span className="text-sm text-muted-foreground">{server.name}</span>
                                <span className="text-[10px] text-muted-foreground/60 bg-muted px-1.5 py-0.5 rounded">
                                  {group.scope}
                                </span>
                              </div>
                              <div className="text-xs text-muted-foreground/70 font-mono mt-0.5">
                                {server.command} {server.args.join(' ')}
                              </div>
                            </div>
                          ))
                        )}
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* Collisions Section */}
        {totalCollisions > 0 && (
          <div className="tars-panel rounded-lg overflow-hidden border-destructive/50">
            {renderSectionHeader('collisions', AlertTriangle, 'Collisions', totalCollisions)}
            {expandedSections.has('collisions') && (
              <div className="p-4 space-y-3">
                <p className="text-sm text-muted-foreground">
                  These items exist in multiple scopes, which may cause conflicts.
                </p>
                {collisions.skills.map((c) => (
                  <div key={c.name} className="bg-destructive/10 rounded p-3">
                    <div className="font-medium text-destructive">Skill: {c.name}</div>
                    <div className="text-xs text-muted-foreground mt-1">
                      Found in: {c.occurrences.map((o) => formatCollisionScope(o.scope)).join(', ')}
                    </div>
                    {c.winner_scope && (
                      <div className="text-xs text-muted-foreground mt-1">
                        Winner: {formatCollisionScope(c.winner_scope)}
                      </div>
                    )}
                  </div>
                ))}
                {collisions.commands.map((c) => (
                  <div key={c.name} className="bg-destructive/10 rounded p-3">
                    <div className="font-medium text-destructive">Command: /{c.name}</div>
                    <div className="text-xs text-muted-foreground mt-1">
                      Found in: {c.occurrences.map((o) => formatCollisionScope(o.scope)).join(', ')}
                    </div>
                    {c.winner_scope && (
                      <div className="text-xs text-muted-foreground mt-1">
                        Winner: {formatCollisionScope(c.winner_scope)}
                      </div>
                    )}
                  </div>
                ))}
                {collisions.agents.map((c) => (
                  <div key={c.name} className="bg-destructive/10 rounded p-3">
                    <div className="font-medium text-destructive">Agent: {c.name}</div>
                    <div className="text-xs text-muted-foreground mt-1">
                      Found in: {c.occurrences.map((o) => formatCollisionScope(o.scope)).join(', ')}
                    </div>
                    {c.winner_scope && (
                      <div className="text-xs text-muted-foreground mt-1">
                        Winner: {formatCollisionScope(c.winner_scope)}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Local Tool Picker Dialog */}
      <ProfileToolPicker
        open={isToolPickerOpen}
        onOpenChange={setIsToolPickerOpen}
        onAddTools={handleAddLocalTools}
        existingTools={combinedTools.map((t) => ({
          name: t.name,
          tool_type: t.tool_type,
          source_scope: t.source_scope,
          permissions: t.permissions,
          source_ref: t.source_ref ?? null,
        }))}
      />
    </div>
  );
}
