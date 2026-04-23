import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Bot,
  Boxes,
  Cpu,
  RefreshCw,
  Search,
  Server,
  Terminal,
  Trash2,
  Webhook,
} from 'lucide-react';
import { useDeferredValue, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import {
  deleteAgent,
  deleteCodexSkill,
  deleteCommand,
  deleteSkill,
  getProfileHooks,
  getProjectHooks,
  getUserHooks,
  listCodexPluginBridges,
  listCodexSkillBridges,
  listProfileMcpServers,
  listProfiles,
  listProjects,
  scanProfiles,
  scanProjects,
  scanUserScope,
  type ProfileMcpServer,
} from '../lib/ipc';
import type {
  AgentInfo,
  CodexAgentInfo,
  CommandInfo,
  Inventory,
  ProfileToolInventory,
  RuntimeSupportLevel,
  SettingsHooksConfig,
  SkillInfo,
} from '../lib/types';
import type { RuntimeSupportItem } from '../components/RuntimeBadges';
import { getRuntimeSupportForKind, toRuntimeSupportItems } from '../components/RuntimeBadges';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { ConfirmDialog } from '../components/config/ConfirmDialog';

type InventoryKind = 'skill' | 'agent' | 'command' | 'hook' | 'mcp';
type RuntimeFilter = 'all' | 'claude-code' | 'codex';
type SupportFilter = 'all' | RuntimeSupportLevel;

interface InventoryItem {
  id: string;
  name: string;
  kind: InventoryKind;
  description: string | null;
  details: string | null;
  path: string;
  scopeLabel: string;
  containerLabel: string | null;
  sourceRuntime: Exclude<RuntimeFilter, 'all'>;
  runtimeSupport: ReturnType<typeof getRuntimeSupportForKind>;
  pluginLabel: string | null;
}

type InventoryDeleteMode = 'skill' | 'command' | 'agent' | 'codex-skill';

type InventoryDeleteTarget = {
  name: string;
  path: string;
  kind: InventoryKind;
  mode: InventoryDeleteMode;
};

const kindMeta: Record<
  InventoryKind,
  { label: string; route: string; icon: React.ElementType; emptyLabel: string }
> = {
  skill: {
    label: 'Skills',
    route: '/skills',
    icon: Cpu,
    emptyLabel: 'Reusable prompts and workflow packs.',
  },
  agent: {
    label: 'Agents',
    route: '/agents',
    icon: Bot,
    emptyLabel: 'Specialized task handlers and roles.',
  },
  command: {
    label: 'Commands',
    route: '/commands',
    icon: Terminal,
    emptyLabel: 'Slash commands and shortcuts.',
  },
  hook: {
    label: 'Hooks',
    route: '/hooks',
    icon: Webhook,
    emptyLabel: 'Automations bound to agent lifecycle events.',
  },
  mcp: {
    label: 'MCP Servers',
    route: '/mcp',
    icon: Server,
    emptyLabel: 'Model Context Protocol integrations.',
  },
};

const kindOrder: InventoryKind[] = ['skill', 'agent', 'command', 'hook', 'mcp'];
const runtimeFilterLabels: Record<RuntimeFilter, string> = {
  all: 'All runtimes',
  'claude-code': 'Claude Code',
  codex: 'Codex',
};
const supportFilters: SupportFilter[] = ['all', 'Native', 'Convertible', 'Partial', 'Unsupported'];

function normalizeInventoryPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function pathIncludesSkillDir(path: string, dirName: string): boolean {
  return normalizeInventoryPath(path).includes(`/${dirName}/`);
}

function supportForRuntime(
  runtimeSupport: RuntimeSupportItem[],
  runtime: Exclude<RuntimeFilter, 'all'>
): RuntimeSupportLevel | null {
  const match = runtimeSupport.find((entry) => matchesCanonicalRuntime(entry.runtime, runtime));
  return match?.support ?? null;
}

function isProfileToolPath(path: string): boolean {
  const normalized = path.replace(/\\/g, '/').toLowerCase();
  return (
    normalized.includes('/.tars/profiles/') ||
    normalized.includes('/.claude/plugins/marketplaces/tars-profiles/')
  );
}

function isProfileMarketplacePath(path: string): boolean {
  const normalized = path.replace(/\\/g, '/').toLowerCase();
  return (
    normalized.includes('/.claude/plugins/marketplaces/tars-profiles/') ||
    normalized.includes('/.claude/plugins/cache/tars-profiles/')
  );
}

function scopeLabelFromValue(scope: { type: string } | string, path?: string): string {
  if (path && isProfileToolPath(path)) {
    return 'Bundle';
  }

  if (typeof scope === 'string') {
    switch (scope) {
      case 'user':
        return 'User';
      case 'project':
        return 'Project';
      case 'local':
        return 'Local';
      case 'profile':
        return 'Bundle';
      case 'managed':
        return 'Managed';
      default:
        return scope;
    }
  }

  switch (scope.type) {
    case 'User':
      return 'User';
    case 'Project':
      return 'Project';
    case 'Local':
      return 'Local';
    case 'Managed':
      return 'Managed';
    case 'Plugin':
      return path && isProfileToolPath(path) ? 'Bundle' : 'Plugin';
    default:
      return scope.type;
  }
}

function pluginIdFromScope(scope: unknown): string | null {
  if (!scope || typeof scope !== 'object' || !('type' in scope)) {
    return null;
  }

  const typedScope = scope as { type?: string; plugin_id?: string };
  return typedScope.type === 'Plugin' ? (typedScope.plugin_id ?? null) : null;
}

function buildSkillItems(skills: SkillInfo[]): InventoryItem[] {
  return skills
    .filter((skill) => !(skill.scope.type === 'Plugin' && isProfileMarketplacePath(skill.path)))
    .map((skill) => ({
      id: `skill-${skill.path}`,
      name: skill.name,
      kind: 'skill',
      description: skill.description || null,
      details: skill.user_invocable ? 'Invocable directly in chat' : null,
      path: skill.path,
      scopeLabel: scopeLabelFromValue(skill.scope, skill.path),
      containerLabel: null,
      sourceRuntime: 'claude-code' as const,
      runtimeSupport: skill.runtime_support?.length
        ? toRuntimeSupportItems(skill.runtime_support)
        : getRuntimeSupportForKind('skill'),
      pluginLabel: pluginIdFromScope(skill.scope),
    }));
}

function buildCodexSkillItems(skills: SkillInfo[]): InventoryItem[] {
  return skills.map((skill) => ({
    id: `skill-${skill.path}`,
    name: skill.name,
    kind: 'skill',
    description: skill.description || null,
    details: skill.user_invocable ? 'Invocable directly in chat' : null,
    path: skill.path,
    scopeLabel: scopeLabelFromValue(skill.scope, skill.path),
    containerLabel: null,
    sourceRuntime: 'codex' as const,
    runtimeSupport: skill.runtime_support?.length
      ? toRuntimeSupportItems(skill.runtime_support)
      : getRuntimeSupportForKind('skill'),
    pluginLabel: pluginIdFromScope(skill.scope),
  }));
}

function buildAgentItems(agents: AgentInfo[]): InventoryItem[] {
  return agents
    .filter((agent) => !(agent.scope.type === 'Plugin' && isProfileMarketplacePath(agent.path)))
    .map((agent) => ({
      id: `agent-${agent.path}`,
      name: agent.name,
      kind: 'agent',
      description: agent.description || null,
      details: agent.model || null,
      path: agent.path,
      scopeLabel: scopeLabelFromValue(agent.scope, agent.path),
      containerLabel: null,
      sourceRuntime: 'claude-code' as const,
      runtimeSupport: agent.runtime_support?.length
        ? toRuntimeSupportItems(agent.runtime_support)
        : getRuntimeSupportForKind('agent'),
      pluginLabel: pluginIdFromScope(agent.scope),
    }));
}

function buildCodexAgentItems(agents: CodexAgentInfo[]): InventoryItem[] {
  return agents.map((agent) => ({
    id: `codex-agent-${agent.path}`,
    name: agent.name,
    kind: 'agent',
    description: agent.description || null,
    details: 'Codex custom agent',
    path: agent.path,
    scopeLabel: scopeLabelFromValue(agent.scope, agent.path),
    containerLabel: null,
    sourceRuntime: 'codex' as const,
    runtimeSupport: agent.runtime_support?.length
      ? toRuntimeSupportItems(agent.runtime_support)
      : getRuntimeSupportForKind('agent'),
    pluginLabel: pluginIdFromScope(agent.scope),
  }));
}

function buildCommandItems(commands: CommandInfo[]): InventoryItem[] {
  return commands
    .filter(
      (command) => !(command.scope.type === 'Plugin' && isProfileMarketplacePath(command.path))
    )
    .map((command) => ({
      id: `command-${command.path}`,
      name: `/${command.name}`,
      kind: 'command',
      description: command.description || null,
      details: command.thinking ? 'Extended reasoning enabled' : null,
      path: command.path,
      scopeLabel: scopeLabelFromValue(command.scope, command.path),
      containerLabel: null,
      sourceRuntime: 'claude-code' as const,
      runtimeSupport: command.runtime_support?.length
        ? toRuntimeSupportItems(command.runtime_support)
        : getRuntimeSupportForKind('command'),
      pluginLabel: pluginIdFromScope(command.scope),
    }));
}

function flattenHookConfig(
  config: SettingsHooksConfig,
  scopeLabel: string,
  containerLabel: string | null
): InventoryItem[] {
  return config.events.flatMap((event) =>
    event.matchers.flatMap((matcher, matcherIndex) =>
      matcher.hooks.map((hook, hookIndex) => ({
        id: `hook-${scopeLabel}-${event.event}-${matcherIndex}-${hookIndex}-${config.path}`,
        name: event.event,
        kind: 'hook' as const,
        description: hook.type === 'command' ? hook.command || null : hook.prompt || null,
        details: matcher.matcher === '*' ? hook.type : `${hook.type} • ${matcher.matcher}`,
        path: config.path,
        scopeLabel,
        containerLabel,
        sourceRuntime: 'claude-code',
        runtimeSupport: getRuntimeSupportForKind('hook'),
        pluginLabel: null,
      }))
    )
  );
}

function matchesCanonicalRuntime(runtime: string, runtimeFilter: RuntimeFilter): boolean {
  return (
    (runtimeFilter === 'claude-code' &&
      (runtime === 'ClaudeCode' || runtime === 'Claude Code' || runtime === 'claude-code')) ||
    (runtimeFilter === 'codex' && runtime === 'Codex')
  );
}

function runtimeMatches(item: InventoryItem, runtimeFilter: RuntimeFilter): boolean {
  if (runtimeFilter === 'all') return true;
  return item.sourceRuntime === runtimeFilter;
}

function supportMatches(
  item: InventoryItem,
  runtimeFilter: RuntimeFilter,
  support: SupportFilter
): boolean {
  if (support === 'all') return true;

  const relevantSupport =
    runtimeFilter === 'all'
      ? item.runtimeSupport
      : item.runtimeSupport.filter((entry) =>
          matchesCanonicalRuntime(entry.runtime, runtimeFilter)
        );

  return relevantSupport.some((entry) => entry.support === support);
}

export function InventoryPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedKind, setSelectedKind] = useState<InventoryKind | 'all'>('all');
  const [runtimeFilter, setRuntimeFilter] = useState<RuntimeFilter>('all');
  const [supportFilter, setSupportFilter] = useState<SupportFilter>('all');
  const [itemToDelete, setItemToDelete] = useState<InventoryDeleteTarget | null>(null);
  const [deleting, setDeleting] = useState(false);
  const deferredSearch = useDeferredValue(searchQuery);

  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const { data: profiles = [] } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  const {
    data: inventory,
    isLoading: isLoadingUserScope,
    refetch: refetchUserScope,
  } = useQuery({
    queryKey: ['user-scope'],
    queryFn: scanUserScope,
  });

  const {
    data: projectsInventory,
    isLoading: isLoadingProjects,
    refetch: refetchProjects,
  } = useQuery({
    queryKey: ['projects-scan', projects.map((project) => project.path)],
    queryFn: () =>
      projects.length > 0
        ? scanProjects(projects.map((project) => project.path))
        : Promise.resolve(null),
    enabled: projects.length > 0,
  });

  const {
    data: profilesInventory,
    isLoading: isLoadingProfiles,
    refetch: refetchProfiles,
  } = useQuery({
    queryKey: ['profiles-scan'],
    queryFn: scanProfiles,
  });

  const {
    data: hooksData,
    isLoading: isLoadingHooks,
    refetch: refetchHooks,
  } = useQuery({
    queryKey: [
      'inventory-hooks',
      projects.map((project) => project.path),
      profiles.map((profile) => profile.id),
    ],
    queryFn: async () => {
      const [userHooks, projectHooks, profileHooks] = await Promise.all([
        getUserHooks(),
        Promise.all(
          projects.map(async (project) => ({
            project,
            config: await getProjectHooks(project.path),
          }))
        ),
        Promise.all(
          profiles.map(async (profile) => ({
            profile,
            config: await getProfileHooks(profile.id),
          }))
        ),
      ]);

      return { userHooks, projectHooks, profileHooks };
    },
  });

  const { data: codexSkillBridges = [] } = useQuery({
    queryKey: ['codex-skill-bridges'],
    queryFn: listCodexSkillBridges,
  });

  const { data: codexPluginBridges = [] } = useQuery({
    queryKey: ['codex-plugin-bridges'],
    queryFn: listCodexPluginBridges,
  });

  const {
    data: profileMcpServers = [],
    isLoading: isLoadingProfileMcp,
    refetch: refetchProfileMcp,
  } = useQuery({
    queryKey: ['profile-mcp-servers'],
    queryFn: listProfileMcpServers,
  });

  const isLoading =
    isLoadingUserScope ||
    isLoadingProjects ||
    isLoadingProfiles ||
    isLoadingHooks ||
    isLoadingProfileMcp;

  const allItems = useMemo(() => {
    const rows: InventoryItem[] = [];
    const baseInventory: Inventory | null = inventory ?? null;
    const projectInventory = projectsInventory?.projects ?? [];
    const scannedProfiles: ProfileToolInventory | null = profilesInventory ?? null;

    if (baseInventory?.user_scope.skills) {
      rows.push(...buildSkillItems(baseInventory.user_scope.skills));
    }
    if (baseInventory?.user_scope.codex?.skills) {
      rows.push(...buildCodexSkillItems(baseInventory.user_scope.codex.skills));
    }

    if (baseInventory?.user_scope.agents) {
      rows.push(...buildAgentItems(baseInventory.user_scope.agents));
    }
    if (baseInventory?.user_scope.codex?.agents) {
      rows.push(...buildCodexAgentItems(baseInventory.user_scope.codex.agents));
    }

    if (baseInventory?.user_scope.commands) {
      rows.push(...buildCommandItems(baseInventory.user_scope.commands));
    }

    for (const project of projectInventory) {
      rows.push(
        ...buildSkillItems(project.skills).map((item) => ({
          ...item,
          containerLabel: project.name,
        }))
      );
      rows.push(
        ...buildCodexSkillItems(project.codex?.skills || []).map((item) => ({
          ...item,
          containerLabel: project.name,
        }))
      );
      rows.push(
        ...buildAgentItems(project.agents).map((item) => ({
          ...item,
          containerLabel: project.name,
        }))
      );
      rows.push(
        ...buildCodexAgentItems(project.codex?.agents || []).map((item) => ({
          ...item,
          containerLabel: project.name,
        }))
      );
      rows.push(
        ...buildCommandItems(project.commands).map((item) => ({
          ...item,
          containerLabel: project.name,
        }))
      );

      if (project.mcp) {
        rows.push(
          ...project.mcp.servers.map((server) => ({
            id: `mcp-project-${project.path}-${server.name}`,
            name: server.name,
            kind: 'mcp' as const,
            description: `${server.command} ${server.args.join(' ')}`.trim(),
            details: 'Project',
            path: project.mcp!.path,
            scopeLabel: 'Project',
            containerLabel: project.name,
            sourceRuntime: 'claude-code' as const,
            runtimeSupport: server.runtime_support?.length
              ? toRuntimeSupportItems(server.runtime_support)
              : getRuntimeSupportForKind('mcp'),
            pluginLabel: null,
          }))
        );
      }
    }

    if (scannedProfiles?.skills) {
      rows.push(...buildSkillItems(scannedProfiles.skills));
    }
    if (scannedProfiles?.agents) {
      rows.push(...buildAgentItems(scannedProfiles.agents));
    }
    if (scannedProfiles?.commands) {
      rows.push(...buildCommandItems(scannedProfiles.commands));
    }

    if (baseInventory?.user_scope.mcp) {
      rows.push(
        ...baseInventory.user_scope.mcp.servers.map((server) => ({
          id: `mcp-user-${server.name}`,
          name: server.name,
          kind: 'mcp' as const,
          description: `${server.command} ${server.args.join(' ')}`.trim(),
          details: 'User',
          path: baseInventory.user_scope.mcp!.path,
          scopeLabel: 'User',
          containerLabel: null,
          sourceRuntime: 'claude-code' as const,
          runtimeSupport: server.runtime_support?.length
            ? toRuntimeSupportItems(server.runtime_support)
            : getRuntimeSupportForKind('mcp'),
          pluginLabel: null,
        }))
      );
    }

    if (baseInventory?.managed_scope?.mcp) {
      rows.push(
        ...baseInventory.managed_scope.mcp.servers.map((server) => ({
          id: `mcp-managed-${server.name}`,
          name: server.name,
          kind: 'mcp' as const,
          description: `${server.command} ${server.args.join(' ')}`.trim(),
          details: 'Managed',
          path: baseInventory.managed_scope!.mcp!.path,
          scopeLabel: 'Managed',
          containerLabel: null,
          sourceRuntime: 'claude-code' as const,
          runtimeSupport: server.runtime_support?.length
            ? toRuntimeSupportItems(server.runtime_support)
            : getRuntimeSupportForKind('mcp'),
          pluginLabel: null,
        }))
      );
    }

    rows.push(
      ...profileMcpServers.map((server: ProfileMcpServer) => ({
        id: `mcp-profile-${server.profileId}-${server.name}`,
        name: server.name,
        kind: 'mcp' as const,
        description: server.url || `${server.command || ''} ${server.args.join(' ')}`.trim(),
        details: server.transport,
        path: server.filePath,
        scopeLabel: 'Bundle',
        containerLabel: server.profileName,
        sourceRuntime: 'claude-code' as const,
        runtimeSupport: getRuntimeSupportForKind('mcp'),
        pluginLabel: null,
      }))
    );

    if (hooksData) {
      rows.push(...flattenHookConfig(hooksData.userHooks, 'User', null));
      rows.push(
        ...hooksData.projectHooks.flatMap(({ project, config }) =>
          flattenHookConfig(config, 'Project', project.name)
        )
      );
      rows.push(
        ...hooksData.profileHooks.flatMap(({ profile, config }) =>
          flattenHookConfig(config, 'Bundle', profile.name)
        )
      );
    }

    return rows.sort((a, b) => {
      const kindCompare = kindOrder.indexOf(a.kind) - kindOrder.indexOf(b.kind);
      if (kindCompare !== 0) return kindCompare;
      return a.name.localeCompare(b.name);
    });
  }, [hooksData, inventory, profileMcpServers, profilesInventory, projectsInventory]);

  const filteredItems = useMemo(() => {
    const normalizedSearch = deferredSearch.trim().toLowerCase();

    return allItems.filter((item) => {
      if (selectedKind !== 'all' && item.kind !== selectedKind) return false;
      if (!runtimeMatches(item, runtimeFilter)) return false;
      if (!supportMatches(item, runtimeFilter, supportFilter)) return false;

      if (!normalizedSearch) return true;

      const haystack = [
        item.name,
        item.description,
        item.details,
        item.path,
        item.scopeLabel,
        item.containerLabel,
        kindMeta[item.kind].label,
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();

      return haystack.includes(normalizedSearch);
    });
  }, [allItems, deferredSearch, runtimeFilter, selectedKind, supportFilter]);

  const groupedItems = useMemo(
    () =>
      kindOrder.map((kind) => ({
        kind,
        items: filteredItems.filter((item) => item.kind === kind),
      })),
    [filteredItems]
  );

  const countsByKind = useMemo(
    () =>
      kindOrder.reduce(
        (acc, kind) => {
          acc[kind] = allItems.filter((item) => item.kind === kind).length;
          return acc;
        },
        {} as Record<InventoryKind, number>
      ),
    [allItems]
  );

  async function handleRefresh() {
    await Promise.all([
      refetchUserScope(),
      refetchProjects(),
      refetchProfiles(),
      refetchHooks(),
      refetchProfileMcp(),
    ]);
  }

  function findCodexSkillBridge(item: InventoryItem) {
    if (item.kind !== 'skill' || item.sourceRuntime !== 'codex') {
      return null;
    }

    return (
      codexSkillBridges.find((bridge) => pathIncludesSkillDir(item.path, bridge.codex_dir_name)) ??
      null
    );
  }

  function findCodexPluginBridge(item: InventoryItem) {
    if (item.kind !== 'skill' || item.sourceRuntime !== 'codex') {
      return null;
    }

    return (
      codexPluginBridges.find((bridge) =>
        bridge.codex_skill_dirs.some((dirName) => pathIncludesSkillDir(item.path, dirName))
      ) ?? null
    );
  }

  function getDeleteTarget(item: InventoryItem): InventoryDeleteTarget | null {
    const pluginBridge = findCodexPluginBridge(item);

    if (pluginBridge || item.scopeLabel === 'Managed' || item.scopeLabel === 'Plugin') {
      return null;
    }

    if (item.kind === 'skill') {
      return {
        name: item.name,
        path: item.path,
        kind: item.kind,
        mode: item.sourceRuntime === 'codex' ? 'codex-skill' : 'skill',
      };
    }

    if (item.sourceRuntime !== 'claude-code') {
      return null;
    }

    if (item.kind === 'command' || item.kind === 'agent') {
      return {
        name: item.name,
        path: item.path,
        kind: item.kind,
        mode: item.kind,
      };
    }

    return null;
  }

  async function handleDeleteItem() {
    if (!itemToDelete) return;

    setDeleting(true);
    try {
      switch (itemToDelete.mode) {
        case 'skill':
          await deleteSkill(itemToDelete.path);
          break;
        case 'command':
          await deleteCommand(itemToDelete.path);
          break;
        case 'agent':
          await deleteAgent(itemToDelete.path);
          break;
        case 'codex-skill':
          await deleteCodexSkill(itemToDelete.path);
          break;
      }

      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['codex-skill-bridges'] }),
        queryClient.invalidateQueries({ queryKey: ['codex-plugin-bridges'] }),
      ]);
      await handleRefresh();

      toast.success(`Deleted ${itemToDelete.kind} "${itemToDelete.name}"`);
      setItemToDelete(null);
    } catch (err) {
      toast.error(`Failed to delete ${itemToDelete.kind}`, {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDeleting(false);
    }
  }

  return (
    <div className="h-full flex flex-col">
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Inventory</h2>
        </div>
        <Button variant="ghost" size="sm" onClick={handleRefresh} disabled={isLoading}>
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
          Refresh
        </Button>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        <section className="rounded-md border border-border bg-muted/20 p-5">
          <div className="space-y-4">
            <div className="max-w-3xl">
              <div className="flex flex-wrap items-center gap-2 mb-2">
                <Badge variant="outline">Unified browser</Badge>
                <Badge variant="secondary">Runtime-aware</Badge>
                <Badge variant="outline">Claude + Codex</Badge>
              </div>
              <p className="text-sm text-muted-foreground">
                Inventory brings skills, agents, commands, hooks, and MCP servers into one place,
                while runtime badges show what is native, portable, or unsupported at a glance.
              </p>
            </div>
            <div className="flex flex-wrap gap-3">
              {kindOrder.map((kind) => {
                const Icon = kindMeta[kind].icon;
                return (
                  <div
                    key={kind}
                    className="min-w-[10rem] rounded-md border border-border/70 bg-card/60 px-3 py-2"
                  >
                    <div className="flex items-center gap-2 text-sm font-medium">
                      <Icon className="h-4 w-4 text-primary" />
                      {kindMeta[kind].label}
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                      {countsByKind[kind] || 0} items
                    </p>
                  </div>
                );
              })}
            </div>
          </div>
        </section>

        <section className="rounded-md border border-border bg-card/70 p-4 space-y-4">
          <div className="grid gap-3 lg:grid-cols-[minmax(0,1.3fr)_auto] lg:items-center">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="Search names, descriptions, paths, scopes..."
                className="pl-9"
              />
            </div>
            <div className="flex flex-wrap gap-2">
              {(['all', 'claude-code', 'codex'] as RuntimeFilter[]).map((filterValue) => (
                <Button
                  key={filterValue}
                  variant={runtimeFilter === filterValue ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setRuntimeFilter(filterValue)}
                >
                  {runtimeFilterLabels[filterValue]}
                </Button>
              ))}
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              variant={selectedKind === 'all' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setSelectedKind('all')}
            >
              All kinds
            </Button>
            {kindOrder.map((kind) => {
              const Icon = kindMeta[kind].icon;
              return (
                <Button
                  key={kind}
                  variant={selectedKind === kind ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setSelectedKind(kind)}
                >
                  <Icon className="h-4 w-4" />
                  {kindMeta[kind].label}
                </Button>
              );
            })}
          </div>

          <div className="flex flex-wrap gap-2">
            {supportFilters.map((filterValue) => (
              <Button
                key={filterValue}
                variant={supportFilter === filterValue ? 'secondary' : 'ghost'}
                size="sm"
                onClick={() => setSupportFilter(filterValue)}
              >
                {filterValue === 'all' ? 'All support states' : filterValue}
              </Button>
            ))}
          </div>
        </section>

        {isLoading ? (
          <div className="flex h-48 items-center justify-center text-muted-foreground">
            <RefreshCw className="h-5 w-5 animate-spin mr-2" />
            Scanning inventory...
          </div>
        ) : filteredItems.length === 0 ? (
          <div className="rounded-md border border-border bg-card/70 p-10 text-center">
            <Boxes className="h-10 w-10 text-muted-foreground/50 mx-auto mb-3" />
            <p className="font-medium">No inventory items match these filters</p>
            <p className="text-sm text-muted-foreground mt-1">
              Try a broader runtime or support filter, or clear the search box.
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {groupedItems.map(({ kind, items }) => {
              if (items.length === 0) return null;

              const Icon = kindMeta[kind].icon;

              return (
                <section
                  key={kind}
                  className="rounded-md border border-border bg-card/70 overflow-hidden"
                >
                  <div className="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
                    <div className="flex items-center gap-3">
                      <Icon className="h-4 w-4 text-primary" />
                      <div>
                        <p className="font-medium">{kindMeta[kind].label}</p>
                        <p className="text-xs text-muted-foreground">{kindMeta[kind].emptyLabel}</p>
                      </div>
                      <Badge variant="outline">{items.length}</Badge>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() =>
                        navigate(kindMeta[kind].route, {
                          state: {
                            returnTo: '/inventory',
                            returnLabel: 'Back to Inventory',
                          },
                        })
                      }
                    >
                      Open editor
                    </Button>
                  </div>
                  <div className="divide-y divide-border">
                    {items.map((item) => (
                      <div key={item.id} className="px-4 py-3">
                        {(() => {
                          const localCodexBridge = findCodexSkillBridge(item);
                          const pluginCodexBridge = findCodexPluginBridge(item);
                          const deleteTarget = getDeleteTarget(item);
                          const codexSupport = supportForRuntime(item.runtimeSupport, 'codex');
                          const claudeSupport = supportForRuntime(
                            item.runtimeSupport,
                            'claude-code'
                          );
                          const pluginLabel = pluginCodexBridge?.plugin_name ?? item.pluginLabel;
                          const bridgePathLabel = localCodexBridge
                            ? `Bridged from ${localCodexBridge.source_path}`
                            : null;

                          return (
                            <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                              <div className="min-w-0 flex-1">
                                <div className="flex flex-wrap items-center gap-2">
                                  <p className="font-medium">{item.name}</p>
                                  <Badge variant="outline">{item.scopeLabel}</Badge>
                                  {item.containerLabel && (
                                    <Badge variant="secondary">{item.containerLabel}</Badge>
                                  )}
                                  {pluginLabel && <Badge variant="secondary">{pluginLabel}</Badge>}
                                  {pluginCodexBridge && !pluginLabel && (
                                    <Badge variant="secondary">Plugin managed</Badge>
                                  )}
                                  {localCodexBridge && <Badge variant="secondary">Bridged</Badge>}
                                  {item.kind === 'command' &&
                                    item.sourceRuntime === 'claude-code' &&
                                    codexSupport === 'Unsupported' && (
                                      <Badge
                                        variant="outline"
                                        className="text-amber-700 border-amber-300 bg-amber-50"
                                      >
                                        Codex incompatible
                                      </Badge>
                                    )}
                                  {item.kind === 'command' &&
                                    item.sourceRuntime === 'claude-code' &&
                                    codexSupport === 'Partial' && (
                                      <Badge
                                        variant="outline"
                                        className="text-amber-700 border-amber-300 bg-amber-50"
                                      >
                                        Codex partial
                                      </Badge>
                                    )}
                                  {item.kind === 'skill' &&
                                    item.sourceRuntime === 'codex' &&
                                    claudeSupport === 'Unsupported' && (
                                      <Badge variant="outline" className="text-slate-600">
                                        Claude only
                                      </Badge>
                                    )}
                                </div>
                                {item.description && (
                                  <p className="mt-2 text-sm text-muted-foreground break-words">
                                    {item.description}
                                  </p>
                                )}
                                {item.details && (
                                  <p className="mt-1 text-xs text-muted-foreground">
                                    {item.details}
                                  </p>
                                )}
                                <p className="mt-2 text-xs text-muted-foreground/80 break-all">
                                  {bridgePathLabel ?? item.path}
                                </p>
                              </div>
                              {deleteTarget && (
                                <div className="shrink-0">
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    className="justify-start text-destructive hover:text-destructive"
                                    onClick={() => setItemToDelete(deleteTarget)}
                                  >
                                    <Trash2 className="h-4 w-4" />
                                    Delete
                                  </Button>
                                </div>
                              )}
                            </div>
                          );
                        })()}
                      </div>
                    ))}
                  </div>
                </section>
              );
            })}
          </div>
        )}
      </div>
      <ConfirmDialog
        open={!!itemToDelete}
        onOpenChange={(open) => {
          if (!open) {
            setItemToDelete(null);
          }
        }}
        title={`Delete ${itemToDelete?.kind ?? 'item'}?`}
        description={
          itemToDelete
            ? `Delete "${itemToDelete.name}" from ${itemToDelete.mode === 'codex-skill' ? 'Codex' : 'its current scope'}? This cannot be undone.`
            : ''
        }
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={handleDeleteItem}
        loading={deleting}
      />
    </div>
  );
}
