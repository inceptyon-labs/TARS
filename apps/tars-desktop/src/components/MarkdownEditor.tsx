import { useState, useCallback, useEffect, useRef } from 'react';
import { Save, RotateCcw, ArrowRightLeft, Pencil } from 'lucide-react';
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
  /** Start in view mode with Edit button to switch to edit mode */
  defaultViewMode?: boolean;
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

export function MarkdownEditor({ item, onSave, onMove, readOnly = false, defaultViewMode = false }: MarkdownEditorProps) {
  const editorRef = useRef<MDXEditorMethods>(null);
  const [content, setContent] = useState(item.content);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [isViewMode, setIsViewMode] = useState(defaultViewMode);

  // Sync content when item changes
  useEffect(() => {
    setContent(item.content);
    setError(null);
    setEditorKey((k) => k + 1); // Force editor remount
    // Reset to view mode when item changes (if defaultViewMode is enabled)
    if (defaultViewMode) {
      setIsViewMode(true);
    }
  }, [item.path, item.content, defaultViewMode]);

  const hasChanges = content !== item.content && !readOnly && !isViewMode;

  // Determine if we're in an editable state (not read-only and not in view mode)
  const isEditing = !readOnly && !isViewMode;

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    setSaving(true);
    setError(null);
    try {
      const currentContent = editorRef.current?.getMarkdown() || content;
      await onSave(item.path, currentContent);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [content, hasChanges, onSave, item.path]);

  const handleReset = useCallback(() => {
    setContent(item.content);
    setError(null);
    setEditorKey((k) => k + 1); // Force editor remount
    // Return to view mode if defaultViewMode is enabled
    if (defaultViewMode) {
      setIsViewMode(true);
    }
  }, [item.content, defaultViewMode]);

  const handleEnterEdit = useCallback(() => {
    setIsViewMode(false);
    setEditorKey((k) => k + 1); // Force editor remount with edit mode
  }, []);

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
          {isEditing && hasChanges && (
            <span className="text-xs text-muted-foreground">Unsaved changes</span>
          )}
          {/* Edit button shown in view mode */}
          {!readOnly && isViewMode && (
            <button
              onClick={handleEnterEdit}
              className="inline-flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
            >
              <Pencil className="h-4 w-4" />
              Edit
            </button>
          )}
          {/* Move/Reset/Save buttons shown in edit mode */}
          {onMove && isEditing && (
            <button
              onClick={() => onMove(item.path)}
              disabled={hasChanges}
              className="p-2 rounded-lg hover:bg-muted disabled:opacity-50"
              title={hasChanges ? 'Save changes before moving' : 'Move to different scope'}
            >
              <ArrowRightLeft className="h-4 w-4" />
            </button>
          )}
          {isEditing && (
            <>
              <button
                onClick={handleReset}
                disabled={!hasChanges && !defaultViewMode}
                className="p-2 rounded-lg hover:bg-muted disabled:opacity-50"
                title={defaultViewMode ? 'Cancel editing' : 'Reset changes'}
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
      <div className="flex-1 mdx-editor-container">
        <MDXEditor
          key={`editor-${editorKey}`}
          ref={isEditing ? editorRef : undefined}
          markdown={content}
          onChange={isEditing ? (markdown) => setContent(markdown) : undefined}
          readOnly={readOnly || isViewMode}
          plugins={isEditing ? editorPlugins : readOnlyPlugins}
          contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
        />
      </div>
    </div>
  );
}
