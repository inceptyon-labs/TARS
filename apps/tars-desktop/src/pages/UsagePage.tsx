import { useMemo } from 'react';
import {
  BarChart3,
  ExternalLink,
  MessageSquare,
  Terminal,
  Zap,
  Calendar,
  TrendingUp,
} from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { openUrl } from '@tauri-apps/plugin-opener';
import { cn } from '../lib/utils';
import { getClaudeUsageStats } from '../lib/ipc';

// Format large numbers with commas
function formatNumber(num: number): string {
  return num.toLocaleString();
}

// Format token counts (e.g., 1.2M, 500K)
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000_000) {
    return `${(tokens / 1_000_000_000).toFixed(1)}B`;
  }
  if (tokens >= 1_000_000) {
    return `${(tokens / 1_000_000).toFixed(1)}M`;
  }
  if (tokens >= 1_000) {
    return `${(tokens / 1_000).toFixed(1)}K`;
  }
  return tokens.toString();
}

// Get friendly model name
function getModelName(modelId: string): string {
  if (modelId.includes('opus')) return 'Opus';
  if (modelId.includes('sonnet')) return 'Sonnet';
  if (modelId.includes('haiku')) return 'Haiku';
  return modelId;
}

// Get model color
function getModelColor(modelId: string): { bg: string; text: string } {
  if (modelId.includes('opus')) return { bg: 'bg-purple-500', text: 'text-purple-500' };
  if (modelId.includes('sonnet')) return { bg: 'bg-blue-500', text: 'text-blue-500' };
  if (modelId.includes('haiku')) return { bg: 'bg-green-500', text: 'text-green-500' };
  return { bg: 'bg-muted', text: 'text-muted-foreground' };
}

// Simple bar chart component
const ACTIVITY_CHART_HEIGHT = 128; // h-32 = 128px

