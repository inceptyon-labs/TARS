import { Badge } from './ui/badge';
import { cn } from '../lib/utils';
import type { CanonicalRuntime, RuntimeCompatibility, RuntimeSupportLevel } from '../lib/types';

export interface RuntimeSupportItem {
  runtime: CanonicalRuntime | string;
  support: RuntimeSupportLevel;
}

export type InventoryRuntimeKind = 'skill' | 'agent' | 'command' | 'hook' | 'mcp';

const inventoryRuntimeSupport: Record<InventoryRuntimeKind, RuntimeSupportItem[]> = {
  skill: [
    { runtime: 'ClaudeCode', support: 'Native' },
    { runtime: 'Codex', support: 'Convertible' },
  ],
  agent: [
    { runtime: 'ClaudeCode', support: 'Native' },
    { runtime: 'Codex', support: 'Convertible' },
  ],
  command: [
    { runtime: 'ClaudeCode', support: 'Native' },
    { runtime: 'Codex', support: 'Unsupported' },
  ],
  hook: [
    { runtime: 'ClaudeCode', support: 'Native' },
    { runtime: 'Codex', support: 'Partial' },
  ],
  mcp: [
    { runtime: 'ClaudeCode', support: 'Native' },
    { runtime: 'Codex', support: 'Convertible' },
  ],
};

const supportClasses: Record<RuntimeSupportLevel, string> = {
  Native: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  Convertible: 'border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300',
  Partial: 'border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300',
  Unsupported:
    'border-muted-foreground/20 bg-muted/50 text-muted-foreground dark:text-muted-foreground',
};

function runtimeLabel(runtime: string) {
  if (runtime === 'ClaudeCode' || runtime === 'Claude Code') {
    return 'Claude';
  }
  if (runtime === 'Universal') {
    return 'Universal';
  }
  return runtime;
}

export function getRuntimeSupportForKind(kind: InventoryRuntimeKind): RuntimeSupportItem[] {
  return inventoryRuntimeSupport[kind];
}

export function toRuntimeSupportItems(items: RuntimeCompatibility[]): RuntimeSupportItem[] {
  return items.map((item) => ({
    runtime: item.runtime,
    support: item.support,
  }));
}

export function RuntimeBadges({
  items,
  className,
}: {
  items: RuntimeSupportItem[];
  className?: string;
}) {
  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      {items.map((item) => (
        <Badge
          key={`${item.runtime}-${item.support}`}
          variant="outline"
          className={cn(
            'h-6 px-2.5 text-[11px] font-medium whitespace-nowrap leading-none',
            supportClasses[item.support]
          )}
        >
          {runtimeLabel(item.runtime)} {item.support}
        </Badge>
      ))}
    </div>
  );
}
