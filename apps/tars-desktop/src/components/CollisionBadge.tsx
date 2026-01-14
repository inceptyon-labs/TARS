import { AlertTriangle } from 'lucide-react';
import type { Collision, CollisionOccurrence } from '../lib/types';

interface CollisionBadgeProps {
  collision: Collision;
  type: 'skill' | 'command' | 'agent';
}

export function CollisionBadge({ collision, type }: CollisionBadgeProps) {
  const formatScope = (scope: CollisionOccurrence['scope']) => {
    if (typeof scope === 'string') {
      return scope;
    }
    switch (scope.type) {
      case 'User':
        return 'User';
      case 'Project':
        return 'Project';
      case 'Local':
        return 'Local';
      case 'Managed':
        return 'Managed';
      case 'Plugin':
        return scope.plugin_id ? `Plugin (${scope.plugin_id})` : 'Plugin';
      default:
        return 'Unknown';
    }
  };

  return (
    <div className="border border-destructive/20 bg-destructive/5 rounded-lg p-3 mb-2">
      <div className="flex items-center gap-2 text-destructive">
        <AlertTriangle className="h-4 w-4" />
        <span className="font-medium">
          {type === 'command' ? '/' : ''}
          {collision.name}
        </span>
        <span className="text-xs bg-destructive/10 px-1.5 py-0.5 rounded">
          {collision.occurrences.length} occurrences
        </span>
      </div>
      <div className="mt-2 space-y-1">
        {collision.occurrences.map((occurrence, i) => (
          <div key={i} className="text-xs text-muted-foreground flex items-center gap-2">
            <span className="font-medium text-foreground">{formatScope(occurrence.scope)}:</span>
            <span className="truncate">{occurrence.path}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
