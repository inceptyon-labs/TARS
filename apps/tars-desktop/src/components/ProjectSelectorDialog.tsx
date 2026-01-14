import { useQuery } from '@tanstack/react-query';
import { FolderOpen, Loader2 } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog';
import { Button } from './ui/button';
import { listProjects } from '../lib/ipc';
import type { ProjectInfo } from '../lib/types';

interface ProjectSelectorDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelectProject: (project: ProjectInfo) => void;
  title?: string;
  description?: string;
}

export function ProjectSelectorDialog({
  open,
  onOpenChange,
  onSelectProject,
  title = 'Select a Project',
  description = 'Choose a project to apply this action to.',
}: ProjectSelectorDialogProps) {
  const {
    data: projects,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
    enabled: open,
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <div className="max-h-[300px] overflow-y-auto">
          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          )}

          {error && (
            <div className="py-8 text-center text-sm text-destructive">Failed to load projects</div>
          )}

          {!isLoading && !error && projects?.length === 0 && (
            <div className="py-8 text-center text-sm text-muted-foreground">
              No projects found. Add a project first.
            </div>
          )}

          {!isLoading && !error && projects && projects.length > 0 && (
            <div className="space-y-2">
              {projects.map((project) => (
                <button
                  key={project.id}
                  type="button"
                  onClick={() => onSelectProject(project)}
                  className="w-full flex items-center gap-3 p-3 rounded-lg border bg-muted/30 hover:bg-muted/50 transition-colors text-left"
                >
                  <FolderOpen className="h-5 w-5 text-muted-foreground shrink-0" />
                  <div className="min-w-0 flex-1">
                    <div className="font-medium truncate">{project.name}</div>
                    <div className="text-xs text-muted-foreground truncate">{project.path}</div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="flex justify-end">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
