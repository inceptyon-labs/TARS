import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, FolderOpen, RefreshCw, AlertCircle, Search, FolderGit2 } from 'lucide-react';
import { useState, useRef } from 'react';
import { toast } from 'sonner';
import {
  listProjects,
  addProject,
  removeProject,
  scanProject,
  assignProfileAsPlugin,
  unassignProfilePlugin,
  getProjectTools,
} from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { ProjectList } from '../components/ProjectList';
import { ProjectOverview } from '../components/ProjectOverview';
import { AddProjectDialog } from '../components/AddProjectDialog';
import { AssignProfileDialog } from '../components/AssignProfileDialog';
import type { Inventory, ProjectInfo, ProjectToolsResponse } from '../lib/types';

export function ProjectsPage() {
  const queryClient = useQueryClient();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [selectedProject, setSelectedProject] = useState<ProjectInfo | null>(null);
  const [inventory, setInventory] = useState<Inventory | null>(null);
  const [projectTools, setProjectTools] = useState<ProjectToolsResponse | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [isAssignDialogOpen, setIsAssignDialogOpen] = useState(false);
  const scanRequestRef = useRef(0); // Guard against stale scan results

  const isDialogOpen = useUIStore((state) => state.isAddProjectDialogOpen);
  const setDialogOpen = useUIStore((state) => state.setAddProjectDialogOpen);

  const { data: projects = [], isLoading } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const addMutation = useMutation({
    mutationFn: addProject,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      setDialogOpen(false);
    },
  });

  const removeMutation = useMutation({
    mutationFn: removeProject,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      if (selectedPath) {
        setSelectedPath(null);
        setSelectedProject(null);
        setProjectTools(null);
      }
    },
  });

  const assignMutation = useMutation({
    mutationFn: ({ projectId, profileId }: { projectId: string; profileId: string }) =>
      assignProfileAsPlugin(projectId, profileId),
    onSuccess: async (result) => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      if (selectedProject) {
        await handleScan(selectedProject);
      }
      setIsAssignDialogOpen(false);
      toast.success(`Profile installed as plugin: ${result.plugin_id}`);
    },
    onError: (err) => {
      toast.error(`Failed to assign profile: ${err}`);
    },
  });

  const unassignMutation = useMutation({
    mutationFn: (projectId: string) => unassignProfilePlugin(projectId),
    onSuccess: async () => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      if (selectedProject) {
        await handleScan(selectedProject);
      }
      setIsAssignDialogOpen(false);
      toast.success('Profile plugin uninstalled');
    },
    onError: (err) => {
      toast.error(`Failed to unassign profile: ${err}`);
    },
  });

  async function handleScan(project: ProjectInfo) {
    const path = project.path;
    const requestId = ++scanRequestRef.current; // Increment to invalidate previous requests

    setScanning(true);
    setScanError(null);
    // Clear previous state before setting new project to prevent stale data
    setSelectedPath(path);
    setSelectedProject(project);
    setInventory(null);
    setProjectTools(null);

    try {
      // Fetch scan and tools in parallel, but handle failures independently
      const [scanResult, toolsResult] = await Promise.allSettled([
        scanProject(path),
        getProjectTools(project.id),
      ]);

      // Guard against stale results from slower requests
      if (requestId !== scanRequestRef.current) return;

      // Handle scan result
      if (scanResult.status === 'fulfilled') {
        setInventory(scanResult.value);
      } else {
        setScanError(String(scanResult.reason));
        setSelectedProject(null);
        setSelectedPath(null);
        return; // Exit early if scan failed
      }

      // Handle tools result (non-critical, just log error)
      if (toolsResult.status === 'fulfilled') {
        setProjectTools(toolsResult.value);
      } else {
        console.error('Failed to fetch project tools:', toolsResult.reason);
        // Don't clear selection - scan succeeded, tools are optional
      }
    } finally {
      if (requestId === scanRequestRef.current) {
        setScanning(false);
      }
    }
  }

  async function handleRefreshTools() {
    if (selectedProject) {
      try {
        const tools = await getProjectTools(selectedProject.id);
        setProjectTools(tools);
      } catch (err) {
        console.error('Failed to refresh project tools:', err);
      }
    }
  }

  function handleAssignProfile(profileId: string) {
    if (selectedProject) {
      assignMutation.mutate({ projectId: selectedProject.id, profileId });
    }
  }

  function handleUnassignProfile() {
    if (selectedProject) {
      unassignMutation.mutate(selectedProject.id);
    }
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Projects</h2>
        </div>
        <button
          onClick={() => setDialogOpen(true)}
          className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
        >
          <Plus className="h-4 w-4" />
          Add Project
        </button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Project list sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search projects..."
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

          {/* Segment line */}
          <div className="tars-segment-line" />

          <div className="flex-1 overflow-auto p-3">
            {isLoading ? (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <RefreshCw className="h-5 w-5 animate-spin text-primary" />
                <span className="text-xs text-muted-foreground">Loading...</span>
              </div>
            ) : projects.length === 0 ? (
              <div className="text-center py-12 px-4">
                <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
                  <FolderGit2 className="h-8 w-8 text-muted-foreground" />
                </div>
                <p className="text-sm font-medium text-foreground">No projects</p>
                <p className="text-xs text-muted-foreground mt-1">Add a project to get started</p>
              </div>
            ) : (
              <ProjectList
                projects={projects}
                selectedPath={selectedPath}
                onSelect={(project) => handleScan(project)}
                onRemove={(id) => removeMutation.mutate(id)}
              />
            )}
          </div>
        </div>

        {/* Inventory view */}
        <div className="flex-1 overflow-auto bg-background">
          {scanning ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <div className="text-center">
                <p className="text-sm font-medium">Scanning project...</p>
                <p className="text-xs text-muted-foreground mt-1">Analyzing configuration</p>
              </div>
            </div>
          ) : scanError ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-16 h-16 rounded-lg bg-destructive/10 flex items-center justify-center">
                <AlertCircle className="h-8 w-8 text-destructive" />
              </div>
              <div className="text-center max-w-md">
                <p className="font-medium text-destructive">Scan failed</p>
                <p className="text-sm mt-2 text-muted-foreground">{scanError}</p>
              </div>
            </div>
          ) : inventory && selectedPath && selectedProject ? (
            <ProjectOverview
              inventory={inventory}
              projectPath={selectedPath}
              projectTools={projectTools}
              onAssignProfile={() => setIsAssignDialogOpen(true)}
              onRefreshTools={handleRefreshTools}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <FolderOpen className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">Select a project to scan</p>
                <p className="text-xs text-muted-foreground/60 mt-1">
                  View skills, commands, agents, and more
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Add Project Dialog */}
      <AddProjectDialog
        open={isDialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={(path) => addMutation.mutate(path)}
        isLoading={addMutation.isPending}
        error={addMutation.error ? String(addMutation.error) : undefined}
      />

      {/* Assign Profile Dialog */}
      <AssignProfileDialog
        open={isAssignDialogOpen}
        onOpenChange={setIsAssignDialogOpen}
        onAssign={handleAssignProfile}
        onUnassign={handleUnassignProfile}
        currentProfileId={projectTools?.profile?.id || null}
        projectName={selectedProject?.name || ''}
        isLoading={assignMutation.isPending || unassignMutation.isPending}
      />
    </div>
  );
}
