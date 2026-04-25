import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Bot,
  Box,
  Boxes,
  Check,
  ChevronDown,
  ChevronRight,
  Code2,
  Download,
  ExternalLink,
  FolderOpen,
  HardDrive,
  Package,
  Plus,
  RefreshCw,
  Search,
  Store,
  Terminal,
  Trash2,
  AlertTriangle,
  Power,
  PowerOff,
} from 'lucide-react';
import { HelpButton } from '../components/HelpButton';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import {
  scanProjects,
  scanUserScope,
  listProfiles,
  listProjects,
  addPluginToTargets,
  bridgeClaudePluginToCodex,
  deleteProfileCleanup,
  installPlugin as installPluginByKey,
  listCodexPluginBridges,
  listPluginSubscriptions,
  removePluginSubscription,
  syncCodexPluginBridges,
  syncPluginSubscription,
  trackPluginVersions,
  updatePlugin as updatePluginByKey,
} from '../lib/ipc';
import type {
  AvailablePlugin,
  CacheStatusResponse,
  CodexAvailablePlugin,
  CodexMarketplace,
  Marketplace,
  PluginInventory,
  PluginSkillInfo,
} from '../lib/types';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';

type RuntimeFilter = 'all' | 'claude-code' | 'codex';

type CodexMarketplaceSurface = {
  label: string;
  path: string;
  status: string;
  variant: 'default' | 'secondary' | 'outline';
};

type CodexMarketplaceRow = CodexMarketplace & {
  scopeLabel: string;
  projectName?: string;
};

type CodexPluginRow = CodexAvailablePlugin & {
  marketplaceName: string;
  scopeLabel: string;
  projectName?: string;
};

