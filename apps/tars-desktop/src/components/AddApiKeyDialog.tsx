import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { addApiKey, validateApiKey, type ProviderMetadata } from '../lib/ipc';

export interface AddApiKeyDialogProps {
  provider: ProviderMetadata | null;
  onOpenChange: (open: boolean) => void;
}

export function AddApiKeyDialog({ provider, onOpenChange }: AddApiKeyDialogProps) {
  const [label, setLabel] = useState('');
  const [keyValue, setKeyValue] = useState('');
  const queryClient = useQueryClient();

  // Reset state on every open/close transition so the plaintext key never
  // lingers in component state after the dialog closes (success, cancel, or
  // explicit close). Tracking provider's truthiness as the open signal lets
  // this fire on close (provider→null) too, not only on the next open.
  useEffect(() => {
    setLabel('');
    setKeyValue('');
  }, [provider]);

  const mutation = useMutation({
    mutationFn: async (input: { provider_id: string; label: string; key: string }) => {
      const id = await addApiKey({
        provider_id: input.provider_id as ProviderMetadata['id'],
        label: input.label,
        key: input.key,
      });
      // Validation failure is non-fatal — the key is stored, the UI surfaces
      // the bad badge. Swallow the validation error so the dialog still closes.
      try {
        await validateApiKey(id);
      } catch (err) {
        toast.error(`Validation failed: ${String(err)}`);
      }
      return id;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      toast.success('Key added');
      // Drop the plaintext key from React Query's mutation variables/data
      // before closing — otherwise it sits in mutation state until GC.
      mutation.reset();
      onOpenChange(false);
    },
    onError: (err) => {
      toast.error(`Failed to add key: ${String(err)}`);
    },
  });

  if (!provider) return null;

  const canSubmit = label.trim().length > 0 && keyValue.trim().length > 0 && !mutation.isPending;

  return (
    <Dialog open={!!provider} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add {provider.display_name} key</DialogTitle>
          <DialogDescription>
            Expected format: <span className="font-mono">{provider.key_format_hint}</span>
          </DialogDescription>
        </DialogHeader>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!canSubmit) return;
            mutation.mutate({
              provider_id: provider.id,
              label: label.trim(),
              key: keyValue.trim(),
            });
          }}
          className="space-y-4"
        >
          <div className="space-y-1">
            <label htmlFor="api-key-label" className="text-sm font-medium">
              Label
            </label>
            <input
              id="api-key-label"
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="e.g. work, personal"
              autoComplete="off"
              spellCheck={false}
              className="w-full px-3 py-2 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="api-key-value" className="text-sm font-medium">
              API key
            </label>
            <input
              id="api-key-value"
              type="password"
              value={keyValue}
              onChange={(e) => setKeyValue(e.target.value)}
              placeholder={provider.key_format_hint}
              autoComplete="new-password"
              spellCheck={false}
              className="w-full px-3 py-2 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <DialogFooter>
            <button
              type="button"
              onClick={() => onOpenChange(false)}
              className="px-4 py-2 text-sm rounded-md border border-border hover:bg-muted/50 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!canSubmit}
              className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {mutation.isPending ? 'Saving…' : 'Save'}
            </button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
