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
  Pencil,
  X,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  listProjectSecrets,
  getProjectSecret,
  saveProjectSecret,
  updateProjectSecret,
  deleteProjectSecret,
} from '../lib/ipc';
import type { SecretResponse, SecretInput } from '../lib/types';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Textarea } from './ui/textarea';

interface ProjectSecretsPanelProps {
  projectId: string;
}

const emptyInput: SecretInput = { name: '', key: '', url: '', notes: '' };

function SecretForm({
  initial,
  onSave,
  onCancel,
  saving,
}: {
  initial: SecretInput;
  onSave: (input: SecretInput) => void;
  onCancel: () => void;
  saving: boolean;
}) {
  const [form, setForm] = useState<SecretInput>(initial);

  const handleSave = () => {
    if (!form.name.trim()) {
      toast.error('Name is required');
      return;
    }
    if (!form.key.trim()) {
      toast.error('Key / secret value is required');
      return;
    }
    onSave({
      ...form,
      name: form.name.trim(),
      key: form.key.trim(),
      url: form.url.trim(),
      notes: form.notes.trim(),
    });
  };

  return (
    <div className="border border-border rounded-md p-3 space-y-3 bg-muted/20">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <Label className="text-xs text-muted-foreground">Name</Label>
          <Input
            className="h-8 text-sm"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            placeholder="e.g. OpenAI API Key"
            autoFocus
          />
        </div>
        <div className="space-y-1">
          <Label className="text-xs text-muted-foreground">Key / Secret</Label>
          <Input
            className="h-8 text-sm font-mono"
            type="password"
            value={form.key}
            onChange={(e) => setForm({ ...form, key: e.target.value })}
            placeholder="sk-..."
          />
        </div>
      </div>
      <div className="space-y-1">
        <Label className="text-xs text-muted-foreground">URL</Label>
        <Input
          className="h-8 text-sm"
          value={form.url}
          onChange={(e) => setForm({ ...form, url: e.target.value })}
          placeholder="https://api.example.com (optional)"
        />
      </div>
      <div className="space-y-1">
        <Label className="text-xs text-muted-foreground">Notes</Label>
        <Textarea
          className="text-sm min-h-[60px] resize-none"
          value={form.notes}
          onChange={(e) => setForm({ ...form, notes: e.target.value })}
          placeholder="Additional details (optional)"
          rows={2}
        />
      </div>
      <div className="flex gap-2 justify-end">
        <Button size="sm" variant="ghost" onClick={onCancel} className="h-7 text-xs">
          Cancel
        </Button>
        <Button
          size="sm"
          onClick={handleSave}
          disabled={saving}
          className="h-7 text-xs"
          onKeyDown={(e) => e.key === 'Enter' && handleSave()}
        >
          Save
        </Button>
      </div>
    </div>
  );
}

