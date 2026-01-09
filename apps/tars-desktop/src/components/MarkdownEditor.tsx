import { useState, useCallback, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { Save, RotateCcw, ArrowRightLeft } from 'lucide-react';
import { useUIStore } from '../stores/ui-store';

export interface EditableItem {
  name: string;
  path: string;
  content: string;
  scope?: string;
}

interface MarkdownEditorProps {
  item: EditableItem;
  onSave: (path: string, content: string) => Promise<void>;
  onMove?: (path: string) => void;
  readOnly?: boolean;
}

/** Get display label for scope */
function getScopeLabel(scope?: string): string {
  switch (scope) {
    case 'user':
      return 'User';
    case 'project':
      return 'Project';
    case 'local':
      return 'Local';
    case 'managed':
      return 'Managed';
    case 'plugin':
      return 'Plugin';
    default:
      return scope || 'Unknown';
  }
}

export function MarkdownEditor({ item, onSave, onMove, readOnly = false }: MarkdownEditorProps) {
  const [content, setContent] = useState(item.content);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const theme = useUIStore((state) => state.theme);

  // Sync content when item changes
  useEffect(() => {
    setContent(item.content);
    setError(null);
  }, [item.path, item.content]);

  const hasChanges = content !== item.content && !readOnly;

  // Determine Monaco theme based on app theme
  const monacoTheme = theme === 'light' ? 'light' : 'vs-dark';

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    setSaving(true);
    setError(null);
    try {
      await onSave(item.path, content);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [content, hasChanges, onSave, item.path]);

  const handleReset = useCallback(() => {
    setContent(item.content);
    setError(null);
  }, [item.content]);

  // Handle Cmd+S / Ctrl+S
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    },
    [handleSave]
  );

  return (
    <div className="h-full flex flex-col" onKeyDown={handleKeyDown}>
      {/* Header */}
      <div className="border-b p-3 flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="font-medium">{item.name}</h3>
            {item.scope && (
              <span className="text-xs px-2 py-0.5 bg-primary/10 text-primary rounded font-medium">
                {getScopeLabel(item.scope)}
              </span>
            )}
            {readOnly && (
              <span className="text-xs px-2 py-0.5 bg-muted text-muted-foreground rounded">
                Read-only
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">{item.path}</p>
        </div>
        <div className="flex items-center gap-2">
          {!readOnly && hasChanges && (
            <span className="text-xs text-muted-foreground">Unsaved changes</span>
          )}
          {onMove && !readOnly && (
            <button
              onClick={() => onMove(item.path)}
              disabled={hasChanges}
              className="p-2 rounded-lg hover:bg-muted disabled:opacity-50"
              title={hasChanges ? 'Save changes before moving' : 'Move to different scope'}
            >
              <ArrowRightLeft className="h-4 w-4" />
            </button>
          )}
          {!readOnly && (
            <>
              <button
                onClick={handleReset}
                disabled={!hasChanges}
                className="p-2 rounded-lg hover:bg-muted disabled:opacity-50"
                title="Reset changes"
              >
                <RotateCcw className="h-4 w-4" />
              </button>
              <button
                onClick={handleSave}
                disabled={!hasChanges || saving}
                className="inline-flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
              >
                <Save className="h-4 w-4" />
                {saving ? 'Saving...' : 'Save'}
              </button>
            </>
          )}
        </div>
      </div>

      {/* Error message */}
      {error && (
        <div className="px-3 py-2 bg-destructive/10 text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Editor */}
      <div className="flex-1">
        <Editor
          height="100%"
          defaultLanguage="markdown"
          value={content}
          onChange={(value) => setContent(value || '')}
          theme={monacoTheme}
          options={{
            minimap: { enabled: false },
            fontSize: 14,
            lineNumbers: 'on',
            wordWrap: 'on',
            scrollBeyondLastLine: false,
            automaticLayout: true,
            readOnly,
          }}
        />
      </div>
    </div>
  );
}
