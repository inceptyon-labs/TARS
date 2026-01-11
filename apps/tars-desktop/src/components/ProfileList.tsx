import { Trash2, Boxes, Wrench } from 'lucide-react';
import type { ProfileInfo } from '../lib/types';

interface ProfileListProps {
  profiles: ProfileInfo[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

export function ProfileList({ profiles, selectedId, onSelect, onDelete }: ProfileListProps) {
  return (
    <ul className="space-y-1">
      {profiles.map((profile) => (
        <li key={profile.id} className="group relative">
          <div
            role="button"
            tabIndex={0}
            onClick={() => onSelect(profile.id)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onSelect(profile.id);
              }
            }}
            className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all cursor-pointer ${
              selectedId === profile.id
                ? 'active text-foreground font-medium'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-2 min-w-0">
                <Boxes className="h-4 w-4 shrink-0 text-primary/70" />
                <span className="font-medium truncate">{profile.name}</span>
              </div>
              {/* Spacer for delete button */}
              <div className="w-6 shrink-0" />
            </div>
            {profile.description && (
              <div className="text-xs text-muted-foreground/60 mt-0.5 line-clamp-2">
                {profile.description}
              </div>
            )}
            <div className="flex items-center gap-3 mt-1">
              <div className="flex items-center gap-1 text-[10px] text-muted-foreground/60">
                <Wrench className="h-3 w-3" />
                <span>
                  {profile.tool_count} {profile.tool_count === 1 ? 'tool' : 'tools'}
                </span>
              </div>
              <div className="text-[10px] text-muted-foreground/40 uppercase tracking-wider">
                {new Date(profile.created_at).toLocaleDateString()}
              </div>
            </div>
          </div>
          {/* Delete button positioned absolutely to avoid nesting */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete(profile.id);
            }}
            className="absolute top-2.5 right-3 opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded text-destructive"
            aria-label={`Delete ${profile.name}`}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </li>
      ))}
    </ul>
  );
}
