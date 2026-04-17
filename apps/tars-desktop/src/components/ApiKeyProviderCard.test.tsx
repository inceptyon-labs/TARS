import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act } from '../test/test-utils';
import { ApiKeyProviderCard } from './ApiKeyProviderCard';
import type { ApiKeySummary, ProviderMetadata } from '../lib/ipc';
import { invoke } from '@tauri-apps/api/core';

const invokeMock = vi.mocked(invoke);

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
function renderWithUser(ui: ReactElement, opts: { fakeTimers?: boolean } = {}) {
  const user = opts.fakeTimers
    ? userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
    : userEvent.setup();
  return { user, ...render(ui) };
}

describe('ApiKeyProviderCard — interactions', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('reveals the plaintext key on click and schedules auto-hide at 10s', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'reveal_api_key') return 'sk-revealed-12345';
      throw new Error(`unexpected ${cmd}`);
    });
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout');
    const { user } = renderWithUser(
      <ApiKeyProviderCard
        provider={openaiMeta}
        keys={[makeKey({ id: 1, label: 'work' })]}
        onAddKey={vi.fn()}
      />
    );
    await user.click(screen.getByRole('button', { name: /reveal key/i }));
    await waitFor(() => {
      expect(screen.getByText('sk-revealed-12345')).toBeInTheDocument();
    });
    // Component must schedule the auto-hide with the documented 10s timeout.
    const tenSecondCall = setTimeoutSpy.mock.calls.find(([, ms]) => ms === 10_000);
    expect(tenSecondCall).toBeDefined();
    setTimeoutSpy.mockRestore();
  });

  it('auto-hides the revealed key after the 10s timeout fires', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'reveal_api_key') return 'sk-autohide';
      throw new Error(`unexpected ${cmd}`);
    });
    // Capture the auto-hide callback by spying on setTimeout. We invoke it
    // manually so we don't have to thread fake timers through userEvent.
    const realSetTimeout = globalThis.setTimeout;
    let captured: (() => void) | null = null;
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout').mockImplementation(((
      cb: () => void,
      ms?: number
    ) => {
      if (ms === 10_000) {
        captured = cb;
        return 0 as unknown as ReturnType<typeof setTimeout>;
      }
      return realSetTimeout(cb, ms);
    }) as typeof setTimeout);

    try {
      const { user } = renderWithUser(
        <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 1 })]} onAddKey={vi.fn()} />
      );
      await user.click(screen.getByRole('button', { name: /reveal key/i }));
      await waitFor(() => expect(screen.getByText('sk-autohide')).toBeInTheDocument());

      expect(captured).not.toBeNull();
      await act(async () => {
        captured!();
      });
      expect(screen.queryByText('sk-autohide')).not.toBeInTheDocument();
    } finally {
      setTimeoutSpy.mockRestore();
    }
  });

  it('does not setState if the component unmounts before reveal resolves', async () => {
    let resolveReveal: ((value: string) => void) | null = null;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'reveal_api_key') {
        return new Promise<string>((resolve) => {
          resolveReveal = resolve;
        });
      }
      throw new Error(`unexpected ${cmd}`);
    });

    const { user, unmount } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 1 })]} onAddKey={vi.fn()} />
    );
    await user.click(screen.getByRole('button', { name: /reveal key/i }));
    // Tear down before the IPC resolves.
    unmount();
    // Resolve after unmount — must not throw or warn about setState on
    // unmounted component.
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      await act(async () => {
        resolveReveal?.('sk-late');
        // Yield so the mutation resolution flushes.
        await Promise.resolve();
      });
      const setStateWarnings = errorSpy.mock.calls.filter(
        ([msg]) => typeof msg === 'string' && msg.includes("can't perform a React state update")
      );
      expect(setStateWarnings).toHaveLength(0);
    } finally {
      errorSpy.mockRestore();
    }
  });

  it('toggles back to masked when reveal is clicked a second time', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'reveal_api_key') return 'sk-toggled';
      throw new Error(`unexpected ${cmd}`);
    });
    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 1 })]} onAddKey={vi.fn()} />
    );
    await user.click(screen.getByRole('button', { name: /reveal key/i }));
    await waitFor(() => expect(screen.getByText('sk-toggled')).toBeInTheDocument());
    await user.click(screen.getByRole('button', { name: /hide key/i }));
    expect(screen.queryByText('sk-toggled')).not.toBeInTheDocument();
  });

  it('copies the decrypted key to clipboard', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'reveal_api_key') return 'sk-clip-XYZ';
      throw new Error(`unexpected ${cmd}`);
    });

    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 1 })]} onAddKey={vi.fn()} />
    );
    // userEvent.setup() installs a clipboard polyfill on `navigator.clipboard`.
    // Spy on its writeText so we can assert the component wrote the right value
    // without disturbing the polyfill itself.
    const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    try {
      await user.click(screen.getByRole('button', { name: /copy key/i }));
      await waitFor(() => expect(writeText).toHaveBeenCalledWith('sk-clip-XYZ'));
    } finally {
      writeText.mockRestore();
    }
  });

  it('confirms before deleting and calls delete_api_key on confirm', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'delete_api_key') return true;
      throw new Error(`unexpected ${cmd}`);
    });
    const { user } = renderWithUser(
      <ApiKeyProviderCard
        provider={openaiMeta}
        keys={[makeKey({ id: 7, label: 'work' })]}
        onAddKey={vi.fn()}
      />
    );
    await user.click(screen.getByRole('button', { name: /delete key/i }));
    expect(confirmSpy).toHaveBeenCalled();
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('delete_api_key', { id: 7 }));
  });

  it('does NOT call delete_api_key when user cancels confirm', async () => {
    vi.spyOn(window, 'confirm').mockReturnValue(false);
    invokeMock.mockResolvedValue([]);
    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 7 })]} onAddKey={vi.fn()} />
    );
    await user.click(screen.getByRole('button', { name: /delete key/i }));
    expect(invokeMock).not.toHaveBeenCalledWith('delete_api_key', expect.anything());
  });

  it('renders a valid badge when last_valid is true', () => {
    render(
      <ApiKeyProviderCard
        provider={openaiMeta}
        keys={[makeKey({ id: 1, last_valid: true, last_validated_at: '2026-04-17T12:00:00Z' })]}
        onAddKey={vi.fn()}
      />
    );
    expect(screen.getByText(/^valid$/i)).toBeInTheDocument();
  });

  it('renders an invalid badge when last_valid is false', () => {
    render(
      <ApiKeyProviderCard
        provider={openaiMeta}
        keys={[makeKey({ id: 1, last_valid: false, last_validated_at: '2026-04-17T12:00:00Z' })]}
        onAddKey={vi.fn()}
      />
    );
    expect(screen.getByText(/^invalid$/i)).toBeInTheDocument();
  });

  it('calls validate_api_key when Validate is clicked', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'validate_api_key') return { valid: true, message: null };
      throw new Error(`unexpected ${cmd}`);
    });
    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 9 })]} onAddKey={vi.fn()} />
    );
    await user.click(screen.getByRole('button', { name: /validate key/i }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('validate_api_key', { id: 9 }));
  });

  it('does not render Invalid badge for an unverifiable provider (Perplexity)', () => {
    const perplexityMeta: ProviderMetadata = {
      id: 'perplexity',
      display_name: 'Perplexity',
      docs_url: 'https://www.perplexity.ai/settings/api',
      key_format_hint: 'pplx-...',
      supports_models: false,
      supports_balance: false,
    };
    // last_valid is left null because the backend skips update_validation for
    // unverifiable providers. The badge column must stay empty in that case —
    // no Valid, no Invalid.
    render(
      <ApiKeyProviderCard
        provider={perplexityMeta}
        keys={[
          makeKey({
            id: 2,
            provider_id: 'perplexity',
            last_valid: null,
            last_validated_at: null,
          }),
        ]}
        onAddKey={vi.fn()}
      />
    );
    expect(screen.queryByText(/^valid$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^invalid$/i)).not.toBeInTheDocument();
  });
});

