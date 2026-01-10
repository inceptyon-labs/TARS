import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../test/test-utils';
import userEvent from '@testing-library/user-event';
import { ProjectList } from './ProjectList';
import type { ProjectInfo } from '../lib/types';

const mockProjects: ProjectInfo[] = [
  {
    id: '1',
    name: 'Project Alpha',
    path: '/Users/dev/projects/alpha',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-02T00:00:00Z',
  },
  {
    id: '2',
    name: 'Project Beta',
    path: '/Users/dev/projects/beta',
    created_at: '2024-01-03T00:00:00Z',
    updated_at: '2024-01-04T00:00:00Z',
  },
];

describe('ProjectList', () => {
  it('renders a list of projects', () => {
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList
        projects={mockProjects}
        selectedPath={null}
        onSelect={onSelect}
        onRemove={onRemove}
      />
    );

    expect(screen.getByText('Project Alpha')).toBeInTheDocument();
    expect(screen.getByText('Project Beta')).toBeInTheDocument();
    expect(screen.getByText('/Users/dev/projects/alpha')).toBeInTheDocument();
    expect(screen.getByText('/Users/dev/projects/beta')).toBeInTheDocument();
  });

  it('renders empty list when no projects', () => {
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList projects={[]} selectedPath={null} onSelect={onSelect} onRemove={onRemove} />
    );

    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('calls onSelect when project is clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList
        projects={mockProjects}
        selectedPath={null}
        onSelect={onSelect}
        onRemove={onRemove}
      />
    );

    await user.click(screen.getByText('Project Alpha'));

    expect(onSelect).toHaveBeenCalledWith(mockProjects[0]);
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it('calls onRemove when delete button is clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList
        projects={mockProjects}
        selectedPath={null}
        onSelect={onSelect}
        onRemove={onRemove}
      />
    );

    // Find the first list item and hover to reveal delete button
    const listItems = screen.getAllByRole('listitem');
    const firstItem = listItems[0];

    // Find the delete button within the first item
    const deleteButtons = firstItem.querySelectorAll('button');
    const deleteButton = deleteButtons[deleteButtons.length - 1]; // Last button is delete

    await user.click(deleteButton);

    expect(onRemove).toHaveBeenCalledWith('1');
    expect(onSelect).not.toHaveBeenCalled(); // Should not trigger select
  });

  it('highlights selected project', () => {
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList
        projects={mockProjects}
        selectedPath="/Users/dev/projects/alpha"
        onSelect={onSelect}
        onRemove={onRemove}
      />
    );

    const projectButton = screen.getByText('Project Alpha').closest('button');
    expect(projectButton).toHaveClass('active');
  });

  it('does not highlight unselected projects', () => {
    const onSelect = vi.fn();
    const onRemove = vi.fn();

    render(
      <ProjectList
        projects={mockProjects}
        selectedPath="/Users/dev/projects/alpha"
        onSelect={onSelect}
        onRemove={onRemove}
      />
    );

    const projectButton = screen.getByText('Project Beta').closest('button');
    expect(projectButton).not.toHaveClass('active');
  });
});
