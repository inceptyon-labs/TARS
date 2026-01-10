import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../test/test-utils';
import userEvent from '@testing-library/user-event';
import { DiffPreview } from './DiffPreview';
import type { DiffPreview as DiffPreviewType } from '../lib/types';

// Mock Monaco Editor since it requires browser APIs
vi.mock('@monaco-editor/react', () => ({
  default: ({ defaultValue }: { defaultValue: string }) => (
    <pre data-testid="monaco-editor">{defaultValue}</pre>
  ),
  DiffEditor: ({ original, modified }: { original: string; modified: string }) => (
    <div data-testid="monaco-diff-editor">
      <pre data-testid="diff-original">{original}</pre>
      <pre data-testid="diff-modified">{modified}</pre>
    </div>
  ),
}));

const mockPreview: DiffPreviewType = {
  summary: "Apply profile 'Development' to project",
  warnings: [],
  terminal_output: '',
  operations: [
    {
      operation_type: 'create',
      path: '/project/.claude/skills/my-skill/SKILL.md',
      diff: '+# My Skill\n+\n+Description of the skill',
      size: 256,
    },
    {
      operation_type: 'modify',
      path: '/project/.claude/settings.json',
      diff: ' {\n-  "hooks": []\n+  "hooks": ["pre-commit"]\n }',
      size: 128,
    },
    {
      operation_type: 'delete',
      path: '/project/.claude/old-config.json',
      diff: null,
      size: 64,
    },
  ],
};

describe('DiffPreview', () => {
  it('renders summary', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.getByText("Apply profile 'Development' to project")).toBeInTheDocument();
  });

  it('renders operation counts', () => {
    render(<DiffPreview preview={mockPreview} />);

    // Check that operation count elements exist (each type has 1 operation)
    const countElements = screen.getAllByText('1');
    // We should have counts for create, modify, and delete
    expect(countElements.length).toBeGreaterThanOrEqual(1);
  });

  it('renders file paths for all operations', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.getByText('SKILL.md')).toBeInTheDocument();
    expect(screen.getByText('settings.json')).toBeInTheDocument();
    expect(screen.getByText('old-config.json')).toBeInTheDocument();
  });

  it('renders operation badges', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.getByText('Create')).toBeInTheDocument();
    expect(screen.getByText('Modify')).toBeInTheDocument();
    expect(screen.getByText('Delete')).toBeInTheDocument();
  });

  it('displays file sizes', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.getByText('256 B')).toBeInTheDocument();
    expect(screen.getByText('128 B')).toBeInTheDocument();
    expect(screen.getByText('64 B')).toBeInTheDocument();
  });

  it('expands operation to show diff when clicked', async () => {
    const user = userEvent.setup();
    render(<DiffPreview preview={mockPreview} />);

    // Click on the first operation (create)
    await user.click(screen.getByText('SKILL.md'));

    // Monaco DiffEditor should be rendered
    expect(screen.getByTestId('monaco-diff-editor')).toBeInTheDocument();
  });

  it('calls onCancel when cancel button is clicked', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onConfirm = vi.fn();

    render(<DiffPreview preview={mockPreview} onCancel={onCancel} onConfirm={onConfirm} />);

    await user.click(screen.getByText('Cancel'));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('calls onConfirm when apply button is clicked', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onConfirm = vi.fn();

    render(<DiffPreview preview={mockPreview} onCancel={onCancel} onConfirm={onConfirm} />);

    await user.click(screen.getByText('Apply Changes'));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('does not render action buttons when callbacks not provided', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.queryByText('Cancel')).not.toBeInTheDocument();
    expect(screen.queryByText('Apply Changes')).not.toBeInTheDocument();
  });

  it('renders warnings when present', () => {
    const previewWithWarnings: DiffPreviewType = {
      ...mockPreview,
      warnings: ['File already exists', 'Backup recommended'],
    };

    render(<DiffPreview preview={previewWithWarnings} />);

    expect(screen.getByText('Warnings')).toBeInTheDocument();
    expect(screen.getByText('File already exists')).toBeInTheDocument();
    expect(screen.getByText('Backup recommended')).toBeInTheDocument();
  });

  it('does not render warnings section when empty', () => {
    render(<DiffPreview preview={mockPreview} />);

    expect(screen.queryByText('Warnings')).not.toBeInTheDocument();
  });

  it('renders terminal output when present', () => {
    const previewWithOutput: DiffPreviewType = {
      ...mockPreview,
      terminal_output: 'npm install completed successfully',
    };

    render(<DiffPreview preview={previewWithOutput} />);

    expect(screen.getByText('Terminal Output')).toBeInTheDocument();
    expect(screen.getByText('npm install completed successfully')).toBeInTheDocument();
  });

  it('expands all operations when expand all is clicked', async () => {
    const user = userEvent.setup();
    render(<DiffPreview preview={mockPreview} />);

    await user.click(screen.getByText('Expand all'));

    // Both operations with diffs should be expanded
    const diffEditors = screen.getAllByTestId('monaco-diff-editor');
    expect(diffEditors.length).toBe(2); // create and modify have diffs
  });

  it('collapses all operations when collapse all is clicked', async () => {
    const user = userEvent.setup();
    render(<DiffPreview preview={mockPreview} />);

    // First expand all
    await user.click(screen.getByText('Expand all'));
    expect(screen.getAllByTestId('monaco-diff-editor').length).toBe(2);

    // Then collapse all
    await user.click(screen.getByText('Collapse all'));
    expect(screen.queryByTestId('monaco-diff-editor')).not.toBeInTheDocument();
  });
});
