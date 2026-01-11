/**
 * MCP Server management panel
 *
 * Lists all MCP servers grouped by scope with add/remove capabilities.
 */

import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { HelpButton } from '../HelpButton';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Label } from '../ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../ui/table';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { ConfirmDialog } from './ConfirmDialog';
import { McpForm } from './McpForm';
import { listProjects } from '../../lib/ipc';
import type { McpServer, OperationResult, Scope } from './types';

interface McpListResult {
  servers: McpServer[];
}

interface GroupedServers {
  [key: string]: McpServer[];
}

export function McpPanel() {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [serverToRemove, setServerToRemove] = useState<McpServer | null>(null);
  const [removing, setRemoving] = useState(false);
  const [selectedProjectPath, setSelectedProjectPath] = useState<string | null>(null);
  const [serverToMove, setServerToMove] = useState<McpServer | null>(null);
  const [moveTargetScope, setMoveTargetScope] = useState<Scope>('user');
  const [moving, setMoving] = useState(false);

  // Fetch projects for the selector
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const loadServers = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<McpListResult>('mcp_list', {
        projectPath: selectedProjectPath,
      });
      setServers(result.servers);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [selectedProjectPath]);

  useEffect(() => {
    loadServers();
  }, [loadServers]);

  const handleRemove = async () => {
    if (!serverToRemove) return;

    setRemoving(true);
    try {
      const result = await invoke<OperationResult>('mcp_remove', {
        params: {
          name: serverToRemove.name,
          scope: serverToRemove.scope,
          dryRun: false,
        },
        projectPath: selectedProjectPath,
      });

      if (result.success) {
        toast.success(`Removed "${serverToRemove.name}"`, {
          description: 'MCP server removed successfully',
        });
        // Refresh the list
        await loadServers();
      } else {
        toast.error('Failed to remove server', {
          description: result.error || 'Unknown error',
        });
        setError(result.error || 'Failed to remove server');
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error('Failed to remove server', { description: message });
      setError(message);
    } finally {
      setRemoving(false);
      setServerToRemove(null);
    }
  };

  const handleAdd = async () => {
    setShowAddForm(false);
    await loadServers();
  };

  const handleMove = async () => {
    if (!serverToMove) return;

    setMoving(true);
    try {
      interface MoveResult {
        success: boolean;
        error?: string;
      }
      const result = await invoke<MoveResult>('mcp_move', {
        params: {
          name: serverToMove.name,
          fromScope: serverToMove.scope,
          toScope: moveTargetScope,
          dryRun: false,
        },
        projectPath: selectedProjectPath,
      });

      if (result.success) {
        toast.success(`Moved "${serverToMove.name}"`, {
          description: `Server moved to ${moveTargetScope} scope`,
        });
        await loadServers();
      } else {
        toast.error('Failed to move server', {
          description: result.error || 'Unknown error',
        });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error('Failed to move server', { description: message });
    } finally {
      setMoving(false);
      setServerToMove(null);
    }
  };

  const openMoveDialog = (server: McpServer) => {
    setServerToMove(server);
    // Pre-select a different scope than current
    const otherScopes: Scope[] = ['user', 'project', 'local'].filter(
      (s) => s !== server.scope
    ) as Scope[];
    setMoveTargetScope(otherScopes[0] || 'user');
  };

  // Group servers by scope
  const groupedServers: GroupedServers = servers.reduce((acc, server) => {
    const scope = server.scope;
    if (!acc[scope]) {
      acc[scope] = [];
    }
    acc[scope].push(server);
    return acc;
  }, {} as GroupedServers);

  const scopeOrder: Scope[] = ['project', 'local', 'user'];
  const scopeLabels: Record<string, string> = {
    user: 'User (Global)',
    project: 'Project',
    local: 'Local',
  };

  if (loading) {
    return (
      <div className="p-4">
        <p className="text-muted-foreground">Loading MCP servers...</p>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold">MCP Servers</h2>
            <HelpButton section="MCP" />
          </div>
          <p className="text-sm text-muted-foreground">
            Manage Model Context Protocol servers for Claude Code
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Select
            value={selectedProjectPath ?? 'global'}
            onValueChange={(value) => setSelectedProjectPath(value === 'global' ? null : value)}
          >
            <SelectTrigger className="w-[200px]">
              <SelectValue placeholder="Select context" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="global">Global (User Scope)</SelectItem>
              {projects.map((project) => (
                <SelectItem key={project.id} value={project.path}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button onClick={() => setShowAddForm(true)}>Add Server</Button>
        </div>
      </div>

      {error && (
        <div className="p-4 bg-destructive/10 text-destructive rounded-md">
          {error}
          <Button variant="ghost" size="sm" className="ml-2" onClick={() => setError(null)}>
            Dismiss
          </Button>
        </div>
      )}

      {servers.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            No MCP servers configured. Click "Add Server" to get started.
          </CardContent>
        </Card>
      ) : (
        scopeOrder.map((scope) => {
          const scopeServers = groupedServers[scope];
          if (!scopeServers?.length) return null;

          return (
            <Card key={scope}>
              <CardHeader className="py-3">
                <CardTitle className="text-base">{scopeLabels[scope]}</CardTitle>
                <CardDescription>
                  {scope === 'user' && '~/.claude.json - Available in all projects'}
                  {scope === 'project' && '.mcp.json - Project-specific servers'}
                  {scope === 'local' && '.claude/settings.local.json - Not committed'}
                </CardDescription>
              </CardHeader>
              <CardContent className="pt-0">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Transport</TableHead>
                      <TableHead>Command / URL</TableHead>
                      <TableHead className="w-[100px]">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {scopeServers.map((server) => (
                      <TableRow key={`${server.scope}-${server.name}`}>
                        <TableCell className="font-medium">
                          <div className="flex items-center gap-2">
                            {server.name}
                            {server.sourcePlugin && (
                              <span className="px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded text-xs" title={`From plugin: ${server.sourcePlugin}`}>
                                plugin
                              </span>
                            )}
                          </div>
                        </TableCell>
                        <TableCell>
                          <span className="px-2 py-1 bg-muted rounded text-xs">
                            {server.transport}
                          </span>
                        </TableCell>
                        <TableCell className="text-muted-foreground text-sm truncate max-w-[300px]">
                          {server.command || server.url || '-'}
                        </TableCell>
                        <TableCell>
                          {server.sourcePlugin ? (
                            <span className="text-xs text-muted-foreground">
                              Managed by plugin
                            </span>
                          ) : (
                            <div className="flex gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => openMoveDialog(server)}
                              >
                                Move
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                className="text-destructive hover:text-destructive"
                                onClick={() => setServerToRemove(server)}
                              >
                                Remove
                              </Button>
                            </div>
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          );
        })
      )}

      {/* Add Server Dialog */}
      <McpForm
        open={showAddForm}
        onClose={() => setShowAddForm(false)}
        onSuccess={handleAdd}
        projectPath={selectedProjectPath}
      />

      {/* Remove Confirmation Dialog */}
      <ConfirmDialog
        open={!!serverToRemove}
        onOpenChange={(open) => !open && setServerToRemove(null)}
        title="Remove MCP Server"
        description={`Are you sure you want to remove "${serverToRemove?.name}"? This action cannot be undone.`}
        confirmLabel="Remove"
        confirmVariant="destructive"
        onConfirm={handleRemove}
        loading={removing}
      />

      {/* Move Server Dialog */}
      {serverToMove && (
        <Dialog open={!!serverToMove} onOpenChange={(open) => !open && setServerToMove(null)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Move MCP Server</DialogTitle>
              <DialogDescription>
                Move "{serverToMove.name}" from {serverToMove.scope} scope to a different scope.
              </DialogDescription>
            </DialogHeader>
            <div className="py-4">
              <Label>Target Scope</Label>
              <Select
                value={moveTargetScope}
                onValueChange={(value: Scope) => setMoveTargetScope(value)}
              >
                <SelectTrigger className="mt-2">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="user" disabled={serverToMove.scope === 'user'}>
                    User (~/.claude.json)
                  </SelectItem>
                  <SelectItem
                    value="project"
                    disabled={serverToMove.scope === 'project' || !selectedProjectPath}
                  >
                    Project (.mcp.json)
                  </SelectItem>
                  <SelectItem
                    value="local"
                    disabled={serverToMove.scope === 'local' || !selectedProjectPath}
                  >
                    Local (.claude/settings.local.json)
                  </SelectItem>
                </SelectContent>
              </Select>
              {!selectedProjectPath && moveTargetScope !== 'user' && (
                <p className="text-xs text-destructive mt-2">
                  Select a project from the dropdown above to move to project/local scope
                </p>
              )}
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setServerToMove(null)} disabled={moving}>
                Cancel
              </Button>
              <Button onClick={handleMove} disabled={moving}>
                {moving ? 'Moving...' : 'Move Server'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}
