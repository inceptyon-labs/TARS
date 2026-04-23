import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, FileCode2, RefreshCcw, Save, Wand2 } from 'lucide-react';
import Editor, { useMonaco } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useQuery } from '@tanstack/react-query';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { toast } from 'sonner';
import { cn } from '../../lib/utils';
import { useUIStore } from '../../stores/ui-store';

interface EditableConfigFile {
  path: string;
  content: string | null;
  exists: boolean;
}

interface ConfigFileEditorProps {
  cacheKey: string;
  title: string;
  subtitle?: string;
  language: 'json' | 'toml';
  defaultContent: string;
  readFile: () => Promise<EditableConfigFile>;
  saveFile: (content: string) => Promise<void>;
}

export function ConfigFileEditor({
  cacheKey,
  title,
  subtitle,
  language,
  defaultContent,
  readFile,
  saveFile,
}: ConfigFileEditorProps) {
  const theme = useUIStore((state) => state.theme);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const [fileValue, setFileValue] = useState('');
  const [fileBase, setFileBase] = useState('');
  const [filePath, setFilePath] = useState('');
  const [savingFile, setSavingFile] = useState(false);
  const [fileMarkers, setFileMarkers] = useState<editor.IMarker[]>([]);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const monaco = useMonaco();

  const {
    data: configFile,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['config-file', cacheKey],
    queryFn: readFile,
  });

  useEffect(() => {
    if (error) {
      console.error(`Failed to load ${title}:`, error);
      toast.error(`Failed to load ${title}`);
    }
  }, [error, title]);

  useEffect(() => {
    setFileValue('');
    setFileBase('');
    setFilePath('');
  }, [cacheKey]);

  useEffect(() => {
    if (!configFile) {
      return;
    }

    const nextContent = configFile.content ?? defaultContent;
    setFileValue(nextContent);
    setFileBase(nextContent);
    setFilePath(configFile.path);
  }, [configFile, defaultContent]);

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

  const errorCount = useMemo(() => {
    const errorSeverity = monaco?.MarkerSeverity.Error ?? 8;
    return fileMarkers.filter((marker) => marker.severity === errorSeverity).length;
  }, [fileMarkers, monaco]);

  const isDirty = useMemo(() => fileValue !== fileBase, [fileValue, fileBase]);
  const canFormat = language === 'json';

  const handleRevealFile = useCallback(async () => {
    if (!filePath) {
      return;
    }
    try {
      await revealItemInDir(filePath);
    } catch (err) {
      toast.error(`Failed to reveal ${title}`);
      console.error(`Failed to reveal ${title}:`, err);
    }
  }, [filePath, title]);

  const handleReloadFile = useCallback(async () => {
    try {
      await refetch();
      toast.success(`Reloaded ${title}`);
    } catch (err) {
      toast.error(`Failed to reload ${title}`);
      console.error(`Failed to reload ${title}:`, err);
    }
  }, [refetch, title]);

  const handleFormatFile = useCallback(() => {
    if (!canFormat) {
      return;
    }

    try {
      const formatted = JSON.stringify(JSON.parse(fileValue), null, 2);
      setFileValue(`${formatted}\n`);
    } catch (err) {
      toast.error('Fix JSON errors before formatting');
      console.error(`Failed to format ${title}:`, err);
    }
  }, [canFormat, fileValue, title]);

  const handleSaveFile = useCallback(async () => {
    if (canFormat && errorCount > 0) {
      toast.error('Fix validation errors before saving');
      return;
    }

    setSavingFile(true);
    try {
      await saveFile(fileValue);
      setFileBase(fileValue);
      toast.success(`Saved ${title}`);
      await refetch();
    } catch (err) {
      toast.error(`Failed to save ${title}`);
      console.error(`Failed to save ${title}:`, err);
    } finally {
      setSavingFile(false);
    }
  }, [canFormat, errorCount, fileValue, refetch, saveFile, title]);

  return (
    <div className="p-4 rounded-lg border border-border bg-card space-y-4 h-full flex flex-col">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <FileCode2 className="h-4 w-4 text-muted-foreground" />
            <h3 className="font-medium">{title}</h3>
            {!configFile?.exists && (
              <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                New file
              </span>
            )}
          </div>
          {subtitle && <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={handleRevealFile}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
            disabled={!filePath}
          >
            Reveal in Finder
          </button>
          <button
            onClick={handleReloadFile}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
            disabled={isLoading}
          >
            <RefreshCcw className="h-4 w-4" />
            Reload
          </button>
          <button
            onClick={handleFormatFile}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
            disabled={!canFormat || isLoading || fileValue.trim().length === 0}
          >
            <Wand2 className="h-4 w-4" />
            Format
          </button>
          <button
            onClick={handleSaveFile}
            className={cn(
              'px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-2',
              isDirty
                ? 'border-primary text-primary hover:bg-primary/10'
                : 'border-border text-muted-foreground'
            )}
            disabled={isLoading || savingFile || (canFormat && errorCount > 0)}
          >
            <Save className="h-4 w-4" />
            {savingFile ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>

      <div
        ref={editorContainerRef}
        className="rounded-lg border border-border overflow-hidden flex-1 min-h-[280px] resize-y"
        style={{ resize: 'vertical' }}
      >
        <Editor
          key={cacheKey}
          height="100%"
          language={language}
          value={fileValue}
          theme={theme === 'dark' ? 'vs-dark' : 'vs'}
          onChange={(value) => setFileValue(value ?? '')}
          onValidate={(markers) => setFileMarkers(markers)}
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
            formatOnPaste: canFormat,
            formatOnType: canFormat,
            scrollBeyondLastLine: false,
            wordWrap: 'on',
            renderLineHighlight: 'all',
          }}
        />
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>{filePath || title}</span>
        {canFormat ? (
          errorCount > 0 ? (
            <span className="text-destructive flex items-center gap-1">
              <AlertTriangle className="h-3 w-3" />
              {errorCount} JSON error{errorCount === 1 ? '' : 's'} detected
            </span>
          ) : (
            <span>JSON looks valid</span>
          )
        ) : (
          <span>TOML validation runs on save</span>
        )}
      </div>
    </div>
  );
}
