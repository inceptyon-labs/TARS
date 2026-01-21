import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ChevronRight, ChevronDown, Save, StickyNote } from 'lucide-react';
import { toast } from 'sonner';
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
import { useUIStore } from '../stores/ui-store';
import { readProjectNotes, saveProjectNotes } from '../lib/ipc';
import { Button } from './ui/button';
import { codeBlockShortcutPlugin } from '../lib/mdx-plugins/codeBlockShortcutPlugin';
import { CodeBlockCopyButton } from './CodeBlockCopyButton';

interface ProjectNotesProps {
  projectPath: string;
}

export function ProjectNotes({ projectPath }: ProjectNotesProps) {
  const queryClient = useQueryClient();
  const theme = useUIStore((state) => state.theme);
  const [isExpanded, setIsExpanded] = useState(true);
  const [notesContent, setNotesContent] = useState<string>('');
  const [isDirty, setIsDirty] = useState(false);
  const [editorKey, setEditorKey] = useState(0);
  const editorRef = useRef<MDXEditorMethods>(null);

  // Load notes content
  const {
    data: notesInfo,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['project-notes', projectPath],
    queryFn: () => readProjectNotes(projectPath),
  });

  useEffect(() => {
    if (notesInfo?.content !== undefined) {
      setNotesContent(notesInfo.content || '');
      setIsDirty(false);
      setEditorKey((k) => k + 1); // Force editor remount
    }
  }, [notesInfo]);

  // Save notes mutation
  const saveMutation = useMutation({
    mutationFn: () => {
      const content = editorRef.current?.getMarkdown() || notesContent;
      return saveProjectNotes(projectPath, content);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-notes', projectPath] });
      setIsDirty(false);
      toast.success('Notes saved');
    },
    onError: (err) => {
      toast.error('Failed to save notes', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Keyboard shortcut for save (Cmd/Ctrl+S)
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (isDirty && !saveMutation.isPending) {
          saveMutation.mutate();
        }
      }
    },
    [isDirty, saveMutation]
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  const isDarkMode =
    theme === 'dark' ||
    (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

  return (
    <div className="tars-panel rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center gap-3 hover:text-primary transition-colors"
        >
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <StickyNote className="h-4 w-4 text-primary" />
          <span className="font-medium">Notes</span>
          {isDirty && (
            <span className="text-xs text-amber-500 bg-amber-500/10 px-2 py-0.5 rounded">
              Unsaved changes
            </span>
          )}
        </button>
      </div>

      {/* Content */}
      {isExpanded && (
        <div className="p-4">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading...</p>
          ) : error ? (
            <div className="text-sm text-destructive">
              Error loading notes: {error instanceof Error ? error.message : String(error)}
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between mb-3">
                <p className="text-xs text-muted-foreground">
                  {notesInfo?.exists
                    ? 'Personal notes for this project (gitignored)'
                    : 'No notes yet. Start typing to create NOTES.md (gitignored)'}
                </p>
                <Button
                  size="sm"
                  onClick={() => saveMutation.mutate()}
                  disabled={!isDirty || saveMutation.isPending}
                >
                  <Save className="h-3 w-3 mr-1" />
                  {saveMutation.isPending ? 'Saving...' : 'Save'}
                </Button>
              </div>
              <CodeBlockCopyButton>
                <div className="mdx-editor-container h-64 border border-border rounded overflow-hidden">
                  <MDXEditor
                    key={`notes-${editorKey}`}
                    ref={editorRef}
                    markdown={notesContent}
                    onChange={(markdown) => {
                      setNotesContent(markdown);
                      setIsDirty(true);
                    }}
                    plugins={[
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
                      codeBlockShortcutPlugin(),
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
                    ]}
                    className={isDarkMode ? 'dark' : ''}
                    contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
                  />
                </div>
              </CodeBlockCopyButton>
            </>
          )}
        </div>
      )}
    </div>
  );
}
