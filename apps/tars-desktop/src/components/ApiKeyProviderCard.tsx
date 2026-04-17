import { Plus, RefreshCw } from 'lucide-react';
import type { ApiKeySummary, ProviderMetadata } from '../lib/ipc';

export interface ApiKeyProviderCardProps {
  provider: ProviderMetadata;
  keys: ApiKeySummary[];
  onAddKey: (provider: ProviderMetadata) => void;
}

function formatBalance(balance: unknown): string | null {
  if (balance == null) return null;
  // DeepSeek shape: { total_balance: "10.00", currency: "USD" } or similar.
  // We accept any object with a numeric/string total_balance, or a bare number.
  if (typeof balance === 'number') return `$${balance.toFixed(2)}`;
  if (typeof balance === 'object') {
    const o = balance as Record<string, unknown>;
    const v = o.total_balance ?? o.balance ?? o.value;
    if (typeof v === 'string' && v.trim() !== '') return `$${v}`;
    if (typeof v === 'number') return `$${v.toFixed(2)}`;
  }
  return null;
}

export function ApiKeyProviderCard({ provider, keys, onAddKey }: ApiKeyProviderCardProps) {
  const balanceText = provider.supports_balance
    ? (keys.map((k) => formatBalance(k.balance)).find((b) => b != null) ?? null)
    : null;

  return (
    <div className="rounded-lg border border-border bg-card p-4 flex flex-col gap-4">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="font-semibold truncate">{provider.display_name}</h2>
          <p className="text-xs text-muted-foreground truncate" title={provider.key_format_hint}>
            Format: {provider.key_format_hint}
          </p>
        </div>
        {provider.supports_balance && (
          <span
            data-testid="balance-badge"
            className="shrink-0 text-xs font-mono px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30"
            title="Account balance"
          >
            {balanceText ?? '—'}
          </span>
        )}
      </div>

      {keys.length === 0 ? (
        <p className="text-sm text-muted-foreground italic">
          No keys stored. Click Add Key to get started.
        </p>
      ) : (
        <ul className="space-y-2">
          {keys.map((k) => (
            <li
              key={k.id}
              className="flex items-center justify-between gap-3 rounded border border-border/60 bg-background/40 px-3 py-2"
            >
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium truncate">{k.label}</div>
                <div className="text-xs text-muted-foreground font-mono select-none">
                  {'•'.repeat(20)}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => onAddKey(provider)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
        >
          <Plus className="h-3.5 w-3.5" />
          Add Key
        </button>
        {provider.supports_models && (
          <button
            type="button"
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Refresh Models
          </button>
        )}
      </div>
    </div>
  );
}
