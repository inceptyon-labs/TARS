import { useCallback, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { openPath, openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { useNavigate } from 'react-router-dom';
import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  FileCode2,
  FolderOpen,
  RefreshCw,
  Terminal,
  XCircle,
} from 'lucide-react';
import { toast } from 'sonner';
import { ProviderLogo } from '../components/ProviderLogo';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { getRuntimeStatuses, scanUserScope } from '../lib/ipc';
import type { Inventory, RuntimeStatus } from '../lib/types';
import { cn } from '../lib/utils';

const runtimeAccents: Record<string, string> = {
  'claude-code': 'border-amber-500/20 bg-amber-500/8',
  codex: 'border-emerald-500/20 bg-emerald-500/8',
  'gemini-cli': 'border-sky-500/20 bg-sky-500/8',
};

function runtimeProvider(runtimeId: string) {
  if (runtimeId === 'claude-code') return { providerId: 'claude', providerName: 'Claude' };
  if (runtimeId === 'codex') return { providerId: 'openai', providerName: 'OpenAI' };
  return { providerId: 'gemini', providerName: 'Google Gemini' };
}

function statusLabel(runtime: RuntimeStatus) {
  if (!runtime.installed) {
    return 'Not found';
  }

  return runtime.version ? `Installed ${runtime.version}` : 'Installed';
}

function runtimeDetailSummary(runtime: RuntimeStatus) {
  const foundCount = runtime.paths.filter((item) => item.exists).length;
  return `${foundCount} of ${runtime.paths.length} config surfaces found`;
}

function inferredInstallMethod(runtime: RuntimeStatus): string | null {
  if (runtime.install_method) {
    return runtime.install_method === 'Volta' ? 'npm' : runtime.install_method;
  }

  const binaryPath = runtime.binary_path?.replace(/\\/g, '/').toLowerCase() ?? '';
  if (!binaryPath) return null;
  if (binaryPath.includes('/.volta/')) return 'npm';
  if (binaryPath.includes('/opt/homebrew/') || binaryPath.includes('/home/linuxbrew/.linuxbrew/')) {
    return 'Homebrew';
  }
  if (
    binaryPath.includes('/appdata/roaming/npm/') ||
    binaryPath.includes('/.npm-global/') ||
    binaryPath.includes('/.local/bin/') ||
    binaryPath.includes('/.nvm/versions/node/')
  ) {
    return 'npm';
  }
  return null;
}

function runtimeInstallSummary(runtime: RuntimeStatus) {
  const status = statusLabel(runtime);
  const installMethod = inferredInstallMethod(runtime);
  if (!runtime.installed || !installMethod) {
    return status;
  }
  return `${status} via ${installMethod}`;
}

function runtimeDiscoverySummary(runtimeId: string, inventory: Inventory | undefined) {
  if (!inventory) {
    return null;
  }

  if (runtimeId === 'codex') {
    const codex = inventory.user_scope.codex;
    const managedCodex = inventory.managed_scope?.codex;
    const files = [
      codex.config
        ? {
            label: 'User config',
            path: codex.config.path,
          }
        : null,
      ...codex.instructions.map((file) => ({
        label: 'Instruction layer',
        path: file.path,
      })),
      ...codex.marketplaces.map((marketplace) => ({
        label: 'Marketplace index',
        path: marketplace.path,
      })),
      managedCodex?.system_config
        ? {
            label: 'System config',
            path: managedCodex.system_config.path,
          }
        : null,
      managedCodex?.managed_config
        ? {
            label: 'Managed config',
            path: managedCodex.managed_config.path,
          }
        : null,
    ].filter(Boolean) as Array<{ label: string; path: string }>;

    const pluginCount = codex.marketplaces.reduce(
      (total, marketplace) => total + marketplace.plugins.length,
      0
    );

    const stats = [
      { label: 'Skills', value: `${codex.skills.length} discovered` },
      { label: 'Custom agents', value: `${codex.agents.length} discovered` },
      { label: 'Instruction layers', value: `${codex.instructions.length} discovered` },
      { label: 'Marketplace files', value: `${codex.marketplaces.length} discovered` },
      { label: 'Plugins', value: `${pluginCount} surfaced` },
    ];

    return { title: 'Detected user inventory', stats, files };
  }

  if (runtimeId === 'gemini-cli') {
    return null;
  }

  const files = [
    inventory.user_scope.settings
      ? {
          label: 'User settings',
          path: inventory.user_scope.settings.path,
        }
      : null,
    inventory.user_scope.mcp
      ? {
          label: 'User MCP',
          path: inventory.user_scope.mcp.path,
        }
      : null,
  ].filter(Boolean) as Array<{ label: string; path: string }>;

  const stats = [
    { label: 'Skills', value: `${inventory.user_scope.skills.length} discovered` },
    { label: 'Commands', value: `${inventory.user_scope.commands.length} discovered` },
    { label: 'Agents', value: `${inventory.user_scope.agents.length} discovered` },
    {
      label: 'MCP servers',
      value: `${inventory.user_scope.mcp?.servers.length ?? 0} configured`,
    },
  ];

  return { title: 'Detected user inventory', stats, files };
}

export function RuntimesPage() {
  const navigate = useNavigate();
  const [expandedRuntimeIds, setExpandedRuntimeIds] = useState<Set<string>>(new Set());
  const {
    data: runtimes = [],
    isLoading,
    refetch,
    isFetching,
  } = useQuery({
    queryKey: ['runtime-statuses'],
    queryFn: getRuntimeStatuses,
  });
  const {
    data: inventory,
    refetch: refetchInventory,
    isFetching: isFetchingInventory,
  } = useQuery({
    queryKey: ['runtime-user-scope'],
    queryFn: scanUserScope,
  });

  const openDocs = useCallback(async (url: string) => {
    try {
      await openUrl(url);
    } catch (err) {
      toast.error('Failed to open runtime docs');
      console.error('Failed to open runtime docs:', err);
    }
  }, []);

  const openRuntimePath = useCallback(async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch (err) {
      try {
        await openPath(path);
      } catch (fallbackErr) {
        toast.error('Failed to open path');
        console.error('Failed to open runtime path:', err, fallbackErr);
      }
    }
  }, []);

  const revealRuntimePath = useCallback(async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch (err) {
      toast.error('Failed to reveal path');
      console.error('Failed to reveal runtime path:', err);
    }
  }, []);

  const toggleRuntimeDetails = useCallback((runtimeId: string) => {
    setExpandedRuntimeIds((prev) => {
      const next = new Set(prev);
      if (next.has(runtimeId)) {
        next.delete(runtimeId);
      } else {
        next.add(runtimeId);
      }
      return next;
    });
  }, []);

  return (
    <div className="h-full flex flex-col">
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Runtimes</h2>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            void Promise.all([refetch(), refetchInventory()]);
          }}
          disabled={isFetching || isFetchingInventory}
          className="text-muted-foreground"
        >
          <RefreshCw
            className={cn('h-4 w-4', (isFetching || isFetchingInventory) && 'animate-spin')}
          />
          Rescan
        </Button>
      </header>

      <div className="flex-1 overflow-auto">
        <section className="border-b border-border bg-muted/20 px-6 py-5">
          <div className="max-w-6xl">
            <div className="flex flex-wrap items-center gap-2 mb-2">
              <Badge variant="outline">Claude</Badge>
              <Badge variant="outline">Codex</Badge>
              <Badge variant="outline">Gemini</Badge>
            </div>
            <p className="text-sm text-muted-foreground max-w-3xl">
              Runtime health, install channel, config surfaces, and editor entry points for Claude
              Code, Codex, and Gemini CLI live here, with direct links into runtime-specific
              settings.
            </p>
          </div>
        </section>

        <main className="p-6 space-y-6 max-w-7xl">
          {isLoading ? (
            <div className="flex h-48 items-center justify-center text-muted-foreground">
              <RefreshCw className="h-5 w-5 animate-spin mr-2" />
              Scanning runtimes...
            </div>
          ) : (
            <div className="grid gap-4 xl:grid-cols-2">
              {runtimes.map((runtime) => {
                const binaryPath = runtime.binary_path;
                const discovery = runtimeDiscoverySummary(runtime.id, inventory);
                const detailsExpanded = expandedRuntimeIds.has(runtime.id);
                const provider = runtimeProvider(runtime.id);

                return (
                  <section
                    key={runtime.id}
                    className="rounded-md border border-border bg-card/70 overflow-hidden"
                  >
                    <div className="border-b border-border p-5">
                      <div className="flex items-start justify-between gap-4">
                        <div className="min-w-0">
                          <div className="flex items-center gap-3">
                            <div
                              className={cn(
                                'h-10 w-10 rounded-md border flex items-center justify-center',
                                runtimeAccents[runtime.id] ?? 'border-primary/20 bg-primary/8'
                              )}
                            >
                              <ProviderLogo
                                providerId={provider.providerId}
                                providerName={provider.providerName}
                                className="h-5 w-5 object-contain"
                              />
                            </div>
                            <div>
                              <h3 className="font-semibold text-base">{runtime.name}</h3>
                              <p className="text-xs text-muted-foreground">
                                {runtimeInstallSummary(runtime)}
                              </p>
                            </div>
                          </div>
                          <p className="text-sm text-muted-foreground mt-4">{runtime.summary}</p>
                          {runtime.version && (
                            <div className="mt-3 flex flex-wrap gap-2">
                              <Badge variant="outline">v{runtime.version}</Badge>
                            </div>
                          )}
                        </div>

                        <Badge
                          variant={runtime.installed ? 'default' : 'secondary'}
                          className={cn(
                            runtime.installed
                              ? 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30'
                              : 'bg-muted text-muted-foreground'
                          )}
                        >
                          {runtime.installed ? (
                            <CheckCircle2 className="h-3.5 w-3.5 mr-1" />
                          ) : (
                            <XCircle className="h-3.5 w-3.5 mr-1" />
                          )}
                          {runtime.installed ? 'Ready' : 'Missing'}
                        </Badge>
                      </div>

                      <div className="mt-4 flex flex-wrap gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => openDocs(runtime.docs_url)}
                        >
                          <ExternalLink className="h-4 w-4" />
                          Docs
                        </Button>
                        {binaryPath && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => revealRuntimePath(binaryPath)}
                          >
                            <Terminal className="h-4 w-4" />
                            Binary
                          </Button>
                        )}
                        {(runtime.id === 'claude-code' || runtime.id === 'codex') && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() =>
                              navigate(
                                runtime.id === 'claude-code'
                                  ? '/runtimes/claude-code'
                                  : '/runtimes/codex',
                                {
                                  state: {
                                    returnTo: '/runtimes',
                                    returnLabel: 'Back to Runtimes',
                                  },
                                }
                              )
                            }
                          >
                            <FileCode2 className="h-4 w-4" />
                            Settings
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() =>
                            navigate('/updates', {
                              state: {
                                returnTo: '/runtimes',
                                returnLabel: 'Back to Runtimes',
                              },
                            })
                          }
                        >
                          <ExternalLink className="h-4 w-4" />
                          Updates
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => toggleRuntimeDetails(runtime.id)}
                        >
                          {detailsExpanded ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                          {detailsExpanded ? 'Hide details' : 'Show details'}
                        </Button>
                      </div>
                    </div>
                    <div className="border-t border-border px-5 py-3 bg-muted/10">
                      <p className="text-xs text-muted-foreground">
                        {runtimeDetailSummary(runtime)}
                      </p>
                    </div>

                    {detailsExpanded && (
                      <>
                        {discovery && (
                          <div className="border-t border-border p-5">
                            <div className="flex items-center justify-between gap-3 mb-3">
                              <p className="text-sm font-medium">{discovery.title}</p>
                              <Badge variant="outline" className="text-[10px]">
                                Live scan
                              </Badge>
                            </div>

                            <div className="grid gap-2 sm:grid-cols-2">
                              {discovery.stats.map((item) => (
                                <div
                                  key={`${runtime.id}-${item.label}`}
                                  className="rounded-md border border-border/70 bg-muted/20 px-3 py-2"
                                >
                                  <p className="text-[11px] uppercase tracking-wide text-muted-foreground">
                                    {item.label}
                                  </p>
                                  <p className="text-sm font-medium mt-1">{item.value}</p>
                                </div>
                              ))}
                            </div>

                            {discovery.files.length ? (
                              <div className="mt-3 space-y-2">
                                {discovery.files.map((item) => (
                                  <div
                                    key={`${runtime.id}-${item.label}-${item.path}`}
                                    className="flex items-center gap-3 rounded-md border border-border/70 px-3 py-2"
                                  >
                                    <FileCode2 className="h-4 w-4 text-muted-foreground" />
                                    <div className="min-w-0 flex-1">
                                      <p className="text-sm font-medium">{item.label}</p>
                                      <p className="text-xs text-muted-foreground truncate">
                                        {item.path}
                                      </p>
                                    </div>
                                    <Button
                                      variant="ghost"
                                      size="icon-sm"
                                      onClick={() => openRuntimePath(item.path)}
                                      title={`Open ${item.label}`}
                                    >
                                      <FolderOpen className="h-4 w-4" />
                                    </Button>
                                  </div>
                                ))}
                              </div>
                            ) : null}
                          </div>
                        )}

                        <div className="divide-y divide-border border-t border-border">
                          {runtime.paths.map((item) => (
                            <div
                              key={`${runtime.id}-${item.label}`}
                              className="flex items-center gap-3 px-5 py-3"
                            >
                              <div className="text-muted-foreground">
                                {item.kind === 'directory' ? (
                                  <FolderOpen className="h-4 w-4" />
                                ) : (
                                  <FileCode2 className="h-4 w-4" />
                                )}
                              </div>
                              <div className="min-w-0 flex-1">
                                <div className="flex items-center gap-2">
                                  <span className="text-sm font-medium">{item.label}</span>
                                  <Badge
                                    variant="outline"
                                    className={cn(
                                      'text-[10px] h-5 px-1.5',
                                      item.exists
                                        ? 'border-emerald-500/30 text-emerald-300'
                                        : 'border-muted-foreground/30 text-muted-foreground'
                                    )}
                                  >
                                    {item.exists ? 'Found' : 'Not found'}
                                  </Badge>
                                </div>
                                <p className="text-xs text-muted-foreground truncate">
                                  {item.path}
                                </p>
                              </div>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                disabled={!item.exists}
                                onClick={() => openRuntimePath(item.path)}
                                title={`Open ${item.label}`}
                              >
                                <FolderOpen className="h-4 w-4" />
                              </Button>
                            </div>
                          ))}
                        </div>
                      </>
                    )}
                  </section>
                );
              })}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
