import { Trash2, FolderOpen, GitBranch } from 'lucide-react';
import { useState, useEffect } from 'react';
import { homeDir } from '@tauri-apps/api/path';
import type { ProjectInfo } from '../lib/types';
import type { ProjectGitStatus } from '../lib/ipc';

interface ProjectListProps {
  projects: ProjectInfo[];
  selectedPath: string | null;
  gitStatusMap?: Record<string, ProjectGitStatus>;
  onSelect: (project: ProjectInfo) => void;
  onRemove: (id: string) => void;
}

// Shorten paths by replacing home directory with ~
function shortenPath(path: string, home: string | null): string {
  if (home && path.startsWith(home)) {
    return '~' + path.slice(home.length);
  }
  return path;
}

export function ProjectList({
  projects,
  selectedPath,
  gitStatusMap = {},
  onSelect,
  onRemove,
}: ProjectListProps) {
  const [home, setHome] = useState<string | null>(null);

  useEffect(() => {
    homeDir()
      .then(setHome)
      .catch(() => setHome(null));
  }, []);

  return (
    <ul className="space-y-1">
      {projects.map((project) => {
        const gitStatus = gitStatusMap[project.path];
        return (
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
                  {gitStatus?.is_git_repo && (
                    <span
                      className={`w-2 h-2 rounded-full shrink-0 ${
                        gitStatus.is_dirty ? 'bg-amber-500' : 'bg-emerald-500'
                      }`}
                      title={gitStatus.is_dirty ? 'Uncommitted changes' : 'Clean'}
                    />
                  )}
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
              <div className="flex items-center gap-2 text-xs text-muted-foreground/60 mt-0.5 pl-6">
                <span className="truncate">{shortenPath(project.path, home)}</span>
                {gitStatus?.is_git_repo && gitStatus.branch && (
                  <span className="flex items-center gap-1 shrink-0 text-muted-foreground/80">
                    <GitBranch className="h-3 w-3" />
                    <span className="max-w-[80px] truncate">{gitStatus.branch}</span>
                  </span>
                )}
              </div>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
