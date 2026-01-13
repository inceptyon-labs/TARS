import { useQuery } from '@tanstack/react-query';
import { Terminal, Plus, RefreshCw, Search, Trash2, FolderOpen } from 'lucide-react';
import { useState, useMemo } from 'react';
import { toast } from 'sonner';
import {
  scanUserScope,
  scanProjects,
  readCommand,
  saveCommand,
  createCommand,
  deleteCommand,
  moveCommand,
  listProjects,
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
import type { CommandInfo, CommandDetails } from '../lib/types';

/** Check if a command is editable (user-created commands only) */
function isCommandEditable(scope: { type: string }): boolean {
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

export function CommandsPage() {
  const [selectedCommand, setSelectedCommand] = useState<CommandDetails | null>(null);
  const [loadingCommand, setLoadingCommand] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newCommandName, setNewCommandName] = useState('');
  const [createScope, setCreateScope] = useState<'user' | 'project'>('user');
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [commandToDelete, setCommandToDelete] = useState<CommandInfo | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [showMoveDialog, setShowMoveDialog] = useState(false);
  const [moveTargetScope, setMoveTargetScope] = useState<'user' | 'project'>('user');
  const [moveTargetProjects, setMoveTargetProjects] = useState<string[]>([]);
  const [moving, setMoving] = useState(false);

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

  const isLoading = isLoadingUserScope || isLoadingProjects;

  async function refetch() {
    await Promise.all([refetchUserScope(), refetchProjects()]);
  }

  // Combine commands from user scope and all projects
  const commands = useMemo(() => {
    const allCommands: CommandInfo[] = [];

    // Add user scope commands
    if (inventory?.user_scope.commands) {
      allCommands.push(...inventory.user_scope.commands);
    }

    // Add commands from scanned projects
    if (projectsInventory?.projects) {
      for (const project of projectsInventory.projects) {
        if (project.commands) {
          allCommands.push(...project.commands);
        }
      }
    }

    return allCommands;
  }, [inventory, projectsInventory]);

  // Group commands by category
  const groupedCommands = useMemo(() => {
    const filtered = searchQuery
      ? commands.filter(
          (c) =>
            c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            c.description?.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : commands;

    const groups: Record<string, CommandInfo[]> = {
      user: [],
      project: [],
      plugin: [],
      managed: [],
    };

    for (const command of filtered) {
      const category = getScopeCategory(command.scope);
      groups[category].push(command);
    }

    return groups;
  }, [commands, searchQuery]);

  async function handleSelectCommand(command: CommandInfo) {
    setLoadingCommand(true);
    try {
      const details = await readCommand(command.path);
      setSelectedCommand(details);
    } catch (err) {
      console.error('Failed to load command:', err);
      toast.error('Failed to load command', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setLoadingCommand(false);
    }
  }

  async function handleSaveCommand(path: string, content: string) {
    try {
      await saveCommand(path, content);
      // Reload the command
      const details = await readCommand(path);
      setSelectedCommand(details);
      toast.success('Command saved');
    } catch (err) {
      console.error('Failed to save command:', err);
      toast.error('Failed to save command', {
        description: err instanceof Error ? err.message : String(err),
      });
      throw err;
    }
  }

  async function handleCreateCommand() {
    if (!newCommandName.trim()) return;
    if (createScope === 'project' && !selectedProject) {
      toast.error('Please select a project');
      return;
    }

    setCreating(true);
    try {
      const details = await createCommand(
        newCommandName.trim(),
        createScope,
        createScope === 'project' ? (selectedProject ?? undefined) : undefined
      );
      const scopeDesc =
        createScope === 'user'
          ? 'user scope'
          : `project "${projects.find((p) => p.path === selectedProject)?.name}"`;
      toast.success(`Created command "${newCommandName}"`, {
        description: `Added to ${scopeDesc}`,
      });
      setShowCreateDialog(false);
      setNewCommandName('');
      setCreateScope('user');
      setSelectedProject(null);
      // Refresh the list and select the new command
      await refetch();
      setSelectedCommand(details);
    } catch (err) {
      console.error('Failed to create command:', err);
      toast.error('Failed to create command', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setCreating(false);
    }
  }

  async function handleDeleteCommand() {
    if (!commandToDelete) return;

    setDeleting(true);
    try {
      await deleteCommand(commandToDelete.path);
      toast.success(`Deleted command "${commandToDelete.name}"`);

      // Clear selection if the deleted command was selected
      if (selectedCommand?.path === commandToDelete.path) {
        setSelectedCommand(null);
      }

      // Refresh the list
      await refetch();
    } catch (err) {
      console.error('Failed to delete command:', err);
      toast.error('Failed to delete command', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDeleting(false);
      setCommandToDelete(null);
    }
  }

  function handleOpenMoveDialog() {
    if (!selectedCommand) return;
    // Set initial target to the opposite of current scope
    const currentScope = selectedCommand.scope;
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

  async function handleMoveCommand() {
    if (!selectedCommand) return;
    if (moveTargetScope === 'project' && moveTargetProjects.length === 0) {
      toast.error('Please select at least one project');
      return;
    }

    setMoving(true);
    try {
      const details = await moveCommand(
        selectedCommand.path,
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

      toast.success(`Moved command "${selectedCommand.name}"`, {
        description: `Now in ${scopeDesc}`,
      });
      setShowMoveDialog(false);
      setMoveTargetProjects([]);
      // Refresh the list and update selection
      await refetch();
      setSelectedCommand(details);
    } catch (err) {
      console.error('Failed to move command:', err);
      toast.error('Failed to move command', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setMoving(false);
    }
  }

  function renderCommandGroup(title: string, groupCommands: CommandInfo[], showActions = false) {
    if (groupCommands.length === 0) return null;

    return (
      <div key={title} className="mb-4">
        <h3 className="text-xs font-semibold text-primary uppercase tracking-wider px-3 py-2 border-b border-primary/20 mb-2">
          {title} <span className="text-primary/60">({groupCommands.length})</span>
        </h3>
        <ul className="space-y-1">
          {groupCommands.map((command) => (
            <li key={command.path} className="group relative">
              <button
                onClick={() => handleSelectCommand(command)}
                className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
                  selectedCommand?.path === command.path
                    ? 'active text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="font-medium flex-1 truncate">/{command.name}</span>
                </div>
                {command.description && (
                  <div className="text-xs opacity-60 truncate mt-0.5">{command.description}</div>
                )}
              </button>
              {showActions && isCommandEditable(command.scope) && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setCommandToDelete(command);
                  }}
                  className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-all"
                  title="Delete command"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
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
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Commands</h2>
          <HelpButton section="COMMANDS" />
        </div>
        <Button onClick={() => setShowCreateDialog(true)} size="sm">
          <Plus className="h-4 w-4 mr-2" />
          New Command
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Commands list sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search commands..."
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
            ) : commands.length === 0 ? (
              <div className="text-center py-12 px-4">
                <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
                  <Terminal className="h-8 w-8 text-muted-foreground" />
                </div>
                <p className="text-sm font-medium text-foreground">No commands found</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Commands live in ~/.claude/commands/
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4"
                  onClick={() => setShowCreateDialog(true)}
                >
                  <Plus className="h-4 w-4 mr-2" />
                  Create your first command
                </Button>
              </div>
            ) : (
              <>
                {renderCommandGroup('User Commands', groupedCommands.user, true)}
                {renderCommandGroup('Project Commands', groupedCommands.project, true)}
                {renderCommandGroup('Plugin Commands', groupedCommands.plugin, false)}
                {renderCommandGroup('Managed Commands', groupedCommands.managed, false)}
              </>
            )}
          </div>
        </div>

        {/* Command editor */}
        <div className="flex-1 overflow-hidden bg-background">
          {loadingCommand ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <p className="text-sm text-muted-foreground">Loading command...</p>
            </div>
          ) : selectedCommand ? (
            <MarkdownEditor
              item={selectedCommand}
              onSave={handleSaveCommand}
              onMove={handleOpenMoveDialog}
              readOnly={selectedCommand.scope !== 'user' && selectedCommand.scope !== 'project'}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <Terminal className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">Select a command to edit</p>
                <p className="text-xs text-muted-foreground/60 mt-1">
                  Slash commands for quick actions
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Create Command Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Command</DialogTitle>
            <DialogDescription>
              Create a new slash command in your user or project scope.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label htmlFor="command-name">Command Name</Label>
              <Input
                id="command-name"
                value={newCommandName}
                onChange={(e) => setNewCommandName(e.target.value)}
                placeholder="my-command"
                className="mt-2"
                onKeyDown={(e) => {
                  if (
                    e.key === 'Enter' &&
                    newCommandName.trim() &&
                    (createScope === 'user' || selectedProject)
                  ) {
                    handleCreateCommand();
                  }
                }}
              />
              <p className="text-xs text-muted-foreground mt-2">
                Use lowercase letters, numbers, and hyphens. Will be invoked as /
                {newCommandName || 'my-command'}.
              </p>
            </div>

            <div>
              <Label>Scope</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="command-scope"
                    checked={createScope === 'user'}
                    onChange={() => {
                      setCreateScope('user');
                      setSelectedProject(null);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">User (~/.claude/commands/)</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="command-scope"
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
                          name="command-project"
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
              onClick={handleCreateCommand}
              disabled={
                creating ||
                !newCommandName.trim() ||
                (createScope === 'project' && !selectedProject)
              }
            >
              {creating ? 'Creating...' : 'Create Command'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={!!commandToDelete}
        onOpenChange={(open) => !open && setCommandToDelete(null)}
        title="Delete Command"
        description={`Are you sure you want to delete "/${commandToDelete?.name}"? This action cannot be undone.`}
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={handleDeleteCommand}
        loading={deleting}
      />

      {/* Move Command Dialog */}
      <Dialog open={showMoveDialog} onOpenChange={setShowMoveDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Move Command</DialogTitle>
            <DialogDescription>
              Move "/{selectedCommand?.name}" to a different scope.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label>Target Scope</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="move-command-scope"
                    checked={moveTargetScope === 'user'}
                    onChange={() => {
                      setMoveTargetScope('user');
                      setMoveTargetProjects([]);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">User (~/.claude/commands/)</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="move-command-scope"
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
              onClick={handleMoveCommand}
              disabled={
                moving || (moveTargetScope === 'project' && moveTargetProjects.length === 0)
              }
            >
              {moving ? 'Moving...' : 'Move Command'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
