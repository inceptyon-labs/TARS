import type { ProviderMetadata } from '../lib/ipc';

export interface AddApiKeyDialogProps {
  provider: ProviderMetadata | null;
  onOpenChange: (open: boolean) => void;
}

// Minimal stub — full form lands in step 4.
export function AddApiKeyDialog(_props: AddApiKeyDialogProps) {
  return null;
}