function SecretRow({
  projectId,
  secret,
  onDeleted,
  onUpdated,
}: {
  projectId: string;
  secret: { id: number; name: string };
  onDeleted: () => void;
  onUpdated: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [revealed, setRevealed] = useState<SecretResponse | null>(null);
  const [editing, setEditing] = useState(false);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const revealMutation = useMutation({
    mutationFn: () => getProjectSecret(projectId, secret.name),
    onSuccess: (data) => setRevealed(data),
    onError: (err) => toast.error(`Failed to decrypt: ${err}`),
  });

  const updateMutation = useMutation({
    mutationFn: (input: SecretInput) => updateProjectSecret(projectId, secret.id, input),
    onSuccess: () => {
      setEditing(false);
      setRevealed(null);
      onUpdated();
      toast.success('Secret updated');
    },
    onError: (err) => toast.error(`Failed to update: ${err}`),
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteProjectSecret(projectId, secret.name),
    onSuccess: () => {
      onDeleted();
      toast.success('Secret deleted');
    },
    onError: (err) => toast.error(`Failed to delete: ${err}`),
  });

  const handleExpand = () => {
    if (!expanded && !revealed) {
      revealMutation.mutate();
    }
    setExpanded(!expanded);
  };

  const toggleRevealKey = () => {
    if (revealed) {
      setRevealed(null);
    } else {
      revealMutation.mutate();
    }
  };

  const copyToClipboard = async (value: string, field: string) => {
    await navigator.clipboard.writeText(value);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  const handleEdit = async () => {
    if (!revealed) {
      try {
        const data = await getProjectSecret(projectId, secret.name);
        setRevealed(data);
        setEditing(true);
      } catch {
        toast.error('Failed to decrypt for editing');
      }
    } else {
      setEditing(true);
    }
  };

  if (editing && revealed) {
    return (
      <SecretForm
        initial={{
          name: revealed.name,
          key: revealed.key,
          url: revealed.url,
          notes: revealed.notes,
        }}
        onSave={(input) => updateMutation.mutate(input)}
        onCancel={() => setEditing(false)}
        saving={updateMutation.isPending}
      />
    );
  }

  return (
    <div className="border border-border rounded-md bg-muted/10 overflow-hidden">
      <div className="flex items-center gap-3 px-3 py-2">
        <button
          onClick={handleExpand}
          className="flex items-center gap-2 min-w-0 flex-1 text-left hover:text-primary transition-colors"
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
          )}
          <span className="text-sm font-medium truncate">{secret.name}</span>
        </button>
        <div className="flex items-center gap-1 flex-shrink-0">
          <Button
            size="sm"
            variant="ghost"
            onClick={handleEdit}
            className="h-7 w-7 p-0"
            title="Edit"
          >
            <Pencil className="h-3.5 w-3.5" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => deleteMutation.mutate()}
            className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
            title="Delete"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {expanded && (
        <div className="px-3 pb-3 pt-1 space-y-2 border-t border-border/50">
          {/* Key / Secret */}
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground w-12 flex-shrink-0">Key</span>
            <div className="flex-1 min-w-0">
              {revealed ? (
                <span className="text-sm font-mono text-muted-foreground truncate block">
                  {revealed.key}
                </span>
              ) : (
                <span className="text-sm text-muted-foreground/40 tracking-widest">••••••••</span>
              )}
            </div>
            <Button
              size="sm"
              variant="ghost"
              onClick={toggleRevealKey}
              className="h-6 w-6 p-0"
              title={revealed ? 'Hide' : 'Reveal'}
            >
              {revealed ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={async () => {
                if (revealed) {
                  copyToClipboard(revealed.key, 'key');
                } else {
                  try {
                    const data = await getProjectSecret(projectId, secret.name);
                    copyToClipboard(data.key, 'key');
                  } catch {
                    toast.error('Failed to copy');
                  }
                }
              }}
              className="h-6 w-6 p-0"
              title="Copy key"
            >
              {copiedField === 'key' ? (
                <Check className="h-3 w-3 text-emerald-400" />
              ) : (
                <Copy className="h-3 w-3" />
              )}
            </Button>
          </div>

          {/* URL */}
          {(revealed?.url || !revealed) && (
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground w-12 flex-shrink-0">URL</span>
              <div className="flex-1 min-w-0">
                {revealed ? (
                  revealed.url ? (
                    <span className="text-sm font-mono text-muted-foreground truncate block">
                      {revealed.url}
                    </span>
                  ) : (
                    <span className="text-xs text-muted-foreground/30 italic">none</span>
                  )
                ) : (
                  <span className="text-sm text-muted-foreground/40 tracking-widest">••••••••</span>
                )}
              </div>
              {revealed?.url && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => copyToClipboard(revealed.url, 'url')}
                  className="h-6 w-6 p-0"
                  title="Copy URL"
                >
                  {copiedField === 'url' ? (
                    <Check className="h-3 w-3 text-emerald-400" />
                  ) : (
                    <Copy className="h-3 w-3" />
                  )}
                </Button>
              )}
            </div>
          )}

          {/* Notes */}
          {(revealed?.notes || !revealed) && (
            <div className="flex items-start gap-2">
              <span className="text-xs text-muted-foreground w-12 flex-shrink-0 pt-0.5">Notes</span>
              <div className="flex-1 min-w-0">
                {revealed ? (
                  revealed.notes ? (
                    <span className="text-sm text-muted-foreground whitespace-pre-wrap">
                      {revealed.notes}
                    </span>
                  ) : (
                    <span className="text-xs text-muted-foreground/30 italic">none</span>
                  )
                ) : (
                  <span className="text-sm text-muted-foreground/40 tracking-widest">••••••••</span>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function ProjectSecretsPanel({ projectId }: ProjectSecretsPanelProps) {
  const queryClient = useQueryClient();
  const [isExpanded, setIsExpanded] = useState(false);
  const [isAdding, setIsAdding] = useState(false);

  const { data: secrets = [] } = useQuery({
    queryKey: ['project-secrets', projectId],
    queryFn: () => listProjectSecrets(projectId),
    enabled: !!projectId && isExpanded,
  });

  const saveMutation = useMutation({
    mutationFn: (input: SecretInput) => saveProjectSecret(projectId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-secrets', projectId] });
      setIsAdding(false);
      toast.success('Secret saved (encrypted)');
    },
    onError: (err) => toast.error(`Failed to save: ${err}`),
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['project-secrets', projectId] });
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
            {isAdding ? <X className="h-3 w-3" /> : <Plus className="h-3 w-3" />}
            {isAdding ? 'Cancel' : 'Add'}
          </Button>
        )}
      </div>

      {isExpanded && (
        <div className="p-4 space-y-3">
          {isAdding && (
            <SecretForm
              initial={emptyInput}
              onSave={(input) => saveMutation.mutate(input)}
              onCancel={() => setIsAdding(false)}
              saving={saveMutation.isPending}
            />
          )}

          {secrets.length === 0 && !isAdding && (
            <p className="text-xs text-muted-foreground/60 italic py-2">
              No secrets stored. Store API keys, passwords, and credentials — encrypted with
              AES-256-GCM using a key in your OS keychain.
            </p>
          )}

          {secrets.map((secret) => (
            <SecretRow
              key={secret.id}
              projectId={projectId}
              secret={secret}
              onDeleted={invalidate}
              onUpdated={invalidate}
            />
          ))}
        </div>
      )}
    </div>
  );
}
