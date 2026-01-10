import { useState, useMemo } from 'react';
import Editor, { DiffEditor } from '@monaco-editor/react';
import {
  ChevronDown,
  ChevronRight,
  Plus,
  Pencil,
  Trash2,
  AlertTriangle,
  FileText,
} from 'lucide-react';
import type { DiffPreview as DiffPreviewType, OperationPreview } from '../lib/types';

interface DiffPreviewProps {
  preview: DiffPreviewType;
  onCancel?: () => void;
  onConfirm?: () => void;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function OperationIcon({ type }: { type: OperationPreview['operation_type'] }) {
  switch (type) {
    case 'create':
      return <Plus className="h-4 w-4 text-green-500" />;
    case 'modify':
      return <Pencil className="h-4 w-4 text-yellow-500" />;
    case 'delete':
      return <Trash2 className="h-4 w-4 text-red-500" />;
  }
}

function OperationBadge({ type }: { type: OperationPreview['operation_type'] }) {
  const colors = {
    create: 'bg-green-500/10 text-green-600 border-green-500/20',
    modify: 'bg-yellow-500/10 text-yellow-600 border-yellow-500/20',
    delete: 'bg-red-500/10 text-red-600 border-red-500/20',
  };

  return (
    <span className={`px-2 py-0.5 text-xs rounded border ${colors[type]}`}>
      {type.charAt(0).toUpperCase() + type.slice(1)}
    </span>
  );
}

interface OperationItemProps {
  operation: OperationPreview;
  isExpanded: boolean;
  onToggle: () => void;
}

function OperationItem({ operation, isExpanded, onToggle }: OperationItemProps) {
  const fileName = operation.path.split('/').pop() || operation.path;
  const dirPath = operation.path.slice(0, -(fileName.length + 1)) || '.';

  // Parse unified diff to get old and new content for Monaco DiffEditor
  const { oldContent, newContent } = useMemo(() => {
    if (!operation.diff) {
      return { oldContent: '', newContent: '' };
    }

    const lines = operation.diff.split('\n');
    const oldLines: string[] = [];
    const newLines: string[] = [];

    for (const line of lines) {
      if (line.startsWith('-')) {
        oldLines.push(line.slice(1));
      } else if (line.startsWith('+')) {
        newLines.push(line.slice(1));
      } else if (line.startsWith(' ')) {
        oldLines.push(line.slice(1));
        newLines.push(line.slice(1));
      }
    }

    return {
      oldContent: oldLines.join('\n'),
      newContent: newLines.join('\n'),
    };
  }, [operation.diff]);

  return (
    <div className="border rounded-lg overflow-hidden">
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-3 p-3 hover:bg-muted/50 transition-colors text-left"
      >
        {operation.diff ? (
          isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
          )
        ) : (
          <span className="w-4 shrink-0" />
        )}

        <OperationIcon type={operation.operation_type} />

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
            <span className="font-medium truncate">{fileName}</span>
            <OperationBadge type={operation.operation_type} />
          </div>
          <div className="text-xs text-muted-foreground truncate">{dirPath}</div>
        </div>

        {operation.size && (
          <span className="text-xs text-muted-foreground shrink-0">
            {formatFileSize(operation.size)}
          </span>
        )}
      </button>

      {isExpanded && operation.diff && (
        <div className="border-t">
          <DiffEditor
            height="300px"
            original={oldContent}
            modified={newContent}
            language="markdown"
            theme="vs-dark"
            options={{
              readOnly: true,
              renderSideBySide: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              fontSize: 12,
              lineNumbers: 'on',
              folding: false,
              wordWrap: 'on',
            }}
          />
        </div>
      )}
    </div>
  );
}

export function DiffPreviewComponent({ preview, onCancel, onConfirm }: DiffPreviewProps) {
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  const toggleItem = (path: string) => {
    setExpandedItems((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const expandAll = () => {
    setExpandedItems(new Set(preview.operations.filter((op) => op.diff).map((op) => op.path)));
  };

  const collapseAll = () => {
    setExpandedItems(new Set());
  };

  const counts = useMemo(() => {
    return {
      create: preview.operations.filter((op) => op.operation_type === 'create').length,
      modify: preview.operations.filter((op) => op.operation_type === 'modify').length,
      delete: preview.operations.filter((op) => op.operation_type === 'delete').length,
    };
  }, [preview.operations]);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div>
          <h2 className="text-lg font-semibold">Review Changes</h2>
          <p className="text-sm text-muted-foreground">{preview.summary}</p>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-3 text-sm">
            {counts.create > 0 && (
              <span className="flex items-center gap-1 text-green-600">
                <Plus className="h-4 w-4" />
                {counts.create}
              </span>
            )}
            {counts.modify > 0 && (
              <span className="flex items-center gap-1 text-yellow-600">
                <Pencil className="h-4 w-4" />
                {counts.modify}
              </span>
            )}
            {counts.delete > 0 && (
              <span className="flex items-center gap-1 text-red-600">
                <Trash2 className="h-4 w-4" />
                {counts.delete}
              </span>
            )}
          </div>
          <div className="flex gap-2">
            <button
              onClick={expandAll}
              className="text-xs text-muted-foreground hover:text-foreground"
            >
              Expand all
            </button>
            <span className="text-muted-foreground">/</span>
            <button
              onClick={collapseAll}
              className="text-xs text-muted-foreground hover:text-foreground"
            >
              Collapse all
            </button>
          </div>
        </div>
      </div>

      {/* Warnings */}
      {preview.warnings.length > 0 && (
        <div className="p-4 bg-yellow-500/10 border-b border-yellow-500/20">
          <div className="flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 text-yellow-600 shrink-0 mt-0.5" />
            <div>
              <h3 className="font-medium text-yellow-700">Warnings</h3>
              <ul className="mt-1 space-y-1">
                {preview.warnings.map((warning, idx) => (
                  <li key={idx} className="text-sm text-yellow-600">
                    {warning}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      )}

      {/* Operations list */}
      <div className="flex-1 overflow-auto p-4">
        <div className="space-y-2">
          {preview.operations.map((operation) => (
            <OperationItem
              key={operation.path}
              operation={operation}
              isExpanded={expandedItems.has(operation.path)}
              onToggle={() => toggleItem(operation.path)}
            />
          ))}
        </div>
      </div>

      {/* Terminal output (if any) */}
      {preview.terminal_output && (
        <div className="border-t p-4">
          <h3 className="text-sm font-medium mb-2">Terminal Output</h3>
          <Editor
            height="150px"
            defaultValue={preview.terminal_output}
            language="plaintext"
            theme="vs-dark"
            options={{
              readOnly: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              fontSize: 11,
              lineNumbers: 'off',
              folding: false,
              wordWrap: 'on',
            }}
          />
        </div>
      )}

      {/* Actions */}
      {(onCancel || onConfirm) && (
        <div className="flex justify-end gap-3 p-4 border-t bg-muted/30">
          {onCancel && (
            <button
              onClick={onCancel}
              className="px-4 py-2 text-sm border rounded-md hover:bg-muted transition-colors"
            >
              Cancel
            </button>
          )}
          {onConfirm && (
            <button
              onClick={onConfirm}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              Apply Changes
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export { DiffPreviewComponent as DiffPreview };
