/**
 * MCP Server add form
 *
 * Dialog form for adding a new MCP server configuration.
 */

import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import {
  addToolsFromSource,
  createProfileMcpServer,
  listProfiles,
  removeProfileMcpServer,
} from '../../lib/ipc';
import type { Scope, McpTransport, OperationResult, McpServer } from './types';

interface McpFormProps {
  /** Whether the dialog is open */
  open: boolean;
  /** Called when dialog is closed */
  onClose: () => void;
  /** Called when server is successfully added/updated */
  onSuccess: () => void;
  /** Project path for context (null = global/user scope only) */
  projectPath?: string | null;
  /** Server to edit (if provided, form is in edit mode) */
  editServer?: McpServer | null;
}

interface FormData {
  name: string;
  scope: Scope;
  transport: McpTransport;
  command: string; // Full command with args, e.g. "npx -y @anthropic/mcp-server"
  url: string;
  env: string;
  docsUrl: string;
}

const defaultFormData: FormData = {
  name: '',
  scope: 'project',
  transport: 'stdio',
  command: '',
  url: '',
  env: '',
  docsUrl: '',
};

/**
 * Parse a command string into command and args, handling quoted strings
 * e.g. "npx -y @foo/bar" -> { command: "npx", args: ["-y", "@foo/bar"] }
 * e.g. 'node "path with spaces/server.js"' -> { command: "node", args: ["path with spaces/server.js"] }
 */
function parseCommand(input: string): { command: string; args: string[] } {
  const parts: string[] = [];
  let current = '';
  let inQuote = false;
  let quoteChar = '';

  for (let i = 0; i < input.length; i++) {
    const char = input[i];

    if ((char === '"' || char === "'") && !inQuote) {
      inQuote = true;
      quoteChar = char;
    } else if (char === quoteChar && inQuote) {
      inQuote = false;
      quoteChar = '';
    } else if (char === ' ' && !inQuote) {
      if (current) {
        parts.push(current);
        current = '';
      }
    } else {
      current += char;
    }
  }

  if (current) {
    parts.push(current);
  }

  const [command, ...args] = parts;
  return { command: command || '', args };
}

