import { Sparkles, Terminal, Bot, FileText, Calendar, Upload } from 'lucide-react';
import type { ProfileDetails } from '../lib/types';

interface ProfileDetailProps {
  profile: ProfileDetails;
}

export function ProfileDetail({ profile }: ProfileDetailProps) {
  const stats = [
    { label: 'Skills', value: profile.skills_count, icon: Sparkles },
    { label: 'Commands', value: profile.commands_count, icon: Terminal },
    { label: 'Agents', value: profile.agents_count, icon: Bot },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h3 className="text-xl font-bold">{profile.name}</h3>
        {profile.description && <p className="text-muted-foreground mt-1">{profile.description}</p>}
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4">
        {stats.map((stat) => (
          <div key={stat.label} className="border rounded-lg p-4 text-center">
            <stat.icon className="h-6 w-6 mx-auto text-muted-foreground mb-2" />
            <div className="text-2xl font-bold">{stat.value}</div>
            <div className="text-xs text-muted-foreground">{stat.label}</div>
          </div>
        ))}
      </div>

      {/* CLAUDE.md indicator */}
      {profile.has_claude_md && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <FileText className="h-4 w-4" />
          <span>Includes CLAUDE.md</span>
        </div>
      )}

      {/* Metadata */}
      <div className="border-t pt-4 space-y-2 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4" />
          <span>Created: {new Date(profile.created_at).toLocaleString()}</span>
        </div>
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4" />
          <span>Updated: {new Date(profile.updated_at).toLocaleString()}</span>
        </div>
      </div>

      {/* Actions */}
      <div className="border-t pt-4 flex gap-2">
        <button className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90">
          Apply to Project
        </button>
        <button className="inline-flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted">
          <Upload className="h-4 w-4" />
          Export as Plugin
        </button>
      </div>
    </div>
  );
}