function ActivityChart({
  data,
  maxValue,
}: {
  data: { date: string; value: number }[];
  maxValue: number;
}) {
  if (data.length === 0) return null;

  return (
    <div className="flex items-end gap-1 h-32">
      {data.map((item, index) => {
        const heightPercent = maxValue > 0 ? (item.value / maxValue) * 100 : 0;
        const heightPx = Math.max((heightPercent / 100) * ACTIVITY_CHART_HEIGHT, 2);
        const date = new Date(item.date);
        const isToday = new Date().toDateString() === date.toDateString();

        return (
          <div
            key={item.date}
            className="flex-1 flex flex-col items-end justify-end group relative"
          >
            {/* Tooltip */}
            <div className="absolute bottom-full mb-2 hidden group-hover:block z-10">
              <div className="bg-popover text-popover-foreground text-xs rounded px-2 py-1 shadow-lg border whitespace-nowrap">
                <div className="font-medium">{date.toLocaleDateString()}</div>
                <div>{formatNumber(item.value)} messages</div>
              </div>
            </div>

            {/* Bar */}
            <div
              className={cn(
                'w-full rounded-t transition-all',
                isToday ? 'bg-primary' : 'bg-primary/60',
                'hover:bg-primary'
              )}
              style={{ height: `${heightPx}px` }}
            />

            {/* Date label (show every 7th day or first/last) */}
            {(index === 0 || index === data.length - 1 || index % 7 === 0) && (
              <span className="absolute -bottom-4 text-[9px] text-muted-foreground whitespace-nowrap">
                {date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

// Hour distribution chart
const HOUR_CHART_HEIGHT = 80; // h-20 = 80px

function HourChart({ hourCounts }: { hourCounts: Record<string, number> }) {
  const hours = Array.from({ length: 24 }, (_, i) => i);
  const maxCount = Math.max(...Object.values(hourCounts), 1);

  return (
    <div className="flex items-end gap-0.5 h-20">
      {hours.map((hour) => {
        const count = hourCounts[hour.toString()] || 0;
        const heightPercent = (count / maxCount) * 100;
        const heightPx = Math.max((heightPercent / 100) * HOUR_CHART_HEIGHT, count > 0 ? 2 : 0);

        return (
          <div key={hour} className="flex-1 flex flex-col items-center justify-end group relative">
            {/* Tooltip */}
            <div className="absolute bottom-full mb-2 hidden group-hover:block z-10">
              <div className="bg-popover text-popover-foreground text-xs rounded px-2 py-1 shadow-lg border whitespace-nowrap">
                <div className="font-medium">{hour.toString().padStart(2, '0')}:00</div>
                <div>{count} sessions</div>
              </div>
            </div>

            {/* Bar */}
            <div
              className="w-full bg-primary/60 hover:bg-primary rounded-t transition-all"
              style={{ height: `${heightPx}px` }}
            />
          </div>
        );
      })}
    </div>
  );
}

export function UsagePage() {
  const {
    data: usageStats,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['claude-usage-stats'],
    queryFn: getClaudeUsageStats,
    staleTime: 60_000,
  });

  const handleOpenClaudeSettings = async () => {
    try {
      await openUrl('https://claude.ai/settings/usage');
    } catch (err) {
      console.error('Failed to open Claude settings:', err);
    }
  };

  // Prepare chart data (last 30 days)
  const chartData = useMemo(() => {
    if (!usageStats?.dailyActivity) return { data: [], max: 0 };

    const last30Days = usageStats.dailyActivity.slice(-30);
    const data = last30Days.map((d) => ({
      date: d.date,
      value: d.messageCount,
    }));
    const max = Math.max(...data.map((d) => d.value), 1);

    return { data, max };
  }, [usageStats?.dailyActivity]);

  // Calculate total tool calls
  const totalToolCalls = useMemo(() => {
    if (!usageStats?.dailyActivity) return 0;
    return usageStats.dailyActivity.reduce((sum, d) => sum + d.toolCallCount, 0);
  }, [usageStats?.dailyActivity]);

  // Calculate average daily messages (last 30 days)
  const avgDailyMessages = useMemo(() => {
    if (!chartData.data.length) return 0;
    const total = chartData.data.reduce((sum, d) => sum + d.value, 0);
    return Math.round(total / chartData.data.length);
  }, [chartData.data]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="shrink-0 border-b border-border bg-card/50 px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold flex items-center gap-2">
              <BarChart3 className="h-5 w-5" />
              Usage
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              Claude Code usage statistics and activity
            </p>
          </div>
          <button
            onClick={handleOpenClaudeSettings}
            className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors flex items-center gap-2"
          >
            View Limits in Claude
            <ExternalLink className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <p className="text-muted-foreground">Loading usage statistics...</p>
          </div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <p className="text-muted-foreground mb-2">No usage data found.</p>
            <p className="text-sm text-muted-foreground/70">
              Start using Claude Code to see your statistics here.
            </p>
          </div>
        ) : usageStats ? (
          <div className="max-w-4xl space-y-6">
            {/* Summary Stats */}
            <div className="grid grid-cols-4 gap-4">
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <Terminal className="h-4 w-4" />
                  <span className="text-xs">Total Sessions</span>
                </div>
                <p className="text-2xl font-semibold">{formatNumber(usageStats.totalSessions)}</p>
              </div>
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <MessageSquare className="h-4 w-4" />
                  <span className="text-xs">Total Messages</span>
                </div>
                <p className="text-2xl font-semibold">{formatNumber(usageStats.totalMessages)}</p>
              </div>
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <Zap className="h-4 w-4" />
                  <span className="text-xs">Total Tool Calls</span>
                </div>
                <p className="text-2xl font-semibold">{formatNumber(totalToolCalls)}</p>
              </div>
              <div className="p-4 rounded-lg border border-border bg-card">
                <div className="flex items-center gap-2 text-muted-foreground mb-1">
                  <TrendingUp className="h-4 w-4" />
                  <span className="text-xs">Avg Daily Messages</span>
                </div>
                <p className="text-2xl font-semibold">{formatNumber(avgDailyMessages)}</p>
              </div>
            </div>

            {/* Activity Chart */}
            <div className="p-4 rounded-lg border border-border bg-card">
              <h3 className="font-medium mb-4 flex items-center gap-2">
                <Calendar className="h-4 w-4" />
                Daily Activity (Last 30 Days)
              </h3>
              <div className="mb-6">
                <ActivityChart data={chartData.data} maxValue={chartData.max} />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              {/* Model Usage */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <h3 className="font-medium mb-4">Token Usage by Model</h3>
                <div className="space-y-4">
                  {Object.entries(usageStats.modelUsage)
                    .sort(([, a], [, b]) => b.outputTokens - a.outputTokens)
                    .map(([modelId, usage]) => {
                      const totalTokens = usage.inputTokens + usage.outputTokens;
                      const colors = getModelColor(modelId);
                      const maxTokens = Math.max(
                        ...Object.values(usageStats.modelUsage).map(
                          (u) => u.inputTokens + u.outputTokens
                        )
                      );
                      const percentage = (totalTokens / maxTokens) * 100;

                      return (
                        <div key={modelId}>
                          <div className="flex items-center justify-between mb-1">
                            <span className={cn('font-medium', colors.text)}>
                              {getModelName(modelId)}
                            </span>
                            <span className="font-mono text-sm">{formatTokens(totalTokens)}</span>
                          </div>
                          <div className="h-2 bg-muted rounded-full overflow-hidden">
                            <div
                              className={cn('h-full rounded-full', colors.bg)}
                              style={{ width: `${percentage}%` }}
                            />
                          </div>
                          <div className="flex justify-between text-xs text-muted-foreground mt-1">
                            <span>In: {formatTokens(usage.inputTokens)}</span>
                            <span>Out: {formatTokens(usage.outputTokens)}</span>
                          </div>
                        </div>
                      );
                    })}
                </div>
                {usageStats.firstSessionDate && (
                  <p className="text-xs text-muted-foreground mt-4 pt-3 border-t border-border">
                    Since {new Date(usageStats.firstSessionDate).toLocaleDateString()}
                  </p>
                )}
              </div>

              {/* Hour Distribution */}
              <div className="p-4 rounded-lg border border-border bg-card">
                <h3 className="font-medium mb-4">Activity by Hour</h3>
                <HourChart hourCounts={usageStats.hourCounts} />
                <div className="flex justify-between text-xs text-muted-foreground mt-2">
                  <span>12 AM</span>
                  <span>6 AM</span>
                  <span>12 PM</span>
                  <span>6 PM</span>
                  <span>12 AM</span>
                </div>
                <p className="text-xs text-muted-foreground mt-3 text-center">
                  Sessions started by hour of day
                </p>
              </div>
            </div>

            {/* Note about live quotas */}
            <div className="p-4 rounded-lg border border-border bg-muted/30">
              <p className="text-sm text-muted-foreground">
                <strong>Note:</strong> This shows historical usage from Claude Code's local cache.
                For live quota percentages and reset times, click "View Limits in Claude" above.
              </p>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
