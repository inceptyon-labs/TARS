import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  ChevronRight,
  ChevronDown,
  KeyRound,
  Eye,
  EyeOff,
  Plus,
  Trash2,
  Shield,
  Copy,
  Check,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  listProjectSecrets,
  getProjectSecret,
  saveProjectSecret,
  deleteProjectSecret,
} from '../lib/ipc';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';

interface ProjectSecretsPanelProps {
  projectId: string;
}

export function ProjectSecretsPanel({ projectId }: ProjectSecretsPanelProps) {
  const queryClient = useQueryClient();
  const [isExpanded, setIsExpanded] = useState(false);
  const [revealedSecrets, setRevealedSecrets] = useState<Map<string, string>>(new Map());
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [isAdding, setIsAdding] = useState(false);

  const { data: secrets = [] } = useQuery({
    queryKey: ['project-secrets', projectId],
    queryFn: () => listProjectSecrets(projectId),
    enabled: !!projectId && isExpanded,
  });

  const revealMutation = useMutation({
    mutationFn: (key: string) => getProjectSecret(projectId, key),
    onSuccess: (data) => {
      setRevealedSecrets((prev) => new Map(prev).set(data.key, data.value));
    },
    onError: (err) => toast.error(`Failed to decrypt: ${err}`),
  });

  const saveMutation = useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) =>
      saveProjectSecret(projectId, key, value),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-secrets', projectId] });
      setNewKey('');
      setNewValue('');
      setIsAdding(false);
      toast.success('Secret saved (encrypted)');
    },
    onError: (err) => toast.error(`Failed to save: ${err}`),
  });

  const deleteMutation = useMutation({
    mutationFn: (key: string) => deleteProjectSecret(projectId, key),
    onSuccess: (_, key) => {
      queryClient.invalidateQueries({ queryKey: ['project-secrets', projectId] });
      setRevealedSecrets((prev) => {
        const next = new Map(prev);
        next.delete(key);
        return next;
      });
      toast.success('Secret deleted');
    },
    onError: (err) => toast.error(`Failed to delete: ${err}`),
  });

  const toggleReveal = (key: string) => {
    if (revealedSecrets.has(key)) {
      setRevealedSecrets((prev) => {
        const next = new Map(prev);
        next.delete(key);
        return next;
      });
    } else {
      revealMutation.mutate(key);
    }
  };

  const copySecret = async (key: string) => {
    const value = revealedSecrets.get(key);
    if (!value) {
      // Need to decrypt first
      try {
        const data = await getProjectSecret(projectId, key);
        await navigator.clipboard.writeText(data.value);
        setCopiedKey(key);
        setTimeout(() => setCopiedKey(null), 2000);
      } catch {
        toast.error('Failed to copy');
      }
    } else {
      await navigator.clipboard.writeText(value);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 2000);
    }
  };

  const handleSave = () => {
    const trimmedKey = newKey.trim();
    if (!trimmedKey) {
      toast.error('Key is required');
      return;
    }
    if (!newValue) {
      toast.error('Value is required');
      return;
    }
    saveMutation.mutate({ key: trimmedKey, value: newValue });
  };

  return (
    <div className="tars-panel rounded-lg overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center gap-3 hover:text-primary transition-colors"
        >
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <KeyRound className="h-4 w-4 text-amber-400" />
          <span className="font-medium">Secrets</span>
          {secrets.length > 0 && (
            <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
              {secrets.length}
            </span>
          )}
          <span className="flex items-center gap-1 text-xs text-emerald-400/70">
            <Shield className="h-3 w-3" />
            AES-256 encrypted
          </span>
        </button>
        {isExpanded && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setIsAdding(!isAdding)}
            className="h-7 gap-1.5 text-xs"
          >
            <Plus className="h-3 w-3" />
            Add
          </Button>
        )}
      </div>

      {isExpanded && (
        <div className="p-4 space-y-3">
          {/* Add new secret form */}
          {isAdding && (
            <div className="border border-border rounded-md p-3 space-y-3 bg-muted/20">
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Key</Label>
                  <Input
                    className="h-8 text-sm font-mono"
                    value={newKey}
                    onChange={(e) => setNewKey(e.target.value)}
                    placeholder="API_KEY"
                    autoFocus
                  />
                </div>
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">Value</Label>
                  <Input
                    className="h-8 text-sm font-mono"
                    type="password"
                    value={newValue}
                    onChange={(e) => setNewValue(e.target.value)}
                    placeholder="sk-..."
                    onKeyDown={(e) => e.key === 'Enter' && handleSave()}
                  />
                </div>
              </div>
              <div className="flex gap-2 justify-end">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    setIsAdding(false);
                    setNewKey('');
                    setNewValue('');
                  }}
                  className="h-7 text-xs"
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  onClick={handleSave}
                  disabled={saveMutation.isPending}
                  className="h-7 text-xs"
                >
                  Save
                </Button>
              </div>
            </div>
          )}

          {/* Secret list */}
          {secrets.length === 0 && !isAdding && (
            <p className="text-xs text-muted-foreground/60 italic py-2">
              No secrets stored. Values are encrypted with AES-256-GCM using a key in your OS
              keychain.
            </p>
          )}

          {secrets.map((secret) => {
            const isRevealed = revealedSecrets.has(secret.key);
            const value = revealedSecrets.get(secret.key);

            return (
              <div
                key={secret.key}
                className="flex items-center gap-3 border border-border rounded-md px-3 py-2 bg-muted/10"
              >
                <span className="text-sm font-mono font-medium min-w-0 truncate flex-shrink-0 max-w-[180px]">
                  {secret.key}
                </span>
                <div className="flex-1 min-w-0">
                  {isRevealed ? (
                    <span className="text-sm font-mono text-muted-foreground truncate block">
                      {value}
                    </span>
                  ) : (
                    <span className="text-sm text-muted-foreground/40 tracking-widest">
                      ••••••••
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-1 flex-shrink-0">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => toggleReveal(secret.key)}
                    className="h-7 w-7 p-0"
                    title={isRevealed ? 'Hide' : 'Reveal'}
                  >
                    {isRevealed ? (
                      <EyeOff className="h-3.5 w-3.5" />
                    ) : (
                      <Eye className="h-3.5 w-3.5" />
                    )}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => copySecret(secret.key)}
                    className="h-7 w-7 p-0"
                    title="Copy to clipboard"
                  >
                    {copiedKey === secret.key ? (
                      <Check className="h-3.5 w-3.5 text-emerald-400" />
                    ) : (
                      <Copy className="h-3.5 w-3.5" />
                    )}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => deleteMutation.mutate(secret.key)}
                    className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
                    title="Delete"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
