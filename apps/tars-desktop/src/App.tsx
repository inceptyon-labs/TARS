import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect } from 'react';
import { Toaster } from 'sonner';
import { MainLayout } from './pages/MainLayout';
import { ProjectsPage } from './pages/ProjectsPage';
import { ProfilesPage } from './pages/ProfilesPage';
import { SkillsPage } from './pages/SkillsPage';
import { AgentsPage } from './pages/AgentsPage';
import { CommandsPage } from './pages/CommandsPage';
import { HooksPage } from './pages/HooksPage';
import { McpPage } from './pages/McpPage';
import { PluginsPage } from './pages/PluginsPage';
import { useUIStore } from './stores/ui-store';
import './App.css';

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
        const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
          .matches
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
              <Route path="skills" element={<SkillsPage />} />
              <Route path="agents" element={<AgentsPage />} />
              <Route path="commands" element={<CommandsPage />} />
              <Route path="hooks" element={<HooksPage />} />
              <Route path="mcp" element={<McpPage />} />
              <Route path="plugins" element={<PluginsPage />} />
            </Route>
          </Routes>
        </BrowserRouter>
        <Toaster
          theme={theme === 'system' ? 'system' : theme}
          position="bottom-right"
          richColors
        />
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
