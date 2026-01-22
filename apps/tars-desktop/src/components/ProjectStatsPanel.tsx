import { useQuery } from '@tanstack/react-query';
import {
  Code2,
  FileCode,
  Package,
  CheckCircle2,
  AlertTriangle,
  ListTodo,
  ChevronDown,
  ChevronRight,
  BarChart3,
} from 'lucide-react';
import { useState } from 'react';
import { getProjectStats } from '../lib/ipc';
import type { LanguageStats } from '../lib/ipc';

interface ProjectStatsPanelProps {
  projectPath: string;
}

function formatNumber(n: number): string {
  if (n >= 1000000) {
    return (n / 1000000).toFixed(1) + 'M';
  }
  if (n >= 1000) {
    return (n / 1000).toFixed(1) + 'K';
  }
  return n.toString();
}

function LanguageBar({ languages }: { languages: Record<string, LanguageStats> }) {
  const sorted = Object.entries(languages)
    .map(([name, stats]) => ({ name, ...stats }))
    .sort((a, b) => b.code - a.code);

  const total = sorted.reduce((sum, l) => sum + l.code, 0);
  if (total === 0) return null;

  // Language colors
  const colors: Record<string, string> = {
    TypeScript: 'bg-blue-500',
    JavaScript: 'bg-yellow-400',
    Rust: 'bg-orange-500',
    Dart: 'bg-sky-400',
    Python: 'bg-green-500',
    Go: 'bg-cyan-500',
    Java: 'bg-red-500',
    'C++': 'bg-pink-500',
    C: 'bg-gray-500',
    'C#': 'bg-purple-500',
    Ruby: 'bg-red-400',
    PHP: 'bg-indigo-400',
    Swift: 'bg-orange-400',
    Kotlin: 'bg-purple-400',
    HTML: 'bg-orange-300',
    CSS: 'bg-blue-400',
    Shell: 'bg-green-400',
    JSON: 'bg-gray-400',
    YAML: 'bg-red-300',
    Markdown: 'bg-gray-300',
    Vue: 'bg-emerald-500',
    Svelte: 'bg-orange-600',
  };

  const topLanguages = sorted.slice(0, 6);

  return (
    <div className="space-y-2">
      <div className="flex h-2 rounded-full overflow-hidden bg-muted">
        {topLanguages.map((lang) => (
          <div
            key={lang.name}
            className={`${colors[lang.name] || 'bg-gray-400'}`}
            style={{ width: `${(lang.code / total) * 100}%` }}
            title={`${lang.name}: ${formatNumber(lang.code)} lines`}
          />
        ))}
      </div>
      <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs">
        {topLanguages.map((lang) => (
          <div key={lang.name} className="flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${colors[lang.name] || 'bg-gray-400'}`} />
            <span className="text-muted-foreground">
              {lang.name}{' '}
              <span className="text-foreground font-medium">
                {((lang.code / total) * 100).toFixed(1)}%
              </span>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  subValue,
  color = 'text-primary',
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  subValue?: string;
  color?: string;
}) {
  return (
    <div className="flex items-start gap-3 p-4 rounded-lg bg-muted/50">
      <div className={`${color} mt-0.5`}>
        <Icon className="h-5 w-5" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="text-xl font-semibold">{value}</div>
        <div className="text-xs text-muted-foreground">{label}</div>
        {subValue && (
          <div className="text-xs text-muted-foreground/70 mt-0.5 truncate">{subValue}</div>
        )}
      </div>
    </div>
  );
}

export function ProjectStatsPanel({ projectPath }: ProjectStatsPanelProps) {
  const [expanded, setExpanded] = useState(false);

  const { data: stats, isLoading } = useQuery({
    queryKey: ['project-stats', projectPath],
    queryFn: () => getProjectStats(projectPath),
    staleTime: 60000, // 1 minute
  });

  if (isLoading) {
    return (
      <div className="tars-panel rounded-lg p-4 mb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          Analyzing project...
        </div>
      </div>
    );
  }

  if (!stats) return null;

  const totalDeps = stats.dependencies.reduce((sum, d) => sum + d.production + d.development, 0);

  // Get top languages for badges (exclude config/docs languages)
  const excludedLanguages = new Set(['JSON', 'YAML', 'Markdown', 'TOML', 'XML', 'HTML', 'CSS']);
  const codeLanguages = Object.entries(stats.languages)
    .filter(([name]) => !excludedLanguages.has(name))
    .map(([name, s]) => ({ name, code: s.code }))
    .sort((a, b) => b.code - a.code);

  const totalCodeLines = codeLanguages.reduce((sum, l) => sum + l.code, 0);
  const topLanguages = codeLanguages.slice(0, 2).filter((l) => {
    const pct = totalCodeLines > 0 ? (l.code / totalCodeLines) * 100 : 0;
    return pct >= 10; // Only show if >= 10% of code
  });

  return (
    <div className="tars-panel rounded-lg overflow-hidden mb-6">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center justify-between w-full px-4 py-3 bg-muted/30 border-b border-border hover:bg-muted/40 transition-colors"
      >
        <div className="flex items-center gap-3">
          {expanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <BarChart3 className="h-4 w-4 text-primary" />
          <span className="font-medium">Project Statistics</span>
          <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
            {formatNumber(stats.total_code)} LoC
          </span>
        </div>
        {!expanded && topLanguages.length > 0 && (
          <div className="flex items-center gap-2">
            {topLanguages.map((lang) => {
              const pct = totalCodeLines > 0 ? ((lang.code / totalCodeLines) * 100).toFixed(0) : 0;
              return (
                <span
                  key={lang.name}
                  className="text-xs bg-muted px-2 py-0.5 rounded-full text-muted-foreground"
                >
                  {lang.name} {pct}%
                </span>
              );
            })}
          </div>
        )}
      </button>

      {expanded && (
        <div className="p-4 space-y-4">
          {/* Language breakdown */}
          <LanguageBar languages={stats.languages} />

          {/* Stats grid */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
            <StatCard
              icon={Code2}
              label="Lines of Code"
              value={formatNumber(stats.total_code)}
              subValue={`${formatNumber(stats.total_lines)} total lines`}
              color="text-blue-500"
            />
            <StatCard
              icon={FileCode}
              label="Source Files"
              value={formatNumber(stats.total_files)}
              subValue={`${Object.keys(stats.languages).length} languages`}
              color="text-emerald-500"
            />
            <StatCard
              icon={Package}
              label="Dependencies"
              value={totalDeps}
              subValue={stats.dependencies.map((d) => d.source).join(', ') || 'none detected'}
              color="text-purple-500"
            />
            {stats.coverage ? (
              <StatCard
                icon={CheckCircle2}
                label="Test Coverage"
                value={`${stats.coverage.line_coverage?.toFixed(1)}%`}
                subValue={`${formatNumber(stats.coverage.lines_covered || 0)}/${formatNumber(stats.coverage.lines_total || 0)} lines covered`}
                color={
                  (stats.coverage.line_coverage || 0) >= 80
                    ? 'text-emerald-500'
                    : (stats.coverage.line_coverage || 0) >= 50
                      ? 'text-amber-500'
                      : 'text-red-500'
                }
              />
            ) : (
              <StatCard
                icon={CheckCircle2}
                label="Test Coverage"
                value="N/A"
                subValue="no coverage report found"
                color="text-muted-foreground"
              />
            )}
          </div>

          {/* TODOs and FIXMEs */}
          {(stats.todo_count > 0 || stats.fixme_count > 0) && (
            <div className="flex items-center gap-4 text-sm">
              {stats.todo_count > 0 && (
                <div className="flex items-center gap-1.5 text-amber-500">
                  <ListTodo className="h-4 w-4" />
                  <span>{stats.todo_count} TODOs</span>
                </div>
              )}
              {stats.fixme_count > 0 && (
                <div className="flex items-center gap-1.5 text-red-500">
                  <AlertTriangle className="h-4 w-4" />
                  <span>{stats.fixme_count} FIXMEs</span>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
