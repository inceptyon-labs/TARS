import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Layers, RefreshCw, Search } from 'lucide-react';
import { useState } from 'react';
import { listProfiles, createProfile, deleteProfile, getProfile } from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { ProfileList } from '../components/ProfileList';
import { ProfileDetail } from '../components/ProfileDetail';
import { CreateProfileDialog } from '../components/CreateProfileDialog';
import type { ProfileDetails } from '../lib/types';

export function ProfilesPage() {
  const queryClient = useQueryClient();
  const [selectedProfile, setSelectedProfile] = useState<ProfileDetails | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);

  const isDialogOpen = useUIStore((state) => state.isCreateProfileDialogOpen);
  const setDialogOpen = useUIStore((state) => state.setCreateProfileDialogOpen);

  const { data: profiles = [], isLoading } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  const createMutation = useMutation({
    mutationFn: ({
      name,
      sourcePath,
      description,
    }: {
      name: string;
      sourcePath: string;
      description?: string;
    }) => createProfile(name, sourcePath, description),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      setDialogOpen(false);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteProfile,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      if (selectedProfile) setSelectedProfile(null);
    },
  });

  async function handleSelectProfile(id: string) {
    setLoadingDetails(true);
    try {
      const details = await getProfile(id);
      setSelectedProfile(details);
    } catch (err) {
      console.error('Failed to load profile:', err);
    } finally {
      setLoadingDetails(false);
    }
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 brushed-metal relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Profiles</h2>
        </div>
        <button
          onClick={() => setDialogOpen(true)}
          className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
        >
          <Plus className="h-4 w-4" />
          Create Profile
        </button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Profile list sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search profiles..."
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
            ) : profiles.length === 0 ? (
              <div className="text-center py-12 px-4">
                <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
                  <Layers className="h-8 w-8 text-muted-foreground" />
                </div>
                <p className="text-sm font-medium text-foreground">No profiles</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Create a profile from a project
                </p>
              </div>
            ) : (
              <ProfileList
                profiles={profiles}
                selectedId={selectedProfile?.id || null}
                onSelect={handleSelectProfile}
                onDelete={(id) => deleteMutation.mutate(id)}
              />
            )}
          </div>
        </div>

        {/* Profile detail view */}
        <div className="flex-1 overflow-auto bg-background">
          {loadingDetails ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <p className="text-sm text-muted-foreground">Loading profile...</p>
            </div>
          ) : selectedProfile ? (
            <ProfileDetail profile={selectedProfile} />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <Layers className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">Select a profile to view details</p>
                <p className="text-xs text-muted-foreground/60 mt-1">
                  Apply configurations to projects
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Create Profile Dialog */}
      <CreateProfileDialog
        open={isDialogOpen}
        onOpenChange={setDialogOpen}
        onCreate={(name, sourcePath, description) =>
          createMutation.mutate({ name, sourcePath, description })
        }
        isLoading={createMutation.isPending}
        error={createMutation.error ? String(createMutation.error) : undefined}
      />
    </div>
  );
}
