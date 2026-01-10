import { useQuery } from '@tanstack/react-query';
import { Bot, Plus, RefreshCw, Search, Trash2, FolderOpen, Power, PowerOff } from 'lucide-react';
import { useState, useMemo } from 'react';
import { toast } from 'sonner';
import {
  scanUserScope,
  scanProjects,
  readAgent,
  saveAgent,
  createAgent,
  deleteAgent,
  moveAgent,
  listProjects,
  disableAgent,
  enableAgent,
  listDisabledAgents,
} from '../lib/ipc';
import { MarkdownEditor } from '../components/MarkdownEditor';
import { Button } from '../components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { ConfirmDialog } from '../components/config/ConfirmDialog';
import { HelpButton } from '../components/HelpButton';
import type { AgentInfo, AgentDetails } from '../lib/types';

/** Check if an agent is editable (user-created agents only) */
function isAgentEditable(scope: { type: string }): boolean {
  return scope.type === 'User' || scope.type === 'Project' || scope.type === 'Local';
}

/** Get scope category for grouping */
function getScopeCategory(scope: { type: string }): string {
  switch (scope.type) {
    case 'User':
      return 'user';
    case 'Project':
    case 'Local':
      return 'project';
    case 'Managed':
      return 'managed';
    case 'Plugin':
      return 'plugin';
    default:
      return 'user';
  }
}

