/**
 * Diff preview component for config operations
 *
 * Shows a unified diff of changes before they're applied.
 */

import { useMemo } from 'react';
import { cn } from '../../lib/utils';

interface DiffPreviewProps {
  /** The diff content (unified diff format) */
  diff: string;
  /** Title for the diff (optional) */
  title?: string;
  /** Additional CSS classes */
  className?: string;
}

interface DiffLine {
  type: 'context' | 'addition' | 'deletion' | 'header';
  content: string;
}

function parseDiff(diff: string): DiffLine[] {
  return diff.split('\n').map((line) => {
    if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('@@')) {
      return { type: 'header', content: line };
    }
    if (line.startsWith('+')) {
      return { type: 'addition', content: line };
    }
    if (line.startsWith('-')) {
      return { type: 'deletion', content: line };
    }
    return { type: 'context', content: line };
  });
}

const lineStyles: Record<DiffLine['type'], string> = {
  context: 'text-muted-foreground',
  addition: 'bg-green-500/20 text-green-400',
  deletion: 'bg-red-500/20 text-red-400',
  header: 'text-blue-400 font-semibold',
};

export function DiffPreview({ diff, title, className }: DiffPreviewProps) {
  const lines = useMemo(() => parseDiff(diff), [diff]);

  if (!diff.trim()) {
    return (
      <div className={cn('rounded-md border p-4 text-center text-muted-foreground', className)}>
        No changes to preview
      </div>
    );
  }

  return (
    <div className={cn('rounded-md border overflow-hidden', className)}>
      {title && (
        <div className="border-b bg-muted/50 px-4 py-2 font-medium">
          {title}
        </div>
      )}
      <div className="overflow-x-auto">
        <pre className="p-4 text-sm font-mono leading-relaxed">
          {lines.map((line, i) => (
            <div
              key={i}
              className={cn('px-2 -mx-2', lineStyles[line.type])}
            >
              {line.content || ' '}
            </div>
          ))}
        </pre>
      </div>
    </div>
  );
}

/**
 * Inline diff for showing changes in a more compact format
 */
interface InlineDiffProps {
  before: string;
  after: string;
  label?: string;
}

export function InlineDiff({ before, after, label }: InlineDiffProps) {
  if (before === after) {
    return (
      <span className="text-muted-foreground">{before}</span>
    );
  }

  return (
    <span className="inline-flex items-center gap-2">
      {label && <span className="text-muted-foreground">{label}:</span>}
      <span className="line-through text-red-400">{before}</span>
      <span className="text-muted-foreground">→</span>
      <span className="text-green-400">{after}</span>
    </span>
  );
}
