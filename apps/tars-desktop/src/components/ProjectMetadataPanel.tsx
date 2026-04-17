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
  Smartphone,
  Layers,
  Copy,
  Check,
  FolderOpen,
} from 'lucide-react';
import { toast } from 'sonner';
import { open } from '@tauri-apps/plugin-dialog';
import { getProjectMetadata, saveProjectMetadata, fetchGithubDescription } from '../lib/ipc';
import type { ProjectMetadata, CustomField } from '../lib/types';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';

// ── Option constants ───────────────────────────────────────────

const PLATFORMS = [
  'iOS',
  'Android',
  'Web',
  'macOS',
  'Windows',
  'Linux',
  'tvOS',
  'watchOS',
  'Homebrew',
];

const APP_FRAMEWORK_OPTIONS = [
  'SwiftUI',
  'UIKit',
  'Flutter',
  'React Native',
  'Expo',
  'Capacitor',
  'Tauri',
  'Electron',
  '.NET MAUI',
  'KMP',
  'Ionic',
  'Other',
];

const WEB_HOSTING_OPTIONS = [
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
  'CloudKit',
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

const IOS_DEPLOY_TARGETS = ['iOS 15.0', 'iOS 16.0', 'iOS 17.0', 'iOS 18.0', 'iOS 18.4', 'Other'];

const IOS_PROVISIONING_OPTIONS = ['Automatic', 'Development', 'Ad Hoc', 'App Store', 'Enterprise'];

const ANDROID_MIN_SDK_OPTIONS = [
  'API 24 (7.0)',
  'API 26 (8.0)',
  'API 28 (9.0)',
  'API 29 (10)',
  'API 30 (11)',
  'API 31 (12)',
  'API 33 (13)',
  'API 34 (14)',
  'API 35 (15)',
  'Other',
];

const ANDROID_TARGET_SDK_OPTIONS = ['API 33 (13)', 'API 34 (14)', 'API 35 (15)', 'Other'];

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

const MACOS_PROVISIONING_OPTIONS = [
  'Automatic',
  'Development',
  'Developer ID',
  'App Store',
  'Direct Distribution',
];

const MACOS_APP_CATEGORIES = [
  'Business',
  'Developer Tools',
  'Education',
  'Entertainment',
  'Finance',
  'Games',
  'Graphics & Design',
  'Health & Fitness',
  'Lifestyle',
  'Music',
  'News',
  'Photo & Video',
  'Productivity',
  'Reference',
  'Social Networking',
  'Utilities',
  'Weather',
  'Other',
];

interface ProjectMetadataPanelProps {
  projectId: string;
  projectPath: string;
}

const EMPTY_METADATA: ProjectMetadata = {
  description: null,
  icon_path: null,
  platforms: [],
  app_framework: null,
  deploy_target: null,
  web_hosting: null,
  domain: null,
  production_url: null,
  staging_url: null,
  deploy_command: null,
  database_provider: null,
  database_name: null,
  database_dashboard_url: null,
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
  ios_deploy_target: null,
  ios_bundle_id: null,
  ios_signing_team: null,
  ios_cloudkit_container: null,
  ios_cloudkit_dashboard_url: null,
  ios_uses_push_notifications: false,
  ios_provisioning: null,
  ios_deploy_command: null,
  ios_deploy_commands: [],
  android_package_name: null,
  android_min_sdk: null,
  android_target_sdk: null,
  android_signing_key: null,
  android_deploy_command: null,
  android_deploy_commands: [],
  google_play_console_url: null,
  macos_bundle_id: null,
  macos_signing_team: null,
  macos_app_category: null,
  macos_hardened_runtime: false,
  macos_app_sandbox: false,
  macos_provisioning: null,
  macos_deploy_commands: [],
  homebrew_formula_name: null,
  homebrew_tap: null,
  homebrew_deploy_commands: [],
  deploy_commands: [],
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

function CopyableCode({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <button
      onClick={handleCopy}
      className="text-sm font-mono bg-muted/50 px-2 py-1 rounded text-primary hover:bg-muted transition-colors inline-flex items-center gap-1.5 cursor-pointer group"
      title="Click to copy"
    >
      <span>{value}</span>
      {copied ? (
        <Check className="h-3 w-3 text-green-500 flex-shrink-0" />
      ) : (
        <Copy className="h-3 w-3 opacity-0 group-hover:opacity-50 flex-shrink-0 transition-opacity" />
      )}
    </button>
  );
}

function MultiCommandField({
  label,
  values,
  onChange,
  placeholder,
}: {
  label: string;
  values: string[];
  onChange: (vals: string[]) => void;
  placeholder?: string;
}) {
  const addEntry = () => onChange([...values, '']);
  const updateEntry = (i: number, val: string) =>
    onChange(values.map((v, idx) => (idx === i ? val : v)));
  const removeEntry = (i: number) => onChange(values.filter((_, idx) => idx !== i));

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <Label className="text-xs text-muted-foreground">{label}</Label>
        <Button size="sm" variant="ghost" onClick={addEntry} className="h-5 text-xs gap-0.5 px-1.5">
          <Plus className="h-3 w-3" />
          Add
        </Button>
      </div>
      {values.map((val, i) => (
        <div key={i} className="flex items-center gap-1.5">
          <Input
            className="h-8 text-sm font-mono flex-1"
            value={val}
            onChange={(e) => updateEntry(i, e.target.value)}
            placeholder={placeholder}
          />
          <Button
            size="sm"
            variant="ghost"
            onClick={() => removeEntry(i)}
            className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive shrink-0"
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      ))}
      {values.length === 0 && (
        <p className="text-xs text-muted-foreground/50 italic">No commands configured</p>
      )}
    </div>
  );
}

function PlatformChips({
  selected,
  onChange,
}: {
  selected: string[];
  onChange: (platforms: string[]) => void;
}) {
  const toggle = (platform: string) => {
    if (selected.includes(platform)) {
      onChange(selected.filter((p) => p !== platform));
    } else {
      onChange([...selected, platform]);
    }
  };

  return (
    <div className="space-y-1">
      <Label className="text-xs text-muted-foreground">Platforms</Label>
      <div className="flex flex-wrap gap-1.5">
        {PLATFORMS.map((platform) => {
          const isSelected = selected.includes(platform);
          return (
            <button
              key={platform}
              onClick={() => toggle(platform)}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors border ${
                isSelected
                  ? 'bg-primary/10 border-primary text-primary'
                  : 'bg-transparent border-input text-muted-foreground hover:border-muted-foreground/50'
              }`}
            >
              {platform}
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ── View mode helpers ───────────────────────────────────────────

function isUrl(value: string): boolean {
  return /^https?:\/\//i.test(value);
}

function ViewValue({
  value,
  mono,
  copyable,
}: {
  value: string;
  mono?: boolean;
  copyable?: boolean;
}) {
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
  if (copyable || mono) {
    return <CopyableCode value={value} />;
  }
  return <span className="text-sm text-foreground">{value}</span>;
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

export function ProjectMetadataPanel({ projectId, projectPath }: ProjectMetadataPanelProps) {
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
      // Migrate legacy fields
      const migrated = { ...savedMetadata };
      if (!migrated.platforms) migrated.platforms = [];
      if (migrated.deploy_target && !migrated.web_hosting) {
        migrated.web_hosting = migrated.deploy_target;
        migrated.deploy_target = null;
      }
      // Migrate single deploy commands → arrays
      if (!migrated.deploy_commands) migrated.deploy_commands = [];
      if (migrated.deploy_command && migrated.deploy_commands.length === 0) {
        migrated.deploy_commands = [migrated.deploy_command];
        migrated.deploy_command = null;
      }
      if (!migrated.ios_deploy_commands) migrated.ios_deploy_commands = [];
      if (migrated.ios_deploy_command && migrated.ios_deploy_commands.length === 0) {
        migrated.ios_deploy_commands = [migrated.ios_deploy_command];
        migrated.ios_deploy_command = null;
      }
      if (!migrated.android_deploy_commands) migrated.android_deploy_commands = [];
      if (migrated.android_deploy_command && migrated.android_deploy_commands.length === 0) {
        migrated.android_deploy_commands = [migrated.android_deploy_command];
        migrated.android_deploy_command = null;
      }
      // Ensure new array fields exist
      if (!migrated.macos_deploy_commands) migrated.macos_deploy_commands = [];
      if (!migrated.homebrew_deploy_commands) migrated.homebrew_deploy_commands = [];
      setMetadata(migrated);
      setIsDirty(false);
    }
  }, [savedMetadata]);

  // Auto-fetch description from GitHub only if never set (null).
  // Empty string means user intentionally cleared it — don't re-fetch.
  useEffect(() => {
    if (!savedMetadata?.github_url || savedMetadata.description !== null) return;
    let cancelled = false;
    fetchGithubDescription(savedMetadata.github_url)
      .then((desc) => {
        if (cancelled || !desc) return;
        const updated = { ...savedMetadata, description: desc };
        saveProjectMetadata(projectId, updated).then(() => {
          queryClient.invalidateQueries({ queryKey: ['project-metadata', projectId] });
        });
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [savedMetadata, projectId, queryClient]);

  const saveMutation = useMutation({
    mutationFn: () => saveProjectMetadata(projectId, metadata),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-metadata', projectId] });
      queryClient.invalidateQueries({ queryKey: ['project-icon', projectPath] });
      queryClient.invalidateQueries({ queryKey: ['project-categories'] });
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

  // Platform helpers
  const hasPlat = (p: string) => metadata.platforms.includes(p);
  const hasWeb = hasPlat('Web');
  const hasIOS = hasPlat('iOS');
  const hasAndroid = hasPlat('Android');
  const hasMacOS = hasPlat('macOS');
  const hasHomebrew = hasPlat('Homebrew');

  // Count filled fields for badge
  const filledCount =
    metadata.platforms.length +
    [
      metadata.description,
      metadata.app_framework,
      metadata.web_hosting,
      metadata.domain,
      metadata.production_url,
      metadata.staging_url,
      metadata.deploy_command,
      metadata.database_provider,
      metadata.database_name,
      metadata.database_dashboard_url,
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
      metadata.ios_deploy_target,
      metadata.ios_bundle_id,
      metadata.ios_signing_team,
      metadata.ios_cloudkit_container,
      metadata.ios_cloudkit_dashboard_url,
      metadata.ios_provisioning,
      metadata.android_package_name,
      metadata.android_min_sdk,
      metadata.android_target_sdk,
      metadata.android_signing_key,
      metadata.google_play_console_url,
      metadata.macos_bundle_id,
      metadata.macos_signing_team,
      metadata.macos_app_category,
      metadata.macos_provisioning,
      metadata.homebrew_formula_name,
      metadata.homebrew_tap,
    ].filter(Boolean).length +
    (metadata.requires_tunnel ? 1 : 0) +
    (metadata.ios_uses_push_notifications ? 1 : 0) +
    (metadata.macos_hardened_runtime ? 1 : 0) +
    (metadata.macos_app_sandbox ? 1 : 0) +
    metadata.deploy_commands.filter(Boolean).length +
    metadata.ios_deploy_commands.filter(Boolean).length +
    metadata.android_deploy_commands.filter(Boolean).length +
    metadata.macos_deploy_commands.filter(Boolean).length +
    metadata.homebrew_deploy_commands.filter(Boolean).length +
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
            No project info yet. Add platforms, hosting, database, and more.
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
        <ViewSection icon={<Layers className="h-3.5 w-3.5" />} title="Platforms & Framework">
          {metadata.platforms.length > 0 && (
            <div className="flex items-baseline gap-3 min-w-0">
              <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                Platforms
              </span>
              <div className="flex flex-wrap gap-1">
                {metadata.platforms.map((p) => (
                  <span
                    key={p}
                    className="px-2 py-0.5 rounded text-xs font-medium bg-primary/10 text-primary"
                  >
                    {p}
                  </span>
                ))}
              </div>
            </div>
          )}
          <ViewRow label="Framework" value={metadata.app_framework} />
        </ViewSection>

        {hasWeb && (
          <ViewSection icon={<Cloud className="h-3.5 w-3.5" />} title="Web Hosting & Deployment">
            <ViewRow label="Hosting" value={metadata.web_hosting} />
            <ViewRow label="Domain" value={metadata.domain} />
            <ViewRow label="Production" value={metadata.production_url} />
            <ViewRow label="Staging" value={metadata.staging_url} />
            {metadata.deploy_commands.filter(Boolean).map((cmd, i) => (
              <div key={i} className="flex items-baseline gap-3 min-w-0">
                <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                  {i === 0 ? 'Deploy' : ''}
                </span>
                <CopyableCode value={cmd} />
              </div>
            ))}
          </ViewSection>
        )}

        {hasIOS && (
          <ViewSection icon={<Smartphone className="h-3.5 w-3.5" />} title="iOS">
            <ViewRow label="Deploy Target" value={metadata.ios_deploy_target} />
            <ViewRow label="Bundle ID" value={metadata.ios_bundle_id} mono />
            <ViewRow label="Signing Team" value={metadata.ios_signing_team} />
            <ViewRow label="CloudKit Container" value={metadata.ios_cloudkit_container} mono />
            <ViewRow label="CloudKit Dashboard" value={metadata.ios_cloudkit_dashboard_url} />
            {metadata.ios_uses_push_notifications && (
              <ViewRow label="Push Notifications" value="Enabled" />
            )}
            <ViewRow label="Provisioning" value={metadata.ios_provisioning} />
            {metadata.ios_deploy_commands.filter(Boolean).map((cmd, i) => (
              <div key={i} className="flex items-baseline gap-3 min-w-0">
                <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                  {i === 0 ? 'Deploy' : ''}
                </span>
                <CopyableCode value={cmd} />
              </div>
            ))}
          </ViewSection>
        )}

        {hasAndroid && (
          <ViewSection icon={<Smartphone className="h-3.5 w-3.5" />} title="Android">
            <ViewRow label="Package Name" value={metadata.android_package_name} mono />
            <ViewRow label="Min SDK" value={metadata.android_min_sdk} />
            <ViewRow label="Target SDK" value={metadata.android_target_sdk} />
            <ViewRow label="Signing Key" value={metadata.android_signing_key} />
            <ViewRow label="Play Console" value={metadata.google_play_console_url} />
            {metadata.android_deploy_commands.filter(Boolean).map((cmd, i) => (
              <div key={i} className="flex items-baseline gap-3 min-w-0">
                <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                  {i === 0 ? 'Deploy' : ''}
                </span>
                <CopyableCode value={cmd} />
              </div>
            ))}
          </ViewSection>
        )}

        {hasMacOS && (
          <ViewSection icon={<Layers className="h-3.5 w-3.5" />} title="macOS">
            <ViewRow label="Bundle ID" value={metadata.macos_bundle_id} mono />
            <ViewRow label="Signing Team" value={metadata.macos_signing_team} />
            <ViewRow label="Category" value={metadata.macos_app_category} />
            {metadata.macos_hardened_runtime && (
              <ViewRow label="Hardened Runtime" value="Enabled" />
            )}
            {metadata.macos_app_sandbox && <ViewRow label="App Sandbox" value="Enabled" />}
            <ViewRow label="Provisioning" value={metadata.macos_provisioning} />
            {metadata.macos_deploy_commands.filter(Boolean).map((cmd, i) => (
              <div key={i} className="flex items-baseline gap-3 min-w-0">
                <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                  {i === 0 ? 'Deploy' : ''}
                </span>
                <CopyableCode value={cmd} />
              </div>
            ))}
          </ViewSection>
        )}

        {hasHomebrew && (
          <ViewSection icon={<Terminal className="h-3.5 w-3.5" />} title="Homebrew">
            <ViewRow label="Formula" value={metadata.homebrew_formula_name} mono />
            <ViewRow label="Tap" value={metadata.homebrew_tap} mono />
            {metadata.homebrew_deploy_commands.filter(Boolean).map((cmd, i) => (
              <div key={i} className="flex items-baseline gap-3 min-w-0">
                <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                  {i === 0 ? 'Deploy' : ''}
                </span>
                <CopyableCode value={cmd} />
              </div>
            ))}
          </ViewSection>
        )}

        <ViewSection icon={<Database className="h-3.5 w-3.5" />} title="Data & Storage">
          <ViewRow label="Database" value={metadata.database_provider} />
          <ViewRow label="DB Name / ID" value={metadata.database_name} />
          <ViewRow label="Dashboard" value={metadata.database_dashboard_url} />
          <ViewRow label="Object Storage" value={metadata.object_storage} />
          <ViewRow label="Bucket / Path" value={metadata.object_storage_bucket} />
        </ViewSection>

        <ViewSection icon={<Terminal className="h-3.5 w-3.5" />} title="Local Development">
          {metadata.start_command && (
            <div className="flex items-baseline gap-3 min-w-0">
              <span className="text-xs text-muted-foreground w-[140px] flex-shrink-0 text-right">
                Start
              </span>
              <CopyableCode value={metadata.start_command} />
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
      {/* Description & Icon */}
      <TextField
        label="Description"
        value={metadata.description}
        onChange={(v) => update('description', v)}
        placeholder="Short project description (auto-filled from GitHub)"
      />
      <div className="space-y-1">
        <Label className="text-xs text-muted-foreground">Icon Path</Label>
        <div className="flex gap-2">
          <Input
            className="h-8 text-sm font-mono flex-1"
            value={metadata.icon_path || ''}
            onChange={(e) => update('icon_path', e.target.value || null)}
            placeholder="relative/path/to/icon.png"
          />
          <Button
            size="sm"
            variant="outline"
            className="h-8 px-2 shrink-0"
            onClick={async () => {
              try {
                const selected = await open({
                  multiple: false,
                  title: 'Select Project Icon',
                  defaultPath: projectPath,
                  filters: [
                    { name: 'Images', extensions: ['png', 'svg', 'ico', 'jpg', 'jpeg', 'webp'] },
                  ],
                });
                if (selected && typeof selected === 'string') {
                  // Convert absolute path to relative
                  const prefix = projectPath.endsWith('/') ? projectPath : projectPath + '/';
                  const relative = selected.startsWith(prefix)
                    ? selected.slice(prefix.length)
                    : selected;
                  update('icon_path', relative);
                }
              } catch {
                // Ignore errors when opening file picker
              }
            }}
          >
            <FolderOpen className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Platforms & Framework */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Layers className="h-3.5 w-3.5" />
          Platforms & Framework
        </div>
        <PlatformChips selected={metadata.platforms} onChange={(v) => update('platforms', v)} />
        <div className="grid grid-cols-2 gap-3">
          <SelectField
            label="App Framework"
            value={metadata.app_framework}
            options={APP_FRAMEWORK_OPTIONS}
            onChange={(v) => update('app_framework', v)}
          />
        </div>
      </div>

      {/* Web Hosting & Deployment — shown when Web platform selected */}
      {hasWeb && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Cloud className="h-3.5 w-3.5" />
            Web Hosting & Deployment
          </div>
          <div className="grid grid-cols-2 gap-3">
            <SelectField
              label="Hosting"
              value={metadata.web_hosting}
              options={WEB_HOSTING_OPTIONS}
              onChange={(v) => update('web_hosting', v)}
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
          <MultiCommandField
            label="Deploy Commands"
            values={metadata.deploy_commands}
            onChange={(v) => update('deploy_commands', v)}
            placeholder="bun run deploy, vercel --prod, fly deploy, etc."
          />
        </div>
      )}

      {/* iOS — shown when iOS platform selected */}
      {hasIOS && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Smartphone className="h-3.5 w-3.5" />
            iOS
          </div>
          <div className="grid grid-cols-2 gap-3">
            <SelectField
              label="Deploy Target"
              value={metadata.ios_deploy_target}
              options={IOS_DEPLOY_TARGETS}
              onChange={(v) => update('ios_deploy_target', v)}
            />
            <TextField
              label="Bundle ID"
              value={metadata.ios_bundle_id}
              onChange={(v) => update('ios_bundle_id', v)}
              placeholder="com.example.myapp"
              mono
            />
            <TextField
              label="Signing Team ID"
              value={metadata.ios_signing_team}
              onChange={(v) => update('ios_signing_team', v)}
              placeholder="ABCD1234EF"
            />
            <TextField
              label="CloudKit Container"
              value={metadata.ios_cloudkit_container}
              onChange={(v) => update('ios_cloudkit_container', v)}
              placeholder="iCloud.com.example.myapp"
              mono
            />
            <TextField
              label="CloudKit Dashboard URL"
              value={metadata.ios_cloudkit_dashboard_url}
              onChange={(v) => update('ios_cloudkit_dashboard_url', v)}
              placeholder="https://icloud.developer.apple.com/dashboard/..."
            />
            <SelectField
              label="Provisioning"
              value={metadata.ios_provisioning}
              options={IOS_PROVISIONING_OPTIONS}
              onChange={(v) => update('ios_provisioning', v)}
            />
            <div className="space-y-1">
              <Label className="text-xs text-muted-foreground">Push Notifications?</Label>
              <button
                onClick={() =>
                  update('ios_uses_push_notifications', !metadata.ios_uses_push_notifications)
                }
                className={`h-8 w-full rounded-md border text-sm text-left px-3 transition-colors ${
                  metadata.ios_uses_push_notifications
                    ? 'bg-primary/10 border-primary text-primary'
                    : 'bg-transparent border-input text-muted-foreground'
                }`}
              >
                {metadata.ios_uses_push_notifications ? 'Yes' : 'No'}
              </button>
            </div>
          </div>
          <MultiCommandField
            label="Deploy Commands"
            values={metadata.ios_deploy_commands}
            onChange={(v) => update('ios_deploy_commands', v)}
            placeholder="fastlane beta, xcodebuild archive, etc."
          />
        </div>
      )}

      {/* Android — shown when Android platform selected */}
      {hasAndroid && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Smartphone className="h-3.5 w-3.5" />
            Android
          </div>
          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="Package Name"
              value={metadata.android_package_name}
              onChange={(v) => update('android_package_name', v)}
              placeholder="com.example.myapp"
              mono
            />
            <SelectField
              label="Min SDK"
              value={metadata.android_min_sdk}
              options={ANDROID_MIN_SDK_OPTIONS}
              onChange={(v) => update('android_min_sdk', v)}
            />
            <SelectField
              label="Target SDK"
              value={metadata.android_target_sdk}
              options={ANDROID_TARGET_SDK_OPTIONS}
              onChange={(v) => update('android_target_sdk', v)}
            />
            <TextField
              label="Signing Key Alias"
              value={metadata.android_signing_key}
              onChange={(v) => update('android_signing_key', v)}
              placeholder="upload-key"
            />
          </div>
          <TextField
            label="Google Play Console URL"
            value={metadata.google_play_console_url}
            onChange={(v) => update('google_play_console_url', v)}
            placeholder="https://play.google.com/console/..."
          />
          <MultiCommandField
            label="Deploy Commands"
            values={metadata.android_deploy_commands}
            onChange={(v) => update('android_deploy_commands', v)}
            placeholder="fastlane android deploy, ./gradlew bundleRelease, etc."
          />
        </div>
      )}

      {/* macOS — shown when macOS platform selected */}
      {hasMacOS && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Layers className="h-3.5 w-3.5" />
            macOS
          </div>
          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="Bundle ID"
              value={metadata.macos_bundle_id}
              onChange={(v) => update('macos_bundle_id', v)}
              placeholder="com.example.myapp"
              mono
            />
            <TextField
              label="Signing Team ID"
              value={metadata.macos_signing_team}
              onChange={(v) => update('macos_signing_team', v)}
              placeholder="ABCD1234EF"
            />
            <SelectField
              label="App Category"
              value={metadata.macos_app_category}
              options={MACOS_APP_CATEGORIES}
              onChange={(v) => update('macos_app_category', v)}
            />
            <SelectField
              label="Provisioning"
              value={metadata.macos_provisioning}
              options={MACOS_PROVISIONING_OPTIONS}
              onChange={(v) => update('macos_provisioning', v)}
            />
            <div className="space-y-1">
              <Label className="text-xs text-muted-foreground">Hardened Runtime?</Label>
              <button
                onClick={() => update('macos_hardened_runtime', !metadata.macos_hardened_runtime)}
                className={`h-8 w-full rounded-md border text-sm text-left px-3 transition-colors ${
                  metadata.macos_hardened_runtime
                    ? 'bg-primary/10 border-primary text-primary'
                    : 'bg-transparent border-input text-muted-foreground'
                }`}
              >
                {metadata.macos_hardened_runtime ? 'Yes' : 'No'}
              </button>
            </div>
            <div className="space-y-1">
              <Label className="text-xs text-muted-foreground">App Sandbox?</Label>
              <button
                onClick={() => update('macos_app_sandbox', !metadata.macos_app_sandbox)}
                className={`h-8 w-full rounded-md border text-sm text-left px-3 transition-colors ${
                  metadata.macos_app_sandbox
                    ? 'bg-primary/10 border-primary text-primary'
                    : 'bg-transparent border-input text-muted-foreground'
                }`}
              >
                {metadata.macos_app_sandbox ? 'Yes' : 'No'}
              </button>
            </div>
          </div>
          <MultiCommandField
            label="Deploy Commands"
            values={metadata.macos_deploy_commands}
            onChange={(v) => update('macos_deploy_commands', v)}
            placeholder="fastlane mac, xcodebuild archive, notarytool submit, etc."
          />
        </div>
      )}

      {/* Homebrew — shown when Homebrew platform selected */}
      {hasHomebrew && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            <Terminal className="h-3.5 w-3.5" />
            Homebrew
          </div>
          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="Formula Name"
              value={metadata.homebrew_formula_name}
              onChange={(v) => update('homebrew_formula_name', v)}
              placeholder="my-tool"
              mono
            />
            <TextField
              label="Tap Repository"
              value={metadata.homebrew_tap}
              onChange={(v) => update('homebrew_tap', v)}
              placeholder="username/homebrew-tap"
              mono
            />
          </div>
          <MultiCommandField
            label="Deploy Commands"
            values={metadata.homebrew_deploy_commands}
            onChange={(v) => update('homebrew_deploy_commands', v)}
            placeholder="brew bump-formula-pr, goreleaser, etc."
          />
        </div>
      )}

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
          <TextField
            label="Dashboard URL"
            value={metadata.database_dashboard_url}
            onChange={(v) => update('database_dashboard_url', v)}
            placeholder="https://console.neon.tech/..."
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
