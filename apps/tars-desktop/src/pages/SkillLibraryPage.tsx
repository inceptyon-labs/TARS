import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'sonner';
import {
  Plus,
  FolderPlus,
  RefreshCw,
  User,
  FolderGit2,
  X,
  AlertTriangle,
  Link2,
} from 'lucide-react';
import {
  listProjects,
  listSkillSources,
  removeSkillSource,
  addSkillSource,
  getProjectSkillMatrix,
  deploySkill,
  undeploySkill,
  type SkillCell,
  type SkillMatrixRow,
  type SkillAgent,
} from '../lib/ipc';
import type { ProjectInfo } from '../lib/types';
import { cn } from '../lib/utils';

const AGENTS: { key: SkillAgent; label: string }[] = [
  { key: 'claude', label: 'Claude' },
  { key: 'codex', label: 'Codex' },
];

export function SkillLibraryPage() {
  const queryClient = useQueryClient();
  // null = User scope; otherwise a project id.
  const [targetProjectId, setTargetProjectId] = useState<string | null>(null);
  const [busyCell, setBusyCell] = useState<string | null>(null);

  const { data: projects = [] } = useQuery({ queryKey: ['projects'], queryFn: listProjects });
  const { data: sources = [] } = useQuery({
    queryKey: ['skill-sources'],
    queryFn: listSkillSources,
  });
  const {
    data: matrix = [],
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['skill-matrix', targetProjectId],
    queryFn: () => getProjectSkillMatrix(targetProjectId),
  });

  const scope = targetProjectId ? 'project' : 'user';
  const errorMessage = error instanceof Error ? error.message : String(error);

  const invalidateMatrix = () => {
    queryClient.invalidateQueries({ queryKey: ['skill-matrix'] });
  };

  async function handleAddSource() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select a skills library directory',
    });
    if (typeof selected !== 'string') return;
    try {
      await addSkillSource(selected);
      queryClient.invalidateQueries({ queryKey: ['skill-sources'] });
      invalidateMatrix();
      toast.success('Library source added');
    } catch (e) {
      toast.error(`Failed to add source: ${e}`);
    }
  }

  async function handleRemoveSource(id: number) {
    try {
      await removeSkillSource(id);
      queryClient.invalidateQueries({ queryKey: ['skill-sources'] });
      invalidateMatrix();
    } catch (e) {
      toast.error(`Failed to remove source: ${e}`);
    }
  }

  async function toggleCell(row: SkillMatrixRow, agent: SkillAgent, cell: SkillCell) {
    if (cell.status === 'collision') return;
    const key = `${row.name}:${agent}`;
    setBusyCell(key);
    try {
      if (!cell.deployed) {
        await deploySkill({
          skillName: row.name,
          sourceDir: row.sourceDir,
          agent,
          scope,
          projectId: targetProjectId,
          linkKind: 'symlink',
        });
      } else {
        // Turning off: a tracked cell has an id; an adopted (hand-made) symlink
        // must first be recorded before we can remove it.
        let id = cell.deploymentId;
        if (id == null) {
          const dep = await deploySkill({
            skillName: row.name,
            sourceDir: row.sourceDir,
            agent,
            scope,
            projectId: targetProjectId,
            linkKind: 'symlink',
          });
          id = dep.id;
        }
        await undeploySkill(id);
      }
      invalidateMatrix();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyCell(null);
    }
  }

  const deployedCount = useMemo(
    () => matrix.filter((r) => r.claude.deployed || r.codex.deployed).length,
    [matrix]
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="tars-header flex items-center justify-between px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <div>
            <h1 className="text-lg font-semibold text-foreground">Skill Library</h1>
            <p className="text-xs text-muted-foreground">
              Standalone skills, deployed to Claude &amp; Codex per scope
            </p>
          </div>
        </div>
        <button
          onClick={handleAddSource}
          className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
        >
          <FolderPlus className="w-4 h-4" />
          Add source
        </button>
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Left: scope target selector */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="px-4 py-3 tars-segment-line">
            <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Deploy to
            </span>
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            <TargetRow
              icon={<User className="w-4 h-4" />}
              label="User (all projects)"
              active={targetProjectId === null}
              onClick={() => setTargetProjectId(null)}
            />
            {projects.map((p: ProjectInfo) => (
              <TargetRow
                key={p.id}
                icon={<FolderGit2 className="w-4 h-4" />}
                label={p.name}
                active={targetProjectId === p.id}
                onClick={() => setTargetProjectId(p.id)}
              />
            ))}
          </div>
        </div>

        {/* Right: sources + matrix */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Sources bar */}
          <div className="px-6 py-3 tars-segment-line flex items-center gap-2 flex-wrap">
            <span className="text-xs text-muted-foreground mr-1">Sources:</span>
            {sources.length === 0 && (
              <span className="text-xs text-muted-foreground italic">none — add one to begin</span>
            )}
            {sources.map((s) => (
              <span
                key={s.id}
                className="inline-flex items-center gap-1.5 rounded bg-muted px-2 py-1 text-xs"
                title={s.path}
              >
                {s.label ?? shortPath(s.path)}
                <button
                  onClick={() => handleRemoveSource(s.id)}
                  className="text-muted-foreground hover:text-destructive"
                  title="Remove source"
                >
                  <X className="w-3 h-3" />
                </button>
              </span>
            ))}
          </div>

          {/* Matrix */}
          <div className="flex-1 overflow-y-auto">
            {isLoading ? (
              <div className="flex items-center justify-center h-40 text-muted-foreground text-sm gap-2">
                <RefreshCw className="w-4 h-4 animate-spin" /> Scanning library…
              </div>
            ) : isError ? (
              <div className="p-6 text-sm text-destructive">{errorMessage}</div>
            ) : matrix.length === 0 ? (
              <EmptyState hasSources={sources.length > 0} onAdd={handleAddSource} />
            ) : (
              <table className="w-full text-sm">
                <thead className="sticky top-0 bg-background border-b border-border">
                  <tr className="text-left text-xs text-muted-foreground">
                    <th className="px-6 py-2 font-medium">
                      Skill{' '}
                      <span className="text-muted-foreground/60">
                        ({deployedCount}/{matrix.length} active here)
                      </span>
                    </th>
                    {AGENTS.map((a) => (
                      <th key={a.key} className="px-4 py-2 font-medium text-center w-28">
                        {a.label}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {matrix.map((row) => (
                    <tr key={row.name} className="border-b border-border/50 hover:bg-muted/30">
                      <td className="px-6 py-2.5">
                        <div className="font-medium text-foreground">{row.name}</div>
                        <div className="text-xs text-muted-foreground line-clamp-1">
                          {row.description}
                        </div>
                      </td>
                      {AGENTS.map((a) => {
                        const cell = row[a.key];
                        const key = `${row.name}:${a.key}`;
                        return (
                          <td key={a.key} className="px-4 py-2.5 text-center">
                            <CellToggle
                              cell={cell}
                              busy={busyCell === key}
                              onToggle={() => toggleCell(row, a.key, cell)}
                            />
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function TargetRow({
  icon,
  label,
  active,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full flex items-center gap-2 px-3 py-2 rounded text-sm text-left transition-colors',
        active ? 'bg-primary/15 text-foreground' : 'text-muted-foreground hover:bg-muted/50'
      )}
    >
      <span className={active ? 'text-primary' : ''}>{icon}</span>
      <span className="truncate">{label}</span>
    </button>
  );
}

function CellToggle({
  cell,
  busy,
  onToggle,
}: {
  cell: SkillCell;
  busy: boolean;
  onToggle: () => void;
}) {
  if (cell.status === 'collision') {
    return (
      <span
        className="inline-flex items-center justify-center text-amber-500"
        title="A different file or directory already occupies this name here"
      >
        <AlertTriangle className="w-4 h-4" />
      </span>
    );
  }
  return (
    <label className="inline-flex items-center justify-center gap-1.5 cursor-pointer">
      <input
        type="checkbox"
        className="accent-primary w-4 h-4"
        checked={cell.deployed}
        disabled={busy}
        onChange={onToggle}
      />
      {cell.status === 'adopted' && (
        <span title="Deployed via an existing symlink (not created by TARS)">
          <Link2 className="w-3 h-3 text-blue-400" />
        </span>
      )}
    </label>
  );
}

function EmptyState({ hasSources, onAdd }: { hasSources: boolean; onAdd: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center h-64 text-center px-6">
      <div className="w-14 h-14 rounded-lg tars-panel flex items-center justify-center mb-4">
        <Plus className="w-6 h-6 text-muted-foreground" />
      </div>
      <p className="text-sm text-muted-foreground mb-3">
        {hasSources
          ? 'No skills found in your registered sources.'
          : 'Add a directory that holds your standalone skills to get started.'}
      </p>
      {!hasSources && (
        <button
          onClick={onAdd}
          className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
        >
          <FolderPlus className="w-4 h-4" />
          Add source
        </button>
      )}
    </div>
  );
}

function shortPath(path: string): string {
  const parts = path.split('/').filter(Boolean);
  return parts.length <= 2 ? path : `…/${parts.slice(-2).join('/')}`;
}
