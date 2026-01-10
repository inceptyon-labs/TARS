import { useQuery } from '@tanstack/react-query';
import {
  Box,
  Check,
  ChevronDown,
  ChevronRight,
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
  Power,
  PowerOff,
} from 'lucide-react';
import { HelpButton } from '../components/HelpButton';
import { useState } from 'react';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { scanUserScope, listProjects } from '../lib/ipc';
import type { AvailablePlugin, CacheStatusResponse, Marketplace, PluginInventory, PluginSkillInfo } from '../lib/types';
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
import { ConfirmDialog } from '../components/config/ConfirmDialog';

export function PluginsPage() {
  const [showAddMarketplace, setShowAddMarketplace] = useState(false);
  const [marketplaceSource, setMarketplaceSource] = useState('');
  const [addingMarketplace, setAddingMarketplace] = useState(false);
  const [marketplaceToRemove, setMarketplaceToRemove] = useState<string | null>(null);
  const [removingMarketplace, setRemovingMarketplace] = useState(false);
  const [updatingMarketplaces, setUpdatingMarketplaces] = useState(false);
  const [selectedMarketplace, setSelectedMarketplace] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [installingPlugin, setInstallingPlugin] = useState<string | null>(null);
  const [availablePluginsExpanded, setAvailablePluginsExpanded] = useState(false);

  // Install dialog state
  const [showInstallDialog, setShowInstallDialog] = useState(false);
  const [pendingInstall, setPendingInstall] = useState<{ pluginId: string; marketplace: string } | null>(null);
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

  // Get cache status
  const {
    data: cacheStatus,
    refetch: refetchCache,
  } = useQuery({
    queryKey: ['cache-status'],
    queryFn: () => invoke<CacheStatusResponse>('cache_status'),
  });

  const plugins: PluginInventory = inventory?.plugins || { marketplaces: [], installed: [] };

  // Sort marketplaces by name for stable ordering
  const sortedMarketplaces = [...plugins.marketplaces].sort((a, b) => a.name.localeCompare(b.name));

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

  async function handleRemoveMarketplace() {
    if (!marketplaceToRemove) return;

    setRemovingMarketplace(true);
    try {
      await invoke('plugin_marketplace_remove', { name: marketplaceToRemove });
      toast.success('Marketplace removed', {
        description: `Removed ${marketplaceToRemove}`,
      });
      await refetch();
    } catch (err) {
      toast.error('Failed to remove marketplace', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setRemovingMarketplace(false);
      setMarketplaceToRemove(null);
    }
  }

  async function handleUpdateMarketplaces() {
    setUpdatingMarketplaces(true);
    try {
      await invoke('plugin_marketplace_update', { name: null });
      toast.success('Marketplaces updated');
      await refetch();
    } catch (err) {
      toast.error('Failed to update marketplaces', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setUpdatingMarketplaces(false);
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

  async function handleUninstallPlugin(pluginId: string, scope: string) {
    try {
      await invoke('plugin_uninstall', { plugin: pluginId, scope });
      toast.success(`Uninstalled ${pluginId}`);
      await refetch();
    } catch (err) {
      toast.error('Failed to uninstall plugin', {
        description: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleTogglePlugin(pluginId: string, marketplace: string | null, enabled: boolean) {
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
        await invoke('plugin_install', {
          plugin: pluginSpec,
          scope: 'user',
          projectPath: null,
        });
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

        const results = await Promise.allSettled(
          selectedProjects.map(async (projectPath) => {
            await invoke('plugin_install', {
              plugin: pluginSpec,
              scope: installScope,
              projectPath,
            });
            return projectPath;
          })
        );

        const successes = results.filter((r) => r.status === 'fulfilled').length;
        const failures = results.filter((r) => r.status === 'rejected').length;

        if (failures === 0) {
          toast.success(`Installed ${pluginId}`, {
            description: `Added to ${successes} project${successes > 1 ? 's' : ''} (${installScope} scope)`,
          });
        } else if (successes > 0) {
          toast.warning(`Partially installed ${pluginId}`, {
            description: `${successes} succeeded, ${failures} failed`,
          });
        } else {
          toast.error(`Failed to install ${pluginId}`);
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
      prev.includes(projectPath)
        ? prev.filter((p) => p !== projectPath)
        : [...prev, projectPath]
    );
  }

  async function handleCleanCache() {
    setCleaningCache(true);
    try {
      const result = await invoke<{ deleted_count: number; deleted_size_formatted: string; errors: string[] }>('cache_clean');
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

    if (diffDays === 0) return 'Today';
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
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 brushed-metal relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Plugins</h2>
          <HelpButton section="PLUGINS" />
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6 space-y-6">
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
                          <span className="text-sm text-muted-foreground truncate max-w-[300px]">
                            {getSourceDisplay(marketplace)}
                          </span>
                          {marketplace.source_type.type === 'GitHub' && (
                            <a
                              href={`https://github.com/${marketplace.location}`}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="text-muted-foreground hover:text-foreground"
                            >
                              <ExternalLink className="h-3 w-3" />
                            </a>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <button
                          type="button"
                          onClick={() => handleToggleAutoUpdate(marketplace.name, marketplace.auto_update)}
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
                          onClick={() => setMarketplaceToRemove(marketplace.name)}
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
            {plugins.installed.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Box className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p>No plugins installed</p>
                <p className="text-xs mt-1">
                  Browse available plugins below to install
                </p>
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
                  {plugins.installed.map((plugin) => {
                    const skills = plugin.manifest.parsed_skills || [];

                    return (
                      <TableRow key={`${plugin.id}-${plugin.marketplace}`} className={!plugin.enabled ? 'opacity-60' : ''}>
                        <TableCell>
                          <div>
                            <span className="font-medium">{plugin.id}</span>
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
                          <span
                            className="text-xs text-muted-foreground cursor-help"
                            title={`Installed: ${formatFullDate(plugin.installed_at)}\nLast updated: ${formatFullDate(plugin.last_updated)}`}
                          >
                            {formatRelativeDate(plugin.last_updated)}
                          </span>
                        </TableCell>
                        <TableCell>
                          <span
                            className="text-xs px-2 py-0.5 bg-muted rounded"
                            title={plugin.scope.type === 'User'
                              ? 'User scope - available everywhere'
                              : plugin.scope.type === 'Project'
                              ? 'Project scope - specific to a project'
                              : plugin.scope.type === 'Local'
                              ? 'Local scope - project-specific, not tracked by git'
                              : 'Managed scope - controlled by system admin'}
                          >
                            {plugin.scope.type.toLowerCase()}
                          </span>
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
                              onClick={() => handleTogglePlugin(plugin.id, plugin.marketplace, plugin.enabled)}
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
                              onClick={() => handleUninstallPlugin(plugin.id, plugin.scope.type.toLowerCase())}
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
                {cacheStatus.stale_entries.length} stale plugin cache{cacheStatus.stale_entries.length !== 1 ? 's' : ''} from old versions
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
                  <CardDescription className="mt-2">Browse and install plugins from your marketplaces</CardDescription>
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
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
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
                              <p className="text-sm text-muted-foreground mt-1 truncate">
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
                            disabled={plugin.installed || installingPlugin === `${plugin.id}@${plugin.marketplace}`}
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

      </div>

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
      <ConfirmDialog
        open={!!marketplaceToRemove}
        onOpenChange={(open) => !open && setMarketplaceToRemove(null)}
        title="Remove Marketplace"
        description={`Are you sure you want to remove "${marketplaceToRemove}"? Plugins installed from this marketplace will remain installed.`}
        confirmLabel="Remove"
        confirmVariant="destructive"
        onConfirm={handleRemoveMarketplace}
        loading={removingMarketplace}
      />

      {/* Install Plugin Dialog */}
      <Dialog open={showInstallDialog} onOpenChange={(open) => {
        if (!open) {
          setShowInstallDialog(false);
          setPendingInstall(null);
        }
      }}>
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
                      <p className="text-xs mt-1">
                        Add projects in the Projects tab first
                      </p>
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
            <Button variant="outline" onClick={() => {
              setShowInstallDialog(false);
              setPendingInstall(null);
            }}>
              Cancel
            </Button>
            <Button
              onClick={handleConfirmInstall}
              disabled={installScope !== 'user' && selectedProjects.length === 0}
            >
              <Download className="h-4 w-4 mr-2" />
              Install{installScope !== 'user' && selectedProjects.length > 0
                ? ` to ${selectedProjects.length} project${selectedProjects.length > 1 ? 's' : ''}`
                : ''}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Skills Dialog */}
      <Dialog open={!!skillsDialogPlugin} onOpenChange={(open) => !open && setSkillsDialogPlugin(null)}>
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
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleCopySkill(skill)}
                >
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
