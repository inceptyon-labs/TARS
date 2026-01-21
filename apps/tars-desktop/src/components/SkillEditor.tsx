import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import {
  Save,
  RotateCcw,
  ChevronDown,
  ChevronRight,
  FileText,
  FileCode,
  FileJson,
  File,
  Link2,
  Plus,
  Trash2,
  AlertCircle,
} from 'lucide-react';
import { toast } from 'sonner';
import yaml from 'js-yaml';
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
import type { SkillDetails, SupportingFile } from '../lib/types';
import { readSupportingFile, saveSupportingFile, deleteSupportingFile } from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { codeBlockShortcutPlugin } from '../lib/mdx-plugins/codeBlockShortcutPlugin';
import { CodeBlockCopyButton } from './CodeBlockCopyButton';

interface SkillEditorProps {
  skill: SkillDetails;
  onSave: (path: string, content: string) => Promise<void>;
  readOnly?: boolean;
}

/** Parse frontmatter from markdown content, preserving whitespace for roundtrip */
function parseFrontmatter(content: string): {
  frontmatter: string | null;
  body: string;
  leadingWhitespace: string;
  separator: string;
} {
  // Capture leading whitespace
  const leadingMatch = content.match(/^(\s*)/);
  const leadingWhitespace = leadingMatch ? leadingMatch[1] : '';
  const trimmed = content.trimStart();

  if (!trimmed.startsWith('---')) {
    return { frontmatter: null, body: content, leadingWhitespace: '', separator: '' };
  }

  // Find the closing ---
  const endIndex = trimmed.indexOf('---', 3);
  if (endIndex === -1) {
    return { frontmatter: null, body: content, leadingWhitespace: '', separator: '' };
  }

  // Extract frontmatter (without the --- delimiters, but preserve internal whitespace)
  const frontmatterRaw = trimmed.slice(3, endIndex);
  // Trim only leading newline, preserve trailing newline for proper reconstruction
  const frontmatter = frontmatterRaw.replace(/^\n/, '').replace(/\n$/, '');

  // Extract the separator between closing --- and body (preserve exact whitespace)
  const afterClosing = trimmed.slice(endIndex + 3);
  const separatorMatch = afterClosing.match(/^(\s*)/);
  const separator = separatorMatch ? separatorMatch[1] : '';

  // Extract body (after the separator)
  const body = afterClosing.slice(separator.length);

  return { frontmatter, body, leadingWhitespace, separator };
}

/** Combine frontmatter and body back into markdown, preserving original whitespace */
function combineFrontmatter(
  frontmatter: string | null,
  body: string,
  leadingWhitespace = '',
  separator = '\n\n'
): string {
  if (!frontmatter) {
    return body;
  }
  return `${leadingWhitespace}---\n${frontmatter}\n---${separator}${body}`;
}

/** Validate YAML frontmatter and return error message if invalid */
function validateYaml(content: string): string | null {
  if (!content.trim()) {
    return null; // Empty is valid
  }
  try {
    yaml.load(content);
    return null;
  } catch (e) {
    if (e instanceof yaml.YAMLException) {
      // Extract just the useful part of the error message
      const message = e.message.split('\n')[0];
      return message || 'Invalid YAML syntax';
    }
    return 'Invalid YAML syntax';
  }
}

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

/** Get icon for file type */
function getFileIcon(fileType: string) {
  switch (fileType) {
    case 'markdown':
      return FileText;
    case 'script':
      return FileCode;
    case 'config':
      return FileJson;
    default:
      return File;
  }
}

