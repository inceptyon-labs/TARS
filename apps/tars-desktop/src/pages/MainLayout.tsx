import { Outlet, NavLink } from 'react-router-dom';
import { FolderGit2, Layers, Cpu, Sun, Moon, Monitor, Server, Plug, Bot, Terminal, Webhook, BookOpen, FileText } from 'lucide-react';
import { cn } from '../lib/utils';
import { useUIStore, type Theme } from '../stores/ui-store';
import tarsHero from '../assets/tars-hero.png';

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

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* TARS Sidebar - Metallic Panel */}
      <aside className="tars-sidebar w-64 shrink-0 border-r border-[var(--sidebar-border)] flex flex-col relative">
        {/* TARS Watermark - fills sidebar */}
        <img
          src={tarsHero}
          alt=""
          className="absolute inset-0 w-full h-full object-cover opacity-[0.04] pointer-events-none"
          style={{ filter: 'grayscale(100%)' }}
        />
        {/* Logo Section */}
        <div className="p-6 border-b border-border">
          <div className="flex flex-col items-center">
            {/* TARS Logo - Prominent */}
            <div className="relative mb-4">
              <img
                src={tarsHero}
                alt="TARS"
                className="w-24 h-24 rounded-lg shadow-lg"
              />
              {/* Ambient glow behind logo */}
              <div className="absolute inset-0 -z-10 blur-xl opacity-30 bg-primary rounded-full scale-75" />
            </div>

            {/* Title with metallic effect */}
            <h1 className="text-2xl font-bold tracking-[0.2em] text-metallic">
              TARS
            </h1>
            <p className="text-xs text-muted-foreground mt-1 tracking-wide">
              CONFIG MANAGER
            </p>

            {/* Status indicator */}
            <div className="flex items-center gap-2 mt-3">
              <div className="tars-indicator" />
              <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                Online
              </span>
            </div>
          </div>
        </div>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Navigation */}
        <nav className="flex-1 p-4">
          <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-3 px-3">
            Modules
          </p>
          <div className="space-y-1">
            {navigation.map((item) => (
              <NavLink
                key={item.name}
                to={item.href}
                className={({ isActive }) =>
                  cn(
                    'tars-nav-item flex items-center gap-3 px-3 py-2.5 text-sm rounded transition-all',
                    isActive
                      ? 'active text-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground'
                  )
                }
              >
                <item.icon className="h-4 w-4 shrink-0" />
                <span>{item.name}</span>
              </NavLink>
            ))}
          </div>
        </nav>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* Knowledge Section */}
        <div className="p-4">
          <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-3 px-3">
            Knowledge
          </p>
          <div className="space-y-1">
            <NavLink
              to="/case"
              className={({ isActive }) =>
                cn(
                  'tars-nav-item flex items-center gap-3 px-3 py-2.5 text-sm rounded transition-all',
                  isActive
                    ? 'active text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                )
              }
            >
              <BookOpen className="h-4 w-4 shrink-0" />
              <span>CASE</span>
            </NavLink>
            <NavLink
              to="/prompts"
              className={({ isActive }) =>
                cn(
                  'tars-nav-item flex items-center gap-3 px-3 py-2.5 text-sm rounded transition-all',
                  isActive
                    ? 'active text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                )
              }
            >
              <FileText className="h-4 w-4 shrink-0" />
              <span>Prompts</span>
            </NavLink>
          </div>
        </div>

        {/* Segment line */}
        <div className="tars-segment-line" />

        {/* System Controls */}
        <div className="p-4">
          <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-3 px-3">
            System
          </p>

          {/* Theme Switcher */}
          <div className="tars-panel rounded-lg p-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-muted-foreground">Theme</span>
              <div className="flex items-center gap-1 p-1 rounded bg-background/50">
                {(['system', 'light', 'dark'] as Theme[]).map((t) => (
                  <button
                    key={t}
                    onClick={() => setTheme(t)}
                    className={cn(
                      'p-1.5 rounded transition-all',
                      theme === t
                        ? 'bg-primary text-primary-foreground tars-glow-subtle'
                        : 'text-muted-foreground hover:text-foreground hover:bg-muted'
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
