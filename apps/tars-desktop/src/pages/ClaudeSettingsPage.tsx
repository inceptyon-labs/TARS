import { useCallback, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FileJson, RefreshCw, ExternalLink } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { toast } from 'sonner';
import { SettingsFileEditor } from '../components/settings/SettingsFileEditor';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../components/ui/tooltip';
import { listProjects, type SettingsScope } from '../lib/ipc';

type SettingsItem = {
  id: string;
  scope: SettingsScope;
  projectPath?: string | null;
  projectName?: string;
  label: string;
  subtitle: string;
};

export function ClaudeSettingsPage() {
  const { data: projects = [], isLoading } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const userItems = useMemo<SettingsItem[]>(
    () => [
      {
        id: 'user-settings',
        scope: 'user',
        label: 'settings.json',
        subtitle: '~/.claude/settings.json',
      },
    ],
    []
  );

  const projectItems = useMemo<SettingsItem[]>(
    () =>
      projects.map((project) => ({
        id: `project-${project.id}`,
        scope: 'project',
        projectPath: project.path,
        projectName: project.name,
        label: 'settings.json',
        subtitle: project.path,
      })),
    [projects]
  );

  const localItems = useMemo<SettingsItem[]>(
    () =>
      projects.map((project) => ({
        id: `local-${project.id}`,
        scope: 'local',
        projectPath: project.path,
        projectName: project.name,
        label: 'settings.local.json',
        subtitle: project.path,
      })),
    [projects]
  );

  const [selectedItem, setSelectedItem] = useState<SettingsItem>(userItems[0]);

  const handleOpenDocs = useCallback(async () => {
    try {
      await openUrl('https://code.claude.com/docs/en/settings');
    } catch (err) {
      toast.error('Failed to open Claude Code docs');
      console.error('Failed to open Claude docs:', err);
    }
  }, []);

  function renderGroup(title: string, items: SettingsItem[], emptyLabel: string) {
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
                    <FileJson className="h-4 w-4 shrink-0" />
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
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Claude Settings</h2>
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
                <span className="font-medium text-foreground">Settings files</span>
                {isLoading && <RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />}
              </div>
            </div>

            <div className="tars-segment-line" />

            <div className="flex-1 overflow-auto p-3">
              {renderGroup('User', userItems, 'No user settings found.')}
              {renderGroup('Project', projectItems, 'No projects configured yet.')}
              {renderGroup('Local', localItems, 'No projects configured yet.')}
            </div>
          </div>

          <div className="flex-1 overflow-hidden bg-background">
            <div className="h-full p-6">
              <div className="h-full w-full">
                <SettingsFileEditor
                  scope={selectedItem.scope}
                  projectPath={selectedItem.projectPath}
                  title={selectedItem.label}
                  subtitle={selectedItem.subtitle}
                />
              </div>
            </div>
          </div>
        </div>
      </TooltipProvider>
    </div>
  );
}
