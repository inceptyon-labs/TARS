import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
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
  Puzzle,
  ChevronDown,
  ChevronRight,
  FileText,
  EyeOff,
} from 'lucide-react';
import {
  listProjects,
  listSkillSources,
  removeSkillSource,
  addSkillSource,
  getProjectSkillMatrix,
  deploySkill,
  undeploySkill,
  setSkillMute,
  resyncSkillDeployment,
  setProjectPluginEnabled,
  type SkillCell,
  type SkillMatrixRow,
  type SkillAgent,
  type SkillGroup,
  type SkillMuteState,
} from '../lib/ipc';
import type { ProjectInfo } from '../lib/types';
import { cn } from '../lib/utils';

const AGENTS: { key: SkillAgent; label: string }[] = [
  { key: 'claude', label: 'Claude' },
  { key: 'codex', label: 'Codex' },
];

export function SkillLibraryPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  // null = User scope; otherwise a project id.
  const [targetProjectId, setTargetProjectId] = useState<string | null>(null);
  const [busyCell, setBusyCell] = useState<string | null>(null);
  const [busyGroup, setBusyGroup] = useState<string | null>(null);
  // Source roots whose skill list is expanded (default: collapsed).
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const { data: projects = [] } = useQuery({ queryKey: ['projects'], queryFn: listProjects });
  const { data: sources = [] } = useQuery({
    queryKey: ['skill-sources'],
    queryFn: listSkillSources,
  });
  const {
    data: groups = [],
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

  const groupKey = (group: SkillGroup) => `${group.kind}:${group.pluginId ?? group.sourceRoot}`;

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

  // Deploy or remove one skill for one agent (no query invalidation — callers
  // batch it). Adopts a hand-made symlink before removing when there's no
  // tracked deployment id.
  async function applyCell(row: SkillMatrixRow, agent: SkillAgent, cell: SkillCell, on: boolean) {
    if (on) {
      if (cell.deployed) return;
      await deploySkill({
        skillName: row.name,
        sourceDir: row.sourceDir,
        agent,
        scope,
        projectId: targetProjectId,
        linkKind: 'symlink',
      });
    } else {
      if (!cell.deployed) return;
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
  }

  async function toggleCell(row: SkillMatrixRow, agent: SkillAgent, cell: SkillCell) {
    if (cell.status === 'collision') return;
    const key = `${row.name}:${agent}`;
    setBusyCell(key);
    try {
      await applyCell(row, agent, cell, !cell.deployed);
      invalidateMatrix();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyCell(null);
    }
  }

  // Deploy/remove a whole group for one agent at once (the common case —
  // piecemeal plugin skills break the workflow). If everything eligible is on,
  // turn it all off; otherwise fill in the rest.
  async function toggleGroup(group: SkillGroup, agent: SkillAgent) {
    const s = agentSummary(group.skills, agent);
    if (s.eligible.length === 0) return;
    const turnOn = !s.all;
    setBusyGroup(`${groupKey(group)}:${agent}`);
    try {
      for (const row of s.eligible) {
        await applyCell(row, agent, row[agent], turnOn);
      }
      invalidateMatrix();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyGroup(null);
    }
  }

  // Set a Claude standalone skill's mute state (null = fully visible).
  async function muteCell(row: SkillMatrixRow, agent: SkillAgent, next: SkillMuteState | null) {
    const cell = row[agent];
    if (cell.deploymentId == null) return;
    const key = `${row.name}:${agent}`;
    setBusyCell(key);
    try {
      await setSkillMute(cell.deploymentId, next);
      invalidateMatrix();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyCell(null);
    }
  }

  // Re-copy a drifted copy deployment from its source.
  async function resyncCell(row: SkillMatrixRow, agent: SkillAgent) {
    const cell = row[agent];
    if (cell.deploymentId == null) return;
    const key = `${row.name}:${agent}`;
    setBusyCell(key);
    try {
      await resyncSkillDeployment(cell.deploymentId);
      invalidateMatrix();
      toast.success(`Re-synced ${row.name}`);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyCell(null);
    }
  }

  // Enable/disable a whole plugin for the current project (enabledPlugins).
  async function togglePluginHere(group: SkillGroup) {
    if (!targetProjectId || !group.pluginId) return;
    const pluginKey = group.pluginMarketplace
      ? `${group.pluginId}@${group.pluginMarketplace}`
      : group.pluginId;
    setBusyGroup(`${groupKey(group)}:plugin-here`);
    try {
      await setProjectPluginEnabled(targetProjectId, pluginKey, group.pluginDisabledHere);
      invalidateMatrix();
      toast.success(
        group.pluginDisabledHere
          ? `Enabled ${group.label} here`
          : `Disabled ${group.label} for this project`
      );
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setBusyGroup(null);
    }
  }

  const toggleExpand = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const allRows = useMemo(() => groups.flatMap((g) => g.skills), [groups]);
  const deployedCount = allRows.filter((r) => r.claude.deployed || r.codex.deployed).length;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="tars-header relative z-10 shrink-0 border-b border-border flex items-center justify-between px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <div>
            <h1 className="text-lg font-semibold text-foreground">Skill Library</h1>
            <p className="text-xs text-muted-foreground">
              Installed plugins (auto) + standalone skill sources, per Claude/Codex scope
            </p>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/plugins')}
            className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
            title="Plugins (like pasiv) are managed in the Marketplace"
          >
            <Puzzle className="w-3.5 h-3.5" />
            Plugins → Marketplace
          </button>
          <button
            onClick={handleAddSource}
            className="tars-button-primary flex items-center gap-2 px-4 py-2 rounded text-sm font-medium"
          >
            <FolderPlus className="w-4 h-4" />
            Add source
          </button>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden min-h-0">
        {/* Left: scope target selector */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="px-4 py-3 border-b border-border shrink-0">
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
          <div className="px-6 py-3 border-b border-border shrink-0 flex items-center gap-2 flex-wrap">
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
            ) : groups.length === 0 ? (
              <EmptyState hasSources={sources.length > 0} onAdd={handleAddSource} />
            ) : (
              <div className="text-sm">
                {/* Header row */}
                <div className="sticky top-0 z-10 flex items-center bg-background border-b border-border px-6 py-2 text-xs text-muted-foreground">
                  <div className="flex-1 min-w-0 font-medium">
                    Skill{' '}
                    <span className="text-muted-foreground/60">
                      ({deployedCount}/{allRows.length} active here)
                    </span>
                  </div>
                  <div className="flex items-center shrink-0">
                    {AGENTS.map((a) => (
                      <div key={a.key} className="w-20 text-center font-medium">
                        {a.label}
                      </div>
                    ))}
                  </div>
                </div>

                {/* Grouped skill rows */}
                <div className="divide-y divide-border/50">
                  {groups.map((group) => {
                    const key = groupKey(group);

                    // A source that is itself a single skill renders flat (no
                    // collapsible header). A folder of skills stays a group even
                    // when it currently holds just one.
                    if (group.singleSkill) {
                      const row = group.skills[0];
                      return (
                        <div key={key} className="flex items-center px-4 py-2.5 hover:bg-muted/30">
                          <div className="flex-1 min-w-0 flex items-center gap-2 pr-4">
                            <GroupIcon kind={group.kind} single />
                            <div className="min-w-0">
                              <div className="font-medium text-foreground truncate">{row.name}</div>
                              <div className="text-xs text-muted-foreground truncate">
                                {row.description}
                              </div>
                            </div>
                          </div>
                          <div className="flex items-center shrink-0">
                            {AGENTS.map((a) => (
                              <div key={a.key} className="w-20 flex justify-center">
                                <SkillCellControl
                                  row={row}
                                  agent={a.key}
                                  busy={busyCell === `${row.name}:${a.key}`}
                                  onToggle={() => toggleCell(row, a.key, row[a.key])}
                                  onBadgeClick={() => navigate('/plugins')}
                                  onMute={(next) => muteCell(row, a.key, next)}
                                  onResync={() => resyncCell(row, a.key)}
                                />
                              </div>
                            ))}
                          </div>
                        </div>
                      );
                    }

                    const isOpen = expanded.has(key);
                    return (
                      <div key={key}>
                        {/* Group header */}
                        <div className="flex items-center px-4 py-2.5 bg-muted/20 hover:bg-muted/40">
                          <button
                            onClick={() => toggleExpand(key)}
                            className="flex-1 min-w-0 flex items-center gap-2 text-left"
                          >
                            {isOpen ? (
                              <ChevronDown className="w-4 h-4 shrink-0 text-muted-foreground" />
                            ) : (
                              <ChevronRight className="w-4 h-4 shrink-0 text-muted-foreground" />
                            )}
                            <GroupIcon kind={group.kind} />
                            <span className="font-medium text-foreground truncate">
                              {group.label}
                            </span>
                            {group.kind === 'plugin' && (
                              <span className="text-[10px] uppercase tracking-wide text-blue-400/80 shrink-0">
                                plugin
                              </span>
                            )}
                            <span className="text-xs text-muted-foreground shrink-0">
                              {group.skills.length} skill{group.skills.length !== 1 ? 's' : ''}
                            </span>
                          </button>
                          {group.kind === 'plugin' && scope === 'project' && (
                            <button
                              type="button"
                              onClick={() => togglePluginHere(group)}
                              disabled={busyGroup === `${key}:plugin-here`}
                              title={
                                group.pluginDisabledHere
                                  ? 'Disabled for this project — click to re-enable'
                                  : "Disable this whole plugin for this project (writes enabledPlugins). Claude can't mute individual plugin skills."
                              }
                              className={cn(
                                'shrink-0 mr-2 inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] transition-colors',
                                group.pluginDisabledHere
                                  ? 'border-amber-500/50 bg-amber-500/10 text-amber-500'
                                  : 'border-border text-muted-foreground hover:text-foreground'
                              )}
                            >
                              <EyeOff className="w-3 h-3" />
                              {group.pluginDisabledHere ? 'off here' : 'disable here'}
                            </button>
                          )}
                          <div className="flex items-center shrink-0">
                            {AGENTS.map((a) => (
                              <div key={a.key} className="w-20 flex justify-center">
                                <GroupCell
                                  rows={group.skills}
                                  agent={a.key}
                                  busy={busyGroup === `${key}:${a.key}`}
                                  onToggle={() => toggleGroup(group, a.key)}
                                  onBadgeClick={() => navigate('/plugins')}
                                />
                              </div>
                            ))}
                          </div>
                        </div>

                        {/* Individual skills */}
                        {isOpen &&
                          group.skills.map((row) => (
                            <div
                              key={row.name}
                              className="flex items-center pl-12 pr-6 py-2 hover:bg-muted/30"
                            >
                              <div className="flex-1 min-w-0 pr-4">
                                <div className="text-foreground truncate">{row.name}</div>
                                <div className="text-xs text-muted-foreground truncate">
                                  {row.description}
                                </div>
                              </div>
                              <div className="flex items-center shrink-0">
                                {AGENTS.map((a) => (
                                  <div key={a.key} className="w-20 flex justify-center">
                                    <SkillCellControl
                                      row={row}
                                      agent={a.key}
                                      busy={busyCell === `${row.name}:${a.key}`}
                                      onToggle={() => toggleCell(row, a.key, row[a.key])}
                                      onBadgeClick={() => navigate('/plugins')}
                                      onMute={(next) => muteCell(row, a.key, next)}
                                      onResync={() => resyncCell(row, a.key)}
                                    />
                                  </div>
                                ))}
                              </div>
                            </div>
                          ))}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

interface AgentSummary {
  allPlugin: boolean;
  pluginId: string | null;
  eligible: SkillMatrixRow[];
  deployedCount: number;
  all: boolean;
  partial: boolean;
}

/** Summarize a group's state for one agent (using each cell's own status). */
function agentSummary(rows: SkillMatrixRow[], agent: SkillAgent): AgentSummary {
  const pluginIds = rows
    .map((r) => (r[agent].status === 'plugin' ? r[agent].pluginId : null))
    .filter(Boolean) as string[];
  const allPlugin = rows.length > 0 && pluginIds.length === rows.length;
  // Eligible = deployable here (not plugin-provided, not a name collision).
  const eligible = rows.filter(
    (r) => r[agent].status !== 'plugin' && r[agent].status !== 'collision'
  );
  const deployedCount = eligible.filter((r) => r[agent].deployed).length;
  return {
    allPlugin,
    pluginId: allPlugin ? pluginIds[0] : null,
    eligible,
    deployedCount,
    all: eligible.length > 0 && deployedCount === eligible.length,
    partial: deployedCount > 0 && deployedCount < eligible.length,
  };
}

/** Origin icon: plugin (puzzle), a single standalone skill (file), or a
 * folder of standalone skills. */
function GroupIcon({ kind, single }: { kind: 'plugin' | 'source'; single?: boolean }) {
  if (kind === 'plugin') {
    return <Puzzle className="w-3.5 h-3.5 shrink-0 text-blue-400" />;
  }
  if (single) {
    return <FileText className="w-3.5 h-3.5 shrink-0 text-emerald-400" />;
  }
  return <FolderGit2 className="w-3.5 h-3.5 shrink-0 text-muted-foreground" />;
}

/** One agent's control for a single skill: a plugin badge or a toggle. */
function SkillCellControl({
  row,
  agent,
  busy,
  onToggle,
  onBadgeClick,
  onMute,
  onResync,
}: {
  row: SkillMatrixRow;
  agent: SkillAgent;
  busy: boolean;
  onToggle: () => void;
  onBadgeClick: () => void;
  onMute: (next: SkillMuteState | null) => void;
  onResync: () => void;
}) {
  const cell = row[agent];
  if (cell.status === 'plugin' && cell.pluginId) {
    return <PluginBadge pluginId={cell.pluginId} onClick={onBadgeClick} />;
  }
  return (
    <CellToggle
      cell={cell}
      agent={agent}
      busy={busy}
      onToggle={onToggle}
      onMute={onMute}
      onResync={onResync}
    />
  );
}

function GroupCell({
  rows,
  agent,
  busy,
  onToggle,
  onBadgeClick,
}: {
  rows: SkillMatrixRow[];
  agent: SkillAgent;
  busy: boolean;
  onToggle: () => void;
  onBadgeClick: () => void;
}) {
  const s = agentSummary(rows, agent);
  if (s.allPlugin && s.pluginId) {
    return <PluginBadge pluginId={s.pluginId} onClick={onBadgeClick} />;
  }
  if (s.eligible.length === 0) {
    return <span className="text-xs text-muted-foreground">—</span>;
  }
  return (
    <input
      type="checkbox"
      className="accent-primary w-4 h-4"
      checked={s.all}
      disabled={busy}
      ref={(el) => {
        if (el) el.indeterminate = s.partial;
      }}
      onChange={onToggle}
      title={`${s.deployedCount}/${s.eligible.length} deployed for ${agent} — click to ${
        s.all ? 'remove all' : 'deploy all'
      }`}
    />
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

const MUTE_OPTIONS: { value: string; label: string }[] = [
  { value: '', label: 'Full' },
  { value: 'name-only', label: 'Name only' },
  { value: 'user-invocable-only', label: 'Slash-only' },
  { value: 'off', label: 'Muted' },
];

function CellToggle({
  cell,
  agent,
  busy,
  onToggle,
  onMute,
  onResync,
}: {
  cell: SkillCell;
  agent: SkillAgent;
  busy: boolean;
  onToggle: () => void;
  onMute: (next: SkillMuteState | null) => void;
  onResync: () => void;
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
  // Muting only applies to a deployed Claude skill on a build that honors it.
  const canMute = agent === 'claude' && cell.deployed && cell.muteSupported;
  const muted = cell.muteState != null;
  return (
    <div className="inline-flex flex-col items-center gap-1">
      <div className="inline-flex items-center gap-1">
        <input
          type="checkbox"
          className="accent-primary w-4 h-4 cursor-pointer"
          checked={cell.deployed}
          disabled={busy}
          onChange={onToggle}
        />
        {cell.status === 'adopted' && (
          <span title="Deployed via an existing symlink (not created by TARS)">
            <Link2 className="w-3 h-3 text-blue-400" />
          </span>
        )}
        {muted && !canMute && (
          <span title="Hidden from the model in this scope">
            <EyeOff className="w-3 h-3 text-amber-500" />
          </span>
        )}
        {cell.drifted && (
          <button
            type="button"
            onClick={onResync}
            disabled={busy}
            title="Source changed since this copy was deployed — click to re-sync"
            className="text-amber-500 hover:text-amber-400"
          >
            <AlertTriangle className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
      {canMute && (
        <select
          className={cn(
            'text-[10px] bg-transparent border border-border rounded px-0.5 py-0 max-w-[76px] cursor-pointer',
            muted ? 'text-amber-500' : 'text-muted-foreground'
          )}
          value={cell.muteState ?? ''}
          disabled={busy}
          title="Model visibility for this scope (writes skillOverrides in settings.json)"
          onChange={(e) => onMute((e.target.value || null) as SkillMuteState | null)}
        >
          {MUTE_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}

function PluginBadge({ pluginId, onClick }: { pluginId: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      title={`Provided by the ${pluginId} plugin — manage in Marketplace`}
      className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground hover:text-foreground max-w-full"
    >
      <Puzzle className="w-3 h-3 shrink-0" />
      <span className="truncate">{pluginId}</span>
    </button>
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
