import { useCallback, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Boxes, ExternalLink, FileCode2, RefreshCw } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { toast } from 'sonner';
import { ConfigFileEditor } from '../components/settings/ConfigFileEditor';
import { PageBackButton } from '../components/PageBackButton';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../components/ui/tooltip';
import {
  getPlatformInfo,
  listProjects,
  readRuntimeConfigFile,
  saveRuntimeConfigFile,
  type RuntimeConfigScope,
} from '../lib/ipc';

type CodexConfigItem = {
  id: string;
  scope: RuntimeConfigScope;
  projectPath?: string | null;
  projectName?: string;
  label: string;
  subtitle: string;
};

export function CodexSettingsPage() {
  const { data: projects = [], isLoading: isLoadingProjects } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });
  const { data: platformInfo } = useQuery({
    queryKey: ['platform-info'],
    queryFn: getPlatformInfo,
  });

  const userItems = useMemo<CodexConfigItem[]>(
    () => [
      {
        id: 'codex-user-config',
        scope: 'user',
        label: 'config.toml',
        subtitle: '~/.codex/config.toml',
      },
    ],
    []
  );

  const projectItems = useMemo<CodexConfigItem[]>(
    () =>
      projects.map((project) => ({
        id: `codex-project-${project.id}`,
        scope: 'project',
        projectPath: project.path,
        projectName: project.name,
        label: 'config.toml',
        subtitle: project.path,
      })),
    [projects]
  );

  const adminItems = useMemo<CodexConfigItem[]>(() => {
    if (!platformInfo) {
      return [];
    }

    const items: CodexConfigItem[] = [];

    if (platformInfo.os === 'macos' || platformInfo.os === 'linux') {
      items.push({
        id: 'codex-system-config',
        scope: 'system',
        label: 'config.toml',
        subtitle: '/etc/codex/config.toml',
      });
    }

    if (
      platformInfo.os === 'macos' ||
      platformInfo.os === 'linux' ||
      platformInfo.os === 'windows'
    ) {
      items.push({
        id: 'codex-managed-config',
        scope: 'managed',
        label: 'managed_config.toml',
        subtitle:
          platformInfo.os === 'windows'
            ? '~/.codex/managed_config.toml'
            : '/etc/codex/managed_config.toml',
      });
    }

    return items;
  }, [platformInfo]);

  const [selectedItem, setSelectedItem] = useState<CodexConfigItem>(userItems[0]);

  const handleOpenDocs = useCallback(async () => {
    try {
      await openUrl('https://developers.openai.com/codex/config-reference');
    } catch (err) {
      toast.error('Failed to open Codex docs');
      console.error('Failed to open Codex docs:', err);
    }
  }, []);

  function renderGroup(title: string, items: CodexConfigItem[], emptyLabel: string) {
    return (
      <div className="mb-4">
        <h3 className="text-xs font-semibold text-primary uppercase tracking-wider px-3 py-2 border-b border-primary/20 mb-2">
          {title} <span className="text-primary/60">({items.length})</span>
        </h3>
        {items.length === 0 ? (
          <p className="text-xs text-muted-foreground px-3">{emptyLabel}</p>
        ) : (
          <ul className="space-y-1">
            {items.map((item) => (
              <li key={item.id} className="group relative">
                <button
                  onClick={() => setSelectedItem(item)}
                  className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
                    selectedItem.id === item.id
                      ? 'active text-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <FileCode2 className="h-4 w-4 shrink-0" />
                    <span className="font-medium flex-1 truncate">{item.label}</span>
                  </div>
                  <div className="text-xs opacity-60 truncate mt-0.5">
                    {item.projectName ? (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span className="text-foreground font-semibold">{item.projectName}</span>
                        </TooltipTrigger>
                        <TooltipContent side="right">{item.subtitle}</TooltipContent>
                      </Tooltip>
                    ) : (
                      item.subtitle
                    )}
                  </div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <PageBackButton />
          <div className="tars-indicator" />
          <div>
            <p className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
              Runtimes / Codex
            </p>
            <h2 className="text-lg font-semibold tracking-wide">Codex Config</h2>
          </div>
        </div>
        <button
          onClick={handleOpenDocs}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-2"
        >
          View docs
          <ExternalLink className="h-3.5 w-3.5" />
        </button>
      </header>

      <TooltipProvider delayDuration={120}>
        <div className="flex-1 flex overflow-hidden">
          <div className="w-72 border-r border-border flex flex-col tars-panel">
            <div className="p-3 border-b border-border">
              <div className="flex items-center justify-between text-sm">
                <span className="font-medium text-foreground">Config files</span>
                {isLoadingProjects && (
                  <RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />
                )}
              </div>
            </div>

            <div className="tars-segment-line" />

            <div className="flex-1 overflow-auto p-3">
              {renderGroup('User', userItems, 'No user config found.')}
              {renderGroup('Project', projectItems, 'No projects configured yet.')}
              {renderGroup('Admin', adminItems, 'No admin config layers on this platform.')}
            </div>
          </div>

          <div className="flex-1 overflow-hidden bg-background">
            <div className="h-full p-6">
              <div className="h-full w-full space-y-4">
                <div className="rounded-lg border border-border bg-card/40 px-4 py-3">
                  <div className="flex items-center gap-2">
                    <Boxes className="h-4 w-4 text-primary" />
                    <p className="text-sm font-medium">Codex config layers</p>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    Edit user, project, system, and managed Codex TOML directly from TARS. System
                    and managed layers may require elevated filesystem permissions outside dev.
                  </p>
                </div>

                <div className="h-[calc(100%-5rem)]">
                  <ConfigFileEditor
                    cacheKey={`codex:${selectedItem.scope}:${selectedItem.projectPath ?? 'global'}`}
                    title={selectedItem.label}
                    subtitle={selectedItem.subtitle}
                    language="toml"
                    defaultContent="# Codex configuration\n"
                    readFile={() =>
                      readRuntimeConfigFile({
                        runtime: 'codex',
                        scope: selectedItem.scope,
                        projectPath: selectedItem.projectPath ?? null,
                      })
                    }
                    saveFile={(content) =>
                      saveRuntimeConfigFile({
                        runtime: 'codex',
                        scope: selectedItem.scope,
                        projectPath: selectedItem.projectPath ?? null,
                        content,
                      })
                    }
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </TooltipProvider>
    </div>
  );
}
