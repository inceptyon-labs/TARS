import { useCallback } from 'react';
import { useQuery } from '@tanstack/react-query';
import { openPath, openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import {
  Bot,
  Boxes,
  CheckCircle2,
  ExternalLink,
  FileCode2,
  FolderOpen,
  RefreshCw,
  Terminal,
  XCircle,
} from 'lucide-react';
import { toast } from 'sonner';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { getRuntimeStatuses } from '../lib/ipc';
import type { RuntimeStatus } from '../lib/types';
import { cn } from '../lib/utils';

const runtimeAccents: Record<string, string> = {
  'claude-code': 'border-amber-500/30 bg-amber-500/10 text-amber-300',
  codex: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300',
};

function RuntimeIcon({ runtimeId }: { runtimeId: string }) {
  if (runtimeId === 'codex') {
    return <Boxes className="h-5 w-5" />;
  }

  return <Bot className="h-5 w-5" />;
}

function statusLabel(runtime: RuntimeStatus) {
  if (!runtime.installed) {
    return 'Not found';
  }

  return runtime.version ? `Installed ${runtime.version}` : 'Installed';
}

export function RuntimesPage() {
  const {
    data: runtimes = [],
    isLoading,
    refetch,
    isFetching,
  } = useQuery({
    queryKey: ['runtime-statuses'],
    queryFn: getRuntimeStatuses,
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
      await openPath(path);
    } catch (err) {
      toast.error('Failed to open path');
      console.error('Failed to open runtime path:', err);
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
          onClick={() => refetch()}
          disabled={isFetching}
          className="text-muted-foreground"
        >
          <RefreshCw className={cn('h-4 w-4', isFetching && 'animate-spin')} />
          Rescan
        </Button>
      </header>

      <div className="flex-1 overflow-auto">
        <section className="border-b border-border bg-muted/20 px-6 py-5">
          <div className="max-w-6xl">
            <div className="flex flex-wrap items-center gap-2 mb-2">
              <Badge variant="outline">Claude</Badge>
              <Badge variant="outline">Codex</Badge>
              <Badge variant="secondary">Foundation slice</Badge>
            </div>
            <p className="text-sm text-muted-foreground max-w-3xl">
              TARS is expanding into a multi-runtime control plane. This first slice detects local
              agent clients and shows the config surfaces that future Inventory and Bundle flows
              will manage.
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
              {runtimes.map((runtime) => (
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
                              runtimeAccents[runtime.id] ??
                                'border-primary/30 bg-primary/10 text-primary'
                            )}
                          >
                            <RuntimeIcon runtimeId={runtime.id} />
                          </div>
                          <div>
                            <h3 className="font-semibold text-base">{runtime.name}</h3>
                            <p className="text-xs text-muted-foreground">{statusLabel(runtime)}</p>
                          </div>
                        </div>
                        <p className="text-sm text-muted-foreground mt-4">{runtime.summary}</p>
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
                      {runtime.binary_path && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => revealRuntimePath(runtime.binary_path!)}
                        >
                          <Terminal className="h-4 w-4" />
                          Binary
                        </Button>
                      )}
                    </div>
                  </div>

                  <div className="divide-y divide-border">
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
                          <p className="text-xs text-muted-foreground truncate">{item.path}</p>
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
                </section>
              ))}
            </div>
          )}

          <section className="rounded-md border border-border bg-card/50 p-5">
            <div className="flex items-center gap-2 mb-3">
              <Boxes className="h-4 w-4 text-primary" />
              <h3 className="font-semibold">Next UI Moves</h3>
            </div>
            <div className="grid gap-3 md:grid-cols-3">
              <div className="rounded-md border border-border/70 p-4">
                <p className="text-sm font-medium">Inventory</p>
                <p className="text-xs text-muted-foreground mt-1">
                  One browser for skills, roles, MCP servers, commands, hooks, and instructions.
                </p>
              </div>
              <div className="rounded-md border border-border/70 p-4">
                <p className="text-sm font-medium">Compatibility</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Runtime badges plus Native, Convertible, Partial, and Unsupported states.
                </p>
              </div>
              <div className="rounded-md border border-border/70 p-4">
                <p className="text-sm font-medium">Bundles</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Profile evolution into reusable exports for Claude Code and Codex plugins.
                </p>
              </div>
            </div>
          </section>
        </main>
      </div>
    </div>
  );
}
