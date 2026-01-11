import { useState } from 'react';
import { Plus, X, FolderOpen, Wrench, ShieldX } from 'lucide-react';
import { Button } from './ui/button';
import type { ToolPermissions } from '../lib/types';

interface ToolPermissionsEditorProps {
  permissions: ToolPermissions | null;
  onChange: (permissions: ToolPermissions | null) => void;
  compact?: boolean;
}

export function ToolPermissionsEditor({
  permissions,
  onChange,
  compact = false,
}: ToolPermissionsEditorProps) {
  const [newDirectory, setNewDirectory] = useState('');
  const [newAllowedTool, setNewAllowedTool] = useState('');
  const [newDisallowedTool, setNewDisallowedTool] = useState('');

  // Initialize with empty permissions if null
  const currentPerms: ToolPermissions = permissions || {
    allowed_directories: [],
    allowed_tools: [],
    disallowed_tools: [],
  };

  const updatePermissions = (updates: Partial<ToolPermissions>) => {
    const newPerms = { ...currentPerms, ...updates };
    // If all arrays are empty, set to null
    if (
      newPerms.allowed_directories.length === 0 &&
      newPerms.allowed_tools.length === 0 &&
      newPerms.disallowed_tools.length === 0
    ) {
      onChange(null);
    } else {
      onChange(newPerms);
    }
  };

  const addDirectory = () => {
    const trimmed = newDirectory.trim();
    if (trimmed && !currentPerms.allowed_directories.includes(trimmed)) {
      updatePermissions({
        allowed_directories: [...currentPerms.allowed_directories, trimmed],
      });
      setNewDirectory('');
    }
  };

  const removeDirectory = (dir: string) => {
    updatePermissions({
      allowed_directories: currentPerms.allowed_directories.filter((d) => d !== dir),
    });
  };

  const addAllowedTool = () => {
    const trimmed = newAllowedTool.trim();
    if (trimmed && !currentPerms.allowed_tools.includes(trimmed)) {
      updatePermissions({
        allowed_tools: [...currentPerms.allowed_tools, trimmed],
      });
      setNewAllowedTool('');
    }
  };

  const removeAllowedTool = (tool: string) => {
    updatePermissions({
      allowed_tools: currentPerms.allowed_tools.filter((t) => t !== tool),
    });
  };

  const addDisallowedTool = () => {
    const trimmed = newDisallowedTool.trim();
    if (trimmed && !currentPerms.disallowed_tools.includes(trimmed)) {
      updatePermissions({
        disallowed_tools: [...currentPerms.disallowed_tools, trimmed],
      });
      setNewDisallowedTool('');
    }
  };

  const removeDisallowedTool = (tool: string) => {
    updatePermissions({
      disallowed_tools: currentPerms.disallowed_tools.filter((t) => t !== tool),
    });
  };

  if (compact) {
    // Compact display for list items - just show counts
    const totalPerms =
      currentPerms.allowed_directories.length +
      currentPerms.allowed_tools.length +
      currentPerms.disallowed_tools.length;

    if (totalPerms === 0) return null;

    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        {currentPerms.allowed_directories.length > 0 && (
          <span className="flex items-center gap-1" title="Allowed directories">
            <FolderOpen className="h-3 w-3" />
            {currentPerms.allowed_directories.length}
          </span>
        )}
        {currentPerms.allowed_tools.length > 0 && (
          <span className="flex items-center gap-1 text-green-600" title="Allowed tools">
            <Wrench className="h-3 w-3" />+{currentPerms.allowed_tools.length}
          </span>
        )}
        {currentPerms.disallowed_tools.length > 0 && (
          <span className="flex items-center gap-1 text-destructive" title="Disallowed tools">
            <ShieldX className="h-3 w-3" />-{currentPerms.disallowed_tools.length}
          </span>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Allowed Directories */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          <FolderOpen className="h-4 w-4 text-muted-foreground" />
          Allowed Directories
        </label>
        <p className="text-xs text-muted-foreground">
          Directories this tool can access (leave empty for default behavior)
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={newDirectory}
            onChange={(e) => setNewDirectory(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addDirectory();
              }
            }}
            placeholder="/path/to/directory"
            className="flex-1 px-3 py-1.5 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={addDirectory}
            disabled={!newDirectory.trim()}
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
        {currentPerms.allowed_directories.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {currentPerms.allowed_directories.map((dir) => (
              <span
                key={dir}
                className="inline-flex items-center gap-1 px-2 py-1 text-xs bg-muted rounded-md font-mono"
              >
                {dir}
                <button
                  type="button"
                  onClick={() => removeDirectory(dir)}
                  className="text-muted-foreground hover:text-destructive"
                  aria-label={`Remove directory ${dir}`}
                >
                  <X className="h-3 w-3" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Allowed Tools */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          <Wrench className="h-4 w-4 text-green-600" />
          Allowed Tools
        </label>
        <p className="text-xs text-muted-foreground">
          Specific tools this MCP server can use (leave empty for all tools)
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={newAllowedTool}
            onChange={(e) => setNewAllowedTool(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addAllowedTool();
              }
            }}
            placeholder="tool_name"
            className="flex-1 px-3 py-1.5 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={addAllowedTool}
            disabled={!newAllowedTool.trim()}
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
        {currentPerms.allowed_tools.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {currentPerms.allowed_tools.map((tool) => (
              <span
                key={tool}
                className="inline-flex items-center gap-1 px-2 py-1 text-xs bg-green-500/10 text-green-700 rounded-md"
              >
                {tool}
                <button
                  type="button"
                  onClick={() => removeAllowedTool(tool)}
                  className="text-green-600 hover:text-destructive"
                  aria-label={`Remove allowed tool ${tool}`}
                >
                  <X className="h-3 w-3" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Disallowed Tools */}
      <div className="space-y-2">
        <label className="text-sm font-medium flex items-center gap-2">
          <ShieldX className="h-4 w-4 text-destructive" />
          Disallowed Tools
        </label>
        <p className="text-xs text-muted-foreground">
          Tools this MCP server cannot use (blocklist)
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={newDisallowedTool}
            onChange={(e) => setNewDisallowedTool(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addDisallowedTool();
              }
            }}
            placeholder="tool_name"
            className="flex-1 px-3 py-1.5 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={addDisallowedTool}
            disabled={!newDisallowedTool.trim()}
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
        {currentPerms.disallowed_tools.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {currentPerms.disallowed_tools.map((tool) => (
              <span
                key={tool}
                className="inline-flex items-center gap-1 px-2 py-1 text-xs bg-destructive/10 text-destructive rounded-md"
              >
                {tool}
                <button
                  type="button"
                  onClick={() => removeDisallowedTool(tool)}
                  className="text-destructive hover:text-destructive/70"
                  aria-label={`Remove disallowed tool ${tool}`}
                >
                  <X className="h-3 w-3" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
