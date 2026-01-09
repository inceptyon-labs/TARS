import { useState, useCallback, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { Save, RotateCcw } from 'lucide-react';
import type { SkillDetails } from '../lib/types';
import { useUIStore } from '../stores/ui-store';

interface SkillEditorProps {
  skill: SkillDetails;
  onSave: (path: string, content: string) => Promise<void>;
  readOnly?: boolean;
}

export function SkillEditor({ skill, onSave, readOnly = false }: SkillEditorProps) {
  const [content, setContent] = useState(skill.content);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const theme = useUIStore((state) => state.theme);

  // Sync content when skill changes (e.g., user selects a different skill)
  useEffect(() => {
    setContent(skill.content);
    setError(null);
  }, [skill.path, skill.content]);

  const hasChanges = content !== skill.content && !readOnly;

  // Determine Monaco theme based on app theme
  const monacoTheme = theme === 'light' ? 'light' : 'vs-dark';

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    setSaving(true);
    setError(null);
    try {
      await onSave(skill.path, content);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [content, hasChanges, onSave, skill.path]);

  const handleReset = useCallback(() => {
    setContent(skill.content);
    setError(null);
  }, [skill.content]);

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
            <h3 className="font-medium">{skill.name}</h3>
            {readOnly && (
              <span className="text-xs px-2 py-0.5 bg-muted text-muted-foreground rounded">
                Read-only
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">{skill.path}</p>
        </div>
        {!readOnly && (
          <div className="flex items-center gap-2">
            {hasChanges && (
              <span className="text-xs text-muted-foreground">Unsaved changes</span>
            )}
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
          </div>
        )}
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