export function AgentsPage() {
  const [selectedAgent, setSelectedAgent] = useState<AgentDetails | null>(null);
  const [loadingAgent, setLoadingAgent] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newAgentName, setNewAgentName] = useState('');
  const [createScope, setCreateScope] = useState<'user' | 'project'>('user');
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [agentToDelete, setAgentToDelete] = useState<AgentInfo | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [showMoveDialog, setShowMoveDialog] = useState(false);
  const [moveTargetScope, setMoveTargetScope] = useState<'user' | 'project'>('user');
  const [moveTargetProjects, setMoveTargetProjects] = useState<string[]>([]);
  const [moving, setMoving] = useState(false);
  const [showDisabled, setShowDisabled] = useState(false);
  const [disabling, setDisabling] = useState<string | null>(null);

  // Get configured projects for project picker
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  // Scan user scope
  const {
    data: inventory,
    isLoading: isLoadingUserScope,
    refetch: refetchUserScope,
  } = useQuery({
    queryKey: ['user-scope'],
    queryFn: scanUserScope,
  });

  // Scan all configured projects
  const {
    data: projectsInventory,
    isLoading: isLoadingProjects,
    refetch: refetchProjects,
  } = useQuery({
    queryKey: ['projects-scan', projects.map((p) => p.path)],
    queryFn: () =>
      projects.length > 0 ? scanProjects(projects.map((p) => p.path)) : Promise.resolve(null),
    enabled: projects.length > 0,
  });

  // Get disabled agents (user level only - project agents handled separately if needed)
  const { data: disabledAgents = [], refetch: refetchDisabled } = useQuery({
    queryKey: ['disabled-agents'],
    queryFn: async () => {
      // Only fetch user-level disabled agents to avoid duplicates
      const userDisabled = await listDisabledAgents();
      return userDisabled;
    },
  });

  const isLoading = isLoadingUserScope || isLoadingProjects;

  async function refetch() {
    await Promise.all([refetchUserScope(), refetchProjects(), refetchDisabled()]);
  }

  // Combine agents from user scope and all projects
  const agents = useMemo(() => {
    const allAgents: AgentInfo[] = [];

    // Add user scope agents
    if (inventory?.user_scope.agents) {
      allAgents.push(...inventory.user_scope.agents);
    }

    // Add agents from scanned projects
    if (projectsInventory?.projects) {
      for (const project of projectsInventory.projects) {
        if (project.agents) {
          allAgents.push(...project.agents);
        }
      }
    }

    return allAgents;
  }, [inventory, projectsInventory]);

  // Group agents by category
  const groupedAgents = useMemo(() => {
    const filtered = searchQuery
      ? agents.filter(
          (a) =>
            a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            a.description?.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : agents;

    const groups: Record<string, AgentInfo[]> = {
      user: [],
      project: [],
      plugin: [],
      managed: [],
    };

    for (const agent of filtered) {
      const category = getScopeCategory(agent.scope);
      groups[category].push(agent);
    }

    return groups;
  }, [agents, searchQuery]);

  async function handleSelectAgent(agent: AgentInfo) {
    setLoadingAgent(true);
    try {
      const details = await readAgent(agent.path);
      setSelectedAgent(details);
    } catch (err) {
      console.error('Failed to load agent:', err);
      toast.error('Failed to load agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setLoadingAgent(false);
    }
  }

  async function handleSaveAgent(path: string, content: string) {
    try {
      await saveAgent(path, content);
      // Reload the agent
      const details = await readAgent(path);
      setSelectedAgent(details);
      toast.success('Agent saved');
    } catch (err) {
      console.error('Failed to save agent:', err);
      toast.error('Failed to save agent', {
        description: err instanceof Error ? err.message : String(err),
      });
      throw err;
    }
  }

  async function handleCreateAgent() {
    if (!newAgentName.trim()) return;
    if (createScope === 'project' && !selectedProject) {
      toast.error('Please select a project');
      return;
    }

    setCreating(true);
    try {
      const details = await createAgent(
        newAgentName.trim(),
        createScope,
        createScope === 'project' ? (selectedProject ?? undefined) : undefined
      );
      const scopeDesc =
        createScope === 'user'
          ? 'user scope'
          : `project "${projects.find((p) => p.path === selectedProject)?.name}"`;
      toast.success(`Created agent "${newAgentName}"`, {
        description: `Added to ${scopeDesc}`,
      });
      setShowCreateDialog(false);
      setNewAgentName('');
      setCreateScope('user');
      setSelectedProject(null);
      // Refresh the list and select the new agent
      await refetch();
      setSelectedAgent(details);
    } catch (err) {
      console.error('Failed to create agent:', err);
      toast.error('Failed to create agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setCreating(false);
    }
  }

  async function handleDeleteAgent() {
    if (!agentToDelete) return;

    setDeleting(true);
    try {
      await deleteAgent(agentToDelete.path);
      toast.success(`Deleted agent "${agentToDelete.name}"`);

      // Clear selection if the deleted agent was selected
      if (selectedAgent?.path === agentToDelete.path) {
        setSelectedAgent(null);
      }

      // Refresh the list
      await refetch();
    } catch (err) {
      console.error('Failed to delete agent:', err);
      toast.error('Failed to delete agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDeleting(false);
      setAgentToDelete(null);
    }
  }

  function handleOpenMoveDialog() {
    if (!selectedAgent) return;
    // Set initial target to the opposite of current scope
    const currentScope = selectedAgent.scope;
    if (currentScope === 'user') {
      setMoveTargetScope('project');
    } else {
      setMoveTargetScope('user');
    }
    setMoveTargetProjects([]);
    setShowMoveDialog(true);
  }

  function toggleProjectSelection(projectPath: string) {
    setMoveTargetProjects((prev) =>
      prev.includes(projectPath) ? prev.filter((p) => p !== projectPath) : [...prev, projectPath]
    );
  }

  async function handleMoveAgent() {
    if (!selectedAgent) return;
    if (moveTargetScope === 'project' && moveTargetProjects.length === 0) {
      toast.error('Please select at least one project');
      return;
    }

    setMoving(true);
    try {
      const details = await moveAgent(
        selectedAgent.path,
        moveTargetScope,
        moveTargetScope === 'project' ? moveTargetProjects : undefined
      );

      let scopeDesc: string;
      if (moveTargetScope === 'user') {
        scopeDesc = 'user scope';
      } else if (moveTargetProjects.length === 1) {
        const projectName = projects.find((p) => p.path === moveTargetProjects[0])?.name;
        scopeDesc = `project "${projectName}"`;
      } else {
        scopeDesc = `${moveTargetProjects.length} projects`;
      }

      toast.success(`Moved agent "${selectedAgent.name}"`, {
        description: `Now in ${scopeDesc}`,
      });
      setShowMoveDialog(false);
      setMoveTargetProjects([]);
      // Refresh the list and update selection
      await refetch();
      setSelectedAgent(details);
    } catch (err) {
      console.error('Failed to move agent:', err);
      toast.error('Failed to move agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setMoving(false);
    }
  }

  async function handleDisableAgent(agent: AgentInfo) {
    setDisabling(agent.path);
    try {
      await disableAgent(agent.path);
      toast.success(`Disabled agent "${agent.name}"`);
      // Clear selection if this was the selected agent
      if (selectedAgent?.path === agent.path) {
        setSelectedAgent(null);
      }
      await refetch();
    } catch (err) {
      console.error('Failed to disable agent:', err);
      toast.error('Failed to disable agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDisabling(null);
    }
  }

  async function handleEnableAgent(agent: AgentDetails) {
    setDisabling(agent.path);
    try {
      const newPath = await enableAgent(agent.path);
      toast.success(`Enabled agent "${agent.name}"`);
      await refetch();
      // Select the re-enabled agent
      const details = await readAgent(newPath);
      setSelectedAgent(details);
    } catch (err) {
      console.error('Failed to enable agent:', err);
      toast.error('Failed to enable agent', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDisabling(null);
    }
  }

  function renderAgentGroup(title: string, groupAgents: AgentInfo[], showActions = false) {
    if (groupAgents.length === 0) return null;

    return (
      <div key={title} className="mb-4">
        <h3 className="text-xs font-semibold text-primary uppercase tracking-wider px-3 py-2 border-b border-primary/20 mb-2">
          {title} <span className="text-primary/60">({groupAgents.length})</span>
        </h3>
        <ul className="space-y-1">
          {groupAgents.map((agent) => (
            <li key={agent.path} className="group relative">
              <button
                onClick={() => handleSelectAgent(agent)}
                className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
                  selectedAgent?.path === agent.path
                    ? 'active text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="font-medium flex-1 truncate">{agent.name}</span>
                </div>
                {agent.description && (
                  <div className="text-xs opacity-60 truncate mt-0.5">{agent.description}</div>
                )}
              </button>
              {showActions && isAgentEditable(agent.scope) && (
                <div className="absolute right-1 top-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDisableAgent(agent);
                    }}
                    disabled={disabling === agent.path}
                    className="p-1 rounded hover:bg-amber-500/10 text-muted-foreground hover:text-amber-500 transition-all disabled:opacity-50"
                    title="Disable agent"
                  >
                    <PowerOff className="h-3.5 w-3.5" />
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setAgentToDelete(agent);
                    }}
                    className="p-1 rounded hover:bg-amber-500/10 text-muted-foreground hover:text-amber-500 transition-all"
                    title="Delete agent"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              )}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 brushed-metal relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Agents</h2>
          <HelpButton section="AGENTS" />
        </div>
        <Button onClick={() => setShowCreateDialog(true)} size="sm">
          <Plus className="h-4 w-4 mr-2" />
          New Agent
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Agents list sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search agents..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="tars-input w-full pl-9 pr-3 py-2 text-sm rounded"
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
                data-form-type="other"
              />
              <Search className="absolute left-3 h-4 w-4 text-muted-foreground pointer-events-none" />
            </div>
          </div>

          <div className="tars-segment-line" />

          <div className="flex-1 overflow-auto p-3">
            {isLoading ? (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <RefreshCw className="h-5 w-5 animate-spin text-primary" />
                <span className="text-xs text-muted-foreground">Loading...</span>
              </div>
            ) : agents.length === 0 ? (
              <div className="text-center py-12 px-4">
                <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
                  <Bot className="h-8 w-8 text-muted-foreground" />
                </div>
                <p className="text-sm font-medium text-foreground">No agents found</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Agents live in ~/.claude/agents/
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4"
                  onClick={() => setShowCreateDialog(true)}
                >
                  <Plus className="h-4 w-4 mr-2" />
                  Create your first agent
                </Button>
              </div>
            ) : (
              <>
                {renderAgentGroup('User Agents', groupedAgents.user, true)}
                {renderAgentGroup('Project Agents', groupedAgents.project, true)}
                {renderAgentGroup('Plugin Agents', groupedAgents.plugin, false)}
                {renderAgentGroup('Managed Agents', groupedAgents.managed, false)}
                {/* Disabled Agents Section */}
                {disabledAgents.length > 0 && (
                  <div className="mb-4">
                    <button
                      onClick={() => setShowDisabled(!showDisabled)}
                      className="w-full text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider px-3 py-2 border-b border-muted mb-2 hover:text-foreground transition-colors flex items-center gap-2"
                    >
                      <PowerOff className="h-3 w-3" />
                      Disabled <span className="opacity-60">({disabledAgents.length})</span>
                      <span className="ml-auto text-[10px]">{showDisabled ? '▼' : '►'}</span>
                    </button>
                    {showDisabled && (
                      <ul className="space-y-1">
                        {disabledAgents.map((agent) => (
                          <li key={agent.path} className="group relative">
                            <button
                              onClick={() => setSelectedAgent(agent)}
                              className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all opacity-60 ${
                                selectedAgent?.path === agent.path
                                  ? 'active text-foreground font-medium'
                                  : 'text-muted-foreground hover:text-foreground'
                              }`}
                            >
                              <div className="flex items-center gap-2">
                                <span className="font-medium flex-1 truncate">{agent.name}</span>
                              </div>
                              {agent.description && (
                                <div className="text-xs opacity-60 truncate mt-0.5">
                                  {agent.description}
                                </div>
                              )}
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                handleEnableAgent(agent);
                              }}
                              disabled={disabling === agent.path}
                              className="absolute right-1 top-1.5 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-emerald-500/10 text-muted-foreground hover:text-emerald-500 transition-all disabled:opacity-50"
                              title="Enable agent"
                            >
                              <Power className="h-3.5 w-3.5" />
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                )}
              </>
            )}
          </div>
        </div>

        {/* Agent editor */}
        <div className="flex-1 overflow-hidden bg-background">
          {loadingAgent ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <p className="text-sm text-muted-foreground">Loading agent...</p>
            </div>
          ) : selectedAgent ? (
            <MarkdownEditor
              item={selectedAgent}
              onSave={handleSaveAgent}
              onMove={handleOpenMoveDialog}
              readOnly={selectedAgent.scope !== 'user' && selectedAgent.scope !== 'project'}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <Bot className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">Select an agent to edit</p>
                <p className="text-xs text-muted-foreground/60 mt-1">
                  Agents are specialized task handlers
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Create Agent Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Agent</DialogTitle>
            <DialogDescription>Create a new agent in your user or project scope.</DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label htmlFor="agent-name">Agent Name</Label>
              <Input
                id="agent-name"
                value={newAgentName}
                onChange={(e) => setNewAgentName(e.target.value)}
                placeholder="my-agent"
                className="mt-2"
                onKeyDown={(e) => {
                  if (
                    e.key === 'Enter' &&
                    newAgentName.trim() &&
                    (createScope === 'user' || selectedProject)
                  ) {
                    handleCreateAgent();
                  }
                }}
              />
              <p className="text-xs text-muted-foreground mt-2">
                Use lowercase letters, numbers, and hyphens.
              </p>
            </div>

            <div>
              <Label>Scope</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="scope"
                    checked={createScope === 'user'}
                    onChange={() => {
                      setCreateScope('user');
                      setSelectedProject(null);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">User (~/.claude/agents/)</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="scope"
                    checked={createScope === 'project'}
                    onChange={() => setCreateScope('project')}
                    className="accent-primary"
                  />
                  <span className="text-sm">Project</span>
                </label>
              </div>
            </div>

            {createScope === 'project' && (
              <div>
                <Label>Project</Label>
                {projects.length === 0 ? (
                  <p className="text-sm text-muted-foreground mt-2">
                    No projects configured. Add a project first.
                  </p>
                ) : (
                  <div className="mt-2 space-y-1">
                    {projects.map((project) => (
                      <label
                        key={project.path}
                        className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                          selectedProject === project.path
                            ? 'bg-primary/10 border border-primary/30'
                            : 'hover:bg-muted border border-transparent'
                        }`}
                      >
                        <input
                          type="radio"
                          name="project"
                          checked={selectedProject === project.path}
                          onChange={() => setSelectedProject(project.path)}
                          className="accent-primary"
                        />
                        <FolderOpen className="h-4 w-4 text-muted-foreground" />
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium truncate">{project.name}</div>
                          <div className="text-xs text-muted-foreground truncate">
                            {project.path}
                          </div>
                        </div>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateDialog(false)}
              disabled={creating}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateAgent}
              disabled={
                creating || !newAgentName.trim() || (createScope === 'project' && !selectedProject)
              }
            >
              {creating ? 'Creating...' : 'Create Agent'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={!!agentToDelete}
        onOpenChange={(open) => !open && setAgentToDelete(null)}
        title="Delete Agent"
        description={`Are you sure you want to delete "${agentToDelete?.name}"? This action cannot be undone.`}
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={handleDeleteAgent}
        loading={deleting}
      />

      {/* Move Agent Dialog */}
      <Dialog open={showMoveDialog} onOpenChange={setShowMoveDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Move Agent</DialogTitle>
            <DialogDescription>
              Move "{selectedAgent?.name}" to a different scope.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label>Target Scope</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="move-scope"
                    checked={moveTargetScope === 'user'}
                    onChange={() => {
                      setMoveTargetScope('user');
                      setMoveTargetProjects([]);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">User (~/.claude/agents/)</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="move-scope"
                    checked={moveTargetScope === 'project'}
                    onChange={() => setMoveTargetScope('project')}
                    className="accent-primary"
                  />
                  <span className="text-sm">Project(s)</span>
                </label>
              </div>
            </div>

            {moveTargetScope === 'project' && (
              <div>
                <Label>
                  Select Projects{' '}
                  {moveTargetProjects.length > 0 && `(${moveTargetProjects.length} selected)`}
                </Label>
                {projects.length === 0 ? (
                  <p className="text-sm text-muted-foreground mt-2">
                    No projects configured. Add a project first.
                  </p>
                ) : (
                  <div className="mt-2 space-y-1 max-h-48 overflow-auto">
                    {projects.map((project) => (
                      <label
                        key={project.path}
                        className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                          moveTargetProjects.includes(project.path)
                            ? 'bg-primary/10 border border-primary/30'
                            : 'hover:bg-muted border border-transparent'
                        }`}
                      >
                        <input
                          type="checkbox"
                          checked={moveTargetProjects.includes(project.path)}
                          onChange={() => toggleProjectSelection(project.path)}
                          className="accent-primary"
                        />
                        <FolderOpen className="h-4 w-4 text-muted-foreground" />
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium truncate">{project.name}</div>
                          <div className="text-xs text-muted-foreground truncate">
                            {project.path}
                          </div>
                        </div>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowMoveDialog(false)} disabled={moving}>
              Cancel
            </Button>
            <Button
              onClick={handleMoveAgent}
              disabled={
                moving || (moveTargetScope === 'project' && moveTargetProjects.length === 0)
              }
            >
              {moving ? 'Moving...' : 'Move Agent'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
