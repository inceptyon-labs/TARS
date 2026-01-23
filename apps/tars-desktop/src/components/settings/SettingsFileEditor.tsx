import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Save, RefreshCcw, Wand2, FileJson, AlertTriangle } from 'lucide-react';
import Editor, { useMonaco } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { toast } from 'sonner';
import { useQuery } from '@tanstack/react-query';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { cn } from '../../lib/utils';
import { useUIStore } from '../../stores/ui-store';
import { readSettingsFile, saveSettingsFile, type SettingsScope } from '../../lib/ipc';

interface SettingsFileEditorProps {
  scope: SettingsScope;
  projectPath?: string | null;
  title: string;
  subtitle?: string;
}

export function SettingsFileEditor({
  scope,
  projectPath,
  title,
  subtitle,
}: SettingsFileEditorProps) {
  const theme = useUIStore((state) => state.theme);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const [settingsValue, setSettingsValue] = useState('');
  const [settingsBase, setSettingsBase] = useState('');
  const [settingsPath, setSettingsPath] = useState('');
  const [savingSettings, setSavingSettings] = useState(false);
  const [settingsMarkers, setSettingsMarkers] = useState<editor.IMarker[]>([]);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const monaco = useMonaco();

  const {
    data: settingsFile,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['settings-file', scope, projectPath],
    queryFn: () => readSettingsFile({ scope, projectPath }),
  });

  // Log errors for debugging
  useEffect(() => {
    if (error) {
      console.error('Failed to load settings file:', error);
      toast.error(`Failed to load settings: ${error.message || error}`);
    }
  }, [error]);

  useEffect(() => {
    setSettingsValue('');
    setSettingsBase('');
    setSettingsPath('');
  }, [scope, projectPath]);

  useEffect(() => {
    if (!settingsFile) {
      return;
    }

    const nextContent = settingsFile.content ?? '{\n  \n}';
    setSettingsValue(nextContent);
    setSettingsBase(nextContent);
    setSettingsPath(settingsFile.path);
  }, [settingsFile]);

  useEffect(() => {
    const container = editorContainerRef.current;
    if (!container) {
      return;
    }
    const observer = new ResizeObserver(() => {
      editorRef.current?.layout();
    });
    resizeObserverRef.current = observer;
    observer.observe(container);
    return () => {
      observer.disconnect();
      resizeObserverRef.current = null;
    };
  }, []);

  const settingsErrorCount = useMemo(() => {
    const errorSeverity = monaco?.MarkerSeverity.Error ?? 8;
    return settingsMarkers.filter((marker) => marker.severity === errorSeverity).length;
  }, [settingsMarkers, monaco]);

  const settingsDirty = useMemo(
    () => settingsValue !== settingsBase,
    [settingsValue, settingsBase]
  );

  const handleOpenSettingsFolder = useCallback(async () => {
    if (!settingsPath) {
      return;
    }
    try {
      await revealItemInDir(settingsPath);
    } catch (err) {
      toast.error('Failed to reveal settings.json');
      console.error('Failed to reveal settings.json:', err);
    }
  }, [settingsPath]);

  const handleReloadSettings = useCallback(async () => {
    try {
      await refetch();
      toast.success('Reloaded settings.json');
    } catch (err) {
      toast.error('Failed to reload settings.json');
      console.error('Failed to reload settings.json:', err);
    }
  }, [refetch]);

  const handleFormatSettings = useCallback(() => {
    try {
      const formatted = JSON.stringify(JSON.parse(settingsValue), null, 2);
      setSettingsValue(`${formatted}\n`);
    } catch (err) {
      toast.error('Fix JSON errors before formatting');
      console.error('Failed to format settings.json:', err);
    }
  }, [settingsValue]);

  const handleSaveSettings = useCallback(async () => {
    if (settingsErrorCount > 0) {
      toast.error('Fix JSON errors before saving');
      return;
    }

    setSavingSettings(true);
    try {
      await saveSettingsFile({ scope, projectPath, content: settingsValue });
      setSettingsBase(settingsValue);
      toast.success('Saved settings.json');
      await refetch();
    } catch (err) {
      toast.error('Failed to save settings.json');
      console.error('Failed to save settings.json:', err);
    } finally {
      setSavingSettings(false);
    }
  }, [settingsErrorCount, settingsValue, refetch, scope, projectPath]);

  return (
    <div className="p-4 rounded-lg border border-border bg-card space-y-4 h-full flex flex-col">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <FileJson className="h-4 w-4 text-muted-foreground" />
            <h3 className="font-medium">{title}</h3>
            {!settingsFile?.exists && (
              <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                New file
              </span>
            )}
          </div>
          {subtitle && <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={handleOpenSettingsFolder}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
            disabled={!settingsPath}
          >
            Reveal in Finder
          </button>
          <button
            onClick={handleReloadSettings}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
            disabled={isLoading}
          >
            <RefreshCcw className="h-4 w-4" />
            Reload
          </button>
          <button
            onClick={handleFormatSettings}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
            disabled={isLoading || settingsValue.trim().length === 0}
          >
            <Wand2 className="h-4 w-4" />
            Format
          </button>
          <button
            onClick={handleSaveSettings}
            className={cn(
              'px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-2',
              settingsDirty
                ? 'border-primary text-primary hover:bg-primary/10'
                : 'border-border text-muted-foreground'
            )}
            disabled={isLoading || savingSettings || settingsErrorCount > 0}
          >
            <Save className="h-4 w-4" />
            {savingSettings ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>

      <div
        ref={editorContainerRef}
        className="rounded-lg border border-border overflow-hidden flex-1 min-h-[280px] resize-y"
        style={{ resize: 'vertical' }}
      >
        <Editor
          key={`${scope}-${projectPath ?? 'user'}`}
          height="100%"
          language="json"
          value={settingsValue}
          theme={theme === 'dark' ? 'vs-dark' : 'vs'}
          onChange={(value) => setSettingsValue(value ?? '')}
          onValidate={(markers) => setSettingsMarkers(markers)}
          onMount={(editorInstance) => {
            editorRef.current = editorInstance;
          }}
          loading={
            <div className="flex items-center justify-center h-full text-muted-foreground">
              Loading editor...
            </div>
          }
          options={{
            minimap: { enabled: false },
            tabSize: 2,
            insertSpaces: true,
            formatOnPaste: true,
            formatOnType: true,
            scrollBeyondLastLine: false,
            wordWrap: 'on',
            renderLineHighlight: 'all',
          }}
        />
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>{settingsPath || 'settings.json'}</span>
        {settingsErrorCount > 0 ? (
          <span className="text-destructive flex items-center gap-1">
            <AlertTriangle className="h-3 w-3" />
            {settingsErrorCount} JSON error{settingsErrorCount === 1 ? '' : 's'} detected
          </span>
        ) : (
          <span>JSON looks valid</span>
        )}
      </div>
    </div>
  );
}
