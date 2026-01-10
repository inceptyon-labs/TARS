import { useState, useCallback, useEffect, useRef } from 'react';
import { Save, RotateCcw } from 'lucide-react';
import {
  MDXEditor,
  headingsPlugin,
  listsPlugin,
  quotePlugin,
  thematicBreakPlugin,
  markdownShortcutPlugin,
  toolbarPlugin,
  linkPlugin,
  linkDialogPlugin,
  tablePlugin,
  codeBlockPlugin,
  codeMirrorPlugin,
  UndoRedo,
  BoldItalicUnderlineToggles,
  CodeToggle,
  ListsToggle,
  BlockTypeSelect,
  CreateLink,
  InsertTable,
  InsertThematicBreak,
  Separator,
  type MDXEditorMethods,
} from '@mdxeditor/editor';
import '@mdxeditor/editor/style.css';
import type { SkillDetails } from '../lib/types';

interface SkillEditorProps {
  skill: SkillDetails;
  onSave: (path: string, content: string) => Promise<void>;
  readOnly?: boolean;
}

// Editor plugins configuration
const editorPlugins = [
  headingsPlugin(),
  listsPlugin(),
  quotePlugin(),
  thematicBreakPlugin(),
  markdownShortcutPlugin(),
  linkPlugin(),
  linkDialogPlugin(),
  tablePlugin(),
  codeBlockPlugin({ defaultCodeBlockLanguage: '' }),
  codeMirrorPlugin({
    codeBlockLanguages: {
      js: 'JavaScript',
      ts: 'TypeScript',
      tsx: 'TypeScript (React)',
      jsx: 'JavaScript (React)',
      css: 'CSS',
      html: 'HTML',
      json: 'JSON',
      python: 'Python',
      rust: 'Rust',
      bash: 'Bash',
      sql: 'SQL',
      markdown: 'Markdown',
      '': 'Plain Text',
    },
  }),
  toolbarPlugin({
    toolbarContents: () => (
      <>
        <UndoRedo />
        <Separator />
        <BoldItalicUnderlineToggles />
        <CodeToggle />
        <Separator />
        <ListsToggle />
        <Separator />
        <BlockTypeSelect />
        <Separator />
        <CreateLink />
        <InsertTable />
        <InsertThematicBreak />
      </>
    ),
  }),
];

// Read-only plugins (no toolbar)
const readOnlyPlugins = [
  headingsPlugin(),
  listsPlugin(),
  quotePlugin(),
  thematicBreakPlugin(),
  linkPlugin(),
  tablePlugin(),
  codeBlockPlugin({ defaultCodeBlockLanguage: '' }),
  codeMirrorPlugin({
    codeBlockLanguages: {
      js: 'JavaScript',
      ts: 'TypeScript',
      tsx: 'TypeScript (React)',
      jsx: 'JavaScript (React)',
      css: 'CSS',
      html: 'HTML',
      json: 'JSON',
      python: 'Python',
      rust: 'Rust',
      bash: 'Bash',
      sql: 'SQL',
      markdown: 'Markdown',
      '': 'Plain Text',
    },
  }),
];

export function SkillEditor({ skill, onSave, readOnly = false }: SkillEditorProps) {
  const editorRef = useRef<MDXEditorMethods>(null);
  const [content, setContent] = useState(skill.content);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);

  // Sync content when skill changes (e.g., user selects a different skill)
  useEffect(() => {
    setContent(skill.content);
    setError(null);
    setEditorKey((k) => k + 1); // Force editor remount
  }, [skill.path, skill.content]);

  const hasChanges = content !== skill.content && !readOnly;

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    setSaving(true);
    setError(null);
    try {
      const currentContent = editorRef.current?.getMarkdown() || content;
      await onSave(skill.path, currentContent);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [content, hasChanges, onSave, skill.path]);

  const handleReset = useCallback(() => {
    setContent(skill.content);
    setError(null);
    setEditorKey((k) => k + 1); // Force editor remount
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
      <div className="flex-1 mdx-editor-container">
        <MDXEditor
          key={`skill-${editorKey}`}
          ref={readOnly ? undefined : editorRef}
          markdown={content}
          onChange={readOnly ? undefined : (markdown) => setContent(markdown)}
          readOnly={readOnly}
          plugins={readOnly ? readOnlyPlugins : editorPlugins}
          contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
        />
      </div>
    </div>
  );
}
