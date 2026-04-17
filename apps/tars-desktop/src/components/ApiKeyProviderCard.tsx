import { Plus, RefreshCw, Eye, EyeOff, Copy, Trash2, ShieldCheck } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  deleteApiKey,
  listProviderModels,
  refreshModels,
  revealApiKey,
  validateApiKey,
  type ApiKeySummary,
  type ProviderMetadata,
} from '../lib/ipc';

export interface ApiKeyProviderCardProps {
  provider: ProviderMetadata;
  keys: ApiKeySummary[];
  onAddKey: (provider: ProviderMetadata) => void;
}

const REVEAL_TIMEOUT_MS = 10_000;

function formatBalance(balance: unknown): string | null {
  if (balance == null) return null;
  if (typeof balance === 'number') return `$${balance.toFixed(2)}`;
  if (typeof balance === 'object') {
    const o = balance as Record<string, unknown>;
    const v = o.total_balance ?? o.balance ?? o.value;
    if (typeof v === 'string' && v.trim() !== '') return `$${v}`;
    if (typeof v === 'number') return `$${v.toFixed(2)}`;
  }
  return null;
}

function formatPrice(price: number | null): string {
  if (price == null) return '—';
  return `$${price.toFixed(2)}`;
}

function formatContextWindow(n: number | null): string {
  if (n == null) return '—';
  if (n >= 1000) return `${Math.round(n / 1000)}k`;
  return String(n);
}

