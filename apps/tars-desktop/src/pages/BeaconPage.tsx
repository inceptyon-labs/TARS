/**
 * Beacon management page
 *
 * Navigation beacons for GitHub repos, documentation, and resources.
 * Stored in ~/.tars/beacons/ - outside Claude's config locations.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  ExternalLink,
  Github,
  FileText,
  Globe,
  BookOpen,
  Link2,
  Plus,
  RefreshCw,
  Save,
  Trash2,
  X,
  Tag,
  Search,
  ChevronDown,
  ChevronRight,
  FolderOpen,
} from 'lucide-react';
import { useState, useEffect, useMemo } from 'react';
import { toast } from 'sonner';
import { openUrl } from '@tauri-apps/plugin-opener';
import { listBeacons, readBeacon, createBeacon, updateBeacon, deleteBeacon } from '../lib/ipc';
import type { BeaconSummary, BeaconType, BeaconLink } from '../lib/types';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import { ConfirmDialog } from '../components/config/ConfirmDialog';
import { Badge } from '../components/ui/badge';

// Simple Reddit icon component
function RedditIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0zm5.01 4.744c.688 0 1.25.561 1.25 1.249a1.25 1.25 0 0 1-2.498.056l-2.597-.547-.8 3.747c1.824.07 3.48.632 4.674 1.488.308-.309.73-.491 1.207-.491.968 0 1.754.786 1.754 1.754 0 .716-.435 1.333-1.01 1.614a3.111 3.111 0 0 1 .042.52c0 2.694-3.13 4.87-7.004 4.87-3.874 0-7.004-2.176-7.004-4.87 0-.183.015-.366.043-.534A1.748 1.748 0 0 1 4.028 12c0-.968.786-1.754 1.754-1.754.463 0 .898.196 1.207.49 1.207-.883 2.878-1.43 4.744-1.487l.885-4.182a.342.342 0 0 1 .14-.197.35.35 0 0 1 .238-.042l2.906.617a1.214 1.214 0 0 1 1.108-.701zM9.25 12C8.561 12 8 12.562 8 13.25c0 .687.561 1.248 1.25 1.248.687 0 1.248-.561 1.248-1.249 0-.688-.561-1.249-1.249-1.249zm5.5 0c-.687 0-1.248.561-1.248 1.25 0 .687.561 1.248 1.249 1.248.688 0 1.249-.561 1.249-1.249 0-.687-.562-1.249-1.25-1.249zm-5.466 3.99a.327.327 0 0 0-.231.094.33.33 0 0 0 0 .463c.842.842 2.484.913 2.961.913.477 0 2.105-.056 2.961-.913a.361.361 0 0 0 .029-.463.33.33 0 0 0-.464 0c-.547.533-1.684.73-2.512.73-.828 0-1.979-.196-2.512-.73a.326.326 0 0 0-.232-.095z" />
    </svg>
  );
}

// Simple Twitter/X icon component
function TwitterIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}

const BEACON_TYPE_OPTIONS: { value: BeaconType; label: string; icon: React.ReactNode }[] = [
  { value: 'github', label: 'GitHub', icon: <Github className="h-4 w-4" /> },
  { value: 'documentation', label: 'Documentation', icon: <BookOpen className="h-4 w-4" /> },
  { value: 'api', label: 'API', icon: <Globe className="h-4 w-4" /> },
  { value: 'resource', label: 'Resource', icon: <FileText className="h-4 w-4" /> },
  { value: 'reddit', label: 'Reddit', icon: <RedditIcon className="h-4 w-4" /> },
  { value: 'twitter', label: 'Twitter', icon: <TwitterIcon className="h-4 w-4" /> },
  { value: 'custom', label: 'Custom', icon: <Link2 className="h-4 w-4" /> },
];

function getBeaconIcon(type: BeaconType) {
  switch (type) {
    case 'github':
      return <Github className="h-4 w-4" />;
    case 'documentation':
      return <BookOpen className="h-4 w-4" />;
    case 'api':
      return <Globe className="h-4 w-4" />;
    case 'resource':
      return <FileText className="h-4 w-4" />;
    case 'reddit':
      return <RedditIcon className="h-4 w-4" />;
    case 'twitter':
      return <TwitterIcon className="h-4 w-4" />;
    default:
      return <Link2 className="h-4 w-4" />;
  }
}

interface EditableLink {
  label: string;
  url: string;
}

const UNCATEGORIZED = '__uncategorized__';
const NO_CATEGORY = '__none__';

export function BeaconPage() {
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editCategory, setEditCategory] = useState('');
  const [editLinks, setEditLinks] = useState<EditableLink[]>([{ label: '', url: '' }]);
  const [editDescription, setEditDescription] = useState('');
  const [editType, setEditType] = useState<BeaconType>('custom');
  const [editTags, setEditTags] = useState('');
  const [beaconToDelete, setBeaconToDelete] = useState<BeaconSummary | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [isEditingCategories, setIsEditingCategories] = useState(false);
  const [newCategoryName, setNewCategoryName] = useState('');

  // Fetch beacons list
  const { data: beacons = [], isLoading } = useQuery({
    queryKey: ['beacons'],
    queryFn: listBeacons,
  });

  // Fetch selected beacon details
  const { data: selectedBeacon, isLoading: loadingBeacon } = useQuery({
    queryKey: ['beacon', selectedId],
    queryFn: () => (selectedId ? readBeacon(selectedId) : null),
    enabled: !!selectedId && !isCreating,
  });

  // Collect all unique categories
  const allCategories = useMemo(() => {
    const catSet = new Set<string>();
    beacons.forEach((b) => {
      if (b.category) catSet.add(b.category);
    });
    return Array.from(catSet).sort();
  }, [beacons]);

  // Filter beacons by search query and category
  const filteredBeacons = useMemo(() => {
    let result = beacons;

    // Filter by category
    if (selectedCategory === UNCATEGORIZED) {
      result = result.filter((b) => !b.category);
    } else if (selectedCategory) {
      result = result.filter((b) => b.category === selectedCategory);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (b) =>
          b.title.toLowerCase().includes(query) ||
          b.tags.some((t) => t.toLowerCase().includes(query))
      );
    }

    return result;
  }, [beacons, searchQuery, selectedCategory]);

  // Group beacons by category for display
  const groupedBeacons = useMemo(() => {
    if (selectedCategory) {
      // If a category is selected, don't group - just show flat list
      return null;
    }

    const groups: Record<string, BeaconSummary[]> = {};
    const uncategorized: BeaconSummary[] = [];

    filteredBeacons.forEach((b) => {
      if (b.category) {
        if (!groups[b.category]) groups[b.category] = [];
        groups[b.category].push(b);
      } else {
        uncategorized.push(b);
      }
    });

    return { groups, uncategorized };
  }, [filteredBeacons, selectedCategory]);

  // Update edit state when selected beacon changes (only if not actively editing)
  useEffect(() => {
    if (selectedBeacon && !isCreating && !isEditing) {
      setEditTitle(selectedBeacon.title);
      setEditCategory(selectedBeacon.category || '');
      setEditLinks(
        selectedBeacon.links.length > 0
          ? selectedBeacon.links.map((l) => ({ label: l.label || '', url: l.url }))
          : [{ label: '', url: '' }]
      );
      setEditDescription(selectedBeacon.description || '');
      setEditType(selectedBeacon.beacon_type);
      setEditTags(selectedBeacon.tags.join(', '));
    }
  }, [selectedBeacon, isCreating, isEditing]);

  // Create mutation
  const createMutation = useMutation({
    mutationFn: ({
      title,
      category,
      links,
      description,
      beaconType,
      tags,
    }: {
      title: string;
      category: string | null;
      links: BeaconLink[];
      description: string | null;
      beaconType: BeaconType;
      tags: string[];
    }) => createBeacon(title, category, links, description, beaconType, tags),
    onSuccess: (newBeacon) => {
      queryClient.invalidateQueries({ queryKey: ['beacons'] });
      setSelectedId(newBeacon.id);
      setIsCreating(false);
      setIsEditing(false);
      toast.success('Beacon created');
    },
    onError: (err) => {
      toast.error('Failed to create beacon', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({
      id,
      title,
      category,
      links,
      description,
      beaconType,
      tags,
    }: {
      id: string;
      title: string;
      category: string | null;
      links: BeaconLink[];
      description: string | null;
      beaconType: BeaconType;
      tags: string[];
    }) => updateBeacon(id, title, category, links, description, beaconType, tags),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['beacons'] }),
        queryClient.invalidateQueries({ queryKey: ['beacon', selectedId] }),
      ]);
      setIsEditing(false);
      toast.success('Beacon saved');
    },
    onError: (err) => {
      toast.error('Failed to save beacon', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteBeacon(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['beacons'] });
      if (selectedId === beaconToDelete?.id) {
        setSelectedId(null);
        setIsEditing(false);
      }
      setBeaconToDelete(null);
      toast.success('Beacon deleted');
    },
    onError: (err) => {
      toast.error('Failed to delete beacon', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  function handleCreate() {
    setIsCreating(true);
    setSelectedId(null);
    setEditTitle('');
    setEditCategory(selectedCategory && selectedCategory !== UNCATEGORIZED ? selectedCategory : '');
    setEditLinks([{ label: '', url: '' }]);
    setEditDescription('');
    setEditType('custom');
    setEditTags('');
    setIsEditing(true);
  }

  function parseTags(tagString: string): string[] {
    return tagString
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }

  function handleSave() {
    if (!editTitle.trim()) {
      toast.error('Title is required');
      return;
    }

    const tags = parseTags(editTags);
    const description = editDescription.trim() || null;
    const category = editCategory.trim() || null;

    // Filter out empty links and convert to BeaconLink format
    const links: BeaconLink[] = editLinks
      .filter((l) => l.url.trim())
      .map((l) => ({
        label: l.label.trim() || null,
        url: l.url.trim(),
      }));

    if (isCreating) {
      createMutation.mutate({
        title: editTitle,
        category,
        links,
        description,
        beaconType: editType,
        tags,
      });
    } else if (selectedId) {
      updateMutation.mutate({
        id: selectedId,
        title: editTitle,
        category,
        links,
        description,
        beaconType: editType,
        tags,
      });
    }
  }

  function handleCancel() {
    if (isCreating) {
      setIsCreating(false);
      setIsEditing(false);
      setEditTitle('');
      setEditCategory('');
      setEditLinks([{ label: '', url: '' }]);
      setEditDescription('');
      setEditType('custom');
      setEditTags('');
    } else if (selectedBeacon) {
      setEditTitle(selectedBeacon.title);
      setEditCategory(selectedBeacon.category || '');
      setEditLinks(
        selectedBeacon.links.length > 0
          ? selectedBeacon.links.map((l) => ({ label: l.label || '', url: l.url }))
          : [{ label: '', url: '' }]
      );
      setEditDescription(selectedBeacon.description || '');
      setEditType(selectedBeacon.beacon_type);
      setEditTags(selectedBeacon.tags.join(', '));
      setIsEditing(false);
    }
  }

  function handleSelect(beacon: BeaconSummary) {
    if (isCreating) {
      setIsCreating(false);
    }
    setSelectedId(beacon.id);
    setIsEditing(false);
  }

  function handleToggleExpand(beaconId: string, e: React.MouseEvent) {
    e.stopPropagation();
    setExpandedId(expandedId === beaconId ? null : beaconId);
  }

  function handleStartEditing() {
    setIsEditing(true);
  }

  function handleAddLink() {
    setEditLinks([...editLinks, { label: '', url: '' }]);
  }

  function handleRemoveLink(index: number) {
    if (editLinks.length > 1) {
      setEditLinks(editLinks.filter((_, i) => i !== index));
    } else {
      setEditLinks([{ label: '', url: '' }]);
    }
  }

  function handleLinkChange(index: number, field: 'label' | 'url', value: string) {
    const newLinks = [...editLinks];
    newLinks[index] = { ...newLinks[index], [field]: value };
    setEditLinks(newLinks);
  }

  function handleAddCategory() {
    const trimmed = newCategoryName.trim();
    if (trimmed) {
      setEditCategory(trimmed);
      setNewCategoryName('');
      setIsEditingCategories(false);
    }
  }

  function safeOpenUrl(url: string) {
    // Only allow http/https URLs to prevent javascript: XSS
    if (url.startsWith('http://') || url.startsWith('https://')) {
      openUrl(url);
    } else {
      toast.error('Invalid URL', { description: 'Only http and https URLs are supported' });
    }
  }

  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  const isSaving = createMutation.isPending || updateMutation.isPending;

  // Render a single beacon item
  function renderBeaconItem(beacon: BeaconSummary) {
    const isExpanded = expandedId === beacon.id;
    const isSelected = selectedId === beacon.id && !isCreating;

    return (
      <div
        key={beacon.id}
        className={`border-b border-border last:border-b-0 ${isSelected ? 'bg-muted' : ''}`}
      >
        <div
          role="button"
          tabIndex={0}
          onClick={() => handleSelect(beacon)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleSelect(beacon);
            }
          }}
          className="w-full text-left px-3 py-2 hover:bg-muted/50 transition-colors group flex items-center gap-2 cursor-pointer"
        >
          <button
            type="button"
            onClick={(e) => handleToggleExpand(beacon.id, e)}
            className="p-0.5 hover:bg-muted rounded shrink-0"
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground" />
            )}
          </button>
          <span className="text-muted-foreground shrink-0">
            {getBeaconIcon(beacon.beacon_type)}
          </span>
          <span className="font-medium truncate flex-1 text-sm">{beacon.title}</span>
          <Button
            variant="ghost"
            size="sm"
            className="opacity-0 group-hover:opacity-100 shrink-0 h-5 w-5 p-0 text-muted-foreground hover:text-destructive"
            onClick={(e) => {
              e.stopPropagation();
              setBeaconToDelete(beacon);
            }}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>

        {/* Expanded details */}
        {isExpanded && (
          <div className="px-3 pb-2 pl-9 space-y-1">
            {beacon.links.length > 0 && (
              <p className="text-xs text-muted-foreground truncate">
                {beacon.links[0].label || beacon.links[0].url}
                {beacon.links.length > 1 && ` +${beacon.links.length - 1} more`}
              </p>
            )}
            {beacon.tags.length > 0 && (
              <div className="flex gap-1 flex-wrap">
                {beacon.tags.slice(0, 4).map((tag, idx) => (
                  <Badge
                    key={`${tag}-${idx}`}
                    variant="secondary"
                    className="text-[10px] px-1.5 py-0"
                  >
                    {tag}
                  </Badge>
                ))}
                {beacon.tags.length > 4 && (
                  <span className="text-[10px] text-muted-foreground">
                    +{beacon.tags.length - 4}
                  </span>
                )}
              </div>
            )}
            <p className="text-[10px] text-muted-foreground/60">{formatDate(beacon.updated_at)}</p>
          </div>
        )}
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-6 w-6 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Beacon</h2>
          <span className="text-xs text-muted-foreground">Navigation links & resources</span>
        </div>
        <Button size="sm" onClick={handleCreate}>
          <Plus className="h-4 w-4 mr-2" />
          New Beacon
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Beacons List */}
        <div className="w-72 border-r border-border flex flex-col">
          {/* Category Tabs */}
          <div className="p-2 border-b border-border">
            <div className="flex flex-wrap gap-1">
              <Badge
                variant={selectedCategory === null ? 'default' : 'secondary'}
                className="cursor-pointer text-xs"
                onClick={() => setSelectedCategory(null)}
              >
                All
              </Badge>
              {allCategories.map((cat) => (
                <Badge
                  key={cat}
                  variant={selectedCategory === cat ? 'default' : 'secondary'}
                  className="cursor-pointer text-xs"
                  onClick={() => setSelectedCategory(cat)}
                >
                  {cat}
                </Badge>
              ))}
              {beacons.some((b) => !b.category) && (
                <Badge
                  variant={selectedCategory === UNCATEGORIZED ? 'default' : 'secondary'}
                  className="cursor-pointer text-xs"
                  onClick={() => setSelectedCategory(UNCATEGORIZED)}
                >
                  Uncategorized
                </Badge>
              )}
            </div>
          </div>

          {/* Search */}
          <div className="p-2 border-b border-border">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                placeholder="Search..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8 h-7 text-xs"
              />
            </div>
          </div>

          {/* List */}
          <div className="flex-1 overflow-auto">
            {filteredBeacons.length === 0 && !isCreating ? (
              <div className="p-6 text-center text-muted-foreground">
                <Link2 className="h-10 w-10 mx-auto mb-2 opacity-50" />
                <p className="text-sm">{searchQuery ? 'No beacons match' : 'No beacons yet'}</p>
                <p className="text-xs mt-1">
                  {searchQuery ? 'Try a different search' : 'Click "New Beacon" to create one'}
                </p>
              </div>
            ) : groupedBeacons && !selectedCategory ? (
              // Grouped view
              <div>
                {Object.entries(groupedBeacons.groups)
                  .sort(([a], [b]) => a.localeCompare(b))
                  .map(([category, categoryBeacons]) => (
                    <div key={category}>
                      <div className="px-3 py-1.5 bg-muted/50 text-xs font-medium text-muted-foreground flex items-center gap-2 sticky top-0">
                        <FolderOpen className="h-3 w-3" />
                        {category}
                        <span className="text-muted-foreground/60">({categoryBeacons.length})</span>
                      </div>
                      {categoryBeacons.map(renderBeaconItem)}
                    </div>
                  ))}
                {groupedBeacons.uncategorized.length > 0 && (
                  <div>
                    {Object.keys(groupedBeacons.groups).length > 0 && (
                      <div className="px-3 py-1.5 bg-muted/50 text-xs font-medium text-muted-foreground flex items-center gap-2 sticky top-0">
                        <FolderOpen className="h-3 w-3" />
                        Uncategorized
                        <span className="text-muted-foreground/60">
                          ({groupedBeacons.uncategorized.length})
                        </span>
                      </div>
                    )}
                    {groupedBeacons.uncategorized.map(renderBeaconItem)}
                  </div>
                )}
              </div>
            ) : (
              // Flat list view (when category is selected)
              <div>{filteredBeacons.map(renderBeaconItem)}</div>
            )}
          </div>
        </div>

        {/* Detail Panel */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {isCreating || selectedBeacon ? (
            <>
              {/* Detail Header */}
              <div className="h-12 border-b border-border px-4 flex items-center justify-between shrink-0 bg-muted/30">
                <div className="flex items-center gap-3 flex-1 min-w-0">
                  {!isEditing && selectedBeacon && (
                    <>
                      <span className="text-muted-foreground">
                        {getBeaconIcon(selectedBeacon.beacon_type)}
                      </span>
                      <span className="font-medium truncate">{selectedBeacon.title}</span>
                      {selectedBeacon.category && (
                        <Badge variant="secondary" className="text-xs">
                          {selectedBeacon.category}
                        </Badge>
                      )}
                    </>
                  )}
                  {isEditing && (
                    <span className="font-medium">{isCreating ? 'New Beacon' : 'Edit Beacon'}</span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  {!isEditing && !isCreating && selectedBeacon?.links.length === 1 && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => safeOpenUrl(selectedBeacon.links[0].url)}
                    >
                      <ExternalLink className="h-4 w-4 mr-1" />
                      Open
                    </Button>
                  )}
                  {!isEditing && !isCreating && (
                    <Button variant="outline" size="sm" onClick={handleStartEditing}>
                      Edit
                    </Button>
                  )}
                  {isEditing && (
                    <>
                      <Button variant="ghost" size="sm" onClick={handleCancel} disabled={isSaving}>
                        <X className="h-4 w-4 mr-1" />
                        Cancel
                      </Button>
                      <Button size="sm" onClick={handleSave} disabled={isSaving}>
                        {isSaving ? (
                          <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                        ) : (
                          <Save className="h-4 w-4 mr-1" />
                        )}
                        Save
                      </Button>
                    </>
                  )}
                </div>
              </div>

              {/* Detail Content */}
              <div className="flex-1 overflow-auto p-6">
                {loadingBeacon ? (
                  <div className="h-full flex items-center justify-center">
                    <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
                  </div>
                ) : isEditing ? (
                  <div className="max-w-2xl space-y-6">
                    <div className="space-y-2">
                      <label className="text-sm font-medium">Title</label>
                      <Input
                        value={editTitle}
                        onChange={(e) => setEditTitle(e.target.value)}
                        placeholder="Beacon title..."
                        autoFocus={isCreating}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Category</label>
                        {isEditingCategories ? (
                          <div className="flex gap-2">
                            <Input
                              value={newCategoryName}
                              onChange={(e) => setNewCategoryName(e.target.value)}
                              placeholder="New category name..."
                              className="flex-1"
                              autoFocus
                              onKeyDown={(e) => {
                                if (e.key === 'Enter') {
                                  e.preventDefault();
                                  handleAddCategory();
                                }
                              }}
                            />
                            <Button type="button" size="sm" onClick={handleAddCategory}>
                              Add
                            </Button>
                            <Button
                              type="button"
                              variant="ghost"
                              size="sm"
                              onClick={() => setIsEditingCategories(false)}
                            >
                              <X className="h-4 w-4" />
                            </Button>
                          </div>
                        ) : (
                          <div className="flex gap-2">
                            <Select
                              value={editCategory || NO_CATEGORY}
                              onValueChange={(v) => setEditCategory(v === NO_CATEGORY ? '' : v)}
                            >
                              <SelectTrigger className="flex-1">
                                <SelectValue placeholder="Select category..." />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value={NO_CATEGORY}>None</SelectItem>
                                {/* Show current category if it's new (not in allCategories yet) */}
                                {editCategory && !allCategories.includes(editCategory) && (
                                  <SelectItem value={editCategory}>{editCategory} (new)</SelectItem>
                                )}
                                {allCategories.map((cat) => (
                                  <SelectItem key={cat} value={cat}>
                                    {cat}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => setIsEditingCategories(true)}
                            >
                              <Plus className="h-4 w-4" />
                            </Button>
                          </div>
                        )}
                      </div>

                      <div className="space-y-2">
                        <label className="text-sm font-medium">Type</label>
                        <Select
                          value={editType}
                          onValueChange={(value) => setEditType(value as BeaconType)}
                        >
                          <SelectTrigger>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {BEACON_TYPE_OPTIONS.map((option) => (
                              <SelectItem key={option.value} value={option.value}>
                                <div className="flex items-center gap-2">
                                  {option.icon}
                                  {option.label}
                                </div>
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">Links</label>
                        <Button variant="ghost" size="sm" onClick={handleAddLink}>
                          <Plus className="h-3 w-3 mr-1" />
                          Add Link
                        </Button>
                      </div>
                      <div className="space-y-2">
                        {editLinks.map((link, index) => (
                          <div key={`link-${index}-${link.url}`} className="flex gap-2">
                            <Input
                              value={link.label}
                              onChange={(e) => handleLinkChange(index, 'label', e.target.value)}
                              placeholder="Label (optional)"
                              className="w-1/3"
                            />
                            <Input
                              value={link.url}
                              onChange={(e) => handleLinkChange(index, 'url', e.target.value)}
                              placeholder="https://..."
                              type="url"
                              className="flex-1"
                            />
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleRemoveLink(index)}
                              className="shrink-0 h-9 w-9 p-0 text-muted-foreground hover:text-destructive"
                            >
                              <X className="h-4 w-4" />
                            </Button>
                          </div>
                        ))}
                      </div>
                    </div>

                    <div className="space-y-2">
                      <label className="text-sm font-medium">Description</label>
                      <Textarea
                        value={editDescription}
                        onChange={(e) => setEditDescription(e.target.value)}
                        placeholder="Notes about this resource..."
                        rows={4}
                      />
                    </div>

                    <div className="space-y-2">
                      <label className="text-sm font-medium flex items-center gap-2">
                        <Tag className="h-4 w-4" />
                        Tags
                      </label>
                      <Input
                        value={editTags}
                        onChange={(e) => setEditTags(e.target.value)}
                        placeholder="react, frontend, library..."
                      />
                      <p className="text-xs text-muted-foreground">Comma-separated tags</p>
                    </div>
                  </div>
                ) : (
                  <div className="max-w-2xl space-y-6">
                    {selectedBeacon && selectedBeacon.links.length > 0 && (
                      <div className="space-y-2">
                        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                          Links
                        </label>
                        <div className="space-y-2">
                          {selectedBeacon.links.map((link, index) => (
                            <div key={`${link.url}-${index}`} className="flex items-center gap-2">
                              <a
                                href="#"
                                onClick={(e) => {
                                  e.preventDefault();
                                  safeOpenUrl(link.url);
                                }}
                                className="text-primary hover:underline flex items-center gap-1 break-all"
                              >
                                {link.label || link.url}
                                <ExternalLink className="h-3 w-3 shrink-0" />
                              </a>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    <div className="grid grid-cols-2 gap-6">
                      {selectedBeacon?.category && (
                        <div className="space-y-1">
                          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                            Category
                          </label>
                          <div className="flex items-center gap-2">
                            <FolderOpen className="h-4 w-4 text-muted-foreground" />
                            <span>{selectedBeacon.category}</span>
                          </div>
                        </div>
                      )}

                      <div className="space-y-1">
                        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                          Type
                        </label>
                        <div className="flex items-center gap-2">
                          {getBeaconIcon(selectedBeacon!.beacon_type)}
                          <span className="capitalize">{selectedBeacon!.beacon_type}</span>
                        </div>
                      </div>
                    </div>

                    {selectedBeacon?.description && (
                      <div className="space-y-1">
                        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                          Description
                        </label>
                        <p className="text-sm whitespace-pre-wrap">{selectedBeacon.description}</p>
                      </div>
                    )}

                    {selectedBeacon && selectedBeacon.tags.length > 0 && (
                      <div className="space-y-2">
                        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                          Tags
                        </label>
                        <div className="flex gap-2 flex-wrap">
                          {selectedBeacon.tags.map((tag, idx) => (
                            <Badge
                              key={`${tag}-${idx}`}
                              variant="secondary"
                              className="cursor-pointer hover:bg-muted"
                              onClick={() => setSearchQuery(tag)}
                            >
                              {tag}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}

                    <div className="space-y-1 pt-4 border-t border-border">
                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <span>Updated {formatDate(selectedBeacon!.updated_at)}</span>
                        <span>Created {formatDate(selectedBeacon!.created_at)}</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <Link2 className="h-16 w-16 mx-auto mb-4 opacity-30" />
                <p className="text-sm">Select a beacon to view</p>
                <p className="text-xs mt-1">or create a new one</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={!!beaconToDelete}
        onOpenChange={(open) => !open && setBeaconToDelete(null)}
        title="Delete Beacon"
        description={`Are you sure you want to delete "${beaconToDelete?.title}"? This action cannot be undone.`}
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={() => beaconToDelete && deleteMutation.mutate(beaconToDelete.id)}
        loading={deleteMutation.isPending}
      />
    </div>
  );
}
