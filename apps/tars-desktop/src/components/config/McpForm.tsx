/**
 * MCP Server add form
 *
 * Dialog form for adding a new MCP server configuration.
 */

import { useState, useEffect } from 'react';
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select';
import type { Scope, McpTransport, OperationResult } from './types';

interface McpFormProps {
  /** Whether the dialog is open */
  open: boolean;
  /** Called when dialog is closed */
  onClose: () => void;
  /** Called when server is successfully added */
  onSuccess: () => void;
  /** Project path for context (null = global/user scope only) */
  projectPath?: string | null;
}

interface FormData {
  name: string;
  scope: Scope;
  transport: McpTransport;
  command: string;
  args: string;
  url: string;
  env: string;
}

const defaultFormData: FormData = {
  name: '',
  scope: 'project',
  transport: 'stdio',
  command: '',
  args: '',
  url: '',
  env: '',
};

export function McpForm({ open, onClose, onSuccess, projectPath = null }: McpFormProps) {
  const [formData, setFormData] = useState<FormData>(defaultFormData);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Update default scope based on project context
  useEffect(() => {
    if (open) {
      setFormData(prev => ({
        ...prev,
        scope: projectPath ? 'project' : 'user',
      }));
    }
  }, [open, projectPath]);

  const resetForm = () => {
    setFormData({
      ...defaultFormData,
      scope: projectPath ? 'project' : 'user',
    });
    setError(null);
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
      // Parse args from comma-separated string
      const args = formData.args
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);

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

      const result = await invoke<OperationResult>('mcp_add', {
        params: {
          name: formData.name,
          scope: formData.scope,
          transport: formData.transport,
          command: formData.transport === 'stdio' ? formData.command || null : null,
          args: formData.transport === 'stdio' ? args : null,
          env: Object.keys(env).length > 0 ? env : null,
          url: formData.transport !== 'stdio' ? formData.url || null : null,
          dryRun: false,
        },
        projectPath,
      });

      if (result.success) {
        toast.success(`Added "${formData.name}"`, {
          description: `MCP server added to ${formData.scope} scope`,
        });
        resetForm();
        onSuccess();
      } else {
        toast.error('Failed to add server', {
          description: result.error || 'Unknown error',
        });
        setError(result.error || 'Failed to add server');
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error('Failed to add server', { description: message });
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
            <DialogTitle>Add MCP Server</DialogTitle>
            <DialogDescription>
              Configure a new Model Context Protocol server for Claude Code.
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
                onValueChange={(value: Scope) => setFormData({ ...formData, scope: value })}
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
              {(formData.scope === 'project' || formData.scope === 'local') && !projectPath && (
                <p className="text-xs text-destructive">
                  Select a project from the dropdown above to use this scope
                </p>
              )}
            </div>

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
              <>
                <div className="grid gap-2">
                  <Label htmlFor="command">Command *</Label>
                  <Input
                    id="command"
                    value={formData.command}
                    onChange={(e) => setFormData({ ...formData, command: e.target.value })}
                    placeholder="node, npx, python, etc."
                    required={isStdio}
                  />
                </div>

                <div className="grid gap-2">
                  <Label htmlFor="args">Arguments (comma-separated)</Label>
                  <Input
                    id="args"
                    value={formData.args}
                    onChange={(e) => setFormData({ ...formData, args: e.target.value })}
                    placeholder="-y, package-name@latest"
                  />
                  <p className="text-xs text-muted-foreground">
                    Each argument separated by comma. Example: -y, @anthropic/mcp-server
                  </p>
                </div>
              </>
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
                placeholder="API_KEY=your-key&#10;DEBUG=true"
              />
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
              {loading ? 'Adding...' : 'Add Server'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
