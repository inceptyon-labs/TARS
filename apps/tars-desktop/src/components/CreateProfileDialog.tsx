import { useState } from 'react';
import { X, FolderOpen } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { Button } from './ui/button';

interface CreateProfileDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (name: string, sourcePath: string | undefined, description?: string) => void;
  isLoading: boolean;
  error?: string;
}

export function CreateProfileDialog({
  open: isOpen,
  onOpenChange,
  onCreate,
  isLoading,
  error,
}: CreateProfileDialogProps) {
  const [name, setName] = useState('');
  const [sourcePath, setSourcePath] = useState('');
  const [description, setDescription] = useState('');

  if (!isOpen) return null;

  async function handleBrowse() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Source Project',
      });
      if (selected && typeof selected === 'string') {
        setSourcePath(selected);
        // Auto-fill name from directory name if empty
        if (!name) {
          const dirName = selected.split('/').pop() || '';
          setName(dirName);
        }
      }
    } catch (err) {
      console.error('Failed to open directory picker:', err);
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (name.trim()) {
      onCreate(name.trim(), sourcePath.trim() || undefined, description.trim() || undefined);
    }
  }

  function handleClose() {
    setName('');
    setSourcePath('');
    setDescription('');
    onOpenChange(false);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md p-6">
        <button
          onClick={handleClose}
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors"
        >
          <X className="h-4 w-4" />
        </button>

        <h2 className="text-lg font-semibold mb-4">Create Profile</h2>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-1.5">Profile Name</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="my-profile"
                className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1.5">
                Source Project <span className="text-muted-foreground font-normal">(optional)</span>
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={sourcePath}
                  onChange={(e) => setSourcePath(e.target.value)}
                  placeholder="/path/to/project"
                  className="flex-1 px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                />
                <Button type="button" variant="outline" size="icon" onClick={handleBrowse}>
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
              <p className="text-xs text-muted-foreground mt-1.5">
                {sourcePath.trim()
                  ? 'Skills, commands, agents, and CLAUDE.md will be captured'
                  : 'Leave empty to create a blank profile'}
              </p>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1.5">Description (optional)</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="A brief description of this profile..."
                rows={2}
                className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background resize-none focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            {error && <p className="text-sm text-destructive">{error}</p>}

            <div className="flex justify-end gap-2 pt-2">
              <Button type="button" variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button type="submit" disabled={!name.trim() || isLoading}>
                {isLoading ? 'Creating...' : 'Create Profile'}
              </Button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
