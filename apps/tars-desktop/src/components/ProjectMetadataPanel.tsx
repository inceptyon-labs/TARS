import { useState, useEffect, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  ChevronRight,
  ChevronDown,
  Cloud,
  Database,
  HardDrive,
  Terminal,
  Globe,
  Save,
  Plus,
  X,
  Info,
  Pencil,
  ExternalLink,
} from 'lucide-react';
import { toast } from 'sonner';
import { getProjectMetadata, saveProjectMetadata } from '../lib/ipc';
import type { ProjectMetadata, CustomField } from '../lib/types';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';

const DEPLOY_TARGETS = [
  'Vercel',
  'Cloudflare Pages',
  'Cloudflare Workers',
  'AWS',
  'GCP',
  'Azure',
  'Fly.io',
  'Railway',
  'Render',
  'Netlify',
  'DigitalOcean',
  'Hetzner',
  'Self-hosted',
  'Local only',
  'Other',
];

const DATABASE_PROVIDERS = [
  'Neon',
  'Supabase',
  'PlanetScale',
  'Turso',
  'CockroachDB',
  'AWS RDS',
  'GCP Cloud SQL',
  'Azure SQL',
  'Local Postgres',
  'Local MySQL',
  'SQLite',
  'MongoDB Atlas',
  'Redis Cloud',
  'None',
  'Other',
];

const OBJECT_STORAGE_OPTIONS = [
  'AWS S3',
  'Cloudflare R2',
  'Storj',
  'GCS',
  'Azure Blob',
  'Backblaze B2',
  'MinIO',
  'DigitalOcean Spaces',
  'Local',
  'None',
  'Other',
];

const TUNNEL_PROVIDERS = ['Cloudflare Tunnel', 'ngrok', 'localtunnel', 'Tailscale', 'Other'];

const CI_CD_OPTIONS = [
  'GitHub Actions',
  'CircleCI',
  'GitLab CI',
  'Jenkins',
  'Buildkite',
  'Travis CI',
  'Bitbucket Pipelines',
  'None',
  'Other',
];

interface ProjectMetadataPanelProps {
  projectId: string;
  projectPath: string;
}

const EMPTY_METADATA: ProjectMetadata = {
  deploy_target: null,
  domain: null,
  production_url: null,
  staging_url: null,
  deploy_command: null,
  database_provider: null,
  database_name: null,
  object_storage: null,
  object_storage_bucket: null,
  start_command: null,
  requires_tunnel: false,
  tunnel_provider: null,
  tunnel_id: null,
  github_url: null,
  app_store_url: null,
  app_store_connect_url: null,
  play_store_url: null,
  package_registry_url: null,
  ci_cd: null,
  monitoring: null,
  custom_fields: [],
};

