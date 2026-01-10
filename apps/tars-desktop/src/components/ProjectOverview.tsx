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
} from '../lib/types';
import { readClaudeMd, saveClaudeMd, getContextStats } from '../lib/ipc';
import { Button } from './ui/button';

interface ProjectOverviewProps {
  inventory: Inventory;
  projectPath: string;
}

type SectionId =
  | 'context'
  | 'claude-md'
  | 'skills'
  | 'commands'
  | 'agents'
  | 'hooks'
  | 'mcp'
  | 'collisions';

interface ScopeGroup<T> {
  scope: string;
  items: T[];
}

function groupByScope<T extends { scope: { type: string } | string }>(items: T[]): ScopeGroup<T>[] {
  const groups: Record<string, T[]> = {};

  for (const item of items) {
    const scopeType = typeof item.scope === 'string' ? item.scope : item.scope.type;
    if (!groups[scopeType]) {
      groups[scopeType] = [];
    }
    groups[scopeType].push(item);
  }

  // Order: User, Project, Plugin, Managed
  const order = ['User', 'Project', 'Plugin', 'Managed', 'Local'];
  return order
    .filter((scope) => groups[scope]?.length > 0)
    .map((scope) => ({ scope, items: groups[scope] }));
}

export function ProjectOverview({ inventory, projectPath }: ProjectOverviewProps) {
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

  // Combine user and project items for display
  const allSkills: SkillInfo[] = [
    ...userScope.skills,
    ...(projectData?.skills || []),
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

  const allCommands: CommandInfo[] = [...userScope.commands, ...(projectData?.commands || [])];

  const allAgents: AgentInfo[] = [...userScope.agents, ...(projectData?.agents || [])];

  const allHooks: HookInfo[] = projectData?.hooks || [];

  const allMcpServers: { scope: string; servers: McpServer[] }[] = [
    ...(userScope.mcp?.servers.length ? [{ scope: 'User', servers: userScope.mcp.servers }] : []),
    ...(projectData?.mcp?.servers.length
      ? [{ scope: 'Project', servers: projectData.mcp.servers }]
      : []),
    ...(inventory.managed_scope?.mcp?.servers.length
      ? [{ scope: 'Managed', servers: inventory.managed_scope.mcp.servers }]
      : []),
  ];

  const totalCollisions =
    collisions.skills.length + collisions.commands.length + collisions.agents.length;

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

  const renderScopeLabel = (scope: string) => (
    <div className="text-xs font-medium text-primary uppercase tracking-wider px-4 py-2 bg-primary/5 border-b border-primary/10">
      {scope} Scope
    </div>
  );

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

        {/* Skills Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader('skills', Sparkles, 'Skills', allSkills.length, '/skills')}
          {expandedSections.has('skills') && (
            <div>
              {allSkills.length === 0 ? (
                <p className="text-sm text-muted-foreground p-4">No skills configured</p>
              ) : (
                groupByScope(allSkills).map((group) => (
                  <div key={group.scope}>
                    {renderScopeLabel(group.scope)}
                    <div className="divide-y divide-border">
                      {group.items.map((skill) => (
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
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        {/* Commands Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader('commands', Terminal, 'Commands', allCommands.length, '/commands')}
          {expandedSections.has('commands') && (
            <div>
              {allCommands.length === 0 ? (
                <p className="text-sm text-muted-foreground p-4">No commands configured</p>
              ) : (
                groupByScope(allCommands).map((group) => (
                  <div key={group.scope}>
                    {renderScopeLabel(group.scope)}
                    <div className="divide-y divide-border">
                      {group.items.map((cmd) => (
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
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        {/* Agents Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader('agents', Bot, 'Agents', allAgents.length, '/agents')}
          {expandedSections.has('agents') && (
            <div>
              {allAgents.length === 0 ? (
                <p className="text-sm text-muted-foreground p-4">No agents configured</p>
              ) : (
                groupByScope(allAgents).map((group) => (
                  <div key={group.scope}>
                    {renderScopeLabel(group.scope)}
                    <div className="divide-y divide-border">
                      {group.items.map((agent) => (
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
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        {/* Hooks Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader('hooks', Webhook, 'Hooks', allHooks.length, '/hooks')}
          {expandedSections.has('hooks') && (
            <div>
              {allHooks.length === 0 ? (
                <p className="text-sm text-muted-foreground p-4">
                  No hooks configured for this project
                </p>
              ) : (
                <div className="divide-y divide-border">
                  {allHooks.map((hook, i) => (
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
        </div>

        {/* MCP Servers Section */}
        <div className="tars-panel rounded-lg overflow-hidden">
          {renderSectionHeader(
            'mcp',
            Server,
            'MCP Servers',
            allMcpServers.reduce((acc, g) => acc + g.servers.length, 0),
            '/mcp'
          )}
          {expandedSections.has('mcp') && (
            <div>
              {allMcpServers.length === 0 ? (
                <p className="text-sm text-muted-foreground p-4">No MCP servers configured</p>
              ) : (
                allMcpServers.map((group) => (
                  <div key={group.scope}>
                    {renderScopeLabel(group.scope)}
                    <div className="divide-y divide-border">
                      {group.servers.map((server) => (
                        <div key={server.name} className="px-4 py-2.5 hover:bg-muted/30">
                          <div className="font-medium text-sm">{server.name}</div>
                          <div className="text-xs text-muted-foreground font-mono mt-0.5">
                            {server.command} {server.args.join(' ')}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ))
              )}
            </div>
          )}
        </div>

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
                      Found in: {c.occurrences.map((o) => o.scope).join(', ')}
                    </div>
                  </div>
                ))}
                {collisions.commands.map((c) => (
                  <div key={c.name} className="bg-destructive/10 rounded p-3">
                    <div className="font-medium text-destructive">Command: /{c.name}</div>
                    <div className="text-xs text-muted-foreground mt-1">
                      Found in: {c.occurrences.map((o) => o.scope).join(', ')}
                    </div>
                  </div>
                ))}
                {collisions.agents.map((c) => (
                  <div key={c.name} className="bg-destructive/10 rounded p-3">
                    <div className="font-medium text-destructive">Agent: {c.name}</div>
                    <div className="text-xs text-muted-foreground mt-1">
                      Found in: {c.occurrences.map((o) => o.scope).join(', ')}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
