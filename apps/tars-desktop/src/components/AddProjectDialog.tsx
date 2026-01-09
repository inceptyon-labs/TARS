import { useState } from 'react';
import { X, FolderOpen } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';

interface AddProjectDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (path: string) => void;
  isLoading: boolean;
  error?: string;
}

export function AddProjectDialog({
  open: isOpen,
  onOpenChange,
  onAdd,
  isLoading,
  error,
}: AddProjectDialogProps) {
  const [path, setPath] = useState('');

  if (!isOpen) return null;

  async function handleBrowse() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Project Directory',
      });
      if (selected && typeof selected === 'string') {
        setPath(selected);
      }
    } catch (err) {
      console.error('Failed to open directory picker:', err);
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (path.trim()) {
      onAdd(path.trim());
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/70 backdrop-blur-sm"
        onClick={() => onOpenChange(false)}
      />

      {/* Dialog */}
      <div className="relative tars-panel rounded-lg shadow-2xl w-full max-w-md p-6">
        {/* Top highlight line */}
        <div className="absolute top-0 left-4 right-4 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />

        <button
          onClick={() => onOpenChange(false)}
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors"
        >
          <X className="h-4 w-4" />
        </button>

        <div className="flex items-center gap-3 mb-6">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Add Project</h2>
        </div>

        <div className="tars-segment-line mb-6" />

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <div>
              <label className="block text-xs text-muted-foreground uppercase tracking-wider mb-2">
                Project Directory
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={path}
                  onChange={(e) => setPath(e.target.value)}
                  placeholder="/path/to/project"
                  className="tars-input flex-1 px-3 py-2 text-sm rounded"
                />
                <button
                  type="button"
                  onClick={handleBrowse}
                  className="tars-button px-3 py-2 rounded"
                >
                  <FolderOpen className="h-4 w-4" />
                </button>
              </div>
            </div>

            {error && (
              <p className="text-sm text-destructive">{error}</p>
            )}

            <div className="tars-segment-line" />

            <div className="flex justify-end gap-3 pt-2">
              <button
                type="button"
                onClick={() => onOpenChange(false)}
                className="tars-button px-4 py-2 rounded text-sm"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!path.trim() || isLoading}
                className="tars-button-primary px-4 py-2 rounded text-sm font-medium disabled:opacity-50"
              >
                {isLoading ? 'Adding...' : 'Add Project'}
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
