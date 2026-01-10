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
  Download,
} from 'lucide-react';
import { cn } from '../lib/utils';
import { useUIStore, type Theme } from '../stores/ui-store';
import { getClaudeVersionInfo, checkPluginUpdates } from '../lib/ipc';

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

export function MainLayout() {
  const theme = useUIStore((state) => state.theme);
  const setTheme = useUIStore((state) => state.setTheme);
  const sidebarCollapsed = useUIStore((state) => state.sidebarCollapsed);
  const setSidebarCollapsed = useUIStore((state) => state.setSidebarCollapsed);

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

  const claudeUpdateAvailable = versionInfo?.update_available ?? false;
  const pluginsWithUpdates = pluginUpdates?.plugins_with_updates ?? 0;
  const totalUpdates = (claudeUpdateAvailable ? 1 : 0) + pluginsWithUpdates;
  const hasUpdates = totalUpdates > 0;

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* TARS Sidebar - Metallic Panel */}
      <aside
        className={cn(
          'tars-sidebar tars-sidebar-scroll shrink-0 border-r border-[var(--sidebar-border)] flex flex-col relative transition-all duration-200',
          sidebarCollapsed ? 'w-20' : 'w-64'
        )}
      >
        {/* Logo Section */}
        <div className={cn('p-6 border-b border-border relative', sidebarCollapsed && 'px-3 py-4')}>
          <button
            type="button"
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            className={cn(
              'tars-sidebar-toggle absolute right-3 top-3',
              sidebarCollapsed && 'right-2'
            )}
            title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {sidebarCollapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
          </button>

          <div className="flex flex-col items-center">
            {/* TARS Mark - Robot design: 2 legs + body with braille dots */}
            <div className={cn('tars-mark mb-3', sidebarCollapsed && 'tars-mark-collapsed')} aria-hidden="true">
              <span />
              <span />
              <span />
            </div>

            {/* TARS Wordmark - Interstellar-inspired */}
            <div className={cn('tars-wordmark mb-4', sidebarCollapsed && 'tars-wordmark-collapsed')} aria-hidden="true">
              <div className="tars-wordmark-text">
                TARS
              </div>
            </div>

            {/* Title with metallic effect */}
            {!sidebarCollapsed && (
              <>
                <p className="text-[10px] text-muted-foreground mt-1 tracking-[0.3em] uppercase text-center">
                  <span className="font-semibold">T</span>ooling,{' '}
                  <span className="font-semibold">A</span>gents,{' '}
                  <span className="font-semibold">R</span>oles,{' '}
                  <span className="font-semibold">S</span>kills
                </p>
              </>
            )}

          </div>
        </div>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Navigation */}
        <nav className="flex-1 p-4">
          {!sidebarCollapsed && (
            <p className="tars-section-title mb-3 px-3">
              Modules
            </p>
          )}
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
        </nav>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Knowledge Section */}
        <div className="p-4">
          {!sidebarCollapsed && (
            <p className="tars-section-title mb-3 px-3">
              Knowledge
            </p>
          )}
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
        </div>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* System Controls */}
        <div className="p-4">
          {!sidebarCollapsed && (
            <p className="tars-section-title mb-3 px-3">
              System
            </p>
          )}

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
          <div className={cn('flex items-center justify-between px-3', sidebarCollapsed && 'flex-col gap-2 px-2')}>
            {!sidebarCollapsed && <span className="text-sm text-muted-foreground">Theme</span>}
            <div
              className={cn(
                'flex items-center gap-1',
                sidebarCollapsed && 'flex-col'
              )}
            >
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

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Version */}
        <div className="p-4 flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground/50 font-mono">v0.1.0</span>
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-green-500/80" />
            <span className="text-[10px] text-muted-foreground/50">Ready</span>
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
