import { useState, useCallback, useRef, useEffect } from 'react';
import { Outlet, NavLink } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import {
  FolderGit2,
  Layers,
  Cpu,
  Sun,
  Moon,
  Monitor,
  Server,
  Plug,
  Bot,
  Terminal,
  Webhook,
  BookOpen,
  FileText,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  Download,
  GripVertical,
} from 'lucide-react';
import { cn } from '../lib/utils';
import { useUIStore, type Theme } from '../stores/ui-store';
import { getClaudeVersionInfo, checkPluginUpdates, getTarsVersion } from '../lib/ipc';

// Poll interval for update checks: 10 minutes
const UPDATE_POLL_INTERVAL = 10 * 60 * 1000;

const navigation = [
  { name: 'Projects', href: '/projects', icon: FolderGit2 },
  { name: 'Profiles', href: '/profiles', icon: Layers },
  { name: 'Skills', href: '/skills', icon: Cpu },
  { name: 'Agents', href: '/agents', icon: Bot },
  { name: 'Commands', href: '/commands', icon: Terminal },
  { name: 'Hooks', href: '/hooks', icon: Webhook },
  { name: 'MCP Servers', href: '/mcp', icon: Server },
  { name: 'Plugins', href: '/plugins', icon: Plug },
];

// Sidebar constraints
const SIDEBAR_MIN_WIDTH = 200;
const SIDEBAR_MAX_WIDTH = 400;
const SIDEBAR_COLLAPSED_WIDTH = 80;

