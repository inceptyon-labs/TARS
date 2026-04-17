import { describe, it, expect, vi, beforeEach } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen, waitFor } from '../test/test-utils';
import { AddApiKeyDialog } from './AddApiKeyDialog';
import { invoke } from '@tauri-apps/api/core';
import type { ProviderMetadata } from '../lib/ipc';

const openai: ProviderMetadata = {
  id: 'openai',
  display_name: 'OpenAI',
  docs_url: 'https://platform.openai.com',
  key_format_hint: 'sk-...',
  supports_models: true,
  supports_balance: false,
};

const invokeMock = vi.mocked(invoke);

describe('AddApiKeyDialog', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders nothing when provider is null', () => {
    const { container } = render(<AddApiKeyDialog provider={null} onOpenChange={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('shows the provider name and key format hint when open', () => {
    render(<AddApiKeyDialog provider={openai} onOpenChange={vi.fn()} />);
    expect(screen.getByText(/Add OpenAI key/i)).toBeInTheDocument();
    expect(screen.getByText(/sk-\.\.\./)).toBeInTheDocument();
  });

  it('disables Save until both label and key are non-empty', async () => {
    const user = userEvent.setup();
    render(<AddApiKeyDialog provider={openai} onOpenChange={vi.fn()} />);

    const saveBtn = screen.getByRole('button', { name: /save/i });
    expect(saveBtn).toBeDisabled();

    await user.type(screen.getByLabelText(/label/i), 'work');
    expect(saveBtn).toBeDisabled();

    await user.type(screen.getByLabelText(/api key/i), 'sk-abc');
    expect(saveBtn).toBeEnabled();
  });

  it('calls add_api_key then validate_api_key, then closes', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'add_api_key') return 42;
      if (cmd === 'validate_api_key') return { valid: true, message: null };
      throw new Error(`unexpected ${cmd}`);
    });
    const onOpenChange = vi.fn();
    const user = userEvent.setup();
    render(<AddApiKeyDialog provider={openai} onOpenChange={onOpenChange} />);

    await user.type(screen.getByLabelText(/label/i), 'work');
    await user.type(screen.getByLabelText(/api key/i), 'sk-abc');
    await user.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('add_api_key', {
        input: { provider_id: 'openai', label: 'work', key: 'sk-abc' },
      });
      expect(invokeMock).toHaveBeenCalledWith('validate_api_key', { id: 42 });
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('keeps the dialog open and shows error when add_api_key fails', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'add_api_key') throw 'duplicate label';
      throw new Error(`unexpected ${cmd}`);
    });
    const onOpenChange = vi.fn();
    const user = userEvent.setup();
    render(<AddApiKeyDialog provider={openai} onOpenChange={onOpenChange} />);

    await user.type(screen.getByLabelText(/label/i), 'work');
    await user.type(screen.getByLabelText(/api key/i), 'sk-abc');
    await user.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('add_api_key', expect.anything());
    });
    expect(invokeMock).not.toHaveBeenCalledWith('validate_api_key', expect.anything());
    // Dialog stays open on failure so user can correct.
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });

  it('still closes the dialog when validation fails after save', async () => {
    // Add succeeded — key is stored. Validation failure is informational
    // (UI shows red badge); the dialog should not block the user.
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'add_api_key') return 7;
      if (cmd === 'validate_api_key') return { valid: false, message: 'unauthorized' };
      throw new Error(`unexpected ${cmd}`);
    });
    const onOpenChange = vi.fn();
    const user = userEvent.setup();
    render(<AddApiKeyDialog provider={openai} onOpenChange={onOpenChange} />);

    await user.type(screen.getByLabelText(/label/i), 'work');
    await user.type(screen.getByLabelText(/api key/i), 'sk-bad');
    await user.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
