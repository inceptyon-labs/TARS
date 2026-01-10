import { describe, it, expect, beforeEach } from 'vitest';
import { useUIStore } from './ui-store';

describe('useUIStore', () => {
  // Reset store state before each test
  beforeEach(() => {
    useUIStore.setState({
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
    });
  });

  describe('theme', () => {
    it('defaults to dark theme', () => {
      const { theme } = useUIStore.getState();
      expect(theme).toBe('dark');
    });

    it('can set theme to light', () => {
      useUIStore.getState().setTheme('light');
      expect(useUIStore.getState().theme).toBe('light');
    });

    it('can set theme to system', () => {
      useUIStore.getState().setTheme('system');
      expect(useUIStore.getState().theme).toBe('system');
    });
  });

  describe('selectedProjectId', () => {
    it('defaults to null', () => {
      expect(useUIStore.getState().selectedProjectId).toBeNull();
    });

    it('can be set to a project ID', () => {
      useUIStore.getState().setSelectedProjectId('project-123');
      expect(useUIStore.getState().selectedProjectId).toBe('project-123');
    });

    it('can be cleared by setting to null', () => {
      useUIStore.getState().setSelectedProjectId('project-123');
      useUIStore.getState().setSelectedProjectId(null);
      expect(useUIStore.getState().selectedProjectId).toBeNull();
    });
  });

  describe('selectedProjectPath', () => {
    it('defaults to null', () => {
      expect(useUIStore.getState().selectedProjectPath).toBeNull();
    });

    it('can be set to a path', () => {
      useUIStore.getState().setSelectedProjectPath('/path/to/project');
      expect(useUIStore.getState().selectedProjectPath).toBe('/path/to/project');
    });
  });

  describe('selectedProfileId', () => {
    it('defaults to null', () => {
      expect(useUIStore.getState().selectedProfileId).toBeNull();
    });

    it('can be set to a profile ID', () => {
      useUIStore.getState().setSelectedProfileId('profile-456');
      expect(useUIStore.getState().selectedProfileId).toBe('profile-456');
    });
  });

  describe('dialog states', () => {
    it('isAddProjectDialogOpen defaults to false', () => {
      expect(useUIStore.getState().isAddProjectDialogOpen).toBe(false);
    });

    it('can open and close add project dialog', () => {
      useUIStore.getState().setAddProjectDialogOpen(true);
      expect(useUIStore.getState().isAddProjectDialogOpen).toBe(true);

      useUIStore.getState().setAddProjectDialogOpen(false);
      expect(useUIStore.getState().isAddProjectDialogOpen).toBe(false);
    });

    it('can open and close create profile dialog', () => {
      useUIStore.getState().setCreateProfileDialogOpen(true);
      expect(useUIStore.getState().isCreateProfileDialogOpen).toBe(true);

      useUIStore.getState().setCreateProfileDialogOpen(false);
      expect(useUIStore.getState().isCreateProfileDialogOpen).toBe(false);
    });

    it('can open and close apply dialog', () => {
      useUIStore.getState().setApplyDialogOpen(true);
      expect(useUIStore.getState().isApplyDialogOpen).toBe(true);

      useUIStore.getState().setApplyDialogOpen(false);
      expect(useUIStore.getState().isApplyDialogOpen).toBe(false);
    });

    it('can open and close export dialog', () => {
      useUIStore.getState().setExportDialogOpen(true);
      expect(useUIStore.getState().isExportDialogOpen).toBe(true);

      useUIStore.getState().setExportDialogOpen(false);
      expect(useUIStore.getState().isExportDialogOpen).toBe(false);
    });
  });

  describe('currentView', () => {
    it('defaults to projects', () => {
      expect(useUIStore.getState().currentView).toBe('projects');
    });

    it('can switch to profiles view', () => {
      useUIStore.getState().setCurrentView('profiles');
      expect(useUIStore.getState().currentView).toBe('profiles');
    });

    it('can switch to skills view', () => {
      useUIStore.getState().setCurrentView('skills');
      expect(useUIStore.getState().currentView).toBe('skills');
    });
  });

  describe('sidebar', () => {
    it('sidebarCollapsed defaults to false', () => {
      expect(useUIStore.getState().sidebarCollapsed).toBe(false);
    });

    it('can toggle sidebar collapsed state', () => {
      useUIStore.getState().setSidebarCollapsed(true);
      expect(useUIStore.getState().sidebarCollapsed).toBe(true);

      useUIStore.getState().setSidebarCollapsed(false);
      expect(useUIStore.getState().sidebarCollapsed).toBe(false);
    });

    it('sidebarWidth defaults to 256', () => {
      expect(useUIStore.getState().sidebarWidth).toBe(256);
    });

    it('can set sidebar width', () => {
      useUIStore.getState().setSidebarWidth(320);
      expect(useUIStore.getState().sidebarWidth).toBe(320);
    });
  });

  describe('state isolation', () => {
    it('changing one property does not affect others', () => {
      useUIStore.getState().setSelectedProjectId('project-123');
      useUIStore.getState().setTheme('light');

      const state = useUIStore.getState();
      expect(state.selectedProjectId).toBe('project-123');
      expect(state.theme).toBe('light');
      expect(state.currentView).toBe('projects'); // unchanged
      expect(state.sidebarWidth).toBe(256); // unchanged
    });
  });
});
