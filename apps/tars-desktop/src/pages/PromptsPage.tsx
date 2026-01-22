/**
 * Prompts management page
 *
 * Personal notes and prompts storage (NOT loaded by Claude).
 * Stored in ~/.tars/prompts/ - outside Claude's config locations.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { ChevronRight, FileText, Plus, RefreshCw, Save, Trash2, X } from 'lucide-react';
import { useState, useEffect, useRef } from 'react';
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
import { codeBlockShortcutPlugin } from '../lib/mdx-plugins/codeBlockShortcutPlugin';
import { CodeBlockCopyButton } from '../components/CodeBlockCopyButton';
import { useUIStore } from '../stores/ui-store';
import { listPrompts, readPrompt, createPrompt, updatePrompt, deletePrompt } from '../lib/ipc';
import type { PromptSummary } from '../lib/types';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { ConfirmDialog } from '../components/config/ConfirmDialog';

export function PromptsPage() {
  const queryClient = useQueryClient();
  const theme = useUIStore((state) => state.theme);
  const editorRef = useRef<MDXEditorMethods>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editContent, setEditContent] = useState('');
  const [promptToDelete, setPromptToDelete] = useState<PromptSummary | null>(null);
  const [editorKey, setEditorKey] = useState(0); // Force remount of MDXEditor

  // Fetch prompts list
  const { data: prompts = [], isLoading } = useQuery({
    queryKey: ['prompts'],
    queryFn: listPrompts,
  });

  // Fetch selected prompt details
  const { data: selectedPrompt, isLoading: loadingPrompt } = useQuery({
    queryKey: ['prompt', selectedId],
    queryFn: () => (selectedId ? readPrompt(selectedId) : null),
    enabled: !!selectedId && !isCreating,
  });

  // Update edit state when selected prompt changes
  useEffect(() => {
    if (selectedPrompt && !isCreating) {
      setEditTitle(selectedPrompt.title);
      setEditContent(selectedPrompt.content);
    }
  }, [selectedPrompt, isCreating]);

  // Create mutation
  const createMutation = useMutation({
    mutationFn: ({ title, content }: { title: string; content: string }) =>
      createPrompt(title, content),
    onSuccess: (newPrompt) => {
      queryClient.invalidateQueries({ queryKey: ['prompts'] });
      setSelectedId(newPrompt.id);
      setIsCreating(false);
      setIsEditing(false);
      toast.success('Prompt created');
    },
    onError: (err) => {
      toast.error('Failed to create prompt', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, title, content }: { id: string; title: string; content: string }) =>
      updatePrompt(id, title, content),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['prompts'] }),
        queryClient.invalidateQueries({ queryKey: ['prompt', selectedId] }),
      ]);
      setIsEditing(false);
      toast.success('Prompt saved');
    },
    onError: (err) => {
      toast.error('Failed to save prompt', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => deletePrompt(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['prompts'] });
      if (selectedId === promptToDelete?.id) {
        setSelectedId(null);
        setIsEditing(false);
      }
      setPromptToDelete(null);
      toast.success('Prompt deleted');
    },
    onError: (err) => {
      toast.error('Failed to delete prompt', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  function handleCreate() {
    setIsCreating(true);
    setSelectedId(null);
    setEditTitle('');
    setEditContent('');
    setIsEditing(true);
    setEditorKey((k) => k + 1); // Force new editor instance
  }

  function handleSave() {
    if (!editTitle.trim()) {
      toast.error('Title is required');
      return;
    }

    // Get current content from editor
    const content = editorRef.current?.getMarkdown() || editContent;

    if (isCreating) {
      createMutation.mutate({ title: editTitle, content });
    } else if (selectedId) {
      updateMutation.mutate({ id: selectedId, title: editTitle, content });
    }
  }

  function handleCancel() {
    if (isCreating) {
      setIsCreating(false);
      setIsEditing(false);
      setEditTitle('');
      setEditContent('');
    } else if (selectedPrompt) {
      setEditTitle(selectedPrompt.title);
      setEditContent(selectedPrompt.content);
      setIsEditing(false);
    }
    setEditorKey((k) => k + 1); // Force editor remount on cancel
  }

  function handleSelect(prompt: PromptSummary) {
    if (isCreating) {
      setIsCreating(false);
    }
    setSelectedId(prompt.id);
    setIsEditing(false);
  }

  function handleStartEditing() {
    setIsEditing(true);
    setEditorKey((k) => k + 1); // Force new editor instance with current content
  }

  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  const isSaving = createMutation.isPending || updateMutation.isPending;

  // Editor plugins configuration
  const editorPlugins = [
    headingsPlugin(),
    listsPlugin(),
    quotePlugin(),
    thematicBreakPlugin(),
    markdownShortcutPlugin(),
    codeBlockShortcutPlugin(),
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

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-6 w-6 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Prompts</h2>
        </div>
        <Button size="sm" onClick={handleCreate}>
          <Plus className="h-4 w-4 mr-2" />
          New Prompt
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Prompts List */}
        <div className="w-72 border-r border-border overflow-auto">
          {prompts.length === 0 && !isCreating ? (
            <div className="p-6 text-center text-muted-foreground">
              <FileText className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p className="text-sm">No prompts yet</p>
              <p className="text-xs mt-1">Click "New Prompt" to create one</p>
            </div>
          ) : (
            <div className="divide-y divide-border">
              {prompts.map((prompt) => (
                <button
                  key={prompt.id}
                  type="button"
                  onClick={() => handleSelect(prompt)}
                  className={`w-full text-left p-4 hover:bg-muted/50 transition-colors group ${
                    selectedId === prompt.id && !isCreating ? 'bg-muted' : ''
                  }`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <ChevronRight
                          className={`h-4 w-4 shrink-0 transition-transform ${
                            selectedId === prompt.id && !isCreating ? 'rotate-90' : ''
                          }`}
                        />
                        <span className="font-medium truncate">{prompt.title}</span>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1 ml-6 line-clamp-2">
                        {prompt.preview || 'No content'}
                      </p>
                      <p className="text-xs text-muted-foreground/60 mt-1 ml-6">
                        {formatDate(prompt.updated_at)}
                      </p>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="opacity-0 group-hover:opacity-100 shrink-0 h-6 w-6 p-0 text-muted-foreground hover:text-destructive"
                      onClick={(e) => {
                        e.stopPropagation();
                        setPromptToDelete(prompt);
                      }}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Editor Panel */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {isCreating || selectedPrompt ? (
            <>
              {/* Editor Header */}
              <div className="h-12 border-b border-border px-4 flex items-center justify-between shrink-0 bg-muted/30">
                <div className="flex items-center gap-3 flex-1 min-w-0">
                  {isEditing ? (
                    <Input
                      value={editTitle}
                      onChange={(e) => setEditTitle(e.target.value)}
                      placeholder="Prompt title..."
                      className="h-8 text-sm font-medium max-w-md"
                      autoFocus={isCreating}
                    />
                  ) : (
                    <span className="font-medium truncate">
                      {selectedPrompt?.title || 'New Prompt'}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  {!isEditing && !isCreating && (
                    <Button variant="outline" size="sm" onClick={handleStartEditing}>
                      Edit
                    </Button>
                  )}
                  {isEditing && (
                    <>
                      <Button variant="ghost" size="sm" onClick={handleCancel} disabled={isSaving}>
                        <X className="h-4 w-4 mr-1" />
                        Cancel
                      </Button>
                      <Button size="sm" onClick={handleSave} disabled={isSaving}>
                        {isSaving ? (
                          <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                        ) : (
                          <Save className="h-4 w-4 mr-1" />
                        )}
                        Save
                      </Button>
                    </>
                  )}
                </div>
              </div>

              {/* Editor Content */}
              <CodeBlockCopyButton>
                <div className="flex-1 flex flex-col min-h-0">
                  {loadingPrompt ? (
                    <div className="h-full flex items-center justify-center">
                      <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
                    </div>
                  ) : isEditing ? (
                    <div className="h-full mdx-editor-container">
                      <MDXEditor
                        key={`edit-${editorKey}`}
                        ref={editorRef}
                        markdown={editContent}
                        onChange={(markdown) => setEditContent(markdown)}
                        plugins={editorPlugins}
                        className={
                          theme === 'dark' ||
                          (theme === 'system' &&
                            window.matchMedia('(prefers-color-scheme: dark)').matches)
                            ? 'dark'
                            : ''
                        }
                        contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
                      />
                    </div>
                  ) : (
                    <div className="h-full mdx-editor-container">
                      <MDXEditor
                        key={`view-${selectedId}`}
                        markdown={selectedPrompt?.content || ''}
                        readOnly
                        plugins={[
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
                        ]}
                        className={
                          theme === 'dark' ||
                          (theme === 'system' &&
                            window.matchMedia('(prefers-color-scheme: dark)').matches)
                            ? 'dark'
                            : ''
                        }
                        contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full"
                      />
                    </div>
                  )}
                </div>
              </CodeBlockCopyButton>
            </>
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <FileText className="h-16 w-16 mx-auto mb-4 opacity-30" />
                <p className="text-sm">Select a prompt to view</p>
                <p className="text-xs mt-1">or create a new one</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={!!promptToDelete}
        onOpenChange={(open) => !open && setPromptToDelete(null)}
        title="Delete Prompt"
        description={`Are you sure you want to delete "${promptToDelete?.title}"? This action cannot be undone.`}
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={() => promptToDelete && deleteMutation.mutate(promptToDelete.id)}
        loading={deleteMutation.isPending}
      />
    </div>
  );
}
