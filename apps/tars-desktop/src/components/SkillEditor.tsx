import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { Save, RotateCcw, ChevronDown, ChevronRight } from 'lucide-react';
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

/** Parse frontmatter from markdown content */
function parseFrontmatter(content: string): { frontmatter: string | null; body: string } {
  const trimmed = content.trimStart();
  if (!trimmed.startsWith('---')) {
    return { frontmatter: null, body: content };
  }

  // Find the closing ---
  const endIndex = trimmed.indexOf('---', 3);
  if (endIndex === -1) {
    return { frontmatter: null, body: content };
  }

  // Extract frontmatter (without the --- delimiters)
  const frontmatter = trimmed.slice(3, endIndex).trim();
  // Extract body (after the closing ---)
  const body = trimmed.slice(endIndex + 3).trimStart();

  return { frontmatter, body };
}

/** Combine frontmatter and body back into markdown */
function combineFrontmatter(frontmatter: string | null, body: string): string {
  if (!frontmatter) {
    return body;
  }
  return `---\n${frontmatter}\n---\n\n${body}`;
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
      yaml: 'YAML',
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
      yaml: 'YAML',
      '': 'Plain Text',
    },
  }),
];

export function SkillEditor({ skill, onSave, readOnly = false }: SkillEditorProps) {
  const editorRef = useRef<MDXEditorMethods>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [frontmatterExpanded, setFrontmatterExpanded] = useState(true);

  // Parse frontmatter from the original content
  const { frontmatter: originalFrontmatter, body: originalBody } = useMemo(
    () => parseFrontmatter(skill.content),
    [skill.content]
  );

  // State for editable frontmatter and body
  const [frontmatter, setFrontmatter] = useState(originalFrontmatter);
  const [body, setBody] = useState(originalBody);

  // Sync content when skill changes
  useEffect(() => {
    const parsed = parseFrontmatter(skill.content);
    setFrontmatter(parsed.frontmatter);
    setBody(parsed.body);
    setError(null);
    setEditorKey((k) => k + 1); // Force editor remount
  }, [skill.path, skill.content]);

  // Check if content has changed
  const currentContent = combineFrontmatter(frontmatter, body);
  const hasChanges = currentContent !== skill.content && !readOnly;

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    setSaving(true);
    setError(null);
    try {
      const editorBody = editorRef.current?.getMarkdown() || body;
      const fullContent = combineFrontmatter(frontmatter, editorBody);
      await onSave(skill.path, fullContent);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [body, frontmatter, hasChanges, onSave, skill.path]);

  const handleReset = useCallback(() => {
    const parsed = parseFrontmatter(skill.content);
    setFrontmatter(parsed.frontmatter);
    setBody(parsed.body);
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
      <div className="border-b p-3 flex items-center justify-between shrink-0">
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
        <div className="px-3 py-2 bg-destructive/10 text-destructive text-sm shrink-0">
          {error}
        </div>
      )}

      {/* Frontmatter panel */}
      {frontmatter && (
        <div className="border-b border-border shrink-0">
          <button
            onClick={() => setFrontmatterExpanded(!frontmatterExpanded)}
            className="w-full px-4 py-2 flex items-center gap-2 text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
          >
            {frontmatterExpanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
            <span className="uppercase tracking-wider">Frontmatter</span>
            <span className="text-primary font-mono">YAML</span>
          </button>
          {frontmatterExpanded && (
            <div className="px-4 pb-3">
              <textarea
                ref={(el) => {
                  if (el) {
                    el.style.height = 'auto';
                    el.style.height = `${el.scrollHeight + 4}px`;
                  }
                }}
                value={frontmatter}
                onChange={!readOnly ? (e) => {
                  setFrontmatter(e.target.value);
                  // Auto-resize on change
                  const el = e.target;
                  el.style.height = 'auto';
                  el.style.height = `${el.scrollHeight + 4}px`;
                } : undefined}
                readOnly={readOnly}
                className="w-full bg-secondary text-secondary-foreground font-mono text-sm p-3 rounded-lg border border-border focus:outline-none focus:ring-2 focus:ring-primary/50 overflow-hidden"
                spellCheck={false}
              />
            </div>
          )}
        </div>
      )}

      {/* Editor */}
      <div className="flex-1 mdx-editor-container overflow-auto">
        <MDXEditor
          key={`skill-${editorKey}`}
          ref={readOnly ? undefined : editorRef}
          markdown={body}
          onChange={readOnly ? undefined : (markdown) => setBody(markdown)}
          readOnly={readOnly}
          plugins={readOnly ? readOnlyPlugins : editorPlugins}
          contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
        />
      </div>
    </div>
  );
}
