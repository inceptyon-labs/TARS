/**
 * Zustand UI store for application state
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark' | 'system';

interface UIState {
  // Theme
  theme: Theme;
  setTheme: (theme: Theme) => void;

  // Selected items
  selectedProjectId: string | null;
  setSelectedProjectId: (id: string | null) => void;

  selectedProjectPath: string | null;
  setSelectedProjectPath: (path: string | null) => void;

  selectedProfileId: string | null;
  setSelectedProfileId: (id: string | null) => void;

  // Dialog states
  isAddProjectDialogOpen: boolean;
  setAddProjectDialogOpen: (open: boolean) => void;

  isCreateProfileDialogOpen: boolean;
  setCreateProfileDialogOpen: (open: boolean) => void;

  isApplyDialogOpen: boolean;
  setApplyDialogOpen: (open: boolean) => void;

  isExportDialogOpen: boolean;
  setExportDialogOpen: (open: boolean) => void;

  // Current view
  currentView: 'projects' | 'profiles' | 'skills';
  setCurrentView: (view: 'projects' | 'profiles' | 'skills') => void;

  // Sidebar collapsed state
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (collapsed: boolean) => void;

  // Sidebar width (for resizing)
  sidebarWidth: number;
  setSidebarWidth: (width: number) => void;

  // Development folder for discovering Claude projects
  developmentFolder: string | null;
  setDevelopmentFolder: (folder: string | null) => void;

  // Reset to defaults
  reset: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      // Theme - default to dark for TARS aesthetic
      theme: 'dark',
      setTheme: (theme) => set({ theme }),

      // Selected items
      selectedProjectId: null,
      setSelectedProjectId: (id) => set({ selectedProjectId: id }),

      selectedProjectPath: null,
      setSelectedProjectPath: (path) => set({ selectedProjectPath: path }),

      selectedProfileId: null,
      setSelectedProfileId: (id) => set({ selectedProfileId: id }),

      // Dialog states
      isAddProjectDialogOpen: false,
      setAddProjectDialogOpen: (open) => set({ isAddProjectDialogOpen: open }),

      isCreateProfileDialogOpen: false,
      setCreateProfileDialogOpen: (open) => set({ isCreateProfileDialogOpen: open }),

      isApplyDialogOpen: false,
      setApplyDialogOpen: (open) => set({ isApplyDialogOpen: open }),

      isExportDialogOpen: false,
      setExportDialogOpen: (open) => set({ isExportDialogOpen: open }),

      // Current view
      currentView: 'projects',
      setCurrentView: (view) => set({ currentView: view }),

      // Sidebar
      sidebarCollapsed: false,
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

      // Sidebar width - default 256px (w-64)
      sidebarWidth: 256,
      setSidebarWidth: (width) => set({ sidebarWidth: width }),

      // Development folder - null means not set
      developmentFolder: null,
      setDevelopmentFolder: (folder) => set({ developmentFolder: folder }),

      // Reset to defaults
      reset: () =>
        set({
          theme: 'dark',
          selectedProjectId: null,
          selectedProjectPath: null,
          selectedProfileId: null,
          isAddProjectDialogOpen: false,
          isCreateProfileDialogOpen: false,
          isApplyDialogOpen: false,
          isExportDialogOpen: false,
          currentView: 'projects',
          sidebarCollapsed: false,
          sidebarWidth: 256,
          developmentFolder: null,
        }),
    }),
    {
      name: 'tars-ui-settings',
      // Only persist user preferences, not transient state
      partialize: (state) => ({
        theme: state.theme,
        sidebarCollapsed: state.sidebarCollapsed,
        sidebarWidth: state.sidebarWidth,
        developmentFolder: state.developmentFolder,
      }),
    }
  )
);
