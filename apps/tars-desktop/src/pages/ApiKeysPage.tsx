import { Key } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { listApiKeys, listProviders, type ApiKeySummary, type ProviderMetadata } from '../lib/ipc';
import { ApiKeyProviderCard } from '../components/ApiKeyProviderCard';
import { AddApiKeyDialog } from '../components/AddApiKeyDialog';

export function ApiKeysPage() {
  const [addingFor, setAddingFor] = useState<ProviderMetadata | null>(null);

  const providersQuery = useQuery({
    queryKey: ['providers'],
    queryFn: listProviders,
    staleTime: Infinity,
  });

  const keysQuery = useQuery({
    queryKey: ['api-keys'],
    queryFn: listApiKeys,
  });

  const keys = keysQuery.data;
  const keysByProvider = useMemo(() => {
    const map = new Map<string, ApiKeySummary[]>();
    for (const k of keys ?? []) {
      const list = map.get(k.provider_id) ?? [];
      list.push(k);
      map.set(k.provider_id, list);
    }
    return map;
  }, [keys]);

  return (
    <div className="h-full flex flex-col">
      <div className="shrink-0 border-b border-border bg-card/50 px-6 py-4">
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <Key className="h-5 w-5" />
          AI Keys
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Manage AI provider API keys. Stored encrypted at rest.
        </p>
      </div>
      <div className="flex-1 overflow-y-auto p-6">
        {providersQuery.isLoading ? (
          <p className="text-muted-foreground">Loading providers…</p>
        ) : providersQuery.isError ? (
          <p className="text-destructive">
            Failed to load providers: {String(providersQuery.error)}
          </p>
        ) : (
          <div className="grid gap-4 grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
            {providersQuery.data?.map((p) => (
              <ApiKeyProviderCard
                key={p.id}
                provider={p}
                keys={keysByProvider.get(p.id) ?? []}
                onAddKey={setAddingFor}
              />
            ))}
          </div>
        )}
      </div>

      <AddApiKeyDialog
        provider={addingFor}
        onOpenChange={(open) => {
          if (!open) setAddingFor(null);
        }}
      />
    </div>
  );
}