function formatRelativeTime(iso: string | null): string | null {
  if (!iso) return null;
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return null;
  const seconds = Math.floor((Date.now() - then) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

interface ApiKeyRowProps {
  k: ApiKeySummary;
}

function ApiKeyRow({ k }: ApiKeyRowProps) {
  const [revealedValue, setRevealedValue] = useState<string | null>(null);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const queryClient = useQueryClient();

  // Clear pending auto-hide and mark unmounted so any in-flight IPC mutation
  // resolving after teardown doesn't setState on a dead component or stash
  // plaintext key material in mutation cache.
  useEffect(
    () => () => {
      mountedRef.current = false;
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    },
    []
  );

  const clearAutoHide = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  };

  const reveal = useMutation({
    mutationFn: () => revealApiKey(k.id),
    onSuccess: (value) => {
      if (!mountedRef.current) return;
      setRevealedValue(value);
      // Schedule auto-hide here (rather than via a separate helper that
      // captures `reveal` before its declaration) to keep the lifecycle of
      // plaintext exposure obvious in one place.
      clearAutoHide();
      hideTimerRef.current = setTimeout(() => {
        setRevealedValue(null);
        hideTimerRef.current = null;
        // Drop the plaintext from mutation cache too — keeping it would
        // defeat the auto-hide window.
        reveal.reset();
      }, REVEAL_TIMEOUT_MS);
    },
    onError: (err) => toast.error(`Failed to reveal: ${String(err)}`),
  });

  const remove = useMutation({
    mutationFn: () => deleteApiKey(k.id),
    onSuccess: () => {
      toast.success('Key deleted');
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
    },
    onError: (err) => toast.error(`Failed to delete: ${String(err)}`),
  });

  const validate = useMutation({
    mutationFn: () => validateApiKey(k.id),
    onSuccess: (res) => {
      if (res.valid) {
        toast.success('Key is valid');
      } else {
        toast.error(res.message ? `Invalid: ${res.message}` : 'Key is invalid');
      }
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
    },
    onError: (err) => toast.error(`Validation failed: ${String(err)}`),
  });

  const handleToggleReveal = () => {
    if (revealedValue) {
      clearAutoHide();
      setRevealedValue(null);
      reveal.reset();
      return;
    }
    if (reveal.isPending) return;
    reveal.mutate();
  };

  // Copy routes through the same reveal mutation so it inherits:
  //   - the isPending guard (no parallel IPC decryptions),
  //   - the mountedRef guard inside onSuccess,
  //   - the auto-hide timer + reveal.reset() lifecycle for the plaintext.
  // The visible reveal-on-copy is intentional: the user sees what they're
  // about to paste.
  const handleCopy = async () => {
    try {
      const value = revealedValue ?? (await reveal.mutateAsync());
      if (!mountedRef.current) return;
      await navigator.clipboard.writeText(value);
      toast.success('Key copied to clipboard');
    } catch (err) {
      toast.error(`Failed to copy: ${String(err)}`);
    }
  };

  const handleDelete = () => {
    if (!window.confirm(`Delete key "${k.label}"? This cannot be undone.`)) return;
    remove.mutate();
  };

  const validatedRel = formatRelativeTime(k.last_validated_at);

  return (
    <li className="flex items-center justify-between gap-3 rounded border border-border/60 bg-background/40 px-3 py-2">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">{k.label}</span>
          {k.last_valid === true && (
            <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30">
              Valid
            </span>
          )}
          {k.last_valid === false && (
            <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-red-500/15 text-red-600 dark:text-red-400 border border-red-500/30">
              Invalid
            </span>
          )}
        </div>
        <div className="text-xs text-muted-foreground font-mono select-none break-all">
          {revealedValue ?? '•'.repeat(20)}
        </div>
        {validatedRel && (
          <div className="text-[10px] text-muted-foreground mt-0.5">Validated {validatedRel}</div>
        )}
      </div>
      <div className="flex items-center gap-1 shrink-0">
        <button
          type="button"
          onClick={handleToggleReveal}
          disabled={reveal.isPending}
          aria-label={revealedValue ? 'Hide key' : 'Reveal key'}
          title={revealedValue ? 'Hide key' : 'Reveal key (auto-hides in 10s)'}
          className="p-1.5 rounded hover:bg-muted/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {revealedValue ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
        </button>
        <button
          type="button"
          onClick={handleCopy}
          disabled={reveal.isPending}
          aria-label="Copy key"
          title="Copy to clipboard"
          className="p-1.5 rounded hover:bg-muted/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Copy className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={() => validate.mutate()}
          disabled={validate.isPending}
          aria-label="Validate key"
          title="Re-validate against the provider"
          className="flex items-center gap-1 px-2 py-1 text-xs rounded border border-border hover:bg-muted/50 transition-colors disabled:opacity-50"
        >
          <ShieldCheck className="h-3 w-3" />
          Validate
        </button>
        <button
          type="button"
          onClick={handleDelete}
          aria-label="Delete key"
          title="Delete this key"
          className="p-1.5 rounded text-destructive hover:bg-destructive/10 transition-colors"
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </div>
    </li>
  );
}

function ModelTable({ provider }: { provider: ProviderMetadata }) {
  const modelsQuery = useQuery({
    queryKey: ['provider-models', provider.id],
    queryFn: () => listProviderModels(provider.id),
    enabled: provider.supports_models,
  });

  if (modelsQuery.isLoading) {
    return <p className="text-xs text-muted-foreground">Loading models…</p>;
  }
  if (modelsQuery.isError) {
    return <p className="text-xs text-destructive">Failed to load models.</p>;
  }
  const rows = modelsQuery.data ?? [];
  if (rows.length === 0) {
    return (
      <p className="text-xs text-muted-foreground italic">
        No models cached. Click Refresh Models to fetch the latest list.
      </p>
    );
  }

  return (
    <div className="overflow-x-auto rounded border border-border/60">
      <table className="w-full text-xs">
        <thead className="bg-muted/30">
          <tr>
            <th className="text-left px-2 py-1.5 font-medium">Model</th>
            <th className="text-right px-2 py-1.5 font-medium">Context</th>
            <th className="text-right px-2 py-1.5 font-medium">In $/1M</th>
            <th className="text-right px-2 py-1.5 font-medium">Out $/1M</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((m) => (
            <tr key={m.model_id} className="border-t border-border/40">
              <td className="px-2 py-1.5 font-mono">
                {m.display_name && m.display_name !== m.model_id ? m.display_name : m.model_id}
              </td>
              <td className="px-2 py-1.5 text-right font-mono">
                {formatContextWindow(m.context_window)}
              </td>
              <td className="px-2 py-1.5 text-right font-mono">{formatPrice(m.input_price)}</td>
              <td className="px-2 py-1.5 text-right font-mono">{formatPrice(m.output_price)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function ApiKeyProviderCard({ provider, keys, onAddKey }: ApiKeyProviderCardProps) {
  const queryClient = useQueryClient();
  const balanceText = provider.supports_balance
    ? (keys.map((k) => formatBalance(k.balance)).find((b) => b != null) ?? null)
    : null;

  const refresh = useMutation({
    mutationFn: () => refreshModels(provider.id),
    onSuccess: (count) => {
      toast.success(`Refreshed ${count} model${count === 1 ? '' : 's'}`);
      queryClient.invalidateQueries({ queryKey: ['provider-models', provider.id] });
    },
    onError: (err) => toast.error(`Refresh failed: ${String(err)}`),
  });

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
            <ApiKeyRow key={k.id} k={k} />
          ))}
        </ul>
      )}

      {provider.supports_models && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Models
          </h3>
          <ModelTable provider={provider} />
        </div>
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
            onClick={() => refresh.mutate()}
            disabled={refresh.isPending || keys.length === 0}
            title={
              keys.length === 0
                ? 'Add a key first to enable model refresh'
                : 'Re-fetch the model list from the provider'
            }
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${refresh.isPending ? 'animate-spin' : ''}`} />
            Refresh Models
          </button>
        )}
      </div>
    </div>
  );
}
