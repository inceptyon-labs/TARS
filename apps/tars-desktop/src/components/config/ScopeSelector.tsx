/**
 * Scope selector component
 *
 * Allows selecting user/project/local scope for config operations.
 */

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Label } from '../ui/label';
import type { Scope } from './types';

interface ScopeSelectorProps {
  value: Scope;
  onChange: (scope: Scope) => void;
  /** Available scope options (defaults to all) */
  options?: Scope[];
  /** Whether to show the label */
  showLabel?: boolean;
  /** Whether the selector is disabled */
  disabled?: boolean;
}

const scopeLabels: Record<Scope, string> = {
  user: 'User (Global)',
  project: 'Project',
  local: 'Local (Gitignored)',
};

const scopeDescriptions: Record<Scope, string> = {
  user: '~/.claude/ - Available in all projects',
  project: '.claude/ - Committed to repository',
  local: '.claude/settings.local.json - Not committed',
};

export function ScopeSelector({
  value,
  onChange,
  options = ['user', 'project', 'local'],
  showLabel = true,
  disabled = false,
}: ScopeSelectorProps) {
  return (
    <div className="flex flex-col gap-2">
      {showLabel && <Label htmlFor="scope-select">Scope</Label>}
      <Select value={value} onValueChange={(v) => onChange(v as Scope)} disabled={disabled}>
        <SelectTrigger id="scope-select" className="w-full">
          <SelectValue placeholder="Select scope" />
        </SelectTrigger>
        <SelectContent>
          {options.map((scope) => (
            <SelectItem key={scope} value={scope}>
              <div className="flex flex-col">
                <span>{scopeLabels[scope]}</span>
                <span className="text-xs text-muted-foreground">{scopeDescriptions[scope]}</span>
              </div>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