export function McpForm({
  open,
  onClose,
  onSuccess,
  projectPath = null,
  editServer = null,
}: McpFormProps) {
  const [formData, setFormData] = useState<FormData>(defaultFormData);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [addToProfile, setAddToProfile] = useState(false);
  const [selectedProfileId, setSelectedProfileId] = useState('');

  const isEditMode = !!editServer;

  const { data: profiles = [] } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  // Initialize form data based on edit mode or project context
  useEffect(() => {
    if (open) {
      if (editServer) {
        // Reconstruct command string from command + args
        const commandParts = [editServer.command, ...editServer.args].filter(Boolean);
        const commandStr = commandParts.join(' ');

        // Reconstruct env string from env object
        const envStr = Object.entries(editServer.env)
          .map(([k, v]) => `${k}=${v}`)
          .join('\n');

        setFormData({
          name: editServer.name,
          scope: editServer.scope as Scope,
          transport: editServer.transport as McpTransport,
          command: commandStr,
          url: editServer.url || '',
          env: envStr,
          docsUrl: editServer.docsUrl || '',
        });
        setSelectedProfileId(editServer.profileId || '');
      } else {
        setFormData((prev) => ({
          ...prev,
          scope: projectPath ? 'project' : 'user',
        }));
      }
    }
  }, [open, projectPath, editServer]);

  const resetForm = () => {
    setFormData({
      ...defaultFormData,
      scope: projectPath ? 'project' : 'user',
    });
    setError(null);
    setAddToProfile(false);
    setSelectedProfileId('');
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (formData.scope === 'profile' && !selectedProfileId) {
        toast.error('Please select a profile');
        setLoading(false);
        return;
      }

      if (
        !isEditMode &&
        addToProfile &&
        formData.scope !== 'local' &&
        formData.scope !== 'profile' &&
        !selectedProfileId
      ) {
        toast.error('Please select a profile');
        setLoading(false);
        return;
      }

      // Parse command string into command and args
      const { command, args } = parseCommand(formData.command);

      // Parse env from KEY=value format (one per line)
      const env: Record<string, string> = {};
      if (formData.env.trim()) {
        formData.env.split('\n').forEach((line) => {
          const [key, ...valueParts] = line.split('=');
          if (key && valueParts.length > 0) {
            env[key.trim()] = valueParts.join('=').trim();
          }
        });
      }

      // If editing, remove the old server first
      if (isEditMode && editServer) {
        if (editServer.scope === 'profile') {
          if (!editServer.profileId) {
            throw new Error('Missing profile ID for MCP server');
          }
          await removeProfileMcpServer(editServer.profileId, editServer.name);
        } else {
          await invoke<OperationResult>('mcp_remove', {
            params: {
              name: editServer.name,
              scope: editServer.scope,
              dryRun: false,
            },
            projectPath,
          });
        }
      }

      if (formData.scope === 'profile') {
        await createProfileMcpServer({
          profileId: selectedProfileId,
          name: formData.name,
          transport: formData.transport,
          command: formData.transport === 'stdio' ? command || null : null,
          args: formData.transport === 'stdio' && args.length > 0 ? args : null,
          env: Object.keys(env).length > 0 ? env : null,
          url: formData.transport !== 'stdio' ? formData.url || null : null,
          docsUrl: formData.docsUrl.trim() || null,
        });

        toast.success(`${isEditMode ? 'Updated' : 'Added'} "${formData.name}"`, {
          description: `MCP server ${isEditMode ? 'updated in' : 'added to'} profile`,
        });
        resetForm();
        onSuccess();
        return;
      }

      const result = await invoke<OperationResult>('mcp_add', {
        params: {
          name: formData.name,
          scope: formData.scope,
          transport: formData.transport,
          command: formData.transport === 'stdio' ? command || null : null,
          args: formData.transport === 'stdio' && args.length > 0 ? args : null,
          env: Object.keys(env).length > 0 ? env : null,
          url: formData.transport !== 'stdio' ? formData.url || null : null,
          docsUrl: formData.docsUrl.trim() || null,
          dryRun: false,
        },
        projectPath,
      });

      if (result.success) {
        toast.success(isEditMode ? `Updated "${formData.name}"` : `Added "${formData.name}"`, {
          description: `MCP server ${isEditMode ? 'updated in' : 'added to'} ${formData.scope} scope`,
        });

        if (
          !isEditMode &&
          addToProfile &&
          (formData.scope === 'user' || formData.scope === 'project') &&
          selectedProfileId
        ) {
          try {
            await addToolsFromSource(
              selectedProfileId,
              formData.scope === 'project' ? projectPath : undefined,
              [{ name: formData.name, tool_type: 'mcp' }],
              formData.scope === 'user' ? 'user' : 'project'
            );
            const profileName =
              profiles.find((profile) => profile.id === selectedProfileId)?.name || 'profile';
            toast.success(`Added to profile "${profileName}"`);
          } catch (err) {
            toast.error('Failed to add MCP server to profile', {
              description: err instanceof Error ? err.message : String(err),
            });
          }
        }

        resetForm();
        onSuccess();
      } else {
        toast.error(isEditMode ? 'Failed to update server' : 'Failed to add server', {
          description: result.error || 'Unknown error',
        });
        setError(result.error || `Failed to ${isEditMode ? 'update' : 'add'} server`);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(isEditMode ? 'Failed to update server' : 'Failed to add server', {
        description: message,
      });
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  const isStdio = formData.transport === 'stdio';

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && handleClose()}>
      <DialogContent className="sm:max-w-[500px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{isEditMode ? 'Edit MCP Server' : 'Add MCP Server'}</DialogTitle>
            <DialogDescription>
              {isEditMode
                ? `Update configuration for "${editServer?.name}".`
                : 'Configure a new Model Context Protocol server for Claude Code.'}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            {/* Name */}
            <div className="grid gap-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="my-server"
                required
              />
            </div>

            {/* Scope */}
            <div className="grid gap-2">
              <Label>Scope</Label>
              <Select
                value={formData.scope}
                onValueChange={(value: Scope) => {
                  setFormData({ ...formData, scope: value });
                  if (value === 'local') {
                    setAddToProfile(false);
                    setSelectedProfileId('');
                  }
                  if (value === 'profile') {
                    setAddToProfile(false);
                  }
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="project" disabled={!projectPath}>
                    Project (.mcp.json)
                  </SelectItem>
                  <SelectItem value="user">User (~/.claude.json)</SelectItem>
                  <SelectItem value="local" disabled={!projectPath}>
                    Local (.claude/settings.local.json)
                  </SelectItem>
                  <SelectItem value="profile" disabled={profiles.length === 0}>
                    Profile (plugin only)
                  </SelectItem>
                </SelectContent>
              </Select>
              {formData.scope === 'project' && projectPath && (
                <p className="text-xs text-muted-foreground">
                  Will be added to: {projectPath}/.mcp.json
                </p>
              )}
              {formData.scope === 'local' && projectPath && (
                <p className="text-xs text-muted-foreground">
                  Will be added to: {projectPath}/.claude/settings.local.json
                </p>
              )}
              {formData.scope === 'user' && (
                <p className="text-xs text-muted-foreground">
                  Will be added to: ~/.claude.json (available globally)
                </p>
              )}
              {formData.scope === 'profile' && (
                <p className="text-xs text-muted-foreground">
                  Stored in the selected profile (available when installed).
                </p>
              )}
              {(formData.scope === 'project' || formData.scope === 'local') && !projectPath && (
                <p className="text-xs text-destructive">
                  Select a project from the dropdown above to use this scope
                </p>
              )}
            </div>

            {formData.scope === 'profile' && (
              <div className="grid gap-2">
                <Label>Profile</Label>
                {profiles.length === 0 ? (
                  <p className="text-xs text-muted-foreground">
                    Create a profile first to enable this option.
                  </p>
                ) : (
                  <Select
                    value={selectedProfileId || undefined}
                    onValueChange={(value) => setSelectedProfileId(value)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select profile" />
                    </SelectTrigger>
                    <SelectContent>
                      {profiles.map((profile) => (
                        <SelectItem key={profile.id} value={profile.id}>
                          {profile.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            )}

            {!isEditMode && formData.scope !== 'local' && formData.scope !== 'profile' && (
              <div className="rounded-md border border-border p-3 space-y-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <Label htmlFor="add-mcp-to-profile">Add to profile</Label>
                    <p className="text-xs text-muted-foreground mt-1">
                      Copies this MCP server into a profile for reuse.
                    </p>
                  </div>
                  <input
                    id="add-mcp-to-profile"
                    type="checkbox"
                    checked={addToProfile}
                    onChange={(e) => {
                      const next = e.target.checked;
                      setAddToProfile(next);
                      if (!next) {
                        setSelectedProfileId('');
                      }
                    }}
                    className="accent-primary"
                    disabled={
                      profiles.length === 0 || (formData.scope === 'project' && !projectPath)
                    }
                  />
                </div>

                {profiles.length === 0 && (
                  <p className="text-xs text-muted-foreground">
                    Create a profile first to enable this option.
                  </p>
                )}
                {formData.scope === 'project' && !projectPath && (
                  <p className="text-xs text-muted-foreground">
                    Select a project to enable this option.
                  </p>
                )}

                {addToProfile && profiles.length > 0 && (
                  <div className="space-y-2">
                    <Label>Profile</Label>
                    <Select
                      value={selectedProfileId || undefined}
                      onValueChange={(value) => setSelectedProfileId(value)}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select profile" />
                      </SelectTrigger>
                      <SelectContent>
                        {profiles.map((profile) => (
                          <SelectItem key={profile.id} value={profile.id}>
                            {profile.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}
              </div>
            )}

            {/* Transport */}
            <div className="grid gap-2">
              <Label>Transport</Label>
              <Select
                value={formData.transport}
                onValueChange={(value: McpTransport) =>
                  setFormData({ ...formData, transport: value })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="stdio">stdio (Command line)</SelectItem>
                  <SelectItem value="http">HTTP</SelectItem>
                  <SelectItem value="sse">Server-Sent Events</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Command (stdio only) */}
            {isStdio && (
              <div className="grid gap-2">
                <Label htmlFor="command">Command *</Label>
                <Input
                  id="command"
                  value={formData.command}
                  onChange={(e) => setFormData({ ...formData, command: e.target.value })}
                  placeholder="npx -y @anthropic/mcp-server"
                  required={isStdio}
                />
                <p className="text-xs text-muted-foreground">
                  Full command with arguments, e.g. npx -y @modelcontextprotocol/server-memory
                </p>
              </div>
            )}

            {/* URL (http/sse only) */}
            {!isStdio && (
              <div className="grid gap-2">
                <Label htmlFor="url">URL *</Label>
                <Input
                  id="url"
                  value={formData.url}
                  onChange={(e) => setFormData({ ...formData, url: e.target.value })}
                  placeholder="https://mcp.example.com/api"
                  required={!isStdio}
                />
              </div>
            )}

            {/* Environment Variables */}
            <div className="grid gap-2">
              <Label htmlFor="env">Environment Variables (KEY=value, one per line)</Label>
              <textarea
                id="env"
                value={formData.env}
                onChange={(e) => setFormData({ ...formData, env: e.target.value })}
                className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                placeholder="API_KEY=<YOUR_API_KEY>&#10;DEBUG=true"
              />
            </div>

            {/* Documentation URL */}
            <div className="grid gap-2">
              <Label htmlFor="docsUrl">Documentation URL (optional)</Label>
              <Input
                id="docsUrl"
                type="url"
                value={formData.docsUrl}
                onChange={(e) => setFormData({ ...formData, docsUrl: e.target.value })}
                placeholder="https://github.com/org/mcp-server#readme"
              />
              <p className="text-xs text-muted-foreground">
                Link to project page or docs for quick reference
              </p>
            </div>

            {/* Error display */}
            {error && (
              <div className="p-3 bg-destructive/10 text-destructive text-sm rounded-md">
                {error}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleClose} disabled={loading}>
              Cancel
            </Button>
            <Button type="submit" disabled={loading}>
              {loading
                ? isEditMode
                  ? 'Updating...'
                  : 'Adding...'
                : isEditMode
                  ? 'Update Server'
                  : 'Add Server'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
