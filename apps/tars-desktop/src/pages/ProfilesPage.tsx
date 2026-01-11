import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Layers, RefreshCw, Search, Upload, AlertTriangle, X } from 'lucide-react';
import { Button } from '../components/ui/button';
import { useState } from 'react';
import { toast } from 'sonner';
import { save } from '@tauri-apps/plugin-dialog';
import {
  listProfiles,
  createEmptyProfile,
  deleteProfile,
  getProfile,
  updateProfile,
  exportProfileJson,
} from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { ProfileList } from '../components/ProfileList';
import { ProfileDetail } from '../components/ProfileDetail';
import { CreateProfileWizard } from '../components/CreateProfileWizard';
import { ImportProfileDialog } from '../components/ImportProfileDialog';
import type { ProfileDetails, ToolRef, ProfilePluginRef } from '../lib/types';

export function ProfilesPage() {
  const queryClient = useQueryClient();
  const [selectedProfile, setSelectedProfile] = useState<ProfileDetails | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [isImportDialogOpen, setIsImportDialogOpen] = useState(false);
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

  const isDialogOpen = useUIStore((state) => state.isCreateProfileDialogOpen);
  const setDialogOpen = useUIStore((state) => state.setCreateProfileDialogOpen);

  const { data: profiles = [], isLoading } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  const createMutation = useMutation({
    mutationFn: async ({
      name,
      description,
      tools,
    }: {
      name: string;
      description?: string;
      tools: ToolRef[];
    }) => {
      // Create the profile first
      const profile = await createEmptyProfile(name, description);

      // If tools were selected, add them to the profile
      if (tools.length > 0) {
        await updateProfile({ id: profile.id, toolRefs: tools });
      }

      return profile;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      setDialogOpen(false);
      toast.success('Profile created successfully');
    },
    onError: (err) => {
      toast.error(`Failed to create profile: ${err instanceof Error ? err.message : String(err)}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteProfile,
    onSuccess: (response, deletedId) => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      // Only clear selectedProfile if we deleted the currently selected one
      if (selectedProfile?.id === deletedId) {
        setSelectedProfile(null);
      }
      if (response.converted_projects > 0) {
        toast.success(
          `Profile deleted. ${response.converted_projects} project(s) converted to local tools.`
        );
      } else {
        toast.success('Profile deleted.');
      }
    },
    onError: (err) => {
      toast.error(`Failed to delete profile: ${err instanceof Error ? err.message : String(err)}`);
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, toolRefs, pluginRefs }: { id: string; toolRefs?: ToolRef[]; pluginRefs?: ProfilePluginRef[] }) =>
      updateProfile({ id, toolRefs, pluginRefs }),
    onSuccess: async (response, variables) => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      // Refresh the selected profile details using the ID from mutation variables
      // This avoids stale closure issues with selectedProfile
      try {
        const details = await getProfile(variables.id);
        setSelectedProfile(details);
      } catch (err) {
        console.error('Failed to refresh profile:', err);
      }
      if (response.sync_result.affected_projects > 0) {
        toast.success(
          `Profile updated. Synced to ${response.sync_result.affected_projects} project(s).`
        );
      } else {
        toast.success('Profile updated.');
      }
    },
    onError: (err) => {
      toast.error(`Failed to update profile: ${err}`);
    },
  });

  async function handleAddTools(tools: ToolRef[]) {
    if (!selectedProfile) return;
    // Merge new tools with existing ones
    const existingTools = selectedProfile.tool_refs || [];
    const mergedTools = [...existingTools, ...tools];
    updateMutation.mutate({ id: selectedProfile.id, toolRefs: mergedTools });
  }

  async function handleAddPlugins(plugins: ProfilePluginRef[]) {
    if (!selectedProfile) return;
    // Merge new plugins with existing ones
    const existingPlugins = selectedProfile.plugin_refs || [];
    const mergedPlugins = [...existingPlugins, ...plugins];
    updateMutation.mutate({ id: selectedProfile.id, pluginRefs: mergedPlugins });
  }

  async function handleExportProfile() {
    if (!selectedProfile) return;

    try {
      const path = await save({
        defaultPath: `${selectedProfile.name}.tars-profile.json`,
        filters: [{ name: 'TARS Profile', extensions: ['tars-profile.json'] }],
      });

      if (path) {
        await exportProfileJson(selectedProfile.id, path);
        toast.success(`Profile exported to ${path}`);
      }
    } catch (err) {
      toast.error(`Export failed: ${err}`);
    }
  }

  async function handleSelectProfile(id: string) {
    setLoadingDetails(true);
    try {
      const details = await getProfile(id);
      setSelectedProfile(details);
    } catch (err) {
      // Profile load failed silently - user can retry by clicking again
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
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsImportDialogOpen(true)}
            className="flex items-center gap-2 px-4 py-2 border border-border rounded text-sm font-medium hover:bg-muted transition-colors"
          >
            <Upload className="h-4 w-4" />
            Import
          </button>
          <button
            onClick={() => setDialogOpen(true)}
            className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
          >
            <Plus className="h-4 w-4" />
            Create Profile
          </button>
        </div>
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
                  Create a profile to share tools across projects
                </p>
              </div>
            ) : (
              <ProfileList
                profiles={profiles}
                selectedId={selectedProfile?.id || null}
                onSelect={handleSelectProfile}
                onDelete={(id) => setDeleteConfirmId(id)}
              />
            )}
          </div>
        </div>

        {/* Profile detail view */}
        <div className="flex-1 overflow-auto bg-background p-6">
          {loadingDetails ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <p className="text-sm text-muted-foreground">Loading profile...</p>
            </div>
          ) : selectedProfile ? (
            <ProfileDetail
              profile={selectedProfile}
              onAddTools={handleAddTools}
              onAddPlugins={handleAddPlugins}
              onExportProfile={handleExportProfile}
            />
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

      {/* Create Profile Wizard */}
      <CreateProfileWizard
        open={isDialogOpen}
        onOpenChange={setDialogOpen}
        onCreate={(name, description, tools) => createMutation.mutate({ name, description, tools })}
        isLoading={createMutation.isPending}
        error={createMutation.error ? String(createMutation.error) : undefined}
      />

      {/* Import Profile Dialog */}
      <ImportProfileDialog
        open={isImportDialogOpen}
        onOpenChange={setIsImportDialogOpen}
        onImportComplete={() => {
          queryClient.invalidateQueries({ queryKey: ['profiles'] });
          toast.success('Profile imported successfully');
        }}
      />

      {/* Delete Confirmation Dialog */}
      {deleteConfirmId && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center"
          role="dialog"
          aria-modal="true"
          aria-labelledby="delete-dialog-title"
          onKeyDown={(e) => {
            if (e.key === 'Escape') setDeleteConfirmId(null);
          }}
        >
          <div
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
            onClick={() => setDeleteConfirmId(null)}
            aria-hidden="true"
          />
          <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md">
            <div className="flex items-center justify-between p-4 border-b">
              <div className="flex items-center gap-2 text-destructive">
                <AlertTriangle className="h-5 w-5" aria-hidden="true" />
                <h2 id="delete-dialog-title" className="text-lg font-semibold">
                  Delete Profile
                </h2>
              </div>
              <button
                onClick={() => setDeleteConfirmId(null)}
                className="text-muted-foreground hover:text-foreground transition-colors"
                aria-label="Close dialog"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            <div className="p-4 space-y-3">
              <p className="text-sm">Are you sure you want to delete this profile?</p>
              <p className="text-sm text-muted-foreground">
                Projects using this profile will have their tools converted to local overrides. This
                action cannot be undone.
              </p>
            </div>
            <div className="flex justify-end gap-2 p-4 border-t bg-muted/30">
              <Button variant="outline" onClick={() => setDeleteConfirmId(null)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={() => {
                  deleteMutation.mutate(deleteConfirmId);
                  setDeleteConfirmId(null);
                }}
                disabled={deleteMutation.isPending}
              >
                {deleteMutation.isPending ? 'Deleting...' : 'Delete Profile'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
