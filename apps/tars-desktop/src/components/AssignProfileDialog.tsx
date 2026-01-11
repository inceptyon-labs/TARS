import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { X, Layers, Check, Unlink } from 'lucide-react';
import { listProfiles } from '../lib/ipc';
import { Button } from './ui/button';

interface AssignProfileDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAssign: (profileId: string) => void;
  onUnassign: () => void;
  currentProfileId: string | null;
  projectName: string;
  isLoading?: boolean;
}

export function AssignProfileDialog({
  open,
  onOpenChange,
  onAssign,
  onUnassign,
  currentProfileId,
  projectName,
  isLoading,
}: AssignProfileDialogProps) {
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(currentProfileId);

  // Sync selection with currentProfileId when dialog opens or project changes
  useEffect(() => {
    if (open) {
      setSelectedProfileId(currentProfileId);
    }
  }, [open, currentProfileId]);

  const { data: profiles = [], isLoading: loadingProfiles } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
    enabled: open,
  });

  const handleClose = () => {
    setSelectedProfileId(currentProfileId);
    onOpenChange(false);
  };

  const handleAssign = () => {
    if (selectedProfileId) {
      onAssign(selectedProfileId);
    }
  };

  const handleUnassign = () => {
    onUnassign();
    setSelectedProfileId(null);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <div>
            <h2 className="text-lg font-semibold">Assign Profile</h2>
            <p className="text-sm text-muted-foreground">
              Select a profile for <span className="font-medium">{projectName}</span>
            </p>
          </div>
          <button
            onClick={handleClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 max-h-80 overflow-auto">
          {loadingProfiles ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
            </div>
          ) : profiles.length === 0 ? (
            <div className="text-center py-8">
              <Layers className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
              <p className="text-sm text-muted-foreground">No profiles available</p>
              <p className="text-xs text-muted-foreground mt-1">
                Create a profile first from the Profiles page
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {profiles.map((profile) => (
                <button
                  key={profile.id}
                  onClick={() => setSelectedProfileId(profile.id)}
                  className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                    selectedProfileId === profile.id
                      ? 'border-primary bg-primary/10'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <div
                    className={`w-5 h-5 rounded-full border flex items-center justify-center ${
                      selectedProfileId === profile.id
                        ? 'bg-primary border-primary text-primary-foreground'
                        : 'border-muted-foreground/40'
                    }`}
                  >
                    {selectedProfileId === profile.id && <Check className="h-3 w-3" />}
                  </div>
                  <div className="flex-1 text-left">
                    <div className="font-medium text-sm flex items-center gap-2">
                      {profile.name}
                      {profile.id === currentProfileId && (
                        <span className="text-xs bg-primary/20 text-primary px-1.5 py-0.5 rounded">
                          Current
                        </span>
                      )}
                    </div>
                    {profile.description && (
                      <div className="text-xs text-muted-foreground truncate">
                        {profile.description}
                      </div>
                    )}
                    <div className="text-xs text-muted-foreground mt-0.5">
                      {profile.tool_count} tool{profile.tool_count === 1 ? '' : 's'}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t bg-muted/30">
          <div>
            {currentProfileId && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleUnassign}
                disabled={isLoading}
                className="text-destructive hover:text-destructive hover:bg-destructive/10"
              >
                <Unlink className="h-4 w-4 mr-1" />
                Unassign
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleClose}>
              Cancel
            </Button>
            <Button
              onClick={handleAssign}
              disabled={!selectedProfileId || selectedProfileId === currentProfileId || isLoading}
            >
              {isLoading ? 'Assigning...' : 'Assign Profile'}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