export function PluginsPage() {
  const queryClient = useQueryClient();
  const [runtimeFilter, setRuntimeFilter] = useState<RuntimeFilter>('all');
  const [showAddPluginDialog, setShowAddPluginDialog] = useState(false);
  const [pluginSourceKind, setPluginSourceKind] = useState<'direct' | 'marketplace'>('direct');
  const [pluginSource, setPluginSource] = useState('');
  const [marketplacePluginName, setMarketplacePluginName] = useState('');
  const [pluginMarketplaceSource, setPluginMarketplaceSource] = useState('');
  const [pluginMarketplaceName, setPluginMarketplaceName] = useState('');
  const [pluginCodexSource, setPluginCodexSource] = useState('');
  const [addingPluginSource, setAddingPluginSource] = useState(false);
  const [pluginTargets, setPluginTargets] = useState<Array<'claude-code' | 'codex'>>([
    'claude-code',
    'codex',
  ]);
  const [showAddMarketplace, setShowAddMarketplace] = useState(false);
  const [marketplaceSource, setMarketplaceSource] = useState('');
  const [addingMarketplace, setAddingMarketplace] = useState(false);
  const [marketplaceToRemove, setMarketplaceToRemove] = useState<string | null>(null);
  const [removingMarketplace, setRemovingMarketplace] = useState(false);
  const [alsoUninstallPlugins, setAlsoUninstallPlugins] = useState(false);
  const [updatingMarketplaces, setUpdatingMarketplaces] = useState(false);
  const [updatingPlugin, setUpdatingPlugin] = useState<string | null>(null);
  const [bridgingPluginKey, setBridgingPluginKey] = useState<string | null>(null);
  const [syncingSubscriptionId, setSyncingSubscriptionId] = useState<number | null>(null);
  const [removingSubscriptionId, setRemovingSubscriptionId] = useState<number | null>(null);
  const [selectedMarketplace, setSelectedMarketplace] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [installingPlugin, setInstallingPlugin] = useState<string | null>(null);
  const [availablePluginsExpanded, setAvailablePluginsExpanded] = useState(false);

  // Install dialog state
  const [showInstallDialog, setShowInstallDialog] = useState(false);
  const [pendingInstall, setPendingInstall] = useState<{
    pluginId: string;
    marketplace: string;
  } | null>(null);
  const [installScope, setInstallScope] = useState<'user' | 'project' | 'local'>('user');
  const [selectedProjects, setSelectedProjects] = useState<string[]>([]);

  // Cache cleanup state
  const [cleaningCache, setCleaningCache] = useState(false);
  const [showCacheDetails, setShowCacheDetails] = useState(false);

  // Skills dialog state
  const [skillsDialogPlugin, setSkillsDialogPlugin] = useState<{
    name: string;
    skills: PluginSkillInfo[];
  } | null>(null);

  const {
    data: inventory,
    isLoading,
    refetch,
  } = useQuery({
    queryKey: ['user-scope'],
    queryFn: scanUserScope,
  });

  // Get configured projects for project picker
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const projectPaths = useMemo(() => projects.map((project) => project.path), [projects]);
  const { data: projectsInventory } = useQuery({
    queryKey: ['plugins-codex-project-scan', projectPaths],
    queryFn: () => scanProjects(projectPaths),
    enabled: projectPaths.length > 0,
  });

  const { data: profiles = [] } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  const { data: pluginSubscriptions = [] } = useQuery({
    queryKey: ['plugin-subscriptions'],
    queryFn: listPluginSubscriptions,
  });

  const { data: codexPluginBridges = [] } = useQuery({
    queryKey: ['codex-plugin-bridges'],
    queryFn: listCodexPluginBridges,
  });

  // Get cache status
  const { data: cacheStatus, refetch: refetchCache } = useQuery({
    queryKey: ['cache-status'],
    queryFn: () => invoke<CacheStatusResponse>('cache_status'),
  });

  const plugins: PluginInventory = inventory?.plugins || { marketplaces: [], installed: [] };

  const codexMarketplaces = useMemo(() => {
    const marketplaces: CodexMarketplaceRow[] = [];

    for (const marketplace of inventory?.user_scope.codex.marketplaces || []) {
      marketplaces.push({
        ...marketplace,
        scopeLabel: 'user',
      });
    }

    for (const project of projectsInventory?.projects || []) {
      for (const marketplace of project.codex.marketplaces || []) {
        marketplaces.push({
          ...marketplace,
          scopeLabel: 'project',
          projectName: project.name,
        });
      }
    }

    return marketplaces.sort((a, b) => {
      const scopeCompare = a.scopeLabel.localeCompare(b.scopeLabel);
      if (scopeCompare !== 0) return scopeCompare;
      const projectCompare = (a.projectName || '').localeCompare(b.projectName || '');
      if (projectCompare !== 0) return projectCompare;
      return a.name.localeCompare(b.name);
    });
  }, [inventory, projectsInventory]);

  const codexPlugins = useMemo(() => {
    const rows: CodexPluginRow[] = [];

    for (const marketplace of codexMarketplaces) {
      for (const plugin of marketplace.plugins) {
        rows.push({
          ...plugin,
          marketplaceName: marketplace.display_name || marketplace.name,
          scopeLabel: marketplace.scopeLabel,
          projectName: marketplace.projectName,
        });
      }
    }

    return rows.sort((a, b) => {
      const nameCompare = a.id.localeCompare(b.id);
      if (nameCompare !== 0) return nameCompare;
      return a.marketplaceName.localeCompare(b.marketplaceName);
    });
  }, [codexMarketplaces]);

  const userCodexMarketplaceCount = inventory?.user_scope.codex.marketplaces.length || 0;
  const projectCodexMarketplaceCount = (projectsInventory?.projects || []).reduce(
    (total, project) => total + project.codex.marketplaces.length,
    0
  );
  const resolvedCodexManifestCount = codexPlugins.filter((plugin) => plugin.manifest_path).length;

  const codexMarketplaceSurfaces: CodexMarketplaceSurface[] = [
    {
      label: 'User marketplace',
      path: '~/.agents/plugins/marketplace.json',
      status: userCodexMarketplaceCount > 0 ? 'Discovered' : 'Not found',
      variant: userCodexMarketplaceCount > 0 ? 'default' : 'secondary',
    },
    {
      label: 'Project marketplace',
      path: '<repo>/.agents/plugins/marketplace.json',
      status: projectCodexMarketplaceCount > 0 ? 'Discovered' : 'Not found',
      variant: projectCodexMarketplaceCount > 0 ? 'default' : 'secondary',
    },
    {
      label: 'Plugin manifest',
      path: '.codex-plugin/plugin.json',
      status:
        resolvedCodexManifestCount > 0
          ? 'Resolved'
          : codexPlugins.length > 0
            ? 'Referenced only'
            : 'Not found',
      variant:
        resolvedCodexManifestCount > 0
          ? 'default'
          : codexPlugins.length > 0
            ? 'outline'
            : 'secondary',
    },
  ];

  // Track plugin versions to get accurate "version changed at" times
  const { data: versionTracking = {} } = useQuery({
    queryKey: [
      'plugin-versions',
      plugins.installed.map((p) => `${p.id}@${p.marketplace}:${p.version}`),
    ],
    queryFn: async () => {
      if (plugins.installed.length === 0) return {};
      const pluginData: [string, string][] = plugins.installed.map((p) => [
        `${p.id}@${p.marketplace}`,
        p.version || 'unknown',
      ]);
      return trackPluginVersions(pluginData);
    },
    enabled: plugins.installed.length > 0,
  });
  const PROFILE_MARKETPLACE = 'tars-profiles';

  // Sort marketplaces by name for stable ordering
  const sortedMarketplaces = [...plugins.marketplaces].sort((a, b) => a.name.localeCompare(b.name));

  // Sort installed plugins by name, then marketplace, then project for stable ordering
  const sortedInstalledPlugins = [...plugins.installed].sort((a, b) => {
    const nameCompare = a.id.localeCompare(b.id);
    if (nameCompare !== 0) return nameCompare;
    const marketplaceCompare = (a.marketplace || '').localeCompare(b.marketplace || '');
    if (marketplaceCompare !== 0) return marketplaceCompare;
    return (a.project_path || '').localeCompare(b.project_path || '');
  });

  const isProfileMarketplace = marketplaceToRemove === PROFILE_MARKETPLACE;
  const profileSummary = useMemo(() => {
    const totalProfiles = profiles.length;
    const totalTools = profiles.reduce((sum, profile) => sum + (profile.tool_count || 0), 0);
    return { totalProfiles, totalTools };
  }, [profiles]);

  const runtimeOptions = [
    {
      id: 'all' as const,
      label: 'All',
      icon: Boxes,
      count: sortedInstalledPlugins.length + codexPlugins.length,
    },
    {
      id: 'claude-code' as const,
      label: 'Claude Code',
      icon: Bot,
      count: sortedInstalledPlugins.length,
    },
    {
      id: 'codex' as const,
      label: 'Codex',
      icon: Code2,
      count: codexPlugins.length,
    },
  ];

  const showClaudeMarketplace = runtimeFilter !== 'codex';
  const showCodexPreview = runtimeFilter !== 'claude-code';

  const availableByMarketplace = useMemo(() => {
    const map = new Map<string, Map<string, AvailablePlugin>>();
    for (const marketplace of sortedMarketplaces) {
      const pluginMap = new Map<string, AvailablePlugin>();
      for (const plugin of marketplace.available_plugins || []) {
        pluginMap.set(plugin.id, plugin);
      }
      map.set(marketplace.name, pluginMap);
    }
    return map;
  }, [sortedMarketplaces]);

  const installedClaudePluginIds = useMemo(
    () => new Set(sortedInstalledPlugins.map((plugin) => plugin.id)),
    [sortedInstalledPlugins]
  );
  const codexBridgeKeyForPlugin = (plugin: (typeof sortedInstalledPlugins)[number]) =>
    `${plugin.id}|${plugin.marketplace || '-'}|${plugin.scope.type.toLowerCase()}|${plugin.project_path || '-'}`;
  const bridgedCodexPluginKeys = useMemo(
    () => new Set(codexPluginBridges.map((bridge) => bridge.key)),
    [codexPluginBridges]
  );

  const registeredCodexPluginIds = useMemo(
    () => new Set(codexPlugins.map((plugin) => plugin.id)),
    [codexPlugins]
  );

  function togglePluginTarget(target: 'claude-code' | 'codex') {
    setPluginTargets((current) =>
      current.includes(target) ? current.filter((value) => value !== target) : [...current, target]
    );
  }

  async function handleAddPluginToTargets() {
    const isDirect = pluginSourceKind === 'direct';
    const primaryValue = isDirect ? pluginSource.trim() : pluginMarketplaceSource.trim();
    if (!primaryValue) return;
    if (pluginTargets.length === 0) {
      toast.error('Choose at least one runtime target');
      return;
    }

    setAddingPluginSource(true);
    try {
      const result = await addPluginToTargets(
        pluginSourceKind,
        isDirect ? pluginSource.trim() : '',
        isDirect ? null : marketplacePluginName.trim() || null,
        isDirect ? null : pluginMarketplaceSource.trim() || null,
        isDirect ? null : pluginMarketplaceName.trim() || null,
        isDirect ? null : pluginCodexSource.trim() || null,
        pluginTargets
      );
      const successes: string[] = [];
      const failures: string[] = [];

      if (result.claude) {
        if (result.claude.success) {
          successes.push('Claude Code installed');
        } else {
          failures.push(`Claude Code: ${result.claude.message}`);
        }
      }

      if (result.codex) {
        if (result.codex.success) {
          successes.push('Codex marketplace updated');
        } else {
          failures.push(`Codex: ${result.codex.message}`);
        }
      }

      if (failures.length === 0) {
        toast.success(`Added ${result.plugin_name}`, {
          description: successes.join(' • '),
        });
        setShowAddPluginDialog(false);
        setPluginSource('');
        setMarketplacePluginName('');
        setPluginMarketplaceSource('');
        setPluginMarketplaceName('');
        setPluginCodexSource('');
        setPluginSourceKind('direct');
      } else if (successes.length > 0) {
        toast.warning(`Partially added ${result.plugin_name}`, {
          description: [...successes, ...failures].join(' • '),
        });
      } else {
        toast.error(`Failed to add ${result.plugin_name}`, {
          description: failures.join(' • '),
        });
      }

      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['plugin-subscriptions'] });
    } catch (err) {
      toast.error('Failed to add plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setAddingPluginSource(false);
    }
  }

  async function handleSyncManagedPlugin(subscriptionId: number) {
    setSyncingSubscriptionId(subscriptionId);
    try {
      const result = await syncPluginSubscription(subscriptionId);
      const details = [result.claude, result.codex]
        .filter(Boolean)
        .map((entry) => entry!.message)
        .join(' • ');
      const hasFailure = [result.claude, result.codex].some((entry) => entry && !entry.success);

      if (hasFailure) {
        toast.warning(`Reapplied ${result.plugin_name} with issues`, {
          description: details,
        });
      } else {
        toast.success(`Reapplied ${result.plugin_name}`, {
          description: details,
        });
      }

      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['plugin-subscriptions'] });
    } catch (err) {
      toast.error('Failed to reapply managed plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setSyncingSubscriptionId(null);
    }
  }

  async function handleRemoveManagedPlugin(subscriptionId: number) {
    setRemovingSubscriptionId(subscriptionId);
    try {
      const result = await removePluginSubscription(subscriptionId);
      const details = [result.claude, result.codex]
        .filter(Boolean)
        .map((entry) => entry!.message)
        .join(' • ');

      toast.success(`Removed ${result.plugin_name}`, {
        description: details,
      });

      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['plugin-subscriptions'] });
    } catch (err) {
      toast.error('Failed to remove managed plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setRemovingSubscriptionId(null);
    }
  }

  async function handleAddMarketplace() {
    if (!marketplaceSource.trim()) return;

    setAddingMarketplace(true);
    try {
      await invoke('plugin_marketplace_add', { source: marketplaceSource.trim() });
      toast.success('Marketplace added', {
        description: `Added ${marketplaceSource}`,
      });
      setShowAddMarketplace(false);
      setMarketplaceSource('');
      await refetch();
    } catch (err) {
      toast.error('Failed to add marketplace', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setAddingMarketplace(false);
    }
  }

  // Get plugins installed from a specific marketplace
  function getPluginsFromMarketplace(marketplaceName: string) {
    return sortedInstalledPlugins.filter((p) => p.marketplace === marketplaceName);
  }

  function getPluginInstallId(plugin: (typeof sortedInstalledPlugins)[number]) {
    return plugin.marketplace ? `${plugin.id}@${plugin.marketplace}` : plugin.id;
  }

  async function updatePlugin(plugin: (typeof sortedInstalledPlugins)[number]) {
    return updatePluginByKey(
      getPluginInstallId(plugin),
      plugin.scope.type.toLowerCase(),
      plugin.project_path ?? undefined
    );
  }

  async function handleRemoveMarketplace() {
    if (!marketplaceToRemove) return;

    setRemovingMarketplace(true);
    try {
      if (isProfileMarketplace && profiles.length > 0) {
        const failedProfiles: string[] = [];
        for (const profile of profiles) {
          try {
            await deleteProfileCleanup(profile.id);
          } catch {
            failedProfiles.push(profile.name);
          }
        }
        if (failedProfiles.length > 0) {
          toast.error('Failed to delete some bundles', {
            description: failedProfiles.slice(0, 5).join(', '),
          });
        }
        queryClient.invalidateQueries({ queryKey: ['profiles'] });
      }

      // If user wants to uninstall plugins too, do that first
      const shouldUninstall = alsoUninstallPlugins || isProfileMarketplace;
      if (shouldUninstall) {
        const pluginsToRemove = getPluginsFromMarketplace(marketplaceToRemove);
        for (const plugin of pluginsToRemove) {
          try {
            await invoke('plugin_uninstall', {
              plugin: `${plugin.id}@${plugin.marketplace}`,
              scope: plugin.scope.type.toLowerCase(),
              projectPath: plugin.project_path ?? undefined,
            });
          } catch {
            // Continue even if individual plugin uninstall fails
          }
        }
      }

      await invoke('plugin_marketplace_remove', { name: marketplaceToRemove });
      toast.success('Marketplace removed', {
        description: shouldUninstall
          ? `Removed ${marketplaceToRemove} and its plugins`
          : `Removed ${marketplaceToRemove}`,
      });
      await refetch();
    } catch (err) {
      toast.error('Failed to remove marketplace', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setRemovingMarketplace(false);
      setMarketplaceToRemove(null);
      setAlsoUninstallPlugins(false);
    }
  }

  async function handleUpdateMarketplaces() {
    setUpdatingMarketplaces(true);
    try {
      await invoke('plugin_marketplace_update', { name: null });
      const failures: string[] = [];

      for (const plugin of sortedInstalledPlugins) {
        try {
          await updatePlugin(plugin);
        } catch (err) {
          console.error('Failed to update plugin:', plugin.id, err);
          failures.push(plugin.id);
        }
      }

      const bridgeSync = await syncCodexPluginBridges();
      const bridgeFailures = bridgeSync.results.filter((result) => !result.success);
      const bridgeSuccesses = bridgeSync.results.filter((result) => result.success);

      if (failures.length > 0) {
        toast.error('Some plugins failed to update', {
          description: failures.slice(0, 5).join(', '),
        });
      } else if (bridgeFailures.length > 0) {
        toast.warning('Plugins updated with Codex bridge issues', {
          description: bridgeFailures
            .slice(0, 3)
            .map((result) => `${result.plugin_name}: ${result.message}`)
            .join(' • '),
        });
      } else {
        toast.success('Marketplaces and plugins updated', {
          description:
            bridgeSuccesses.length > 0
              ? `Synced ${bridgeSuccesses.length} Codex plugin bridge${bridgeSuccesses.length === 1 ? '' : 's'}`
              : undefined,
        });
      }

      // Invalidate and refetch to ensure fresh data
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['codex-plugin-bridges'] });
    } catch (err) {
      toast.error('Failed to update marketplaces', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setUpdatingMarketplaces(false);
    }
  }

  async function handleUpdatePlugin(plugin: (typeof sortedInstalledPlugins)[number], key: string) {
    setUpdatingPlugin(key);
    try {
      await updatePlugin(plugin);
      if (bridgedCodexPluginKeys.has(codexBridgeKeyForPlugin(plugin))) {
        const bridge = await bridgeClaudePluginToCodex(
          plugin.id,
          plugin.marketplace,
          plugin.scope.type.toLowerCase(),
          plugin.project_path
        );
        toast.success(`Updated ${plugin.id}`, {
          description: `Synced ${bridge.skill_count} Codex skill${bridge.skill_count === 1 ? '' : 's'}`,
        });
      } else {
        toast.success(`Updated ${plugin.id}`);
      }
      // Invalidate and refetch to ensure fresh data after update
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['codex-plugin-bridges'] });
    } catch (err) {
      toast.error('Failed to update plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setUpdatingPlugin(null);
    }
  }

  async function handleMakePluginAvailableForCodex(
    plugin: (typeof sortedInstalledPlugins)[number],
    key: string
  ) {
    setBridgingPluginKey(key);
    try {
      const bridge = await bridgeClaudePluginToCodex(
        plugin.id,
        plugin.marketplace,
        plugin.scope.type.toLowerCase(),
        plugin.project_path
      );
      toast.success(`Made ${plugin.id} available for Codex`, {
        description: `${bridge.skill_count} skill${bridge.skill_count === 1 ? '' : 's'} synced. Restart Codex if it is already running.`,
      });
      await queryClient.invalidateQueries({ queryKey: ['user-scope'] });
      await queryClient.invalidateQueries({ queryKey: ['codex-plugin-bridges'] });
    } catch (err) {
      toast.error('Failed to make plugin available for Codex', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setBridgingPluginKey(null);
    }
  }

  async function handleToggleAutoUpdate(marketplaceName: string, currentValue: boolean) {
    try {
      await invoke('plugin_marketplace_set_auto_update', {
        name: marketplaceName,
        autoUpdate: !currentValue,
      });
      toast.success(`Auto-update ${!currentValue ? 'enabled' : 'disabled'}`, {
        description: marketplaceName,
      });
      await refetch();
    } catch (err) {
      toast.error('Failed to toggle auto-update', {
        description: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleUninstallPlugin(
    pluginId: string,
    scope: string,
    projectPath?: string | null
  ) {
    try {
      await invoke('plugin_uninstall', {
        plugin: pluginId,
        scope,
        projectPath: projectPath ?? undefined,
      });
      await syncCodexPluginBridges();
      toast.success(`Uninstalled ${pluginId}`);
      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['codex-plugin-bridges'] });
    } catch (err) {
      toast.error('Failed to uninstall plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleTogglePlugin(
    pluginId: string,
    marketplace: string | null,
    enabled: boolean
  ) {
    // Use full plugin key: plugin@marketplace
    const pluginKey = marketplace ? `${pluginId}@${marketplace}` : pluginId;
    try {
      if (enabled) {
        await invoke('plugin_disable', { plugin: pluginKey });
        toast.success(`Disabled ${pluginId}`);
      } else {
        await invoke('plugin_enable', { plugin: pluginKey });
        toast.success(`Enabled ${pluginId}`);
      }
      await refetch();
    } catch (err) {
      toast.error(`Failed to ${enabled ? 'disable' : 'enable'} plugin`, {
        description: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleCopySkill(skill: PluginSkillInfo) {
    try {
      await navigator.clipboard.writeText(skill.invocation);
      toast.success('Copied to clipboard', {
        description: `Paste "${skill.invocation}" in Claude Code`,
      });
    } catch (err) {
      toast.error('Failed to copy', {
        description: err instanceof Error ? err.message : String(err),
      });
    }
  }

  function handleShowSkills(pluginName: string, skills: PluginSkillInfo[]) {
    setSkillsDialogPlugin({ name: pluginName, skills });
  }

  function handleInstallClick(pluginId: string, marketplace: string) {
    // Show install dialog with options
    setPendingInstall({ pluginId, marketplace });
    setInstallScope('user');
    setSelectedProjects([]);
    setShowInstallDialog(true);
  }

  async function handleConfirmInstall() {
    if (!pendingInstall) return;

    const { pluginId, marketplace } = pendingInstall;
    const pluginSpec = `${pluginId}@${marketplace}`;

    setShowInstallDialog(false);
    setInstallingPlugin(pluginSpec);

    try {
      if (installScope === 'user') {
        // Install to user scope (global)
        await installPluginByKey(pluginSpec, 'user', null);
        toast.success(`Installed ${pluginId}`, {
          description: 'Available in all projects',
        });
      } else {
        // Install to each selected project
        if (selectedProjects.length === 0) {
          toast.error('No projects selected', {
            description: 'Select at least one project for project/local scope',
          });
          return;
        }

        // Install sequentially to avoid cache race conditions
        // (Claude CLI uses shared cache directory)
        const successfulProjects: string[] = [];
        const failedResults: { project: string; error: string }[] = [];

        for (const projectPath of selectedProjects) {
          try {
            await installPluginByKey(pluginSpec, installScope, projectPath);
            successfulProjects.push(projectPath);
          } catch (err) {
            failedResults.push({
              project: projectPath,
              error: err instanceof Error ? err.message : String(err),
            });
          }
        }

        if (failedResults.length === 0) {
          toast.success(`Installed ${pluginId}`, {
            description: `Added to ${successfulProjects.length} project${successfulProjects.length > 1 ? 's' : ''} (${installScope} scope)`,
          });
        } else if (successfulProjects.length > 0) {
          toast.warning(`Partially installed ${pluginId}`, {
            description: `${successfulProjects.length} succeeded, ${failedResults.length} failed: ${failedResults[0].error}`,
          });
        } else {
          toast.error(`Failed to install ${pluginId}`, {
            description: failedResults[0].error,
          });
        }
      }
      await refetch();
    } catch (err) {
      toast.error(`Failed to install ${pluginId}`, {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setInstallingPlugin(null);
      setPendingInstall(null);
    }
  }

  function toggleProjectSelection(projectPath: string) {
    setSelectedProjects((prev) =>
      prev.includes(projectPath) ? prev.filter((p) => p !== projectPath) : [...prev, projectPath]
    );
  }

  async function handleCleanCache() {
    setCleaningCache(true);
    try {
      const result = await invoke<{
        deleted_count: number;
        deleted_size_formatted: string;
        errors: string[];
      }>('cache_clean');
      if (result.deleted_count > 0) {
        toast.success('Cache cleaned', {
          description: `Removed ${result.deleted_count} entries, freed ${result.deleted_size_formatted}`,
        });
      } else {
        toast.info('No stale cache to clean');
      }
      if (result.errors.length > 0) {
        toast.warning('Some entries could not be removed', {
          description: result.errors[0],
        });
      }
      await refetchCache();
      await refetch();
    } catch (err) {
      toast.error('Failed to clean cache', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setCleaningCache(false);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    }
    if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(2)} KB`;
    }
    return `${bytes} bytes`;
  }

  // Get all available plugins across marketplaces
  function getAvailablePlugins(): Array<AvailablePlugin & { marketplace: string }> {
    const allPlugins: Array<AvailablePlugin & { marketplace: string }> = [];

    for (const marketplace of sortedMarketplaces) {
      // Filter by selected marketplace if one is selected
      if (selectedMarketplace && marketplace.name !== selectedMarketplace) {
        continue;
      }

      for (const plugin of marketplace.available_plugins || []) {
        allPlugins.push({
          ...plugin,
          marketplace: marketplace.name,
        });
      }
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      return allPlugins.filter(
        (p) =>
          p.name.toLowerCase().includes(query) ||
          p.id.toLowerCase().includes(query) ||
          p.description.toLowerCase().includes(query)
      );
    }

    return allPlugins;
  }

  function getSourceDisplay(marketplace: Marketplace): string {
    const { source_type } = marketplace;
    switch (source_type.type) {
      case 'GitHub':
        return `${source_type.owner}/${source_type.repo}`;
      case 'Url':
        return source_type.url;
      case 'Local':
        return source_type.path;
      default:
        return marketplace.location;
    }
  }

  function formatRelativeDate(isoString: string | null): string {
    if (!isoString) return '-';
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      // Show time for today
      return date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
    }
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays}d ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
    if (diffDays < 365) return `${Math.floor(diffDays / 30)}mo ago`;
    return `${Math.floor(diffDays / 365)}y ago`;
  }

  function formatFullDate(isoString: string | null): string {
    if (!isoString) return 'Unknown';
    return new Date(isoString).toLocaleString();
  }

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-6 w-6 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Marketplace</h2>
          <HelpButton section="PLUGINS" />
        </div>
        <Button size="sm" onClick={() => setShowAddPluginDialog(true)}>
          <Plus className="h-4 w-4 mr-2" />
          Add Plugin
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6 space-y-6">
        <section className="rounded-md border border-border bg-muted/20 p-5">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="max-w-3xl">
              <div className="flex flex-wrap items-center gap-2 mb-2">
                <Badge variant="outline">Claude Code native</Badge>
                <Badge variant="secondary">Codex discovery</Badge>
                <Badge variant="outline">Managed targets</Badge>
              </div>
              <h3 className="text-base font-semibold">Runtime-aware plugin marketplace</h3>
              <p className="text-sm text-muted-foreground mt-1">
                Claude Code marketplace management stays fully available here, and Codex support
                surfaces discovered marketplace files and plugin manifests across user and project
                scopes.
              </p>
            </div>

            <div className="flex rounded-md border border-border bg-background p-1">
              {runtimeOptions.map((option) => {
                const Icon = option.icon;
                const active = runtimeFilter === option.id;
                return (
                  <button
                    key={option.id}
                    type="button"
                    onClick={() => setRuntimeFilter(option.id)}
                    className={`flex items-center gap-2 rounded px-3 py-2 text-sm transition-colors ${
                      active
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    }`}
                  >
                    <Icon className="h-4 w-4" />
                    <span>{option.label}</span>
                    <span
                      className={`rounded px-1.5 py-0.5 text-[10px] ${
                        active ? 'bg-primary-foreground/20' : 'bg-muted'
                      }`}
                    >
                      {option.count}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          <div className="mt-5 grid gap-3 md:grid-cols-3">
            <div className="rounded-md border border-border/70 bg-background/70 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  <Bot className="h-4 w-4 text-primary" />
                  <p className="text-sm font-medium">Claude Code</p>
                </div>
                <Badge variant="default">Native</Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-2">
                {sortedInstalledPlugins.length} installed plugin
                {sortedInstalledPlugins.length !== 1 ? 's' : ''} from {sortedMarketplaces.length}{' '}
                marketplace
                {sortedMarketplaces.length !== 1 ? 's' : ''}.
              </p>
            </div>

            <div className="rounded-md border border-border/70 bg-background/70 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  <Code2 className="h-4 w-4 text-primary" />
                  <p className="text-sm font-medium">Codex</p>
                </div>
                <Badge variant={codexMarketplaces.length > 0 ? 'default' : 'secondary'}>
                  {codexMarketplaces.length > 0 ? 'Live' : 'Preview'}
                </Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-2">
                {codexMarketplaces.length > 0
                  ? `${codexPlugins.length} plugins surfaced from ${codexMarketplaces.length} discovered marketplace${codexMarketplaces.length !== 1 ? 's' : ''}.`
                  : 'No Codex marketplace files are currently discovered in the scanned user or project scopes.'}
              </p>
            </div>

            <div className="rounded-md border border-border/70 bg-background/70 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  <Boxes className="h-4 w-4 text-primary" />
                  <p className="text-sm font-medium">Shared bundles</p>
                </div>
                <Badge variant="outline">Preview</Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-2">
                Bundle export is the planned bridge for authoring once and targeting Claude Code
                plugins and Codex plugins from the same source.
              </p>
            </div>
          </div>
        </section>

        <Card>
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <CardTitle className="text-base">Managed Plugins</CardTitle>
                <CardDescription>
                  TARS remembers the plugins you want available across Claude Code, Codex, or both,
                  then lets you reapply or remove them from one place.
                </CardDescription>
              </div>
              <Badge variant="outline">{pluginSubscriptions.length} managed</Badge>
            </div>
          </CardHeader>
          <CardContent>
            {pluginSubscriptions.length === 0 ? (
              <div className="rounded-md border border-dashed border-border p-6 text-sm text-muted-foreground">
                No managed plugins yet. Use <span className="font-medium">Add Plugin</span> to save
                a cross-runtime plugin subscription.
              </div>
            ) : (
              <div className="space-y-3">
                {pluginSubscriptions.map((subscription) => {
                  const claudeSelected = subscription.targets.includes('claude-code');
                  const codexSelected = subscription.targets.includes('codex');
                  const claudeReady = claudeSelected
                    ? installedClaudePluginIds.has(subscription.plugin_name)
                    : null;
                  const codexReady = codexSelected
                    ? registeredCodexPluginIds.has(subscription.plugin_name)
                    : null;

                  return (
                    <div
                      key={subscription.id}
                      className="rounded-md border border-border bg-background/70 p-4"
                    >
                      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <p className="text-sm font-medium">{subscription.plugin_name}</p>
                            <Badge variant="secondary">{subscription.scope}</Badge>
                            {claudeSelected && (
                              <Badge variant={claudeReady ? 'default' : 'outline'}>
                                Claude {claudeReady ? 'ready' : 'pending'}
                              </Badge>
                            )}
                            {codexSelected && (
                              <Badge variant={codexReady ? 'default' : 'outline'}>
                                Codex {codexReady ? 'ready' : 'pending'}
                              </Badge>
                            )}
                          </div>
                          <p className="mt-2 break-all font-mono text-xs text-muted-foreground">
                            {subscription.source_kind === 'marketplace'
                              ? `${subscription.plugin_name}@${subscription.marketplace_name || 'marketplace'} from ${subscription.marketplace_source}`
                              : subscription.source}
                          </p>
                          {subscription.source_kind === 'marketplace' &&
                            subscription.codex_source && (
                              <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                                Codex source: {subscription.codex_source}
                              </p>
                            )}
                          <p className="mt-2 text-xs text-muted-foreground">
                            Updated {formatRelativeDate(subscription.updated_at)}
                          </p>
                        </div>

                        <div className="flex gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleSyncManagedPlugin(subscription.id)}
                            disabled={syncingSubscriptionId === subscription.id}
                          >
                            <RefreshCw
                              className={`h-4 w-4 mr-2 ${
                                syncingSubscriptionId === subscription.id ? 'animate-spin' : ''
                              }`}
                            />
                            Reapply
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleRemoveManagedPlugin(subscription.id)}
                            disabled={removingSubscriptionId === subscription.id}
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Remove
                          </Button>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {showCodexPreview && (
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-center gap-2">
                <Code2 className="h-5 w-5 text-primary" />
                <CardTitle className="text-base">Codex Marketplace</CardTitle>
              </div>
              <CardDescription>
                Live Codex marketplace files and plugin manifests discovered from user and project
                scopes
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="grid gap-3 md:grid-cols-3">
                {codexMarketplaceSurfaces.map((surface) => (
                  <div key={surface.label} className="rounded-md border border-border p-4">
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-sm font-medium">{surface.label}</p>
                      <Badge variant={surface.variant}>{surface.status}</Badge>
                    </div>
                    <p className="mt-2 truncate font-mono text-xs text-muted-foreground">
                      {surface.path}
                    </p>
                  </div>
                ))}
              </div>

              {codexMarketplaces.length === 0 ? (
                <div className="rounded-md border border-dashed border-border p-6 text-sm text-muted-foreground">
                  No Codex marketplace files are discovered yet. Add
                  `~/.agents/plugins/marketplace.json` or a repo `.agents/plugins/marketplace.json`
                  to surface local plugin catalogs.
                </div>
              ) : (
                <>
                  <div className="grid gap-3 lg:grid-cols-2">
                    {codexMarketplaces.map((marketplace) => (
                      <div
                        key={`${marketplace.scopeLabel}-${marketplace.projectName || 'global'}-${marketplace.path}`}
                        className="rounded-md border border-border bg-muted/20 p-4"
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <p className="text-sm font-medium">
                                {marketplace.display_name || marketplace.name}
                              </p>
                              <Badge variant="outline">{marketplace.scopeLabel}</Badge>
                              {marketplace.projectName && (
                                <Badge variant="secondary">{marketplace.projectName}</Badge>
                              )}
                            </div>
                            <p className="mt-2 text-xs text-muted-foreground truncate">
                              {marketplace.path}
                            </p>
                          </div>
                          <Badge variant="default">
                            {marketplace.plugins.length} plugin
                            {marketplace.plugins.length !== 1 ? 's' : ''}
                          </Badge>
                        </div>
                      </div>
                    ))}
                  </div>

                  <div className="rounded-md border border-border">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Plugin</TableHead>
                          <TableHead>Marketplace</TableHead>
                          <TableHead>Scope</TableHead>
                          <TableHead>Version</TableHead>
                          <TableHead>Status</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {codexPlugins.map((plugin) => (
                          <TableRow
                            key={`${plugin.marketplaceName}-${plugin.id}-${plugin.scopeLabel}-${plugin.projectName || 'global'}`}
                          >
                            <TableCell>
                              <div>
                                <p className="font-medium">{plugin.display_name || plugin.id}</p>
                                <p className="text-xs text-muted-foreground">
                                  {plugin.description || 'No manifest description'}
                                </p>
                              </div>
                            </TableCell>
                            <TableCell className="text-muted-foreground">
                              {plugin.marketplaceName}
                            </TableCell>
                            <TableCell>
                              <div className="flex flex-wrap gap-1">
                                <Badge variant="outline">{plugin.scopeLabel}</Badge>
                                {plugin.projectName && (
                                  <Badge variant="secondary">{plugin.projectName}</Badge>
                                )}
                              </div>
                            </TableCell>
                            <TableCell>
                              <span className="text-xs font-mono">{plugin.version || '-'}</span>
                            </TableCell>
                            <TableCell>
                              <div className="flex flex-wrap items-center gap-2">
                                <Badge variant={plugin.resolved ? 'default' : 'secondary'}>
                                  {plugin.resolved ? 'Resolved' : plugin.source_type}
                                </Badge>
                                {plugin.installation_policy && (
                                  <span className="text-xs text-muted-foreground">
                                    {plugin.installation_policy}
                                  </span>
                                )}
                              </div>
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        )}

        {showClaudeMarketplace && (
          <>
            {/* Marketplaces Section */}
            <Card>
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Store className="h-5 w-5 text-primary" />
                    <CardTitle className="text-base">Installed Marketplaces</CardTitle>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleUpdateMarketplaces}
                      disabled={updatingMarketplaces}
                    >
                      <RefreshCw
                        className={`h-4 w-4 mr-2 ${updatingMarketplaces ? 'animate-spin' : ''}`}
                      />
                      Update All
                    </Button>
                    <Button size="sm" onClick={() => setShowAddMarketplace(true)}>
                      <Plus className="h-4 w-4 mr-2" />
                      Add Marketplace
                    </Button>
                  </div>
                </div>
                <CardDescription>
                  Plugin sources - GitHub repos or local paths containing plugins
                </CardDescription>
              </CardHeader>
              <CardContent>
                {plugins.marketplaces.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    <Store className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p>No marketplaces configured</p>
                    <p className="text-xs mt-1">
                      Add a marketplace to discover and install plugins
                    </p>
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Name</TableHead>
                        <TableHead>Source</TableHead>
                        <TableHead>Auto-update</TableHead>
                        <TableHead className="w-[100px]">Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {sortedMarketplaces.map((marketplace) => (
                        <TableRow key={marketplace.name}>
                          <TableCell className="font-medium">{marketplace.name}</TableCell>
                          <TableCell>
                            <div className="flex items-center gap-2">
                              <span className="text-xs px-2 py-0.5 bg-muted rounded">
                                {marketplace.source_type.type}
                              </span>
                              <span
                                className="text-sm text-muted-foreground truncate max-w-[300px]"
                                title={getSourceDisplay(marketplace)}
                              >
                                {getSourceDisplay(marketplace)}
                              </span>
                              {marketplace.source_type.type === 'GitHub' && (
                                <a
                                  href={`https://github.com/${marketplace.location}`}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-muted-foreground hover:text-foreground"
                                  title="View on GitHub"
                                >
                                  <ExternalLink className="h-3 w-3" />
                                </a>
                              )}
                              {marketplace.source_type.type === 'Url' &&
                                marketplace.source_type.url.includes('github.com') && (
                                  <a
                                    href={marketplace.source_type.url.replace(/\.git$/, '')}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-foreground"
                                    title="View on GitHub"
                                  >
                                    <ExternalLink className="h-3 w-3" />
                                  </a>
                                )}
                            </div>
                          </TableCell>
                          <TableCell>
                            <button
                              type="button"
                              onClick={() =>
                                handleToggleAutoUpdate(marketplace.name, marketplace.auto_update)
                              }
                              className="text-xs px-2 py-1 rounded hover:bg-muted transition-colors"
                              title={`Click to ${marketplace.auto_update ? 'disable' : 'enable'} auto-update`}
                            >
                              {marketplace.auto_update ? (
                                <span className="text-green-600 flex items-center gap-1">
                                  <Check className="h-3 w-3" />
                                  On
                                </span>
                              ) : (
                                <span className="text-muted-foreground">Off</span>
                              )}
                            </button>
                          </TableCell>
                          <TableCell>
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-destructive hover:text-destructive"
                              onClick={() => {
                                setMarketplaceToRemove(marketplace.name);
                                if (marketplace.name === PROFILE_MARKETPLACE) {
                                  setAlsoUninstallPlugins(true);
                                }
                              }}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                )}
              </CardContent>
            </Card>

            {/* Installed Plugins Section */}
            <Card>
              <CardHeader className="pb-3">
                <div className="flex items-center gap-2">
                  <Package className="h-5 w-5 text-primary" />
                  <CardTitle className="text-base">Installed Plugins</CardTitle>
                </div>
                <CardDescription>
                  Plugins add skills, commands, agents, hooks, and MCP servers to Claude Code
                </CardDescription>
              </CardHeader>
              <CardContent>
                {sortedInstalledPlugins.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    <Box className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p>No plugins installed</p>
                    <p className="text-xs mt-1">Browse available plugins below to install</p>
                  </div>
                ) : (
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Plugin</TableHead>
                        <TableHead>Marketplace</TableHead>
                        <TableHead>Version</TableHead>
                        <TableHead>Updated</TableHead>
                        <TableHead>Scope</TableHead>
                        <TableHead>Status</TableHead>
                        <TableHead className="w-[120px]">Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {sortedInstalledPlugins.map((plugin, index) => {
                        const skills = plugin.manifest.parsed_skills || [];
                        const availablePlugin = plugin.marketplace
                          ? availableByMarketplace.get(plugin.marketplace)?.get(plugin.id)
                          : undefined;
                        const updateAvailable =
                          !!availablePlugin?.version && availablePlugin.version !== plugin.version;
                        // Include project_path in key to distinguish same plugin installed to multiple projects
                        const uniqueKey = `${plugin.id}-${plugin.marketplace}-${plugin.scope.type}-${plugin.project_path || index}`;
                        const codexBridgeKey = codexBridgeKeyForPlugin(plugin);
                        const codexBridged = bridgedCodexPluginKeys.has(codexBridgeKey);

                        return (
                          <TableRow key={uniqueKey} className={!plugin.enabled ? 'opacity-60' : ''}>
                            <TableCell>
                              <div>
                                <div className="flex flex-wrap items-center gap-2">
                                  <span className="font-medium">{plugin.id}</span>
                                  {codexBridged && <Badge variant="outline">Codex synced</Badge>}
                                </div>
                                {plugin.manifest.description && (
                                  <p className="text-xs text-muted-foreground truncate max-w-[250px]">
                                    {plugin.manifest.description}
                                  </p>
                                )}
                                {skills.length > 0 && (
                                  <div className="flex flex-wrap gap-1 mt-1">
                                    {skills.map((skill) => (
                                      <span
                                        key={skill.name}
                                        className={`text-xs px-1.5 py-0.5 rounded cursor-pointer hover:opacity-80 ${
                                          skill.is_init
                                            ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                                            : skill.is_settings
                                              ? 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300'
                                              : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
                                        }`}
                                        title={`Click to copy ${skill.invocation}`}
                                        onClick={() => handleCopySkill(skill)}
                                      >
                                        {skill.name}
                                      </span>
                                    ))}
                                  </div>
                                )}
                              </div>
                            </TableCell>
                            <TableCell className="text-muted-foreground">
                              {plugin.marketplace || '-'}
                            </TableCell>
                            <TableCell>
                              <span className="text-xs font-mono">{plugin.version}</span>
                            </TableCell>
                            <TableCell>
                              {updateAvailable ? (
                                <Button
                                  variant="outline"
                                  size="sm"
                                  onClick={() => handleUpdatePlugin(plugin, uniqueKey)}
                                  disabled={updatingPlugin === uniqueKey}
                                  title={
                                    availablePlugin?.version
                                      ? `Update available: ${plugin.version} → ${availablePlugin.version}`
                                      : 'Update available'
                                  }
                                >
                                  <RefreshCw
                                    className={`h-3.5 w-3.5 mr-2 ${
                                      updatingPlugin === uniqueKey ? 'animate-spin' : ''
                                    }`}
                                  />
                                  Update
                                </Button>
                              ) : (
                                <span
                                  className="text-xs text-muted-foreground cursor-help"
                                  title={`Installed: ${formatFullDate(plugin.installed_at)}\nVersion updated: ${formatFullDate(versionTracking[`${plugin.id}@${plugin.marketplace}`] || plugin.last_updated)}`}
                                >
                                  {formatRelativeDate(
                                    versionTracking[`${plugin.id}@${plugin.marketplace}`] ||
                                      plugin.last_updated
                                  )}
                                </span>
                              )}
                            </TableCell>
                            <TableCell>
                              <div className="flex flex-col gap-0.5">
                                <span
                                  className="text-xs px-2 py-0.5 bg-muted rounded w-fit"
                                  title={
                                    plugin.scope.type === 'User'
                                      ? 'User scope - available everywhere'
                                      : plugin.scope.type === 'Project'
                                        ? `Project scope - ${plugin.project_path || 'specific to a project'}`
                                        : plugin.scope.type === 'Local'
                                          ? `Local scope - ${plugin.project_path || 'project-specific, not tracked by git'}`
                                          : 'Managed scope - controlled by system admin'
                                  }
                                >
                                  {plugin.scope.type.toLowerCase()}
                                </span>
                                {plugin.project_path && (
                                  <span
                                    className="text-[10px] text-muted-foreground truncate max-w-[150px]"
                                    title={plugin.project_path}
                                  >
                                    {plugin.project_path.split('/').pop() || plugin.project_path}
                                  </span>
                                )}
                              </div>
                            </TableCell>
                            <TableCell>
                              {plugin.enabled ? (
                                <span className="text-xs px-2 py-0.5 bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300 rounded">
                                  Enabled
                                </span>
                              ) : (
                                <span className="text-xs px-2 py-0.5 bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400 rounded">
                                  Disabled
                                </span>
                              )}
                            </TableCell>
                            <TableCell>
                              <div className="flex gap-1">
                                {skills.length > 0 && (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    onClick={() => handleShowSkills(plugin.id, skills)}
                                    title="View available commands"
                                  >
                                    <Terminal className="h-4 w-4 text-primary" />
                                  </Button>
                                )}
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() =>
                                    handleMakePluginAvailableForCodex(plugin, uniqueKey)
                                  }
                                  disabled={bridgingPluginKey === uniqueKey}
                                  title={
                                    codexBridged
                                      ? 'Sync Codex skills again'
                                      : 'Make available for Codex'
                                  }
                                >
                                  <Code2
                                    className={`h-4 w-4 ${
                                      bridgingPluginKey === uniqueKey
                                        ? 'animate-pulse text-primary'
                                        : codexBridged
                                          ? 'text-primary'
                                          : 'text-muted-foreground'
                                    }`}
                                  />
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() =>
                                    handleTogglePlugin(
                                      plugin.id,
                                      plugin.marketplace,
                                      plugin.enabled
                                    )
                                  }
                                  title={plugin.enabled ? 'Disable' : 'Enable'}
                                >
                                  {plugin.enabled ? (
                                    <Power className="h-4 w-4 text-green-600" />
                                  ) : (
                                    <PowerOff className="h-4 w-4 text-muted-foreground" />
                                  )}
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="text-destructive hover:text-destructive"
                                  onClick={() =>
                                    handleUninstallPlugin(
                                      plugin.marketplace
                                        ? `${plugin.id}@${plugin.marketplace}`
                                        : plugin.id,
                                      plugin.scope.type.toLowerCase(),
                                      plugin.project_path
                                    )
                                  }
                                  title="Uninstall"
                                >
                                  <Trash2 className="h-4 w-4" />
                                </Button>
                              </div>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                )}
              </CardContent>
            </Card>

            {/* Cache Management Section */}
            {cacheStatus && cacheStatus.stale_entries.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <HardDrive className="h-5 w-5 text-primary" />
                      <CardTitle className="text-base">Cache Management</CardTitle>
                    </div>
                    <Button
                      size="sm"
                      variant="destructive"
                      onClick={handleCleanCache}
                      disabled={cleaningCache}
                    >
                      {cleaningCache ? (
                        <>
                          <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                          Cleaning...
                        </>
                      ) : (
                        <>
                          <Trash2 className="h-4 w-4 mr-2" />
                          Clean {cacheStatus.total_size_formatted}
                        </>
                      )}
                    </Button>
                  </div>
                  <CardDescription>
                    {cacheStatus.stale_entries.length} stale plugin cache
                    {cacheStatus.stale_entries.length !== 1 ? 's' : ''} from old versions
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <button
                    type="button"
                    onClick={() => setShowCacheDetails(!showCacheDetails)}
                    className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
                  >
                    {showCacheDetails ? (
                      <ChevronDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    {showCacheDetails ? 'Hide' : 'Show'} details
                  </button>
                  {showCacheDetails && (
                    <div className="mt-3 space-y-1">
                      {cacheStatus.stale_entries.map((entry, idx) => (
                        <div
                          key={`${entry.plugin_name}-${entry.version}-${idx}`}
                          className="flex items-center justify-between text-xs py-1 border-b last:border-0"
                        >
                          <span className="text-muted-foreground">
                            {entry.plugin_name}@{entry.marketplace}{' '}
                            <span className="font-mono">v{entry.version}</span>
                          </span>
                          <span className="text-muted-foreground">
                            {formatBytes(entry.size_bytes)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>
            )}

            {/* Available Plugins Section (Collapsible) */}
            {plugins.marketplaces.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <button
                    type="button"
                    onClick={() => setAvailablePluginsExpanded(!availablePluginsExpanded)}
                    className="flex items-center justify-between w-full text-left"
                  >
                    <div className="flex items-center gap-2">
                      {availablePluginsExpanded ? (
                        <ChevronDown className="h-5 w-5 text-primary" />
                      ) : (
                        <ChevronRight className="h-5 w-5 text-primary" />
                      )}
                      <Download className="h-5 w-5 text-primary" />
                      <CardTitle className="text-base">Available Plugins</CardTitle>
                      <span className="text-xs text-muted-foreground ml-2">
                        ({getAvailablePlugins().length} plugins)
                      </span>
                    </div>
                  </button>
                  {availablePluginsExpanded && (
                    <>
                      <CardDescription className="mt-2">
                        Browse and install plugins from your marketplaces
                      </CardDescription>
                      <div className="flex gap-2 mt-3">
                        <div className="relative flex-1">
                          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                          <Input
                            placeholder="Search plugins..."
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            className="pl-9"
                          />
                        </div>
                        <select
                          value={selectedMarketplace || ''}
                          onChange={(e) => setSelectedMarketplace(e.target.value || null)}
                          className="px-3 py-2 rounded-md border border-input bg-background text-sm"
                        >
                          <option value="">All Marketplaces</option>
                          {sortedMarketplaces.map((m) => (
                            <option key={m.name} value={m.name}>
                              {m.name}
                            </option>
                          ))}
                        </select>
                      </div>
                    </>
                  )}
                </CardHeader>
                {availablePluginsExpanded && (
                  <CardContent>
                    {(() => {
                      const availablePlugins = getAvailablePlugins();
                      if (availablePlugins.length === 0) {
                        return (
                          <div className="text-center py-8 text-muted-foreground">
                            <Package className="h-8 w-8 mx-auto mb-2 opacity-50" />
                            <p>No plugins found</p>
                            <p className="text-xs mt-1">
                              {searchQuery
                                ? 'Try a different search term'
                                : 'Update your marketplaces to discover plugins'}
                            </p>
                          </div>
                        );
                      }

                      return (
                        <div className="grid gap-3">
                          {availablePlugins.map((plugin) => (
                            <div
                              key={`${plugin.id}-${plugin.marketplace}`}
                              className="flex items-center justify-between p-3 rounded-lg border bg-card hover:bg-muted/50 transition-colors"
                            >
                              <div className="flex-1 min-w-0 overflow-hidden">
                                <div className="flex items-center gap-2 flex-wrap">
                                  <span className="font-medium">{plugin.name}</span>
                                  <span className="text-xs px-2 py-0.5 bg-muted rounded">
                                    {plugin.marketplace}
                                  </span>
                                  {plugin.installed && (
                                    <span className="text-xs px-2 py-0.5 bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300 rounded flex items-center gap-1">
                                      <Check className="h-3 w-3" />
                                      Installed
                                    </span>
                                  )}
                                </div>
                                {plugin.description && (
                                  <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                                    {plugin.description}
                                  </p>
                                )}
                                {plugin.author && (
                                  <p className="text-xs text-muted-foreground mt-0.5">
                                    by {plugin.author.name}
                                  </p>
                                )}
                              </div>
                              <Button
                                size="sm"
                                variant={plugin.installed ? 'outline' : 'default'}
                                disabled={
                                  plugin.installed ||
                                  installingPlugin === `${plugin.id}@${plugin.marketplace}`
                                }
                                onClick={() => handleInstallClick(plugin.id, plugin.marketplace)}
                              >
                                {installingPlugin === `${plugin.id}@${plugin.marketplace}` ? (
                                  <>
                                    <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                                    Installing...
                                  </>
                                ) : plugin.installed ? (
                                  'Installed'
                                ) : (
                                  <>
                                    <Download className="h-4 w-4 mr-2" />
                                    Install
                                  </>
                                )}
                              </Button>
                            </div>
                          ))}
                        </div>
                      );
                    })()}
                  </CardContent>
                )}
              </Card>
            )}
          </>
        )}
      </div>

      <Dialog open={showAddPluginDialog} onOpenChange={setShowAddPluginDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Plugin</DialogTitle>
            <DialogDescription>
              Add a plugin once, then register it for Claude Code, Codex, or both at user scope.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-5">
            <div>
              <Label>Source style</Label>
              <div className="mt-2 grid gap-2 sm:grid-cols-2">
                {[
                  {
                    id: 'direct' as const,
                    label: 'Direct plugin',
                    description:
                      'Use a local path, repo URL, or owner/repo for one install source.',
                  },
                  {
                    id: 'marketplace' as const,
                    label: 'Plugin in marketplace',
                    description:
                      'Keep the Claude marketplace flow, with an optional direct Codex fallback source.',
                  },
                ].map((option) => {
                  const selected = pluginSourceKind === option.id;
                  return (
                    <button
                      key={option.id}
                      type="button"
                      onClick={() => setPluginSourceKind(option.id)}
                      className={`rounded-md border p-3 text-left transition-colors ${
                        selected
                          ? 'border-primary bg-primary/5'
                          : 'border-border bg-background hover:bg-muted/50'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <span className="text-sm font-medium">{option.label}</span>
                        <Badge variant={selected ? 'default' : 'outline'}>
                          {selected ? 'Selected' : 'Off'}
                        </Badge>
                      </div>
                      <p className="mt-2 text-xs text-muted-foreground">{option.description}</p>
                    </button>
                  );
                })}
              </div>
            </div>

            {pluginSourceKind === 'direct' ? (
              <div>
                <Label htmlFor="plugin-source">Plugin source</Label>
                <Input
                  id="plugin-source"
                  value={pluginSource}
                  onChange={(e) => setPluginSource(e.target.value)}
                  placeholder="owner/repo, https://github.com/..., /path/to/plugin"
                  className="mt-2"
                />
                <p className="text-xs text-muted-foreground mt-2">
                  Best for `Both`. TARS can use the same source for Claude Code and Codex.
                </p>
              </div>
            ) : (
              <div className="space-y-4">
                <div>
                  <Label htmlFor="plugin-marketplace-source">Marketplace source</Label>
                  <Input
                    id="plugin-marketplace-source"
                    value={pluginMarketplaceSource}
                    onChange={(e) => setPluginMarketplaceSource(e.target.value)}
                    placeholder="owner/repo or https://github.com/..."
                    className="mt-2"
                  />
                </div>
                <div>
                  <Label htmlFor="plugin-marketplace-name">Marketplace name override</Label>
                  <Input
                    id="plugin-marketplace-name"
                    value={pluginMarketplaceName}
                    onChange={(e) => setPluginMarketplaceName(e.target.value)}
                    placeholder="Optional. Auto-derived when left blank."
                    className="mt-2"
                  />
                </div>
                <div>
                  <Label htmlFor="marketplace-plugin-name">Plugin name</Label>
                  <Input
                    id="marketplace-plugin-name"
                    value={marketplacePluginName}
                    onChange={(e) => setMarketplacePluginName(e.target.value)}
                    placeholder="superpowers"
                    className="mt-2"
                  />
                </div>
                <div>
                  <Label htmlFor="plugin-codex-source">Codex direct source</Label>
                  <Input
                    id="plugin-codex-source"
                    value={pluginCodexSource}
                    onChange={(e) => setPluginCodexSource(e.target.value)}
                    placeholder="Optional for Claude-only. Required if you want Both."
                    className="mt-2"
                  />
                  <p className="text-xs text-muted-foreground mt-2">
                    Claude can install from the marketplace directly. Codex still needs a direct
                    plugin source like a path, repo, or URL.
                  </p>
                </div>
              </div>
            )}

            {pluginSourceKind === 'marketplace' &&
              pluginTargets.includes('codex') &&
              !pluginCodexSource.trim() && (
                <div className="rounded-md border border-amber-300/40 bg-amber-500/10 p-3 text-xs text-muted-foreground">
                  Codex target selected: add a direct Codex source so TARS can register the plugin
                  outside Claude’s marketplace system.
                </div>
              )}

            <div className="rounded-md border border-border/70 bg-muted/20 p-3 text-xs text-muted-foreground">
              {pluginSourceKind === 'direct'
                ? 'Direct plugin sources stay simple in TARS. Claude is wired through a TARS-managed marketplace behind the scenes, and Codex is registered directly.'
                : 'Marketplace-backed plugins match the normal Claude flow. TARS adds the marketplace for Claude, then installs the plugin from it.'}
            </div>

            <div>
              <Label>Runtime targets</Label>
              <div className="mt-2 grid gap-2 sm:grid-cols-2">
                {[
                  {
                    id: 'claude-code' as const,
                    label: 'Claude Code',
                    description: 'Installs the plugin immediately for your user scope.',
                  },
                  {
                    id: 'codex' as const,
                    label: 'Codex',
                    description: 'Registers the plugin in ~/.agents/plugins/marketplace.json.',
                  },
                ].map((target) => {
                  const selected = pluginTargets.includes(target.id);
                  return (
                    <button
                      key={target.id}
                      type="button"
                      onClick={() => togglePluginTarget(target.id)}
                      className={`rounded-md border p-3 text-left transition-colors ${
                        selected
                          ? 'border-primary bg-primary/5'
                          : 'border-border bg-background hover:bg-muted/50'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <span className="text-sm font-medium">{target.label}</span>
                        <Badge variant={selected ? 'default' : 'outline'}>
                          {selected ? 'Selected' : 'Off'}
                        </Badge>
                      </div>
                      <p className="mt-2 text-xs text-muted-foreground">{target.description}</p>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAddPluginDialog(false)}
              disabled={addingPluginSource}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAddPluginToTargets}
              disabled={
                addingPluginSource ||
                pluginTargets.length === 0 ||
                (pluginSourceKind === 'direct'
                  ? !pluginSource.trim()
                  : !pluginMarketplaceSource.trim() || !marketplacePluginName.trim())
              }
            >
              {addingPluginSource ? 'Adding...' : 'Add Plugin'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Marketplace Dialog */}
      <Dialog open={showAddMarketplace} onOpenChange={setShowAddMarketplace}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Marketplace</DialogTitle>
            <DialogDescription>
              Add a plugin marketplace from GitHub or a local path.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label htmlFor="marketplace-source">Source</Label>
              <Input
                id="marketplace-source"
                value={marketplaceSource}
                onChange={(e) => setMarketplaceSource(e.target.value)}
                placeholder="owner/repo or https://github.com/..."
                className="mt-2"
              />
              <p className="text-xs text-muted-foreground mt-2">
                Examples: <code>anthropics/claude-plugins-official</code> or a full GitHub URL
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAddMarketplace(false)}
              disabled={addingMarketplace}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAddMarketplace}
              disabled={addingMarketplace || !marketplaceSource.trim()}
            >
              {addingMarketplace ? 'Adding...' : 'Add Marketplace'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Remove Marketplace Confirmation */}
      <Dialog
        open={!!marketplaceToRemove}
        onOpenChange={(open) => {
          if (!open) {
            setMarketplaceToRemove(null);
            setAlsoUninstallPlugins(false);
          }
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Remove Marketplace</DialogTitle>
            <DialogDescription>
              {isProfileMarketplace
                ? `Removing "${marketplaceToRemove}" will permanently delete all bundles and their tools.`
                : `Are you sure you want to remove "${marketplaceToRemove}"?`}
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            {isProfileMarketplace && (
              <div className="flex items-start gap-3 rounded-lg border border-destructive/60 bg-destructive/10 p-3 text-sm text-destructive">
                <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
                <div>
                  <div className="font-medium">Destructive action</div>
                  <ul className="mt-1 space-y-1 text-xs text-destructive/90">
                    <li>Deletes all bundles and their stored tools</li>
                    <li>Unassigns bundles from projects and clears matching local overrides</li>
                    <li>Uninstalls any installed bundle plugins</li>
                  </ul>
                </div>
              </div>
            )}
            {(() => {
              const affectedPlugins = marketplaceToRemove
                ? getPluginsFromMarketplace(marketplaceToRemove)
                : [];
              if (isProfileMarketplace) {
                return (
                  <div className="space-y-3">
                    <div className="rounded-lg border border-border bg-muted/30 p-3">
                      <div className="text-sm font-medium">Bundle marketplace cleanup</div>
                      <div className="text-xs text-muted-foreground mt-1">
                        {profileSummary.totalProfiles} bundle
                        {profileSummary.totalProfiles !== 1 ? 's' : ''} with{' '}
                        {profileSummary.totalTools} tool
                        {profileSummary.totalTools !== 1 ? 's' : ''} total.
                      </div>
                    </div>
                    {affectedPlugins.length === 0 ? (
                      <p className="text-sm text-muted-foreground">
                        No bundle plugins are currently installed.
                      </p>
                    ) : (
                      <div className="space-y-2">
                        <p className="text-sm font-medium">
                          {affectedPlugins.length} bundle plugin
                          {affectedPlugins.length !== 1 ? 's' : ''} installed:
                        </p>
                        <div className="max-h-[120px] overflow-auto border rounded-lg divide-y">
                          {affectedPlugins.map((plugin, idx) => (
                            <div
                              key={`${plugin.id}-${plugin.scope.type}-${idx}`}
                              className="px-3 py-2 text-sm flex items-center justify-between"
                            >
                              <span className="font-medium">{plugin.id}</span>
                              <span className="text-xs text-muted-foreground">
                                {plugin.scope.type.toLowerCase()}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                    {affectedPlugins.length > 0 && (
                      <div className="rounded-lg border border-border p-3 text-xs text-muted-foreground">
                        Installed bundle plugins will be uninstalled automatically.
                      </div>
                    )}
                  </div>
                );
              }
              if (affectedPlugins.length === 0) {
                return (
                  <p className="text-sm text-muted-foreground">
                    No plugins are installed from this marketplace.
                  </p>
                );
              }
              return (
                <>
                  <div className="space-y-2">
                    <p className="text-sm font-medium">
                      {affectedPlugins.length} plugin{affectedPlugins.length !== 1 ? 's' : ''}{' '}
                      installed from this marketplace:
                    </p>
                    <div className="max-h-[120px] overflow-auto border rounded-lg divide-y">
                      {affectedPlugins.map((plugin, idx) => (
                        <div
                          key={`${plugin.id}-${plugin.scope.type}-${idx}`}
                          className="px-3 py-2 text-sm flex items-center justify-between"
                        >
                          <span className="font-medium">{plugin.id}</span>
                          <span className="text-xs text-muted-foreground">
                            {plugin.scope.type.toLowerCase()}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                  <label className="flex items-start gap-3 p-3 rounded-lg border cursor-pointer hover:bg-muted/50 transition-colors">
                    <input
                      type="checkbox"
                      checked={alsoUninstallPlugins}
                      onChange={(e) => setAlsoUninstallPlugins(e.target.checked)}
                      className="mt-0.5"
                    />
                    <div>
                      <div className="font-medium text-sm">Also uninstall plugins</div>
                      <div className="text-xs text-muted-foreground">
                        Remove all {affectedPlugins.length} plugin
                        {affectedPlugins.length !== 1 ? 's' : ''} from this marketplace
                      </div>
                    </div>
                  </label>
                </>
              );
            })()}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setMarketplaceToRemove(null);
                setAlsoUninstallPlugins(false);
              }}
              disabled={removingMarketplace}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleRemoveMarketplace}
              disabled={removingMarketplace}
            >
              {removingMarketplace ? (
                <>
                  <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                  Removing...
                </>
              ) : (
                <>
                  <Trash2 className="h-4 w-4 mr-2" />
                  Remove
                  {isProfileMarketplace
                    ? ' & Delete Bundles'
                    : alsoUninstallPlugins
                      ? ' & Uninstall'
                      : ''}
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Install Plugin Dialog */}
      <Dialog
        open={showInstallDialog}
        onOpenChange={(open) => {
          if (!open) {
            setShowInstallDialog(false);
            setPendingInstall(null);
          }
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Install Plugin</DialogTitle>
            <DialogDescription>
              Configure installation options for "{pendingInstall?.pluginId}"
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            {/* Scope Selection */}
            <div className="space-y-3">
              <Label className="text-sm font-medium">Installation Scope</Label>
              <div className="space-y-2">
                <label className="flex items-start gap-3 p-3 rounded-lg border cursor-pointer hover:bg-muted/50 transition-colors">
                  <input
                    type="radio"
                    name="scope"
                    value="user"
                    checked={installScope === 'user'}
                    onChange={() => setInstallScope('user')}
                    className="mt-0.5"
                  />
                  <div>
                    <div className="font-medium">User (Global)</div>
                    <div className="text-xs text-muted-foreground">
                      Available in all projects on this machine
                    </div>
                  </div>
                </label>
                <label className="flex items-start gap-3 p-3 rounded-lg border cursor-pointer hover:bg-muted/50 transition-colors">
                  <input
                    type="radio"
                    name="scope"
                    value="project"
                    checked={installScope === 'project'}
                    onChange={() => setInstallScope('project')}
                    className="mt-0.5"
                  />
                  <div>
                    <div className="font-medium">Project (Shared)</div>
                    <div className="text-xs text-muted-foreground">
                      Saved in project's .claude/ folder, tracked by git
                    </div>
                  </div>
                </label>
                <label className="flex items-start gap-3 p-3 rounded-lg border cursor-pointer hover:bg-muted/50 transition-colors">
                  <input
                    type="radio"
                    name="scope"
                    value="local"
                    checked={installScope === 'local'}
                    onChange={() => setInstallScope('local')}
                    className="mt-0.5"
                  />
                  <div>
                    <div className="font-medium">Local (Private)</div>
                    <div className="text-xs text-muted-foreground">
                      Project-specific but not tracked by git
                    </div>
                  </div>
                </label>
              </div>
            </div>

            {/* Project Selection (for project/local scope) */}
            {installScope !== 'user' && (
              <div className="space-y-3">
                <Label className="text-sm font-medium">
                  Select Projects
                  <span className="text-muted-foreground font-normal ml-1">
                    ({selectedProjects.length} selected)
                  </span>
                </Label>
                <div className="max-h-[200px] overflow-auto border rounded-lg">
                  {projects.length === 0 ? (
                    <div className="text-center py-6 text-muted-foreground">
                      <FolderOpen className="h-6 w-6 mx-auto mb-2 opacity-50" />
                      <p className="text-sm">No projects configured</p>
                      <p className="text-xs mt-1">Add projects in the Projects tab first</p>
                    </div>
                  ) : (
                    <div className="divide-y">
                      {projects.map((project) => (
                        <label
                          key={project.path}
                          className="flex items-center gap-3 p-3 cursor-pointer hover:bg-muted/50 transition-colors"
                        >
                          <input
                            type="checkbox"
                            checked={selectedProjects.includes(project.path)}
                            onChange={() => toggleProjectSelection(project.path)}
                          />
                          <div className="flex-1 min-w-0">
                            <div className="font-medium text-sm">{project.name}</div>
                            <div className="text-xs text-muted-foreground truncate">
                              {project.path}
                            </div>
                          </div>
                        </label>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowInstallDialog(false);
                setPendingInstall(null);
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleConfirmInstall}
              disabled={installScope !== 'user' && selectedProjects.length === 0}
            >
              <Download className="h-4 w-4 mr-2" />
              Install
              {installScope !== 'user' && selectedProjects.length > 0
                ? ` to ${selectedProjects.length} project${selectedProjects.length > 1 ? 's' : ''}`
                : ''}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Skills Dialog */}
      <Dialog
        open={!!skillsDialogPlugin}
        onOpenChange={(open) => !open && setSkillsDialogPlugin(null)}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Terminal className="h-5 w-5" />
              {skillsDialogPlugin?.name} Commands
            </DialogTitle>
            <DialogDescription>
              Copy these commands and paste them in Claude Code to run them.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-3">
            {skillsDialogPlugin?.skills.map((skill) => (
              <div
                key={skill.name}
                className="flex items-center justify-between p-3 rounded-lg border bg-muted/30"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <code className="text-sm font-mono">{skill.invocation}</code>
                    {skill.is_init && (
                      <span className="text-xs px-1.5 py-0.5 bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 rounded">
                        init
                      </span>
                    )}
                    {skill.is_settings && (
                      <span className="text-xs px-1.5 py-0.5 bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300 rounded">
                        settings
                      </span>
                    )}
                  </div>
                </div>
                <Button variant="outline" size="sm" onClick={() => handleCopySkill(skill)}>
                  Copy
                </Button>
              </div>
            ))}
          </div>
          <DialogFooter>
            <p className="text-xs text-muted-foreground mr-auto">
              Paste in your Claude Code terminal to run
            </p>
            <Button variant="outline" onClick={() => setSkillsDialogPlugin(null)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
