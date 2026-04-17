import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../test/test-utils';
import { ApiKeyProviderCard } from './ApiKeyProviderCard';
import type { ApiKeySummary, ProviderMetadata } from '../lib/ipc';

const openaiMeta: ProviderMetadata = {
  id: 'openai',
  display_name: 'OpenAI',
  docs_url: 'https://platform.openai.com/api-keys',
  key_format_hint: 'sk-...',
  supports_models: true,
  supports_balance: false,
};

const deepseekMeta: ProviderMetadata = {
  id: 'deepseek',
  display_name: 'DeepSeek',
  docs_url: 'https://platform.deepseek.com',
  key_format_hint: 'sk-...',
  supports_models: true,
  supports_balance: true,
};

function makeKey(overrides: Partial<ApiKeySummary> = {}): ApiKeySummary {
  return {
    id: 1,
    provider_id: 'openai',
    label: 'work',
    last_validated_at: null,
    last_valid: null,
    balance: null,
    created_at: '2026-04-01T00:00:00Z',
    updated_at: '2026-04-01T00:00:00Z',
    ...overrides,
  };
}

describe('ApiKeyProviderCard — shell', () => {
  it('renders the provider display name', () => {
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(screen.getByText('OpenAI')).toBeInTheDocument();
  });

  it('shows empty state when no keys are stored', () => {
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(screen.getByText(/no keys stored/i)).toBeInTheDocument();
  });

  it('renders an Add Key button that calls onAddKey', async () => {
    const onAddKey = vi.fn();
    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={onAddKey} />
    );
    await user.click(screen.getByRole('button', { name: /add key/i }));
    expect(onAddKey).toHaveBeenCalledWith(openaiMeta);
  });

  it('renders each key label and never the plaintext key', () => {
    const keys = [makeKey({ id: 1, label: 'work' }), makeKey({ id: 2, label: 'personal' })];
    render(<ApiKeyProviderCard provider={openaiMeta} keys={keys} onAddKey={vi.fn()} />);
    expect(screen.getByText('work')).toBeInTheDocument();
    expect(screen.getByText('personal')).toBeInTheDocument();
    // Mask placeholder is rendered (real key never present in summary anyway).
    const masks = screen.getAllByText(/^•+$/);
    expect(masks.length).toBe(2);
  });

  it('shows a Refresh Models button only when the provider supports model discovery', () => {
    const { rerender } = render(
      <ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />
    );
    expect(screen.getByRole('button', { name: /refresh models/i })).toBeInTheDocument();

    const noModels: ProviderMetadata = { ...openaiMeta, supports_models: false };
    rerender(<ApiKeyProviderCard provider={noModels} keys={[]} onAddKey={vi.fn()} />);
    expect(screen.queryByRole('button', { name: /refresh models/i })).not.toBeInTheDocument();
  });

  it('shows balance badge slot only for providers that support balance', () => {
    const { rerender } = render(
      <ApiKeyProviderCard
        provider={deepseekMeta}
        keys={[makeKey({ provider_id: 'deepseek', balance: { total_balance: '10.00' } })]}
        onAddKey={vi.fn()}
      />
    );
    expect(screen.getByTestId('balance-badge')).toBeInTheDocument();

    rerender(
      <ApiKeyProviderCard
        provider={openaiMeta}
        keys={[makeKey({ balance: { ignored: true } })]}
        onAddKey={vi.fn()}
      />
    );
    expect(screen.queryByTestId('balance-badge')).not.toBeInTheDocument();
  });
});

import userEvent from '@testing-library/user-event';
import type { ReactElement } from 'react';
function renderWithUser(ui: ReactElement) {
  const user = userEvent.setup();
  return { user, ...render(ui) };
}
