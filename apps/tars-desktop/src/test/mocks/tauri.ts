import { vi } from 'vitest';

// Mock IPC responses for common commands
export const mockInvokeResponses: Record<string, unknown> = {
  list_projects: [],
  get_project: null,
  scan_project: {
    skills: [],
    commands: [],
    agents: [],
    hooks: [],
    mcp_servers: [],
    plugins: [],
  },
  scan_profiles: {
    skills: [],
    commands: [],
    agents: [],
  },
  list_profiles: [],
  get_profile: null,
  create_profile: { id: 'test-profile-id' },
  delete_profile: { deleted: true, converted_projects: 0 },
  delete_profile_cleanup: {
    deleted: true,
    projects_unassigned: 0,
    local_overrides_removed: 0,
  },
  apply_profile: { success: true },
  rollback_profile: { success: true },
  export_profile_as_plugin: '/tmp/plugin.zip',
  list_profile_mcp_servers: [],
  remove_profile_mcp_server: null,
  create_profile_mcp_server: null,
};

// Helper to set up mock responses
export async function setupTauriMock(responses?: Record<string, unknown>) {
  const allResponses = { ...mockInvokeResponses, ...responses };

  const { invoke } = vi.mocked(await import('@tauri-apps/api/core'));

  invoke.mockImplementation(async (cmd: string, args?: unknown) => {
    if (cmd in allResponses) {
      const response = allResponses[cmd];
      return typeof response === 'function' ? response(args) : response;
    }
    throw new Error(`Unhandled Tauri command: ${cmd}`);
  });

  return invoke;
}

// Helper to create mock project data
export function createMockProject(overrides?: Record<string, unknown>) {
  return {
    id: 'test-project-id',
    name: 'Test Project',
    path: '/path/to/project',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

// Helper to create mock profile data
export function createMockProfile(overrides?: Record<string, unknown>) {
  return {
    id: 'test-profile-id',
    name: 'Test Profile',
    description: 'A test profile',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    items: [],
    ...overrides,
  };
}

// Helper to create mock inventory data
export function createMockInventory(overrides?: Record<string, unknown>) {
  return {
    skills: [],
    commands: [],
    agents: [],
    hooks: [],
    mcp_servers: [],
    plugins: [],
    ...overrides,
  };
}
