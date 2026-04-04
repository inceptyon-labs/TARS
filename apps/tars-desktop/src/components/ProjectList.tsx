import { Trash2, FolderOpen, GitBranch, ChevronRight, ChevronDown } from 'lucide-react';
import { useState, useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { homeDir } from '@tauri-apps/api/path';
import { getProjectIcon } from '../lib/ipc';
import type { ProjectInfo } from '../lib/types';
import type { ProjectGitStatus } from '../lib/ipc';

interface ProjectListProps {
  projects: ProjectInfo[];
  selectedPath: string | null;
  gitStatusMap?: Record<string, ProjectGitStatus>;
  categoryMap?: Record<string, string>;
  onSelect: (project: ProjectInfo) => void;
  onRemove: (id: string) => void;
}

const GROUP_ORDER = ['Apps', 'Websites', 'Tools'];

function shortenPath(path: string, home: string | null): string {
  if (home && path.startsWith(home)) {
    return '~' + path.slice(home.length);
  }
  return path;
}

function ProjectIcon({ projectPath }: { projectPath: string }) {
  const { data: iconUrl } = useQuery({
    queryKey: ['project-icon', projectPath],
    queryFn: () => getProjectIcon(projectPath),
    staleTime: 60000,
  });

  if (iconUrl) {
    return <img src={iconUrl} alt="" className="h-6 w-6 shrink-0 rounded-sm object-contain" />;
  }

  return <FolderOpen className="h-6 w-6 shrink-0 text-primary/70" />;
}

function ProjectRow({
  project,
  isSelected,
  gitStatus,
  home,
  onSelect,
  onRemove,
}: {
  project: ProjectInfo;
  isSelected: boolean;
  gitStatus?: ProjectGitStatus;
  home: string | null;
  onSelect: () => void;
  onRemove: () => void;
}) {
  return (
    <li className="group">
      <button
        onClick={onSelect}
        className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
          isSelected
            ? 'active text-foreground font-medium'
            : 'text-muted-foreground hover:text-foreground'
        }`}
      >
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2 min-w-0">
            <ProjectIcon projectPath={project.path} />
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
              onRemove();
            }}
            className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded text-destructive shrink-0"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground/60 mt-0.5 pl-8">
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
}

export function ProjectList({
  projects,
  selectedPath,
  gitStatusMap = {},
  categoryMap = {},
  onSelect,
  onRemove,
}: ProjectListProps) {
  const [home, setHome] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  useEffect(() => {
    homeDir()
      .then(setHome)
      .catch(() => setHome(null));
  }, []);

  const hasCategories = Object.keys(categoryMap).length > 0;

  const groups = useMemo(() => {
    if (!hasCategories) return null;

    const grouped: Record<string, ProjectInfo[]> = {};
    for (const project of projects) {
      const cat = categoryMap[project.path] || 'Tools';
      if (!grouped[cat]) grouped[cat] = [];
      grouped[cat].push(project);
    }

    return GROUP_ORDER.filter((g) => grouped[g]?.length).map((g) => ({
      name: g,
      projects: grouped[g],
    }));
  }, [projects, categoryMap, hasCategories]);

  const toggleGroup = (name: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  };

  // Flat list if categories haven't loaded yet
  if (!groups) {
    return (
      <ul className="space-y-1">
        {projects.map((project) => (
          <ProjectRow
            key={project.id}
            project={project}
            isSelected={selectedPath === project.path}
            gitStatus={gitStatusMap[project.path]}
            home={home}
            onSelect={() => onSelect(project)}
            onRemove={() => onRemove(project.id)}
          />
        ))}
      </ul>
    );
  }

  return (
    <div className="space-y-2">
      {groups.map(({ name, projects: groupProjects }) => {
        const isCollapsed = collapsed.has(name);
        return (
          <div key={name}>
            <button
              onClick={() => toggleGroup(name)}
              className="flex items-center gap-1.5 px-2 py-1 w-full text-xs font-medium text-muted-foreground/70 uppercase tracking-wider hover:text-muted-foreground transition-colors"
            >
              {isCollapsed ? (
                <ChevronRight className="h-3 w-3" />
              ) : (
                <ChevronDown className="h-3 w-3" />
              )}
              {name}
              <span className="text-muted-foreground/40 normal-case tracking-normal font-normal">
                {groupProjects.length}
              </span>
            </button>
            {!isCollapsed && (
              <ul className="space-y-1">
                {groupProjects.map((project) => (
                  <ProjectRow
                    key={project.id}
                    project={project}
                    isSelected={selectedPath === project.path}
                    gitStatus={gitStatusMap[project.path]}
                    home={home}
                    onSelect={() => onSelect(project)}
                    onRemove={() => onRemove(project.id)}
                  />
                ))}
              </ul>
            )}
          </div>
        );
      })}
    </div>
  );
}
