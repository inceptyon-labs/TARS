import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import {
  AppWindow,
  Clipboard,
  ClipboardCheck,
  ClipboardList,
  Eye,
  EyeOff,
  FileInput,
  FileKey,
  KeyRound,
  Link2,
  Plus,
  Trash2,
} from 'lucide-react';
import { toast } from 'sonner';
import {
  addAppTarget,
  addDeveloperCommand,
  addDeveloperCredential,
  deleteMaterializedDeveloperCredentialFile,
  deleteAppTarget,
  deleteDeveloperCommand,
  deleteDeveloperCredential,
  linkAppTargetCredential,
  listAppTargetCredentials,
  listAppTargets,
  listDeveloperCommands,
  listDeveloperCredentials,
  listProjects,
  materializeDeveloperCredentialFile,
  readDeveloperCredentialFile,
  revealDeveloperCredential,
  unlinkAppTargetCredential,
} from '../lib/ipc';
import type {
  AppTarget,
  AppTargetCredential,
  AppTargetInput,
  DeveloperCommandInput,
  DeveloperCommandPreset,
  DeveloperCredentialInput,
  DeveloperCredentialSummary,
} from '../lib/types';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs';
import { Textarea } from '../components/ui/textarea';

const providerOptions = [
  { value: 'apple', label: 'Apple' },
  { value: 'google_play', label: 'Google Play' },
  { value: 'android_signing', label: 'Android Signing' },
  { value: 'github', label: 'GitHub' },
  { value: 'fastlane', label: 'Fastlane' },
  { value: 'firebase', label: 'Firebase' },
  { value: 'other', label: 'Other' },
];

const credentialTypeOptions = [
  { value: 'asc_api_key', label: 'ASC API Key (.p8)' },
  { value: 'service_account_json', label: 'Service Account JSON' },
  { value: 'upload_keystore', label: 'Upload Keystore' },
  { value: 'p12_certificate', label: 'P12 Certificate' },
  { value: 'token', label: 'Token' },
  { value: 'other', label: 'Other' },
];

const platformOptions = ['ios', 'macos', 'android', 'windows', 'linux', 'web', 'other'];

