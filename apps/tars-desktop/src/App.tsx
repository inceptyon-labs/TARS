import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect, lazy, Suspense } from 'react';
import { Toaster } from 'sonner';
import { MainLayout } from './pages/MainLayout';
import { useUIStore } from './stores/ui-store';
import './App.css';

// Eagerly load frequently accessed pages
import { ProjectsPage } from './pages/ProjectsPage';
import { ProfilesPage } from './pages/ProfilesPage';

// Lazy load larger/less frequently accessed pages for better initial load
const SkillsPage = lazy(() =>
  import('./pages/SkillsPage').then((m) => ({ default: m.SkillsPage }))
);
const AgentsPage = lazy(() =>
  import('./pages/AgentsPage').then((m) => ({ default: m.AgentsPage }))
);
const CommandsPage = lazy(() =>
  import('./pages/CommandsPage').then((m) => ({ default: m.CommandsPage }))
);
const HooksPage = lazy(() => import('./pages/HooksPage').then((m) => ({ default: m.HooksPage })));
const McpPage = lazy(() => import('./pages/McpPage').then((m) => ({ default: m.McpPage })));
const PluginsPage = lazy(() =>
  import('./pages/PluginsPage').then((m) => ({ default: m.PluginsPage }))
);
const CasePage = lazy(() => import('./pages/CasePage').then((m) => ({ default: m.CasePage })));
const PromptsPage = lazy(() =>
  import('./pages/PromptsPage').then((m) => ({ default: m.PromptsPage }))
);
const BeaconPage = lazy(() =>
  import('./pages/BeaconPage').then((m) => ({ default: m.BeaconPage }))
);
const UpdatesPage = lazy(() =>
  import('./pages/UpdatesPage').then((m) => ({ default: m.UpdatesPage }))
);
const SettingsPage = lazy(() =>
  import('./pages/SettingsPage').then((m) => ({ default: m.SettingsPage }))
);
const UsagePage = lazy(() => import('./pages/UsagePage').then((m) => ({ default: m.UsagePage })));
const ClaudeSettingsPage = lazy(() =>
  import('./pages/ClaudeSettingsPage').then((m) => ({ default: m.ClaudeSettingsPage }))
);

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60, // 1 minute
      retry: 1,
    },
  },
});

function ThemeProvider({ children }: { children: React.ReactNode }) {
  const theme = useUIStore((state) => state.theme);

  useEffect(() => {
    const root = window.document.documentElement;

    const applyTheme = () => {
      root.classList.remove('light', 'dark');

      if (theme === 'system') {
        const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'light';
        root.classList.add(systemTheme);
      } else {
        root.classList.add(theme);
      }
    };

    // Apply theme immediately
    applyTheme();

    // Listen for system theme changes when in 'system' mode
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleSystemChange = () => {
      if (theme === 'system') {
        applyTheme();
      }
    };

    mediaQuery.addEventListener('change', handleSystemChange);
    return () => mediaQuery.removeEventListener('change', handleSystemChange);
  }, [theme]);

  return <>{children}</>;
}

// Loading fallback for lazy-loaded pages
function PageLoader() {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="animate-pulse text-muted-foreground">Loading...</div>
    </div>
  );
}

function App() {
  const theme = useUIStore((state) => state.theme);

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<MainLayout />}>
              <Route index element={<Navigate to="/projects" replace />} />
              <Route path="projects" element={<ProjectsPage />} />
              <Route path="profiles" element={<ProfilesPage />} />
              <Route
                path="skills"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <SkillsPage />
                  </Suspense>
                }
              />
              <Route
                path="agents"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <AgentsPage />
                  </Suspense>
                }
              />
              <Route
                path="commands"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <CommandsPage />
                  </Suspense>
                }
              />
              <Route
                path="hooks"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <HooksPage />
                  </Suspense>
                }
              />
              <Route
                path="mcp"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <McpPage />
                  </Suspense>
                }
              />
              <Route
                path="plugins"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <PluginsPage />
                  </Suspense>
                }
              />
              <Route
                path="case"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <CasePage />
                  </Suspense>
                }
              />
              <Route
                path="case/:section"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <CasePage />
                  </Suspense>
                }
              />
              <Route
                path="prompts"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <PromptsPage />
                  </Suspense>
                }
              />
              <Route
                path="beacon"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <BeaconPage />
                  </Suspense>
                }
              />
              <Route
                path="updates"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <UpdatesPage />
                  </Suspense>
                }
              />
              <Route
                path="settings"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <SettingsPage />
                  </Suspense>
                }
              />
              <Route
                path="claude-settings"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <ClaudeSettingsPage />
                  </Suspense>
                }
              />
              <Route
                path="usage"
                element={
                  <Suspense fallback={<PageLoader />}>
                    <UsagePage />
                  </Suspense>
                }
              />
            </Route>
          </Routes>
        </BrowserRouter>
        <Toaster theme={theme === 'system' ? 'system' : theme} position="bottom-right" richColors />
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
