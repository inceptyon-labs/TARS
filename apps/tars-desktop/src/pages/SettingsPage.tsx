import { Sun, Moon, Monitor, FolderOpen, RotateCcw, Info, ExternalLink } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { revealItemInDir, openUrl } from '@tauri-apps/plugin-opener';
import { homeDir } from '@tauri-apps/api/path';
import { cn } from '../lib/utils';
import { useUIStore, type Theme } from '../stores/ui-store';
import { getTarsVersion, getPlatformInfo } from '../lib/ipc';

export function SettingsPage() {
  const theme = useUIStore((state) => state.theme);
  const setTheme = useUIStore((state) => state.setTheme);

  const { data: appVersion } = useQuery({
    queryKey: ['tars-version'],
    queryFn: getTarsVersion,
    staleTime: Infinity,
  });

  const { data: platformInfo } = useQuery({
    queryKey: ['platform-info'],
    queryFn: getPlatformInfo,
    staleTime: Infinity,
  });

  const handleOpenDataDir = async () => {
    try {
      const home = await homeDir();
      const tarsDir = `${home}.tars`;
      await revealItemInDir(tarsDir);
    } catch (err) {
      console.error('Failed to open data directory:', err);
    }
  };

  const handleResetSettings = () => {
    if (window.confirm('Are you sure you want to reset all settings to defaults?')) {
      useUIStore.getState().reset();
    }
  };

  const handleOpenIssues = async () => {
    try {
      await openUrl('https://github.com/inceptyon-labs/TARS/issues');
    } catch (err) {
      console.error('Failed to open issues page:', err);
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="shrink-0 border-b border-border bg-card/50 px-6 py-4">
        <h1 className="text-xl font-semibold">Settings</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Configure TARS preferences and appearance
        </p>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl space-y-8">
          {/* Appearance Section */}
          <section>
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <Sun className="h-5 w-5" />
              Appearance
            </h2>
            <div className="space-y-4">
              {/* Theme Selection */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="font-medium">Theme</h3>
                    <p className="text-sm text-muted-foreground">
                      Choose how TARS looks on your device
                    </p>
                  </div>
                  <div className="flex items-center gap-1 p-1 rounded-lg bg-muted/50">
                    {(
                      [
                        { value: 'system', icon: Monitor, label: 'System' },
                        { value: 'light', icon: Sun, label: 'Light' },
                        { value: 'dark', icon: Moon, label: 'Dark' },
                      ] as const
                    ).map(({ value, icon: Icon, label }) => (
                      <button
                        key={value}
                        onClick={() => setTheme(value as Theme)}
                        className={cn(
                          'flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-all',
                          theme === value
                            ? 'bg-background text-foreground shadow-sm'
                            : 'text-muted-foreground hover:text-foreground'
                        )}
                      >
                        <Icon className="h-4 w-4" />
                        {label}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Data Section */}
          <section>
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <FolderOpen className="h-5 w-5" />
              Data
            </h2>
            <div className="space-y-4">
              {/* Open Data Directory */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="font-medium">Data Directory</h3>
                    <p className="text-sm text-muted-foreground">
                      Open the folder where TARS stores its data
                    </p>
                  </div>
                  <button
                    onClick={handleOpenDataDir}
                    className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
                  >
                    Open Folder
                  </button>
                </div>
              </div>

              {/* Reset Settings */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="font-medium">Reset Settings</h3>
                    <p className="text-sm text-muted-foreground">
                      Reset all preferences to their default values
                    </p>
                  </div>
                  <button
                    onClick={handleResetSettings}
                    className="px-4 py-2 text-sm rounded-md border border-destructive/50 text-destructive hover:bg-destructive/10 transition-colors flex items-center gap-2"
                  >
                    <RotateCcw className="h-4 w-4" />
                    Reset
                  </button>
                </div>
              </div>
            </div>
          </section>

          {/* About Section */}
          <section>
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <Info className="h-5 w-5" />
              About
            </h2>
            <div className="p-4 rounded-lg border border-border bg-card">
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Version</span>
                  <span className="font-mono">v{appVersion ?? '...'}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Platform</span>
                  <span className="font-mono">{platformInfo?.display ?? '...'}</span>
                </div>
                <div className="flex items-center justify-between pt-3 border-t border-border">
                  <div>
                    <p className="text-sm text-muted-foreground">
                      TARS - Tooling, Agents, Roles, Skills
                    </p>
                    <p className="text-xs text-muted-foreground/70 mt-1">
                      A configuration manager for Claude Code
                    </p>
                  </div>
                  <button
                    onClick={handleOpenIssues}
                    className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
                  >
                    Report Issue
                    <ExternalLink className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