function splitTags(value: string): string[] {
  return value
    .split(',')
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function metadataFromPairs(pairs: Array<[string, string]>): Record<string, unknown> {
  return Object.fromEntries(pairs.filter(([key, value]) => key.trim() && value.trim()));
}

function formatMetadata(metadata: Record<string, unknown>): string {
  return readableMetadataEntries(metadata)
    .map(([label, value]) => `${label}: ${value}`)
    .join('  ');
}

function readableMetadataEntries(metadata: Record<string, unknown>): Array<[string, string]> {
  const labels: Record<string, string> = {
    team_id: 'Team ID',
    key_id: 'Key ID',
    issuer_id: 'Issuer ID',
    file_name: 'File',
    source_path: 'Imported From',
    notes: 'Notes',
  };

  return Object.entries(metadata)
    .filter(([key, value]) => {
      if (key === 'credential_extension') return false;
      return value !== null && value !== undefined && String(value).trim();
    })
    .map(([key, value]) => [labels[key] ?? key.replace(/_/g, ' '), String(value)]);
}

function credentialTypeLabel(value: string): string {
  return credentialTypeOptions.find((option) => option.value === value)?.label ?? value;
}

function providerLabel(value: string): string {
  return providerOptions.find((option) => option.value === value)?.label ?? value;
}

function extensionForCredentialType(value: string): string {
  switch (value) {
    case 'service_account_json':
      return 'json';
    case 'p12_certificate':
      return 'p12';
    case 'upload_keystore':
      return 'jks';
    default:
      return 'p8';
  }
}

function CredentialForm() {
  const queryClient = useQueryClient();
  const [form, setForm] = useState({
    provider: 'apple',
    credential_type: 'asc_api_key',
    label: '',
    tags: '',
    team_id: '',
    key_id: '',
    issuer_id: '',
    notes: '',
    secret: '',
    file_name: '',
    source_path: '',
  });

  const mutation = useMutation({
    mutationFn: (input: DeveloperCredentialInput) => addDeveloperCredential(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer-credentials'] });
      setForm((prev) => ({
        ...prev,
        label: '',
        tags: '',
        key_id: '',
        issuer_id: '',
        notes: '',
        secret: '',
        file_name: '',
        source_path: '',
      }));
      toast.success('Credential saved');
    },
    onError: (err) => toast.error(`Failed to save credential: ${err}`),
  });

  const save = () => {
    if (!form.label.trim() || !form.secret.trim()) {
      toast.error('Label and secret are required');
      return;
    }

    mutation.mutate({
      provider: form.provider,
      credential_type: form.credential_type,
      label: form.label.trim(),
      tags: splitTags(form.tags),
      metadata: metadataFromPairs([
        ['team_id', form.team_id],
        ['key_id', form.key_id],
        ['issuer_id', form.issuer_id],
        ['notes', form.notes],
        ['file_name', form.file_name],
        ['source_path', form.source_path],
        ['credential_extension', extensionForCredentialType(form.credential_type)],
      ]),
      secret: form.secret,
    });
  };

  const importFile = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: 'Credential files',
          extensions: ['p8', 'json', 'p12', 'jks', 'keystore', 'txt'],
        },
      ],
    });

    if (!selected || Array.isArray(selected)) return;

    try {
      const file = await readDeveloperCredentialFile(selected);
      const extension = file.file_name.split('.').pop()?.toLowerCase();
      const nextType =
        extension === 'json'
          ? 'service_account_json'
          : extension === 'p12'
            ? 'p12_certificate'
            : extension === 'jks' || extension === 'keystore'
              ? 'upload_keystore'
              : 'asc_api_key';
      setForm((prev) => ({
        ...prev,
        credential_type: nextType,
        label: prev.label || file.file_name.replace(/\.[^.]+$/, ''),
        file_name: file.file_name,
        source_path: file.path,
        secret: file.content,
      }));
      toast.success('Credential file loaded');
    } catch (err) {
      toast.error(`Failed to load credential file: ${err}`);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="h-4 w-4" />
          New Credential
        </CardTitle>
        <CardDescription>
          Reusable encrypted credentials for store and release workflows.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-3 md:grid-cols-3">
          <div className="space-y-1">
            <Label>Provider</Label>
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
              value={form.provider}
              onChange={(e) => setForm({ ...form, provider: e.target.value })}
            >
              {providerOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1">
            <Label>Type</Label>
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
              value={form.credential_type}
              onChange={(e) => setForm({ ...form, credential_type: e.target.value })}
            >
              {credentialTypeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1">
            <Label>Label</Label>
            <Input
              value={form.label}
              onChange={(e) => setForm({ ...form, label: e.target.value })}
              placeholder="ASC production key"
            />
          </div>
        </div>

        <div className="grid gap-3 md:grid-cols-4">
          <div className="space-y-1">
            <Label>Team ID</Label>
            <Input
              value={form.team_id}
              onChange={(e) => setForm({ ...form, team_id: e.target.value })}
            />
          </div>
          <div className="space-y-1">
            <Label>Key ID</Label>
            <Input
              value={form.key_id}
              onChange={(e) => setForm({ ...form, key_id: e.target.value })}
            />
          </div>
          <div className="space-y-1">
            <Label>Issuer ID</Label>
            <Input
              value={form.issuer_id}
              onChange={(e) => setForm({ ...form, issuer_id: e.target.value })}
            />
          </div>
          <div className="space-y-1">
            <Label>Tags</Label>
            <Input
              value={form.tags}
              onChange={(e) => setForm({ ...form, tags: e.target.value })}
              placeholder="ios, production"
            />
          </div>
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between gap-3">
            <Label>Secret</Label>
            <Button type="button" variant="outline" size="sm" onClick={importFile}>
              <FileInput className="h-4 w-4" />
              Import File
            </Button>
          </div>
          {form.file_name && (
            <div className="rounded-md border border-border bg-muted/20 px-3 py-2 text-sm">
              <div className="flex items-center gap-2 font-medium">
                <FileKey className="h-4 w-4 text-muted-foreground" />
                {form.file_name}
              </div>
              <div className="mt-1 truncate text-xs text-muted-foreground">{form.source_path}</div>
            </div>
          )}
          <Textarea
            className="min-h-[120px] font-mono text-xs"
            value={form.secret}
            onChange={(e) => setForm({ ...form, secret: e.target.value })}
            placeholder="Import a credential file or paste .p8, service account JSON, token, or other credential material"
          />
        </div>

        <div className="space-y-1">
          <Label>Notes</Label>
          <Input
            value={form.notes}
            onChange={(e) => setForm({ ...form, notes: e.target.value })}
            placeholder="Rotation or scope notes"
          />
        </div>

        <Button onClick={save} disabled={mutation.isPending}>
          <Plus className="h-4 w-4" />
          Save Credential
        </Button>
      </CardContent>
    </Card>
  );
}

function CredentialList({ credentials }: { credentials: DeveloperCredentialSummary[] }) {
  const queryClient = useQueryClient();
  const [revealed, setRevealed] = useState<Record<number, string>>({});
  const [exportedFiles, setExportedFiles] = useState<Record<number, string>>({});
  const [copiedPath, setCopiedPath] = useState<number | null>(null);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const deleteMutation = useMutation({
    mutationFn: deleteDeveloperCredential,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer-credentials'] });
      queryClient.invalidateQueries({ queryKey: ['app-target-credentials'] });
      toast.success('Credential deleted');
    },
    onError: (err) => toast.error(`Failed to delete credential: ${err}`),
  });

  const toggleReveal = async (credential: DeveloperCredentialSummary) => {
    if (revealed[credential.id]) {
      setRevealed(({ [credential.id]: _removed, ...rest }) => rest);
      return;
    }
    try {
      const secret = await revealDeveloperCredential(credential.id);
      setRevealed((prev) => ({ ...prev, [credential.id]: secret }));
    } catch (err) {
      toast.error(`Failed to decrypt credential: ${err}`);
    }
  };

  const exportCredentialFile = async (credential: DeveloperCredentialSummary) => {
    try {
      const file = await materializeDeveloperCredentialFile(credential.id);
      setExportedFiles((prev) => ({ ...prev, [credential.id]: file.path }));
      await navigator.clipboard.writeText(file.path);
      setCopiedPath(credential.id);
      window.setTimeout(() => setCopiedPath(null), 2000);
      toast.success('Credential file path copied');
    } catch (err) {
      toast.error(`Failed to export credential file: ${err}`);
    }
  };

  const deleteExportedFile = async (credential: DeveloperCredentialSummary) => {
    const path = exportedFiles[credential.id];
    if (!path) return;

    try {
      await deleteMaterializedDeveloperCredentialFile(path);
      setExportedFiles(({ [credential.id]: _removed, ...rest }) => rest);
      toast.success('Exported credential file deleted');
    } catch (err) {
      toast.error(`Failed to delete exported file: ${err}`);
    }
  };

  const copyMetadataValue = async (credentialId: number, label: string, value: string) => {
    await navigator.clipboard.writeText(value);
    const fieldKey = `${credentialId}:${label}`;
    setCopiedField(fieldKey);
    window.setTimeout(() => setCopiedField(null), 1600);
    toast.success(`${label} copied`);
  };

  if (credentials.length === 0) {
    return <p className="text-sm text-muted-foreground">No developer credentials saved.</p>;
  }

  return (
    <div className="grid gap-3">
      {credentials.map((credential) => (
        <Card key={credential.id} className="py-4">
          <CardContent className="space-y-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h3 className="font-medium">{credential.label}</h3>
                  <Badge variant="secondary">{providerLabel(credential.provider)}</Badge>
                  <Badge variant="outline">{credentialTypeLabel(credential.credential_type)}</Badge>
                </div>
              </div>
              <div className="flex shrink-0 gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => exportCredentialFile(credential)}
                  title="Export working file and copy path"
                >
                  {copiedPath === credential.id ? (
                    <ClipboardCheck className="h-4 w-4" />
                  ) : (
                    <Clipboard className="h-4 w-4" />
                  )}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => toggleReveal(credential)}
                  title={revealed[credential.id] ? 'Hide' : 'Reveal'}
                >
                  {revealed[credential.id] ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => deleteMutation.mutate(credential.id)}
                  title="Delete"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </div>
            {credential.tags.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {credential.tags.map((tag) => (
                  <Badge key={tag} variant="secondary">
                    {tag}
                  </Badge>
                ))}
              </div>
            )}
            <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-4">
              {readableMetadataEntries(credential.metadata).map(([label, value]) => (
                <button
                  key={label}
                  type="button"
                  className="rounded-md border border-border bg-muted/10 px-3 py-2 text-left transition-colors hover:border-primary/50 hover:bg-muted/30"
                  onClick={() => copyMetadataValue(credential.id, label, value)}
                  title={`Copy ${label}`}
                >
                  <div className="text-[11px] uppercase text-muted-foreground">{label}</div>
                  <div className="mt-1 flex items-center gap-2">
                    <span className="min-w-0 flex-1 truncate font-mono text-xs">{value}</span>
                    {copiedField === `${credential.id}:${label}` ? (
                      <ClipboardCheck className="h-3.5 w-3.5 shrink-0 text-primary" />
                    ) : (
                      <Clipboard className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    )}
                  </div>
                </button>
              ))}
            </div>
            {exportedFiles[credential.id] && (
              <div className="flex flex-wrap items-center gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs">
                <span className="font-medium">Working file:</span>
                <code className="min-w-0 flex-1 truncate">{exportedFiles[credential.id]}</code>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7"
                  onClick={() => navigator.clipboard.writeText(exportedFiles[credential.id])}
                >
                  Copy
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 text-muted-foreground hover:text-destructive"
                  onClick={() => deleteExportedFile(credential)}
                >
                  Delete
                </Button>
              </div>
            )}
            {revealed[credential.id] && (
              <pre className="max-h-40 overflow-auto rounded-md bg-muted/40 p-3 text-xs">
                {revealed[credential.id]}
              </pre>
            )}
            {!revealed[credential.id] && !exportedFiles[credential.id] && (
              <div className="rounded-md bg-muted/20 px-3 py-3 text-xs text-muted-foreground">
                Secret stored encrypted. Export a working file when a command needs a path.
              </div>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function AppTargetForm() {
  const queryClient = useQueryClient();
  const projectsQuery = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });
  const [form, setForm] = useState({
    name: '',
    platform: 'ios',
    project_id: '',
    bundle_id: '',
    package_name: '',
    store_app_id: '',
    team_id: '',
  });

  const mutation = useMutation({
    mutationFn: (input: AppTargetInput) => addAppTarget(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-targets'] });
      setForm({
        name: '',
        platform: 'ios',
        project_id: '',
        bundle_id: '',
        package_name: '',
        store_app_id: '',
        team_id: '',
      });
      toast.success('App target saved');
    },
    onError: (err) => toast.error(`Failed to save app target: ${err}`),
  });

  const save = () => {
    if (!form.name.trim()) {
      toast.error('Name is required');
      return;
    }
    mutation.mutate({
      name: form.name.trim(),
      platform: form.platform,
      project_id: form.project_id.trim() || null,
      bundle_id: form.bundle_id.trim() || null,
      package_name: form.package_name.trim() || null,
      store_app_id: form.store_app_id.trim() || null,
      metadata: metadataFromPairs([['team_id', form.team_id]]),
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <AppWindow className="h-4 w-4" />
          New App Target
        </CardTitle>
        <CardDescription>Store-facing apps that can reference shared credentials.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-3 md:grid-cols-3">
          <div className="space-y-1">
            <Label>Name</Label>
            <Input
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              placeholder="TARS macOS"
            />
          </div>
          <div className="space-y-1">
            <Label>Platform</Label>
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
              value={form.platform}
              onChange={(e) => setForm({ ...form, platform: e.target.value })}
            >
              {platformOptions.map((platform) => (
                <option key={platform} value={platform}>
                  {platform}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1">
            <Label>Team ID</Label>
            <Input
              value={form.team_id}
              onChange={(e) => setForm({ ...form, team_id: e.target.value })}
            />
          </div>
        </div>
        <div className="grid gap-3 md:grid-cols-4">
          <div className="space-y-1">
            <Label>Project</Label>
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
              value={form.project_id}
              onChange={(e) => setForm({ ...form, project_id: e.target.value })}
            >
              <option value="">None</option>
              {(projectsQuery.data ?? []).map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1">
            <Label>Bundle ID</Label>
            <Input
              value={form.bundle_id}
              onChange={(e) => setForm({ ...form, bundle_id: e.target.value })}
              placeholder="com.example.app"
            />
          </div>
          <div className="space-y-1">
            <Label>Package Name</Label>
            <Input
              value={form.package_name}
              onChange={(e) => setForm({ ...form, package_name: e.target.value })}
              placeholder="com.example.app"
            />
          </div>
          <div className="space-y-1">
            <Label>Store App ID</Label>
            <Input
              value={form.store_app_id}
              onChange={(e) => setForm({ ...form, store_app_id: e.target.value })}
            />
          </div>
        </div>
        <Button onClick={save} disabled={mutation.isPending}>
          <Plus className="h-4 w-4" />
          Save App Target
        </Button>
      </CardContent>
    </Card>
  );
}

function TargetCredentialLinks({
  target,
  credentials,
}: {
  target: AppTarget;
  credentials: DeveloperCredentialSummary[];
}) {
  const queryClient = useQueryClient();
  const [credentialId, setCredentialId] = useState(
    credentials[0]?.id ? String(credentials[0].id) : ''
  );
  const [role, setRole] = useState('asc-api-key');

  const linksQuery = useQuery({
    queryKey: ['app-target-credentials', target.id],
    queryFn: () => listAppTargetCredentials(target.id),
  });

  const linkMutation = useMutation({
    mutationFn: () => linkAppTargetCredential(target.id, Number(credentialId), role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-target-credentials', target.id] });
      toast.success('Credential linked');
    },
    onError: (err) => toast.error(`Failed to link credential: ${err}`),
  });

  const unlinkMutation = useMutation({
    mutationFn: (link: AppTargetCredential) =>
      unlinkAppTargetCredential(target.id, link.credential_id, link.role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-target-credentials', target.id] });
      toast.success('Credential unlinked');
    },
    onError: (err) => toast.error(`Failed to unlink credential: ${err}`),
  });

  return (
    <div className="space-y-3 border-t border-border pt-3">
      <div className="flex flex-wrap gap-2">
        <select
          className="h-8 min-w-[220px] rounded-md border border-input bg-background px-2 text-sm"
          value={credentialId}
          onChange={(e) => setCredentialId(e.target.value)}
          disabled={credentials.length === 0}
        >
          {credentials.map((credential) => (
            <option key={credential.id} value={credential.id}>
              {credential.label}
            </option>
          ))}
        </select>
        <Input
          className="h-8 w-44"
          value={role}
          onChange={(e) => setRole(e.target.value)}
          placeholder="role"
        />
        <Button
          size="sm"
          onClick={() => linkMutation.mutate()}
          disabled={!credentialId || !role.trim()}
        >
          <Link2 className="h-4 w-4" />
          Link
        </Button>
      </div>
      <div className="flex flex-wrap gap-2">
        {(linksQuery.data ?? []).map((link) => (
          <Badge
            key={`${link.credential_id}-${link.role}`}
            variant="outline"
            className="gap-2 py-1"
          >
            {link.role}: {link.credential_label}
            <button
              type="button"
              onClick={() => unlinkMutation.mutate(link)}
              className="text-muted-foreground hover:text-destructive"
            >
              ×
            </button>
          </Badge>
        ))}
        {linksQuery.data?.length === 0 && (
          <span className="text-xs text-muted-foreground">No credentials linked.</span>
        )}
      </div>
    </div>
  );
}

function AppTargetList({
  targets,
  credentials,
}: {
  targets: AppTarget[];
  credentials: DeveloperCredentialSummary[];
}) {
  const queryClient = useQueryClient();
  const deleteMutation = useMutation({
    mutationFn: deleteAppTarget,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-targets'] });
      toast.success('App target deleted');
    },
    onError: (err) => toast.error(`Failed to delete app target: ${err}`),
  });

  if (targets.length === 0) {
    return <p className="text-sm text-muted-foreground">No app targets saved.</p>;
  }

  return (
    <div className="grid gap-3">
      {targets.map((target) => (
        <Card key={target.id} className="py-4">
          <CardContent className="space-y-3">
            <div className="flex items-start justify-between gap-3">
              <div>
                <div className="flex flex-wrap items-center gap-2">
                  <h3 className="font-medium">{target.name}</h3>
                  <Badge variant="secondary">{target.platform}</Badge>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  {[
                    target.bundle_id,
                    target.package_name,
                    target.store_app_id,
                    formatMetadata(target.metadata),
                  ]
                    .filter(Boolean)
                    .join('  ')}
                </p>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => deleteMutation.mutate(target.id)}
                title="Delete"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
            <TargetCredentialLinks target={target} credentials={credentials} />
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function CommandForm({ targets }: { targets: AppTarget[] }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState({
    name: '',
    command: '',
    working_dir: '',
    app_target_id: '',
    tags: '',
  });

  const mutation = useMutation({
    mutationFn: (input: DeveloperCommandInput) => addDeveloperCommand(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer-commands'] });
      setForm({ name: '', command: '', working_dir: '', app_target_id: '', tags: '' });
      toast.success('Command saved');
    },
    onError: (err) => toast.error(`Failed to save command: ${err}`),
  });

  const save = () => {
    if (!form.name.trim() || !form.command.trim()) {
      toast.error('Name and command are required');
      return;
    }
    mutation.mutate({
      name: form.name.trim(),
      command: form.command.trim(),
      working_dir: form.working_dir.trim() || null,
      app_target_id: form.app_target_id ? Number(form.app_target_id) : null,
      tags: splitTags(form.tags),
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ClipboardList className="h-4 w-4" />
          New Command Preset
        </CardTitle>
        <CardDescription>Common build, upload, release, and store commands.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-3 md:grid-cols-3">
          <div className="space-y-1">
            <Label>Name</Label>
            <Input
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              placeholder="TestFlight beta"
            />
          </div>
          <div className="space-y-1">
            <Label>Target</Label>
            <select
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
              value={form.app_target_id}
              onChange={(e) => setForm({ ...form, app_target_id: e.target.value })}
            >
              <option value="">Any target</option>
              {targets.map((target) => (
                <option key={target.id} value={target.id}>
                  {target.name}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1">
            <Label>Tags</Label>
            <Input
              value={form.tags}
              onChange={(e) => setForm({ ...form, tags: e.target.value })}
              placeholder="ios, beta"
            />
          </div>
        </div>
        <div className="space-y-1">
          <Label>Command</Label>
          <Input
            className="font-mono text-sm"
            value={form.command}
            onChange={(e) => setForm({ ...form, command: e.target.value })}
            placeholder="fastlane ios beta"
          />
        </div>
        <div className="space-y-1">
          <Label>Working Directory</Label>
          <Input
            value={form.working_dir}
            onChange={(e) => setForm({ ...form, working_dir: e.target.value })}
            placeholder="{project_path}"
          />
        </div>
        <Button onClick={save} disabled={mutation.isPending}>
          <Plus className="h-4 w-4" />
          Save Command
        </Button>
      </CardContent>
    </Card>
  );
}

function CommandList({
  commands,
  targets,
}: {
  commands: DeveloperCommandPreset[];
  targets: AppTarget[];
}) {
  const queryClient = useQueryClient();
  const targetNames = useMemo(
    () => new Map(targets.map((target) => [target.id, target.name])),
    [targets]
  );
  const deleteMutation = useMutation({
    mutationFn: deleteDeveloperCommand,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer-commands'] });
      toast.success('Command deleted');
    },
    onError: (err) => toast.error(`Failed to delete command: ${err}`),
  });

  if (commands.length === 0) {
    return <p className="text-sm text-muted-foreground">No command presets saved.</p>;
  }

  return (
    <div className="grid gap-3">
      {commands.map((command) => (
        <Card key={command.id} className="py-4">
          <CardContent className="space-y-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h3 className="font-medium">{command.name}</h3>
                  {command.app_target_id && (
                    <Badge variant="secondary">
                      {targetNames.get(command.app_target_id) ?? 'Target'}
                    </Badge>
                  )}
                </div>
                <code className="mt-2 block overflow-x-auto rounded-md bg-muted/40 p-3 text-xs">
                  {command.command}
                </code>
                {command.working_dir && (
                  <p className="mt-1 text-xs text-muted-foreground">cwd: {command.working_dir}</p>
                )}
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => deleteMutation.mutate(command.id)}
                title="Delete"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function useDeveloperData() {
  const credentialsQuery = useQuery({
    queryKey: ['developer-credentials'],
    queryFn: listDeveloperCredentials,
  });
  const targetsQuery = useQuery({
    queryKey: ['app-targets'],
    queryFn: listAppTargets,
  });
  const commandsQuery = useQuery({
    queryKey: ['developer-commands'],
    queryFn: listDeveloperCommands,
  });

  return {
    credentials: credentialsQuery.data ?? [],
    targets: targetsQuery.data ?? [],
    commands: commandsQuery.data ?? [],
    isLoading: credentialsQuery.isLoading || targetsQuery.isLoading || commandsQuery.isLoading,
    error: credentialsQuery.error ?? targetsQuery.error ?? commandsQuery.error,
  };
}

export function DeveloperPage() {
  const { credentials, targets, commands, isLoading, error } = useDeveloperData();

  return (
    <div className="h-full flex flex-col">
      <div className="shrink-0 border-b border-border bg-card/50 px-6 py-4">
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <AppWindow className="h-5 w-5" />
          Developer
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Manage reusable app-store credentials, app targets, and release commands.
        </p>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        {isLoading ? (
          <p className="text-muted-foreground">Loading developer settings…</p>
        ) : error ? (
          <p className="text-destructive">Failed to load developer settings: {String(error)}</p>
        ) : (
          <Tabs defaultValue="credentials" className="gap-4">
            <TabsList>
              <TabsTrigger value="credentials">Credentials</TabsTrigger>
              <TabsTrigger value="targets">App Targets</TabsTrigger>
              <TabsTrigger value="commands">Commands</TabsTrigger>
            </TabsList>
            <TabsContent value="credentials" className="space-y-4">
              <CredentialForm />
              <CredentialList credentials={credentials} />
            </TabsContent>
            <TabsContent value="targets" className="space-y-4">
              <AppTargetForm />
              <AppTargetList targets={targets} credentials={credentials} />
            </TabsContent>
            <TabsContent value="commands" className="space-y-4">
              <CommandForm targets={targets} />
              <CommandList commands={commands} targets={targets} />
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
