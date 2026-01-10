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
  list_profiles: [],
  get_profile: null,
  create_profile: { id: 'test-profile-id' },
  apply_profile: { success: true },
  rollback_profile: { success: true },
};

// Helper to set up mock responses
export function setupTauriMock(responses?: Record<string, unknown>) {
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
