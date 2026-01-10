import { Trash2, FolderOpen } from 'lucide-react';
import type { ProjectInfo } from '../lib/types';

interface ProjectListProps {
  projects: ProjectInfo[];
  selectedPath: string | null;
  onSelect: (project: ProjectInfo) => void;
  onRemove: (id: string) => void;
}

export function ProjectList({ projects, selectedPath, onSelect, onRemove }: ProjectListProps) {
  return (
    <ul className="space-y-1">
      {projects.map((project) => (
        <li key={project.id} className="group">
          <button
            onClick={() => onSelect(project)}
            className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
              selectedPath === project.path
                ? 'active text-foreground font-medium'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-2 min-w-0">
                <FolderOpen className="h-4 w-4 shrink-0 text-primary/70" />
                <span className="font-medium truncate">{project.name}</span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onRemove(project.id);
                }}
                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded text-destructive shrink-0"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
            <div className="text-xs text-muted-foreground/60 mt-0.5 truncate pl-6">
              {project.path}
            </div>
          </button>
        </li>
      ))}
    </ul>
  );
}
