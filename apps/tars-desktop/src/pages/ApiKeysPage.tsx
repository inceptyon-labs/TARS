import { Key } from 'lucide-react';

export function ApiKeysPage() {
  return (
    <div className="h-full flex flex-col">
      <div className="shrink-0 border-b border-border bg-card/50 px-6 py-4">
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <Key className="h-5 w-5" />
          API Keys
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Manage AI provider API keys. Stored encrypted at rest.
        </p>
      </div>
      <div className="flex-1 overflow-y-auto p-6" />
    </div>
  );
}