export function MainLayout() {
  const theme = useUIStore((state) => state.theme);
  const setTheme = useUIStore((state) => state.setTheme);
  const sidebarCollapsed = useUIStore((state) => state.sidebarCollapsed);
  const setSidebarCollapsed = useUIStore((state) => state.setSidebarCollapsed);
  const sidebarWidth = useUIStore((state) => state.sidebarWidth);
  const setSidebarWidth = useUIStore((state) => state.setSidebarWidth);

  // Collapsible section states
  const [modulesExpanded, setModulesExpanded] = useState(true);
  const [knowledgeExpanded, setKnowledgeExpanded] = useState(true);
  const [systemExpanded, setSystemExpanded] = useState(true);

  // Resize state
  const [isResizing, setIsResizing] = useState(false);
  const sidebarRef = useRef<HTMLElement>(null);

  // Handle resize drag
  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  const stopResizing = useCallback(() => {
    setIsResizing(false);
  }, []);

  const resize = useCallback(
    (e: MouseEvent) => {
      if (!isResizing) return;

      const newWidth = e.clientX;

      // If dragged below minimum, collapse the sidebar
      if (newWidth < SIDEBAR_MIN_WIDTH - 50) {
        setSidebarCollapsed(true);
        return;
      }

      // If collapsed and dragged to expand
      if (sidebarCollapsed && newWidth > SIDEBAR_COLLAPSED_WIDTH + 20) {
        setSidebarCollapsed(false);
        setSidebarWidth(Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, newWidth)));
        return;
      }

      if (!sidebarCollapsed) {
        setSidebarWidth(Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, newWidth)));
      }
    },
    [isResizing, sidebarCollapsed, setSidebarCollapsed, setSidebarWidth]
  );

  // Attach/detach mouse event listeners
  useEffect(() => {
    if (isResizing) {
      window.addEventListener('mousemove', resize);
      window.addEventListener('mouseup', stopResizing);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      window.removeEventListener('mousemove', resize);
      window.removeEventListener('mouseup', stopResizing);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing, resize, stopResizing]);

  // Check for updates silently
  const { data: versionInfo } = useQuery({
    queryKey: ['claude-version-info'],
    queryFn: getClaudeVersionInfo,
    refetchInterval: UPDATE_POLL_INTERVAL,
    staleTime: UPDATE_POLL_INTERVAL - 60000,
  });

  const { data: pluginUpdates } = useQuery({
    queryKey: ['plugin-updates'],
    queryFn: checkPluginUpdates,
    refetchInterval: UPDATE_POLL_INTERVAL,
    staleTime: UPDATE_POLL_INTERVAL - 60000,
  });

  const { data: appVersion } = useQuery({
    queryKey: ['tars-version'],
    queryFn: getTarsVersion,
    staleTime: Infinity, // Version doesn't change during runtime
  });

  const claudeUpdateAvailable = versionInfo?.update_available ?? false;
  const pluginsWithUpdates = pluginUpdates?.plugins_with_updates ?? 0;
  const totalUpdates = (claudeUpdateAvailable ? 1 : 0) + pluginsWithUpdates;
  const hasUpdates = totalUpdates > 0;

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* TARS Sidebar - Metallic Panel */}
      <aside
        ref={sidebarRef}
        style={{ width: sidebarCollapsed ? SIDEBAR_COLLAPSED_WIDTH : sidebarWidth }}
        className={cn(
          'tars-sidebar tars-sidebar-scroll shrink-0 border-r border-[var(--sidebar-border)] flex flex-col relative',
          !isResizing && 'transition-all duration-200'
        )}
      >
        {/* Logo Section */}
        <div className={cn('p-4 border-b border-border relative', sidebarCollapsed && 'px-2 py-3')}>
          <button
            type="button"
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            className={cn(
              'tars-sidebar-toggle absolute z-10',
              sidebarCollapsed ? 'right-1/2 translate-x-1/2 top-2' : 'right-2 top-2'
            )}
            title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {sidebarCollapsed ? (
              <ChevronRight className="h-4 w-4" />
            ) : (
              <ChevronLeft className="h-4 w-4" />
            )}
          </button>

          <div className={cn('flex flex-col items-center', sidebarCollapsed && 'mt-8')}>
            {/* TARS Mark - Robot design: 2 legs + body with braille dots */}
            <div
              className={cn('tars-mark-sm mb-2', sidebarCollapsed && 'tars-mark-collapsed')}
              aria-hidden="true"
            >
              <span />
              <span />
              <span />
            </div>

            {/* TARS Wordmark - Interstellar-inspired */}
            {!sidebarCollapsed && (
              <div className="tars-wordmark-sm mb-2" aria-hidden="true">
                <div className="tars-wordmark-text-sm">TARS</div>
              </div>
            )}

            {/* Title with metallic effect */}
            {!sidebarCollapsed && (
              <p className="text-[9px] text-muted-foreground tracking-[0.2em] uppercase text-center leading-tight">
                <span className="font-semibold">T</span>ooling,{' '}
                <span className="font-semibold">A</span>gents,{' '}
                <span className="font-semibold">R</span>oles,{' '}
                <span className="font-semibold">S</span>kills
              </p>
            )}
          </div>
        </div>

        {/* Scrollable sections container */}
        <div className="flex-1 overflow-y-auto tars-sidebar-scroll">
          {/* Segment line */}
          <div className="tars-segment-line" />

          {/* Navigation */}
          <nav className="p-4">
            {!sidebarCollapsed && (
              <button
                type="button"
                onClick={() => setModulesExpanded(!modulesExpanded)}
                className="tars-section-header w-full flex items-center justify-between mb-2 px-3 py-1 rounded hover:bg-muted/30 transition-colors"
              >
                <span className="tars-section-title">Modules</span>
                <ChevronDown
                  className={cn(
                    'h-3 w-3 text-muted-foreground transition-transform',
                    !modulesExpanded && '-rotate-90'
                  )}
                />
              </button>
            )}
            {(modulesExpanded || sidebarCollapsed) && (
              <div className="space-y-1">
                {navigation.map((item) => (
                  <NavLink
                    key={item.name}
                    to={item.href}
                    title={item.name}
                    className={({ isActive }) =>
                      cn(
                        'tars-nav-item flex items-center text-sm rounded transition-all',
                        sidebarCollapsed ? 'justify-center px-2 py-2.5' : 'gap-3 px-3 py-2.5',
                        isActive
                          ? 'active text-foreground font-medium'
                          : 'text-muted-foreground hover:text-foreground'
                      )
                    }
                  >
                    <item.icon className="h-4 w-4 shrink-0" />
                    {!sidebarCollapsed && <span>{item.name}</span>}
                  </NavLink>
                ))}
              </div>
            )}
          </nav>

          {/* Segment line */}
          <div className="tars-segment-line" />

          {/* Knowledge Section */}
          <div className="p-4">
            {!sidebarCollapsed && (
              <button
                type="button"
                onClick={() => setKnowledgeExpanded(!knowledgeExpanded)}
                className="tars-section-header w-full flex items-center justify-between mb-2 px-3 py-1 rounded hover:bg-muted/30 transition-colors"
              >
                <span className="tars-section-title">Knowledge</span>
                <ChevronDown
                  className={cn(
                    'h-3 w-3 text-muted-foreground transition-transform',
                    !knowledgeExpanded && '-rotate-90'
                  )}
                />
              </button>
            )}
            {(knowledgeExpanded || sidebarCollapsed) && (
              <div className="space-y-1">
                <NavLink
                  to="/case"
                  title="CASE"
                  className={({ isActive }) =>
                    cn(
                      'tars-nav-item flex items-center text-sm rounded transition-all',
                      sidebarCollapsed ? 'justify-center px-2 py-2.5' : 'gap-3 px-3 py-2.5',
                      isActive
                        ? 'active text-foreground font-medium'
                        : 'text-muted-foreground hover:text-foreground'
                    )
                  }
                >
                  <BookOpen className="h-4 w-4 shrink-0" />
                  {!sidebarCollapsed && <span>CASE</span>}
                </NavLink>
                <NavLink
                  to="/prompts"
                  title="Prompts"
                  className={({ isActive }) =>
                    cn(
                      'tars-nav-item flex items-center text-sm rounded transition-all',
                      sidebarCollapsed ? 'justify-center px-2 py-2.5' : 'gap-3 px-3 py-2.5',
                      isActive
                        ? 'active text-foreground font-medium'
                        : 'text-muted-foreground hover:text-foreground'
                    )
                  }
                >
                  <FileText className="h-4 w-4 shrink-0" />
                  {!sidebarCollapsed && <span>Prompts</span>}
                </NavLink>
              </div>
            )}
          </div>

          {/* Segment line */}
          <div className="tars-segment-line" />

          {/* System Controls */}
          <div className="p-4">
            {!sidebarCollapsed && (
              <button
                type="button"
                onClick={() => setSystemExpanded(!systemExpanded)}
                className="tars-section-header w-full flex items-center justify-between mb-2 px-3 py-1 rounded hover:bg-muted/30 transition-colors"
              >
                <span className="tars-section-title">System</span>
                <ChevronDown
                  className={cn(
                    'h-3 w-3 text-muted-foreground transition-transform',
                    !systemExpanded && '-rotate-90'
                  )}
                />
              </button>
            )}

            {(systemExpanded || sidebarCollapsed) && (
              <div>
                {/* Updates Link */}
                <div className="space-y-1 mb-3">
                  <NavLink
                    to="/updates"
                    title="Updates"
                    className={({ isActive }) =>
                      cn(
                        'tars-nav-item flex items-center text-sm rounded transition-all relative',
                        sidebarCollapsed ? 'justify-center px-2 py-2.5' : 'gap-3 px-3 py-2.5',
                        isActive
                          ? 'active text-foreground font-medium'
                          : 'text-muted-foreground hover:text-foreground'
                      )
                    }
                  >
                    <div className="relative">
                      <Download className="h-4 w-4 shrink-0" />
                      {hasUpdates && (
                        <span className="absolute -top-1 -right-1 w-2 h-2 bg-primary rounded-full" />
                      )}
                    </div>
                    {!sidebarCollapsed && (
                      <>
                        <span>Updates</span>
                        {hasUpdates && (
                          <span className="ml-auto text-xs px-1.5 py-0.5 rounded-full bg-primary text-primary-foreground">
                            {totalUpdates}
                          </span>
                        )}
                      </>
                    )}
                  </NavLink>
                </div>

                {/* Theme Switcher */}
                <div
                  className={cn(
                    'flex items-center justify-between px-3',
                    sidebarCollapsed && 'flex-col gap-2 px-2'
                  )}
                >
                  {!sidebarCollapsed && (
                    <span className="text-sm text-muted-foreground">Theme</span>
                  )}
                  <div className={cn('flex items-center gap-1', sidebarCollapsed && 'flex-col')}>
                    {(['system', 'light', 'dark'] as Theme[]).map((t) => (
                      <button
                        key={t}
                        onClick={() => setTheme(t)}
                        className={cn(
                          'h-7 w-7 grid place-items-center rounded transition-all',
                          theme === t
                            ? 'bg-primary text-primary-foreground'
                            : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                        )}
                        title={t.charAt(0).toUpperCase() + t.slice(1)}
                      >
                        {t === 'system' && <Monitor className="h-3.5 w-3.5" />}
                        {t === 'light' && <Sun className="h-3.5 w-3.5" />}
                        {t === 'dark' && <Moon className="h-3.5 w-3.5" />}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Version */}
        <div className="p-4 flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground/50 font-mono">
            v{appVersion ?? '...'}
          </span>
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-green-500/80" />
            <span className="text-[10px] text-muted-foreground/50">Ready</span>
          </div>
        </div>

        {/* Resize Handle */}
        <div
          className={cn(
            'absolute top-0 right-0 w-1 h-full cursor-col-resize group z-20',
            'hover:bg-primary/30 active:bg-primary/50',
            isResizing && 'bg-primary/50'
          )}
          onMouseDown={startResizing}
        >
          {/* Visual grip indicator */}
          <div className="absolute top-1/2 -translate-y-1/2 right-0 w-3 h-8 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <GripVertical className="h-4 w-4 text-muted-foreground" />
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 overflow-auto bg-background">
        <Outlet />
      </main>
    </div>
  );
}