// ── Edit mode field components ──────────────────────────────────

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string | null;
  options: string[];
  onChange: (val: string | null) => void;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      <Select value={value || ''} onValueChange={(v) => onChange(v || null)}>
        <SelectTrigger className="h-8 text-sm">
          <SelectValue placeholder="Select..." />
        </SelectTrigger>
        <SelectContent>
          {options.map((opt) => (
            <SelectItem key={opt} value={opt}>
              {opt}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  placeholder,
  mono,
}: {
  label: string;
  value: string | null;
  onChange: (val: string | null) => void;
  placeholder?: string;
  mono?: boolean;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      <Input
        className={`h-8 text-sm ${mono ? 'font-mono' : ''}`}
        value={value || ''}
        onChange={(e) => onChange(e.target.value || null)}
        placeholder={placeholder}
      />
    </div>
  );
}

// ── View mode helpers ───────────────────────────────────────────

function isUrl(value: string): boolean {
  return /^https?:\/\//i.test(value);
}

function ViewValue({ value, mono }: { value: string; mono?: boolean }) {
  if (isUrl(value)) {
    return (
      <a
        href={value}
        target="_blank"
        rel="noopener noreferrer"
        className="text-sm text-primary hover:underline inline-flex items-center gap-1 truncate"
        onClick={(e) => e.stopPropagation()}
      >
        <span className="truncate">{value.replace(/^https?:\/\//, '')}</span>
        <ExternalLink className="h-3 w-3 flex-shrink-0 opacity-50" />
      </a>
    );
  }
  return (
    <span
      className={`text-sm text-foreground ${mono ? 'font-mono bg-muted/50 px-1.5 py-0.5 rounded' : ''}`}
    >
      {value}
    </span>
  );
}

function ViewRow({ label, value, mono }: { label: string; value: string | null; mono?: boolean }) {
  if (!value) return null;
  return (
    <div className="flex items-baseline gap-3 min-w-0">
      <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
        {label}
      </span>
      <ViewValue value={value} mono={mono} />
    </div>
  );
}

interface ViewSectionProps {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}

function ViewSection({ icon, title, children }: ViewSectionProps) {
  // Only render if there are non-null children
  const childArray = Array.isArray(children) ? children : [children];
  const hasContent = childArray.some((c) => c !== null && c !== undefined && c !== false);
  if (!hasContent) return null;

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
        {icon}
        {title}
      </div>
      <div className="space-y-1.5 pl-1">{children}</div>
    </div>
  );
}

// ── Main component ──────────────────────────────────────────────

export function ProjectMetadataPanel({ projectId }: ProjectMetadataPanelProps) {
  const queryClient = useQueryClient();
  const [isExpanded, setIsExpanded] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [isDirty, setIsDirty] = useState(false);
  const [metadata, setMetadata] = useState<ProjectMetadata>(EMPTY_METADATA);

  const { data: savedMetadata } = useQuery({
    queryKey: ['project-metadata', projectId],
    queryFn: () => getProjectMetadata(projectId),
    enabled: !!projectId,
  });

  useEffect(() => {
    if (savedMetadata) {
      setMetadata(savedMetadata);
      setIsDirty(false);
    }
  }, [savedMetadata]);

  const saveMutation = useMutation({
    mutationFn: () => saveProjectMetadata(projectId, metadata),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-metadata', projectId] });
      setIsDirty(false);
      setIsEditing(false);
      toast.success('Project info saved');
    },
    onError: (err) => toast.error(`Failed to save: ${err}`),
  });

  const update = useCallback(
    <K extends keyof ProjectMetadata>(key: K, value: ProjectMetadata[K]) => {
      setMetadata((prev) => ({ ...prev, [key]: value }));
      setIsDirty(true);
    },
    []
  );

  const addCustomField = () => {
    setMetadata((prev) => ({
      ...prev,
      custom_fields: [...prev.custom_fields, { key: '', value: '' }],
    }));
    setIsDirty(true);
  };

  const updateCustomField = (index: number, field: Partial<CustomField>) => {
    setMetadata((prev) => ({
      ...prev,
      custom_fields: prev.custom_fields.map((f, i) => (i === index ? { ...f, ...field } : f)),
    }));
    setIsDirty(true);
  };

  const removeCustomField = (index: number) => {
    setMetadata((prev) => ({
      ...prev,
      custom_fields: prev.custom_fields.filter((_, i) => i !== index),
    }));
    setIsDirty(true);
  };

  const cancelEdit = () => {
    if (savedMetadata) {
      setMetadata(savedMetadata);
    } else {
      setMetadata(EMPTY_METADATA);
    }
    setIsDirty(false);
    setIsEditing(false);
  };

  // Count filled fields for badge
  const filledCount =
    [
      metadata.deploy_target,
      metadata.domain,
      metadata.production_url,
      metadata.staging_url,
      metadata.deploy_command,
      metadata.database_provider,
      metadata.database_name,
      metadata.object_storage,
      metadata.object_storage_bucket,
      metadata.start_command,
      metadata.github_url,
      metadata.ci_cd,
      metadata.monitoring,
      metadata.app_store_url,
      metadata.app_store_connect_url,
      metadata.play_store_url,
      metadata.package_registry_url,
      metadata.tunnel_provider,
      metadata.tunnel_id,
    ].filter(Boolean).length +
    (metadata.requires_tunnel ? 1 : 0) +
    metadata.custom_fields.filter((f) => f.key && f.value).length;

  // Save on Cmd/Ctrl+S
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's' && isDirty && isEditing) {
        e.preventDefault();
        saveMutation.mutate();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isDirty, isEditing, saveMutation]);

  const hasAnyData = filledCount > 0;

  // ── View mode ───────────────────────────────────────────────

  const renderViewMode = () => {
    if (!hasAnyData) {
      return (
        <div className="px-4 py-6 text-center">
          <p className="text-sm text-muted-foreground/60 mb-3">
            No project info yet. Add hosting, database, deploy commands, and more.
          </p>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setIsEditing(true)}
            className="gap-1.5"
          >
            <Pencil className="h-3 w-3" />
            Add Project Info
          </Button>
        </div>
      );
    }

    return (
      <div className="p-4 space-y-4">
        <ViewSection icon={<Cloud className="h-3.5 w-3.5" />} title="Hosting & Deployment">
          <ViewRow label="Deploy Target" value={metadata.deploy_target} />
          <ViewRow label="Domain" value={metadata.domain} />
          <ViewRow label="Production" value={metadata.production_url} />
          <ViewRow label="Staging" value={metadata.staging_url} />
          {metadata.deploy_command && (
            <div className="flex items-baseline gap-3 min-w-0">
              <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                Deploy
              </span>
              <code className="text-sm font-mono bg-muted/50 px-2 py-1 rounded text-primary">
                {metadata.deploy_command}
              </code>
            </div>
          )}
        </ViewSection>

        <ViewSection icon={<Database className="h-3.5 w-3.5" />} title="Data & Storage">
          <ViewRow label="Database" value={metadata.database_provider} />
          <ViewRow label="DB Name / ID" value={metadata.database_name} />
          <ViewRow label="Object Storage" value={metadata.object_storage} />
          <ViewRow label="Bucket / Path" value={metadata.object_storage_bucket} />
        </ViewSection>

        <ViewSection icon={<Terminal className="h-3.5 w-3.5" />} title="Local Development">
          {metadata.start_command && (
            <div className="flex items-baseline gap-3 min-w-0">
              <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                Start
              </span>
              <code className="text-sm font-mono bg-muted/50 px-2 py-1 rounded text-primary">
                {metadata.start_command}
              </code>
            </div>
          )}
          {metadata.requires_tunnel && (
            <ViewRow
              label="Tunnel"
              value={
                [metadata.tunnel_provider, metadata.tunnel_id].filter(Boolean).join(' — ') || 'Yes'
              }
            />
          )}
        </ViewSection>

        <ViewSection icon={<Globe className="h-3.5 w-3.5" />} title="Source & Distribution">
          <ViewRow label="GitHub" value={metadata.github_url} />
          <ViewRow label="Package Registry" value={metadata.package_registry_url} />
          <ViewRow label="App Store" value={metadata.app_store_url} />
          <ViewRow label="App Store Connect" value={metadata.app_store_connect_url} />
          <ViewRow label="Play Store" value={metadata.play_store_url} />
        </ViewSection>

        <ViewSection icon={<HardDrive className="h-3.5 w-3.5" />} title="Infrastructure">
          <ViewRow label="CI/CD" value={metadata.ci_cd} />
          <ViewRow label="Monitoring" value={metadata.monitoring} />
        </ViewSection>

        {metadata.custom_fields.filter((f) => f.key && f.value).length > 0 && (
          <ViewSection icon={<Plus className="h-3.5 w-3.5" />} title="Custom">
            {metadata.custom_fields
              .filter((f) => f.key && f.value)
              .map((field) => (
                <ViewRow key={field.key} label={field.key} value={field.value} />
              ))}
          </ViewSection>
        )}
      </div>
    );
  };

  // ── Edit mode ───────────────────────────────────────────────

  const renderEditMode = () => (
    <div className="p-4 space-y-6">
      {/* Hosting & Deployment */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Cloud className="h-3.5 w-3.5" />
          Hosting & Deployment
        </div>
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label="Deploy Target"
            value={metadata.deploy_target}
            options={DEPLOY_TARGETS}
            onChange={(v) => update('deploy_target', v)}
          />
          <TextField
            label="Domain"
            value={metadata.domain}
            onChange={(v) => update('domain', v)}
            placeholder="example.com"
          />
          <TextField
            label="Production URL"
            value={metadata.production_url}
            onChange={(v) => update('production_url', v)}
            placeholder="https://..."
          />
          <TextField
            label="Staging URL"
            value={metadata.staging_url}
            onChange={(v) => update('staging_url', v)}
            placeholder="https://staging...."
          />
        </div>
        <TextField
          label="Deploy Command"
          value={metadata.deploy_command}
          onChange={(v) => update('deploy_command', v)}
          placeholder="bun run deploy, vercel --prod, fly deploy, etc."
          mono
        />
      </div>

      {/* Data & Storage */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Database className="h-3.5 w-3.5" />
          Data & Storage
        </div>
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label="Database Provider"
            value={metadata.database_provider}
            options={DATABASE_PROVIDERS}
            onChange={(v) => update('database_provider', v)}
          />
          <TextField
            label="Database Name / ID"
            value={metadata.database_name}
            onChange={(v) => update('database_name', v)}
            placeholder="my-app-db"
          />
          <SelectField
            label="Object Storage"
            value={metadata.object_storage}
            options={OBJECT_STORAGE_OPTIONS}
            onChange={(v) => update('object_storage', v)}
          />
          <TextField
            label="Bucket / Path"
            value={metadata.object_storage_bucket}
            onChange={(v) => update('object_storage_bucket', v)}
            placeholder="my-bucket"
          />
        </div>
      </div>

      {/* Local Development */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Terminal className="h-3.5 w-3.5" />
          Local Development
        </div>
        <div className="grid grid-cols-2 gap-3">
          <TextField
            label="Start Command"
            value={metadata.start_command}
            onChange={(v) => update('start_command', v)}
            placeholder="bun run dev"
            mono
          />
          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">Requires Tunnel?</Label>
            <button
              onClick={() => update('requires_tunnel', !metadata.requires_tunnel)}
              className={`h-8 w-full rounded-md border text-sm text-left px-3 transition-colors ${
                metadata.requires_tunnel
                  ? 'bg-primary/10 border-primary text-primary'
                  : 'bg-transparent border-input text-muted-foreground'
              }`}
            >
              {metadata.requires_tunnel ? 'Yes' : 'No'}
            </button>
          </div>
          {metadata.requires_tunnel && (
            <>
              <SelectField
                label="Tunnel Provider"
                value={metadata.tunnel_provider}
                options={TUNNEL_PROVIDERS}
                onChange={(v) => update('tunnel_provider', v)}
              />
              <TextField
                label="Tunnel ID"
                value={metadata.tunnel_id}
                onChange={(v) => update('tunnel_id', v)}
                placeholder="tunnel-abc123"
              />
            </>
          )}
        </div>
      </div>

      {/* Source & Distribution */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Globe className="h-3.5 w-3.5" />
          Source & Distribution
        </div>
        <div className="grid grid-cols-2 gap-3">
          <TextField
            label="GitHub URL"
            value={metadata.github_url}
            onChange={(v) => update('github_url', v)}
            placeholder="https://github.com/..."
          />
          <TextField
            label="Package Registry URL"
            value={metadata.package_registry_url}
            onChange={(v) => update('package_registry_url', v)}
            placeholder="https://npmjs.com/package/..."
          />
          <TextField
            label="App Store URL"
            value={metadata.app_store_url}
            onChange={(v) => update('app_store_url', v)}
            placeholder="https://apps.apple.com/..."
          />
          <TextField
            label="App Store Connect"
            value={metadata.app_store_connect_url}
            onChange={(v) => update('app_store_connect_url', v)}
            placeholder="https://appstoreconnect.apple.com/..."
          />
          <TextField
            label="Play Store URL"
            value={metadata.play_store_url}
            onChange={(v) => update('play_store_url', v)}
            placeholder="https://play.google.com/..."
          />
        </div>
      </div>

      {/* Infrastructure */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <HardDrive className="h-3.5 w-3.5" />
          Infrastructure
        </div>
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label="CI/CD"
            value={metadata.ci_cd}
            options={CI_CD_OPTIONS}
            onChange={(v) => update('ci_cd', v)}
          />
          <TextField
            label="Monitoring"
            value={metadata.monitoring}
            onChange={(v) => update('monitoring', v)}
            placeholder="Sentry, Datadog, etc."
          />
        </div>
      </div>

      {/* Custom Fields */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Plus className="h-3.5 w-3.5" />
            Custom Fields
          </div>
          <Button size="sm" variant="ghost" onClick={addCustomField} className="h-6 text-xs gap-1">
            <Plus className="h-3 w-3" />
            Add
          </Button>
        </div>
        {metadata.custom_fields.map((field, i) => (
          <div key={i} className="flex items-end gap-2">
            <div className="flex-1 space-y-1">
              <Label className="text-xs text-muted-foreground">Key</Label>
              <Input
                className="h-8 text-sm"
                value={field.key}
                onChange={(e) => updateCustomField(i, { key: e.target.value })}
                placeholder="Key"
              />
            </div>
            <div className="flex-1 space-y-1">
              <Label className="text-xs text-muted-foreground">Value</Label>
              <Input
                className="h-8 text-sm"
                value={field.value}
                onChange={(e) => updateCustomField(i, { value: e.target.value })}
                placeholder="Value"
              />
            </div>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => removeCustomField(i)}
              className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        ))}
        {metadata.custom_fields.length === 0 && (
          <p className="text-xs text-muted-foreground/60 italic">
            No custom fields. Add key-value pairs for anything not covered above.
          </p>
        )}
      </div>

      {/* Save / Cancel bar */}
      <div className="flex justify-end gap-2 pt-2 border-t border-border">
        <Button size="sm" variant="ghost" onClick={cancelEdit} className="h-8 text-xs">
          Cancel
        </Button>
        <Button
          size="sm"
          onClick={() => saveMutation.mutate()}
          disabled={saveMutation.isPending || !isDirty}
          className="h-8 text-xs gap-1.5"
        >
          <Save className="h-3 w-3" />
          Save
        </Button>
      </div>
    </div>
  );

  // ── Render ──────────────────────────────────────────────────

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
          <Info className="h-4 w-4 text-blue-400" />
          <span className="font-medium">Project Info</span>
          {filledCount > 0 && !isExpanded && (
            <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
              {filledCount} field{filledCount !== 1 ? 's' : ''}
            </span>
          )}
        </button>
        {isExpanded && !isEditing && hasAnyData && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setIsEditing(true)}
            className="h-7 gap-1.5 text-xs"
          >
            <Pencil className="h-3 w-3" />
            Edit
          </Button>
        )}
      </div>

      {isExpanded && (isEditing ? renderEditMode() : renderViewMode())}
    </div>
  );
}
