import {
  Sun,
  Moon,
  Monitor,
  FolderOpen,
  RotateCcw,
  Info,
  ExternalLink,
  Heart,
  DatabaseBackup,
  Download,
  Upload,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import { homeDir } from '@tauri-apps/api/path';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'sonner';
import { cn } from '../lib/utils';
import { useUIStore, type Theme } from '../stores/ui-store';
import { SupportButton } from '../components/SupportButton';
import {
  createLocalAppDataBackup,
  createPortableAppDataBackup,
  deleteAppDataBackup,
  getAppDataBackupDir,
  getTarsVersion,
  getPlatformInfo,
  listAppDataBackups,
  restoreAppDataBackup,
  setAppDataBackupDir,
} from '../lib/ipc';

export function SettingsPage() {
  const queryClient = useQueryClient();
  const theme = useUIStore((state) => state.theme);
  const setTheme = useUIStore((state) => state.setTheme);
  const [portablePassphrase, setPortablePassphrase] = useState('');
  const [restorePassphrase, setRestorePassphrase] = useState('');

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

  const { data: appDataBackups = [] } = useQuery({
    queryKey: ['app-data-backups'],
    queryFn: listAppDataBackups,
    staleTime: 0,
  });

  const { data: backupDir } = useQuery({
    queryKey: ['app-data-backup-dir'],
    queryFn: getAppDataBackupDir,
    staleTime: 0,
  });

  const localBackupMutation = useMutation({
    mutationFn: () => createLocalAppDataBackup(),
    onSuccess: (backup) => {
      queryClient.invalidateQueries({ queryKey: ['app-data-backups'] });
      toast.success(`Backup created: ${backup.file_name}`);
    },
    onError: (err) => toast.error(`Failed to create backup: ${err}`),
  });

  const portableBackupMutation = useMutation({
    mutationFn: () => createPortableAppDataBackup(portablePassphrase),
    onSuccess: (backup) => {
      queryClient.invalidateQueries({ queryKey: ['app-data-backups'] });
      setPortablePassphrase('');
      toast.success(`Portable backup created: ${backup.file_name}`);
    },
    onError: (err) => toast.error(`Failed to create portable backup: ${err}`),
  });

  const restoreMutation = useMutation({
    mutationFn: async () => {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: 'TARS backups',
            extensions: ['tars-backup', 'tars-portable-backup'],
          },
        ],
      });
      if (!selected || Array.isArray(selected)) return null;
      const isPortable = selected.endsWith('.tars-portable-backup');
      if (
        !window.confirm(
          'Restore this backup? TARS will first create an emergency backup of the current database.'
        )
      ) {
        return null;
      }
      return restoreAppDataBackup(selected, isPortable ? restorePassphrase : null);
    },
    onSuccess: (result) => {
      if (!result) return;
      queryClient.invalidateQueries();
      setRestorePassphrase('');
      toast.success(`Database restored. Emergency backup: ${result.backup_before_restore_path}`);
    },
    onError: (err) => toast.error(`Failed to restore backup: ${err}`),
  });

  const setBackupDirMutation = useMutation({
    mutationFn: (path: string | null) => setAppDataBackupDir(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-data-backup-dir'] });
      queryClient.invalidateQueries({ queryKey: ['app-data-backups'] });
      toast.success('Backup folder updated');
    },
    onError: (err) => toast.error(`Failed to update backup folder: ${err}`),
  });

  const deleteBackupMutation = useMutation({
    mutationFn: deleteAppDataBackup,
    onSuccess: (deleted) => {
      queryClient.invalidateQueries({ queryKey: ['app-data-backups'] });
      toast.success(deleted ? 'Backup deleted' : 'Backup was already deleted');
    },
    onError: (err) => toast.error(`Failed to delete backup: ${err}`),
  });

  const handleOpenDataDir = async () => {
    try {
      const home = await homeDir();
      const tarsDir = `${home}.tars`;
      await openPath(tarsDir);
    } catch (err) {
      toast.error(`Failed to open data directory: ${err}`);
    }
  };

  const handleOpenBackupDir = async () => {
    try {
      const dir = await getAppDataBackupDir();
      await openPath(dir.path);
    } catch (err) {
      toast.error(`Failed to open backup folder: ${err}`);
    }
  };

  const handleChooseBackupDir = async () => {
    const selected = await open({
      multiple: false,
      directory: true,
    });
    if (!selected || Array.isArray(selected)) return;
    setBackupDirMutation.mutate(selected);
  };

  const handleDeleteBackup = (path: string, fileName: string) => {
    if (!window.confirm(`Delete backup "${fileName}"?`)) return;
    deleteBackupMutation.mutate(path);
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

          {/* Backup Section */}
          <section>
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <DatabaseBackup className="h-5 w-5" />
              Backup & Restore
            </h2>
            <div className="space-y-4">
              <div className="p-4 rounded-lg border border-border bg-card space-y-4">
                <div className="flex items-center justify-between gap-4">
                  <div>
                    <h3 className="font-medium">Local Backup</h3>
                    <p className="text-sm text-muted-foreground">
                      Snapshot the TARS database for restore on this machine.
                    </p>
                  </div>
                  <button
                    onClick={() => localBackupMutation.mutate()}
                    disabled={localBackupMutation.isPending}
                    className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2 disabled:opacity-50"
                  >
                    <Download className="h-4 w-4" />
                    Create
                  </button>
                </div>

                <div className="border-t border-border pt-4">
                  <div className="flex items-end justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <h3 className="font-medium">Portable Encrypted Backup</h3>
                      <p className="text-sm text-muted-foreground mb-3">
                        Rewrap stored secrets with a passphrase for restore on another machine.
                      </p>
                      <input
                        type="password"
                        value={portablePassphrase}
                        onChange={(e) => setPortablePassphrase(e.target.value)}
                        placeholder="Backup passphrase"
                        className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                      />
                    </div>
                    <button
                      onClick={() => portableBackupMutation.mutate()}
                      disabled={portableBackupMutation.isPending || portablePassphrase.length < 8}
                      className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2 disabled:opacity-50"
                    >
                      <Download className="h-4 w-4" />
                      Create
                    </button>
                  </div>
                </div>

                <div className="border-t border-border pt-4">
                  <div className="flex items-end justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <h3 className="font-medium">Restore Backup</h3>
                      <p className="text-sm text-muted-foreground mb-3">
                        Restore a local or portable backup. Portable backups require the passphrase.
                      </p>
                      <input
                        type="password"
                        value={restorePassphrase}
                        onChange={(e) => setRestorePassphrase(e.target.value)}
                        placeholder="Portable backup passphrase, if needed"
                        className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                      />
                    </div>
                    <button
                      onClick={() => restoreMutation.mutate()}
                      disabled={restoreMutation.isPending}
                      className="px-4 py-2 text-sm rounded-md border border-destructive/50 text-destructive hover:bg-destructive/10 transition-colors flex items-center gap-2 disabled:opacity-50"
                    >
                      <Upload className="h-4 w-4" />
                      Restore
                    </button>
                  </div>
                </div>
              </div>

              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-start justify-between gap-4 mb-3">
                  <div>
                    <h3 className="font-medium">Recent App Data Backups</h3>
                    <p className="text-sm text-muted-foreground">Stored in:</p>
                    <code className="mt-1 block max-w-lg truncate rounded bg-muted/40 px-2 py-1 text-xs">
                      {backupDir?.path ?? '...'}
                    </code>
                  </div>
                  <div className="flex flex-wrap justify-end gap-2">
                    <button
                      onClick={() =>
                        queryClient.invalidateQueries({ queryKey: ['app-data-backups'] })
                      }
                      className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
                    >
                      <RefreshCw className="h-3.5 w-3.5" />
                      Refresh
                    </button>
                    <button
                      onClick={handleOpenBackupDir}
                      className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
                    >
                      Open Folder
                    </button>
                    <button
                      onClick={handleChooseBackupDir}
                      className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
                    >
                      Change Folder
                    </button>
                    {!backupDir?.is_default && (
                      <button
                        onClick={() => setBackupDirMutation.mutate(null)}
                        className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
                      >
                        Reset
                      </button>
                    )}
                  </div>
                </div>
                <div className="space-y-2">
                  {appDataBackups.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No app data backups yet.</p>
                  ) : (
                    appDataBackups.slice(0, 6).map((backup) => (
                      <div
                        key={backup.path}
                        className="flex items-center justify-between gap-3 rounded-md bg-muted/20 px-3 py-2 text-sm"
                      >
                        <div className="min-w-0">
                          <div className="truncate font-medium">{backup.file_name}</div>
                          <div className="text-xs text-muted-foreground">
                            {backup.backup_type} · {new Date(backup.created_at).toLocaleString()} ·{' '}
                            {(backup.size_bytes / 1024).toFixed(1)} KB
                          </div>
                        </div>
                        <div className="flex shrink-0 gap-2">
                          <button
                            onClick={() => navigator.clipboard.writeText(backup.path)}
                            className="px-2 py-1 text-xs rounded border border-border hover:bg-muted/50"
                          >
                            Copy Path
                          </button>
                          <button
                            onClick={() => handleDeleteBackup(backup.path, backup.file_name)}
                            className="px-2 py-1 text-xs rounded border border-destructive/50 text-destructive hover:bg-destructive/10 flex items-center gap-1"
                            title="Delete backup"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                            Delete
                          </button>
                        </div>
                      </div>
                    ))
                  )}
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

          {/* Support Section */}
          <section>
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <Heart className="h-5 w-5" />
              Support
            </h2>
            <div className="p-4 rounded-lg border border-border bg-card">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium">Support Development</h3>
                  <p className="text-sm text-muted-foreground">
                    If you find TARS useful, consider supporting its development
                  </p>
                </div>
                <SupportButton />
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