export function SkillEditor({ skill, onSave, readOnly = false }: SkillEditorProps) {
  const editorRef = useRef<MDXEditorMethods>(null);
  const theme = useUIStore((state) => state.theme);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [frontmatterExpanded, setFrontmatterExpanded] = useState(true);
  const [supportingFilesExpanded, setSupportingFilesExpanded] = useState(false);
  const [expandedFile, setExpandedFile] = useState<string | null>(null);
  const [fileContents, setFileContents] = useState<Record<string, string>>({});
  const [loadingFile, setLoadingFile] = useState<string | null>(null);
  const [showNewFileForm, setShowNewFileForm] = useState(false);
  const [newFileName, setNewFileName] = useState('');
  const [newFileContent, setNewFileContent] = useState('');
  const [creatingFile, setCreatingFile] = useState(false);
  const [deletingFile, setDeletingFile] = useState<string | null>(null);
  const [localSupportingFiles, setLocalSupportingFiles] = useState<SupportingFile[]>([]);

  // Parse frontmatter from the original content, preserving whitespace
  const {
    frontmatter: originalFrontmatter,
    body: originalBody,
    leadingWhitespace: originalLeadingWhitespace,
    separator: originalSeparator,
  } = useMemo(() => parseFrontmatter(skill.content), [skill.content]);

  // State for editable frontmatter and body
  const [frontmatter, setFrontmatter] = useState(originalFrontmatter);
  const [body, setBody] = useState(originalBody);
  // Store original whitespace for accurate roundtrip
  const [leadingWhitespace, setLeadingWhitespace] = useState(originalLeadingWhitespace);
  const [separator, setSeparator] = useState(originalSeparator);
  // Track if the editor is initializing to ignore MDXEditor's normalization onChange
  const [isInitializing, setIsInitializing] = useState(true);

  // Validate YAML frontmatter in real-time
  const yamlError = useMemo(() => (frontmatter ? validateYaml(frontmatter) : null), [frontmatter]);

  // Sync content when skill changes
  useEffect(() => {
    const parsed = parseFrontmatter(skill.content);
    setFrontmatter(parsed.frontmatter);
    setBody(parsed.body);
    setLeadingWhitespace(parsed.leadingWhitespace);
    setSeparator(parsed.separator);
    setError(null);
    setIsInitializing(true); // Mark as initializing to ignore MDXEditor's first onChange
    setEditorKey((k) => k + 1); // Force editor remount
    // Reset supporting files state
    setExpandedFile(null);
    setFileContents({});
    setSupportingFilesExpanded(false);
    setShowNewFileForm(false);
    setNewFileName('');
    setNewFileContent('');
    // Sync local supporting files from skill
    setLocalSupportingFiles(skill.supporting_files || []);
  }, [skill.path, skill.content, skill.supporting_files]);

  // Clear initializing flag after editor has mounted and fired its initial onChange
  useEffect(() => {
    if (isInitializing) {
      const timer = setTimeout(() => setIsInitializing(false), 100);
      return () => clearTimeout(timer);
    }
  }, [isInitializing, editorKey]);

  // Load supporting file content when expanded
  const handleExpandFile = useCallback(
    async (file: SupportingFile) => {
      if (expandedFile === file.path) {
        setExpandedFile(null);
        return;
      }

      setExpandedFile(file.path);

      // Check if we already have the content cached
      if (fileContents[file.path]) {
        return;
      }

      setLoadingFile(file.path);
      try {
        const content = await readSupportingFile(file.path);
        setFileContents((prev) => ({ ...prev, [file.path]: content }));
      } catch (err) {
        console.error('Failed to load supporting file:', err);
        setFileContents((prev) => ({ ...prev, [file.path]: `Error loading file: ${err}` }));
      } finally {
        setLoadingFile(null);
      }
    },
    [expandedFile, fileContents]
  );

  // Create a new supporting file
  const handleCreateFile = useCallback(async () => {
    if (!newFileName.trim()) return;

    setCreatingFile(true);
    try {
      const newFile = await saveSupportingFile(skill.path, newFileName.trim(), newFileContent);
      setLocalSupportingFiles((prev) => [...prev, newFile]);
      setFileContents((prev) => ({ ...prev, [newFile.path]: newFileContent }));
      setShowNewFileForm(false);
      setNewFileName('');
      setNewFileContent('');
      toast.success(`Created "${newFileName}"`);
    } catch (err) {
      console.error('Failed to create file:', err);
      toast.error('Failed to create file', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setCreatingFile(false);
    }
  }, [skill.path, newFileName, newFileContent]);

  // Delete a supporting file
  const handleDeleteFile = useCallback(
    async (file: SupportingFile) => {
      setDeletingFile(file.path);
      try {
        await deleteSupportingFile(file.path);
        setLocalSupportingFiles((prev) => prev.filter((f) => f.path !== file.path));
        if (expandedFile === file.path) {
          setExpandedFile(null);
        }
        // Remove from cache
        setFileContents((prev) => {
          const copy = { ...prev };
          delete copy[file.path];
          return copy;
        });
        toast.success(`Deleted "${file.name}"`);
      } catch (err) {
        console.error('Failed to delete file:', err);
        toast.error('Failed to delete file', {
          description: err instanceof Error ? err.message : String(err),
        });
      } finally {
        setDeletingFile(null);
      }
    },
    [expandedFile]
  );

  // Check if content has changed (using preserved whitespace for accurate comparison)
  const currentContent = combineFrontmatter(frontmatter, body, leadingWhitespace, separator);
  const hasChanges = currentContent !== skill.content && !readOnly;

  const handleSave = useCallback(async () => {
    if (!hasChanges) return;
    if (yamlError) {
      toast.error('Invalid YAML frontmatter', {
        description: yamlError,
      });
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const editorBody = editorRef.current?.getMarkdown() || body;
      const fullContent = combineFrontmatter(frontmatter, editorBody, leadingWhitespace, separator);
      await onSave(skill.path, fullContent);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [body, frontmatter, leadingWhitespace, separator, hasChanges, onSave, skill.path, yamlError]);

  const handleReset = useCallback(() => {
    const parsed = parseFrontmatter(skill.content);
    setFrontmatter(parsed.frontmatter);
    setBody(parsed.body);
    setLeadingWhitespace(parsed.leadingWhitespace);
    setSeparator(parsed.separator);
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
            {hasChanges && <span className="text-xs text-muted-foreground">Unsaved changes</span>}
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
              disabled={!hasChanges || saving || !!yamlError}
              className="inline-flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
              title={yamlError ? 'Fix YAML errors before saving' : undefined}
            >
              <Save className="h-4 w-4" />
              {saving ? 'Saving...' : 'Save'}
            </button>
          </div>
        )}
      </div>

      {/* Error message */}
      {error && (
        <div className="px-3 py-2 bg-destructive/10 text-destructive text-sm shrink-0">{error}</div>
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
                onChange={
                  !readOnly
                    ? (e) => {
                        setFrontmatter(e.target.value);
                        // Auto-resize on change
                        const el = e.target;
                        el.style.height = 'auto';
                        el.style.height = `${el.scrollHeight + 4}px`;
                      }
                    : undefined
                }
                readOnly={readOnly}
                className={`w-full bg-secondary text-secondary-foreground font-mono text-sm p-3 rounded-lg border focus:outline-none focus:ring-2 overflow-hidden ${
                  yamlError
                    ? 'border-destructive focus:ring-destructive/50'
                    : 'border-border focus:ring-primary/50'
                }`}
                spellCheck={false}
              />
              {yamlError && (
                <div className="mt-2 flex items-start gap-2 text-destructive text-xs">
                  <AlertCircle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
                  <span>{yamlError}</span>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Supporting Files panel */}
      <div className="border-b border-border shrink-0">
        <button
          onClick={() => setSupportingFilesExpanded(!supportingFilesExpanded)}
          className="w-full px-4 py-2 flex items-center gap-2 text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
        >
          {supportingFilesExpanded ? (
            <ChevronDown className="h-3 w-3" />
          ) : (
            <ChevronRight className="h-3 w-3" />
          )}
          <span className="uppercase tracking-wider">Supporting Files</span>
          <span className="text-muted-foreground/60">({localSupportingFiles.length})</span>
        </button>
        {supportingFilesExpanded && (
          <div className="px-4 pb-3 space-y-2">
            {/* File list */}
            {localSupportingFiles.length > 0 && (
              <div className="space-y-1">
                {localSupportingFiles.map((file) => {
                  const FileIcon = getFileIcon(file.file_type);
                  const isExpanded = expandedFile === file.path;
                  const isLoading = loadingFile === file.path;
                  const isDeleting = deletingFile === file.path;
                  const content = fileContents[file.path];

                  return (
                    <div
                      key={file.path}
                      className="border border-border rounded-lg overflow-hidden"
                    >
                      <div className="flex items-center">
                        <button
                          onClick={() => handleExpandFile(file)}
                          className="flex-1 px-3 py-2 flex items-center gap-2 text-sm hover:bg-muted/50 transition-colors"
                        >
                          {isExpanded ? (
                            <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
                          ) : (
                            <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
                          )}
                          <FileIcon className="h-4 w-4 text-muted-foreground shrink-0" />
                          <span className="font-medium truncate">{file.name}</span>
                          {file.is_referenced && (
                            <span className="flex items-center gap-1 text-xs text-primary bg-primary/10 px-1.5 py-0.5 rounded">
                              <Link2 className="h-3 w-3" />
                              linked
                            </span>
                          )}
                          <span className="text-xs text-muted-foreground/60 ml-auto shrink-0">
                            {file.file_type}
                          </span>
                        </button>
                        {!readOnly && (
                          <button
                            onClick={() => handleDeleteFile(file)}
                            disabled={isDeleting}
                            className="p-2 text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                            title="Delete file"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        )}
                      </div>
                      {isExpanded && (
                        <div className="border-t border-border">
                          {isLoading ? (
                            <div className="p-3 text-sm text-muted-foreground">Loading...</div>
                          ) : content ? (
                            <pre className="p-3 text-xs font-mono bg-secondary text-secondary-foreground overflow-auto max-h-64 whitespace-pre-wrap">
                              {content}
                            </pre>
                          ) : null}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}

            {/* New file form */}
            {!readOnly && showNewFileForm && (
              <div className="border border-border rounded-lg p-3 space-y-2">
                <input
                  type="text"
                  value={newFileName}
                  onChange={(e) => setNewFileName(e.target.value)}
                  placeholder="reference.md or scripts/helper.py"
                  className="w-full px-3 py-1.5 text-sm bg-secondary border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary/50"
                  autoFocus
                />
                <textarea
                  value={newFileContent}
                  onChange={(e) => setNewFileContent(e.target.value)}
                  placeholder="File content (optional)"
                  rows={4}
                  className="w-full px-3 py-2 text-sm font-mono bg-secondary border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary/50 resize-none"
                />
                <div className="flex gap-2 justify-end">
                  <button
                    onClick={() => {
                      setShowNewFileForm(false);
                      setNewFileName('');
                      setNewFileContent('');
                    }}
                    className="px-3 py-1 text-sm text-muted-foreground hover:text-foreground"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleCreateFile}
                    disabled={!newFileName.trim() || creatingFile}
                    className="px-3 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50"
                  >
                    {creatingFile ? 'Creating...' : 'Create'}
                  </button>
                </div>
              </div>
            )}

            {/* Add file button */}
            {!readOnly && !showNewFileForm && (
              <button
                onClick={() => setShowNewFileForm(true)}
                className="w-full px-3 py-2 flex items-center justify-center gap-2 text-sm text-muted-foreground hover:text-foreground hover:bg-muted/50 border border-dashed border-border rounded-lg transition-colors"
              >
                <Plus className="h-4 w-4" />
                Add supporting file
              </button>
            )}

            {/* Empty state */}
            {localSupportingFiles.length === 0 && !showNewFileForm && (
              <p className="text-xs text-muted-foreground/60 text-center py-2">
                No supporting files yet
              </p>
            )}
          </div>
        )}
      </div>

      {/* Editor */}
      <CodeBlockCopyButton>
        <div className="flex-1 mdx-editor-container overflow-auto">
          <MDXEditor
            key={`skill-${editorKey}`}
            ref={readOnly ? undefined : editorRef}
            markdown={body}
            onChange={
              readOnly
                ? undefined
                : (markdown) => {
                    // Ignore MDXEditor's initial onChange during mount/normalization
                    if (!isInitializing) {
                      setBody(markdown);
                    }
                  }
            }
            readOnly={readOnly}
            plugins={readOnly ? readOnlyPlugins : editorPlugins}
            className={
              theme === 'dark' ||
              (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
                ? 'dark'
                : ''
            }
            contentEditableClassName="prose prose-sm dark:prose-invert max-w-none p-4 min-h-full focus:outline-none"
          />
        </div>
      </CodeBlockCopyButton>
    </div>
  );
}
