/**
 * Updates Page
 *
 * Shows available updates for Claude Code, TARS app, and plugins.
 * Checks on startup and polls periodically (every 10 minutes).
 */

import { useQuery } from '@tanstack/react-query';
import {
  RefreshCw,
  Download,
  CheckCircle,
  AlertCircle,
  ExternalLink,
  ChevronDown,
  ChevronRight,
  Terminal,
  Package,
  Sparkles,
} from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import {
  getClaudeVersionInfo,
  fetchClaudeChangelog,
  checkPluginUpdates,
  checkTarsUpdate,
  installTarsUpdate,
} from '../lib/ipc';
import type { ChangelogEntry, PluginUpdateInfo } from '../lib/types';
import { Button } from '../components/ui/button';

// Poll interval: 10 minutes
const POLL_INTERVAL = 10 * 60 * 1000;

export function UpdatesPage() {
  const [expandedVersions, setExpandedVersions] = useState<Set<string>>(new Set());
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Fetch version info with polling
  const {
    data: versionInfo,
    isLoading: loadingVersion,
    refetch: refetchVersion,
    dataUpdatedAt: versionUpdatedAt,
  } = useQuery({
    queryKey: ['claude-version-info'],
    queryFn: getClaudeVersionInfo,
    refetchInterval: POLL_INTERVAL,
    staleTime: POLL_INTERVAL - 60000, // Consider stale 1 minute before next poll
  });

  // Fetch changelog with polling
  const {
    data: changelog,
    isLoading: loadingChangelog,
    refetch: refetchChangelog,
  } = useQuery({
    queryKey: ['claude-changelog'],
    queryFn: fetchClaudeChangelog,
    refetchInterval: POLL_INTERVAL,
    staleTime: POLL_INTERVAL - 60000,
  });

  // Fetch plugin updates with polling
  const {
    data: pluginUpdates,
    isLoading: loadingPlugins,
    refetch: refetchPlugins,
  } = useQuery({
    queryKey: ['plugin-updates'],
    queryFn: checkPluginUpdates,
    refetchInterval: POLL_INTERVAL,
    staleTime: POLL_INTERVAL - 60000,
  });

  // Fetch TARS app update info with polling
  const {
    data: tarsUpdate,
    isLoading: loadingTars,
    refetch: refetchTars,
  } = useQuery({
    queryKey: ['tars-update'],
    queryFn: checkTarsUpdate,
    refetchInterval: POLL_INTERVAL,
    staleTime: POLL_INTERVAL - 60000,
  });

  const [isInstalling, setIsInstalling] = useState(false);

  const handleInstallTarsUpdate = async () => {
    setIsInstalling(true);
    try {
      await installTarsUpdate();
      toast.success('Update installed!', {
        description: 'TARS will restart to apply the update.',
      });
    } catch (err) {
      toast.error('Failed to install update', {
        description: err instanceof Error ? err.message : 'Unknown error',
      });
    } finally {
      setIsInstalling(false);
    }
  };

  const toggleVersion = (version: string) => {
    setExpandedVersions((prev) => {
      const next = new Set(prev);
      if (next.has(version)) {
        next.delete(version);
      } else {
        next.add(version);
      }
      return next;
    });
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await Promise.all([refetchVersion(), refetchChangelog(), refetchPlugins(), refetchTars()]);
      toast.success('Updates checked', {
        description: 'All update sources have been refreshed',
      });
    } catch (err) {
      toast.error('Failed to check updates', {
        description: err instanceof Error ? err.message : 'Unknown error',
      });
    } finally {
      setIsRefreshing(false);
    }
  };

  const pluginsWithUpdates = pluginUpdates?.plugins_with_updates ?? 0;

  const formatLastChecked = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString();
  };

  // Get entries to show - latest 5 by default
  const changelogEntries = changelog?.entries.slice(0, 5) || [];
  const hasMoreEntries = (changelog?.entries.length || 0) > 5;

  // Find which versions are newer than installed
  const installedVersion = versionInfo?.installed_version;
  const isNewerVersion = (version: string) => {
    if (!installedVersion) return false;
    return compareVersions(version, installedVersion) > 0;
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 brushed-metal relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Updates</h2>
        </div>
        <div className="flex items-center gap-3">
          {versionUpdatedAt && (
            <span className="text-xs text-muted-foreground">
              Last checked: {formatLastChecked(versionUpdatedAt)}
            </span>
          )}
          <Button variant="outline" size="sm" onClick={handleRefresh} disabled={isRefreshing}>
            <RefreshCw className={`h-4 w-4 mr-2 ${isRefreshing ? 'animate-spin' : ''}`} />
            {isRefreshing ? 'Checking...' : 'Check Now'}
          </Button>
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-4xl mx-auto space-y-8">
          {/* Claude Code Section */}
          <section>
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 rounded-lg bg-primary/10 text-primary">
                <Terminal className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold">Claude Code</h3>
                <p className="text-sm text-muted-foreground">
                  CLI tool for AI-assisted development
                </p>
              </div>
            </div>

            {/* Version Cards */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
              {/* Installed Version */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Installed</span>
                  {versionInfo?.installed_version ? (
                    <CheckCircle className="h-4 w-4 text-green-500" />
                  ) : (
                    <AlertCircle className="h-4 w-4 text-yellow-500" />
                  )}
                </div>
                <div className="text-2xl font-mono font-bold">
                  {loadingVersion ? (
                    <span className="text-muted-foreground">Loading...</span>
                  ) : versionInfo?.installed_version ? (
                    `v${versionInfo.installed_version}`
                  ) : (
                    <span className="text-muted-foreground">Not installed</span>
                  )}
                </div>
              </div>

              {/* Latest Version */}
              <div
                className={`p-4 rounded-lg border bg-card ${
                  versionInfo?.update_available ? 'border-primary bg-primary/5' : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Latest</span>
                  {versionInfo?.update_available && (
                    <span className="flex items-center gap-1 text-xs text-primary font-medium">
                      <Sparkles className="h-3 w-3" />
                      Update available
                    </span>
                  )}
                </div>
                <div className="text-2xl font-mono font-bold">
                  {loadingVersion || loadingChangelog ? (
                    <span className="text-muted-foreground">Loading...</span>
                  ) : versionInfo?.latest_version ? (
                    `v${versionInfo.latest_version}`
                  ) : (
                    <span className="text-muted-foreground">Unknown</span>
                  )}
                </div>
              </div>
            </div>

            {/* Update Instructions */}
            {versionInfo?.update_available && (
              <div className="p-4 rounded-lg border border-primary/50 bg-primary/5 mb-6">
                <div className="flex items-start gap-3">
                  <Download className="h-5 w-5 text-primary mt-0.5" />
                  <div>
                    <p className="font-medium mb-1">Update available!</p>
                    <p className="text-sm text-muted-foreground mb-3">
                      Run the following command to update Claude Code:
                    </p>
                    <code className="block px-3 py-2 bg-secondary text-secondary-foreground rounded font-mono text-sm">
                      npm update -g @anthropic-ai/claude-code
                    </code>
                  </div>
                </div>
              </div>
            )}

            {/* Changelog */}
            <div className="border border-border rounded-lg overflow-hidden">
              <div className="px-4 py-3 bg-muted/30 border-b border-border flex items-center justify-between">
                <h4 className="font-medium">Changelog</h4>
                <a
                  href="https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-primary hover:underline flex items-center gap-1"
                >
                  View on GitHub
                  <ExternalLink className="h-3 w-3" />
                </a>
              </div>

              {loadingChangelog ? (
                <div className="p-8 text-center">
                  <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground mx-auto mb-2" />
                  <p className="text-sm text-muted-foreground">Loading changelog...</p>
                </div>
              ) : changelogEntries.length > 0 ? (
                <>
                  <div className="divide-y divide-border">
                    {changelogEntries.map((entry) => (
                      <ChangelogEntryItem
                        key={entry.version}
                        entry={entry}
                        isExpanded={expandedVersions.has(entry.version)}
                        onToggle={() => toggleVersion(entry.version)}
                        isNewer={isNewerVersion(entry.version)}
                        isCurrent={entry.version === installedVersion}
                      />
                    ))}
                  </div>
                  {hasMoreEntries && (
                    <div className="px-4 py-3 border-t border-border bg-muted/20">
                      <a
                        href="https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-primary hover:underline flex items-center justify-center gap-2"
                      >
                        View full changelog on GitHub
                        <ExternalLink className="h-3.5 w-3.5" />
                      </a>
                    </div>
                  )}
                </>
              ) : (
                <div className="p-8 text-center text-muted-foreground">
                  <p>Failed to load changelog</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={handleRefresh}>
                    Retry
                  </Button>
                </div>
              )}
            </div>
          </section>

          {/* TARS App Section */}
          <section>
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 rounded-lg bg-primary/10 text-primary">
                <Package className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold">TARS Desktop</h3>
                <p className="text-sm text-muted-foreground">Configuration manager app</p>
              </div>
            </div>

            {/* Version Cards */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
              {/* Current Version */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Installed</span>
                  <CheckCircle className="h-4 w-4 text-green-500" />
                </div>
                <div className="text-2xl font-mono font-bold">
                  {loadingTars ? (
                    <span className="text-muted-foreground">Loading...</span>
                  ) : (
                    `v${tarsUpdate?.current_version || '0.0.0'}`
                  )}
                </div>
              </div>

              {/* Latest Version */}
              <div
                className={`p-4 rounded-lg border bg-card ${
                  tarsUpdate?.update_available ? 'border-primary bg-primary/5' : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Latest</span>
                  {tarsUpdate?.update_available && (
                    <span className="flex items-center gap-1 text-xs text-primary font-medium">
                      <Sparkles className="h-3 w-3" />
                      Update available
                    </span>
                  )}
                </div>
                <div className="text-2xl font-mono font-bold">
                  {loadingTars ? (
                    <span className="text-muted-foreground">Loading...</span>
                  ) : tarsUpdate?.latest_version ? (
                    `v${tarsUpdate.latest_version}`
                  ) : (
                    <span className="text-green-600 dark:text-green-400 text-lg">Up to date</span>
                  )}
                </div>
              </div>
            </div>

            {/* Update Button */}
            {tarsUpdate?.update_available && (
              <div className="p-4 rounded-lg border border-primary/50 bg-primary/5 mb-6">
                <div className="flex items-start gap-3">
                  <Download className="h-5 w-5 text-primary mt-0.5" />
                  <div className="flex-1">
                    <p className="font-medium mb-1">
                      TARS v{tarsUpdate.latest_version} is available!
                    </p>
                    {tarsUpdate.release_notes && (
                      <p className="text-sm text-muted-foreground mb-3">
                        {tarsUpdate.release_notes.slice(0, 200)}
                        {tarsUpdate.release_notes.length > 200 ? '...' : ''}
                      </p>
                    )}
                    <Button
                      onClick={handleInstallTarsUpdate}
                      disabled={isInstalling}
                      className="gap-2"
                    >
                      {isInstalling ? (
                        <>
                          <RefreshCw className="h-4 w-4 animate-spin" />
                          Installing...
                        </>
                      ) : (
                        <>
                          <Download className="h-4 w-4" />
                          Download & Install
                        </>
                      )}
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </section>

          {/* Plugin Updates Section */}
          <section>
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 rounded-lg bg-primary/10 text-primary">
                <Sparkles className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold">Plugin Updates</h3>
                <p className="text-sm text-muted-foreground">
                  {pluginUpdates
                    ? `${pluginUpdates.total_plugins} plugins installed`
                    : 'Checking installed plugins...'}
                </p>
              </div>
              {pluginsWithUpdates > 0 && (
                <span className="ml-auto text-xs px-2 py-1 rounded-full bg-primary text-primary-foreground">
                  {pluginsWithUpdates} update{pluginsWithUpdates > 1 ? 's' : ''} available
                </span>
              )}
            </div>

            {loadingPlugins ? (
              <div className="p-8 text-center border border-border rounded-lg">
                <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">Checking for plugin updates...</p>
              </div>
            ) : pluginUpdates && pluginUpdates.updates.length > 0 ? (
              <div className="border border-border rounded-lg overflow-hidden">
                <div className="divide-y divide-border">
                  {pluginUpdates.updates.map((plugin) => (
                    <PluginUpdateItem
                      key={`${plugin.plugin_id}@${plugin.marketplace}`}
                      plugin={plugin}
                    />
                  ))}
                </div>
              </div>
            ) : pluginUpdates ? (
              <div className="p-6 text-center border border-border rounded-lg text-muted-foreground">
                <Package className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p className="text-sm">No plugins installed from marketplaces</p>
              </div>
            ) : (
              <div className="p-6 text-center border border-border rounded-lg text-muted-foreground">
                <AlertCircle className="h-8 w-8 mx-auto mb-2 opacity-50" />
                <p className="text-sm">Failed to check plugin updates</p>
                <Button variant="outline" size="sm" className="mt-2" onClick={handleRefresh}>
                  Retry
                </Button>
              </div>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}

interface ChangelogEntryItemProps {
  entry: ChangelogEntry;
  isExpanded: boolean;
  onToggle: () => void;
  isNewer: boolean;
  isCurrent: boolean;
}

function ChangelogEntryItem({
  entry,
  isExpanded,
  onToggle,
  isNewer,
  isCurrent,
}: ChangelogEntryItemProps) {
  return (
    <div className={isNewer ? 'bg-primary/5' : ''}>
      <button
        type="button"
        onClick={onToggle}
        className="w-full px-4 py-3 flex items-center gap-3 hover:bg-muted/50 transition-colors text-left"
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
        )}
        <span className="font-mono font-medium">v{entry.version}</span>
        {isNewer && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-primary text-primary-foreground">
            New
          </span>
        )}
        {isCurrent && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-green-500/20 text-green-600 dark:text-green-400">
            Installed
          </span>
        )}
      </button>
      {isExpanded && (
        <div className="px-4 pb-4 pl-11">
          <div className="prose prose-sm dark:prose-invert max-w-none">
            <ChangelogContent content={entry.content} />
          </div>
        </div>
      )}
    </div>
  );
}

function ChangelogContent({ content }: { content: string }) {
  // Simple markdown rendering for changelog entries
  const lines = content.split('\n');

  return (
    <ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground">
      {lines
        .filter((line) => line.trim().startsWith('-') || line.trim().startsWith('*'))
        .map((line, i) => {
          const text = line.trim().replace(/^[-*]\s*/, '');
          // Highlight platform tags
          const highlighted = text.replace(
            /\[(VSCode|Windows|macOS|Linux)\]/g,
            '<span class="text-xs px-1 py-0.5 rounded bg-muted text-muted-foreground font-medium">$1</span>'
          );
          return (
            <li
              key={i}
              className="text-foreground"
              dangerouslySetInnerHTML={{ __html: highlighted }}
            />
          );
        })}
    </ul>
  );
}

function PluginUpdateItem({ plugin }: { plugin: PluginUpdateInfo }) {
  return (
    <div className={`px-4 py-3 ${plugin.update_available ? 'bg-primary/5' : ''}`}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Package className="h-4 w-4 text-muted-foreground" />
          <div>
            <div className="flex items-center gap-2">
              <span className="font-medium">{plugin.plugin_name}</span>
              {plugin.update_available && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-primary text-primary-foreground">
                  Update
                </span>
              )}
            </div>
            <div className="text-xs text-muted-foreground">from {plugin.marketplace}</div>
          </div>
        </div>
        <div className="text-right">
          <div className="font-mono text-sm">
            {plugin.update_available ? (
              <>
                <span className="text-muted-foreground">{plugin.installed_version}</span>
                <span className="mx-2 text-muted-foreground">→</span>
                <span className="text-primary font-medium">{plugin.available_version}</span>
              </>
            ) : (
              <span className="text-muted-foreground">{plugin.installed_version}</span>
            )}
          </div>
          {!plugin.update_available && (
            <div className="text-xs text-green-600 dark:text-green-400">Up to date</div>
          )}
        </div>
      </div>
    </div>
  );
}

/** Compare semantic versions. Returns positive if a > b, negative if a < b, 0 if equal */
function compareVersions(a: string, b: string): number {
  const parseVersion = (s: string): number[] => s.split('.').map((part) => parseInt(part, 10) || 0);

  const va = parseVersion(a);
  const vb = parseVersion(b);

  for (let i = 0; i < Math.max(va.length, vb.length); i++) {
    const pa = va[i] || 0;
    const pb = vb[i] || 0;
    if (pa !== pb) return pa - pb;
  }

  return 0;
}
