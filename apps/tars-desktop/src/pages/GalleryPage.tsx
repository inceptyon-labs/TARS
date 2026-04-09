import { useState, useMemo } from 'react';
import { useQuery, useQueries } from '@tanstack/react-query';
import { LayoutGrid, Search, FolderPlus } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { AppCard } from '@/components/AppCard';
import { listProjects, getProjectMetadata, getProjectCategories, getProjectIcon } from '@/lib/ipc';
import { cn } from '@/lib/utils';

const CATEGORY_FILTERS = ['All', 'Apps', 'Websites', 'Tools'] as const;
type CategoryFilter = (typeof CATEGORY_FILTERS)[number];

export function GalleryPage() {
  const [search, setSearch] = useState('');
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>('All');

  const { data: projects = [], isLoading: projectsLoading } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const { data: categories = {} } = useQuery({
    queryKey: ['project-categories'],
    queryFn: getProjectCategories,
    enabled: projects.length > 0,
  });

  const metadataQueries = useQueries({
    queries: projects.map((p) => ({
      queryKey: ['project-metadata', p.id],
      queryFn: () => getProjectMetadata(p.id),
      staleTime: 1000 * 60 * 5,
    })),
  });

  const iconQueries = useQueries({
    queries: projects.map((p) => ({
      queryKey: ['project-icon', p.path],
      queryFn: () => getProjectIcon(p.path),
      staleTime: 1000 * 60 * 10,
    })),
  });

  const galleryItems = useMemo(() => {
    return projects.map((project, i) => ({
      project,
      metadata: metadataQueries[i]?.data ?? null,
      iconDataUrl: iconQueries[i]?.data ?? null,
      category: categories[project.id] ?? 'Tools',
    }));
  }, [projects, metadataQueries, iconQueries, categories]);

  const filtered = useMemo(() => {
    return galleryItems.filter((item) => {
      const matchesSearch =
        !search ||
        item.project.name.toLowerCase().includes(search.toLowerCase()) ||
        item.metadata?.description?.toLowerCase().includes(search.toLowerCase());

      const matchesCategory = categoryFilter === 'All' || item.category === categoryFilter;

      return matchesSearch && matchesCategory;
    });
  }, [galleryItems, search, categoryFilter]);

  const isLoading = projectsLoading;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <div className="tars-indicator" />
          <LayoutGrid className="h-4 w-4 text-muted-foreground" />
          <h1 className="text-sm font-semibold tracking-wide uppercase">Gallery</h1>
          {!isLoading && (
            <span className="text-xs text-muted-foreground ml-1">
              {filtered.length} {filtered.length === 1 ? 'app' : 'apps'}
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {/* Category pills */}
          <div className="flex items-center gap-1">
            {CATEGORY_FILTERS.map((cat) => (
              <button
                key={cat}
                type="button"
                onClick={() => setCategoryFilter(cat)}
                className={cn(
                  'px-2.5 py-1 rounded-full text-xs font-medium transition-colors',
                  categoryFilter === cat
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                )}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              placeholder="Search apps..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="h-8 w-48 pl-8 text-xs"
            />
          </div>
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} className="h-52 rounded-xl border bg-card animate-pulse" />
            ))}
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center gap-3">
            <FolderPlus className="h-12 w-12 text-muted-foreground/40" />
            {projects.length === 0 ? (
              <>
                <p className="text-sm text-muted-foreground">No projects yet</p>
                <p className="text-xs text-muted-foreground/60">
                  Add projects from the Projects page to see them here
                </p>
              </>
            ) : (
              <>
                <p className="text-sm text-muted-foreground">No matching apps</p>
                <p className="text-xs text-muted-foreground/60">
                  Try a different search or category filter
                </p>
              </>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {filtered.map((item) => (
              <AppCard
                key={item.project.id}
                project={item.project}
                metadata={item.metadata}
                iconDataUrl={item.iconDataUrl}
                category={item.category}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default GalleryPage;