const noPricingMeta = { last_refresh_at: null, last_error: null, last_error_at: null };

describe('ApiKeyProviderCard — models', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders the model table when listProviderModels returns rows', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_provider_models') {
        return [
          {
            provider_id: 'openai',
            model_id: 'gpt-4o',
            display_name: 'GPT-4o',
            context_window: 128_000,
            input_price: 2.5,
            output_price: 10,
            fetched_at: '2026-04-17T00:00:00Z',
          },
        ];
      }
      if (cmd === 'get_pricing_metadata') return noPricingMeta;
      throw new Error(`unexpected ${cmd}`);
    });
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(await screen.findByText('GPT-4o')).toBeInTheDocument();
    expect(screen.getByText(/\$2\.50/)).toBeInTheDocument();
    expect(screen.getByText(/\$10\.00/)).toBeInTheDocument();
    expect(screen.getByText(/128k/i)).toBeInTheDocument();
  });

  it('shows an empty model state when listProviderModels returns []', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_provider_models') return [];
      if (cmd === 'get_pricing_metadata') return noPricingMeta;
      throw new Error(`unexpected ${cmd}`);
    });
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(await screen.findByText(/no models cached/i)).toBeInTheDocument();
  });

  it('does not render a model section when the provider does not support models', () => {
    const noModels = { ...openaiMeta, supports_models: false };
    render(<ApiKeyProviderCard provider={noModels} keys={[]} onAddKey={vi.fn()} />);
    expect(screen.queryByRole('heading', { name: /^models$/i })).not.toBeInTheDocument();
  });

  it('calls refresh_models then re-fetches models when Refresh Models is clicked', async () => {
    let listed = 0;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'refresh_models') return 1;
      if (cmd === 'list_provider_models') {
        listed++;
        return [];
      }
      if (cmd === 'get_pricing_metadata') return noPricingMeta;
      throw new Error(`unexpected ${cmd}`);
    });
    const { user } = renderWithUser(
      <ApiKeyProviderCard provider={openaiMeta} keys={[makeKey({ id: 1 })]} onAddKey={vi.fn()} />
    );
    await waitFor(() => expect(listed).toBeGreaterThanOrEqual(1));
    const baseline = listed;
    await user.click(screen.getByRole('button', { name: /refresh models/i }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('refresh_models', { providerId: 'openai' })
    );
    await waitFor(() => expect(listed).toBeGreaterThan(baseline));
  });

  it('shows "prices updated" badge when pricing metadata has a recent refresh', async () => {
    const recentIso = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_provider_models') return [];
      if (cmd === 'get_pricing_metadata')
        return { last_refresh_at: recentIso, last_error: null, last_error_at: null };
      throw new Error(`unexpected ${cmd}`);
    });
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(await screen.findByText(/prices updated/i)).toBeInTheDocument();
  });

  it('shows a stale warning when last pricing refresh is older than 14 days', async () => {
    const staleIso = new Date(Date.now() - 15 * 24 * 60 * 60 * 1000).toISOString();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_provider_models') return [];
      if (cmd === 'get_pricing_metadata')
        return { last_refresh_at: staleIso, last_error: null, last_error_at: null };
      throw new Error(`unexpected ${cmd}`);
    });
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(await screen.findByText(/pricing data may be stale/i)).toBeInTheDocument();
  });

  it('shows a fetch-failed warning when last_error is set', async () => {
    const recentIso = new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_provider_models') return [];
      if (cmd === 'get_pricing_metadata')
        return {
          last_refresh_at: recentIso,
          last_error: 'LiteLLM fetch returned HTTP 503',
          last_error_at: recentIso,
        };
      throw new Error(`unexpected ${cmd}`);
    });
    render(<ApiKeyProviderCard provider={openaiMeta} keys={[]} onAddKey={vi.fn()} />);
    expect(await screen.findByText(/last pricing fetch failed/i)).toBeInTheDocument();
  });
});

afterEach(() => {
  vi.useRealTimers();
});
