import { describe, it, expect } from 'vitest';
import { render, screen } from '../test/test-utils';
import { CollisionBadge } from './CollisionBadge';
import type { Collision } from '../lib/types';

const mockCollision: Collision = {
  name: 'my-skill',
  occurrences: [
    { scope: 'User', path: '~/.claude/skills/my-skill/SKILL.md' },
    { scope: 'Project', path: '/project/.claude/skills/my-skill/SKILL.md' },
  ],
};

describe('CollisionBadge', () => {
  it('renders collision name for skill type', () => {
    render(<CollisionBadge collision={mockCollision} type="skill" />);

    expect(screen.getByText('my-skill')).toBeInTheDocument();
  });

  it('renders collision name with slash prefix for command type', () => {
    render(<CollisionBadge collision={mockCollision} type="command" />);

    expect(screen.getByText('/my-skill')).toBeInTheDocument();
  });

  it('displays occurrence count', () => {
    render(<CollisionBadge collision={mockCollision} type="skill" />);

    expect(screen.getByText('2 occurrences')).toBeInTheDocument();
  });

  it('renders all occurrence details', () => {
    render(<CollisionBadge collision={mockCollision} type="skill" />);

    expect(screen.getByText('User:')).toBeInTheDocument();
    expect(screen.getByText('Project:')).toBeInTheDocument();
    expect(screen.getByText('~/.claude/skills/my-skill/SKILL.md')).toBeInTheDocument();
    expect(screen.getByText('/project/.claude/skills/my-skill/SKILL.md')).toBeInTheDocument();
  });

  it('handles single occurrence', () => {
    const singleCollision: Collision = {
      name: 'single-skill',
      occurrences: [{ scope: 'User', path: '~/.claude/skills/single/SKILL.md' }],
    };

    render(<CollisionBadge collision={singleCollision} type="skill" />);

    expect(screen.getByText('1 occurrences')).toBeInTheDocument();
  });

  it('renders agent type without prefix', () => {
    render(<CollisionBadge collision={mockCollision} type="agent" />);

    // Agent type should not have prefix like command
    expect(screen.getByText('my-skill')).toBeInTheDocument();
    expect(screen.queryByText('/my-skill')).not.toBeInTheDocument();
  });
});
