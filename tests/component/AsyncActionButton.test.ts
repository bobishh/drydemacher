import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AsyncActionButton from '../../src/lib/components/AsyncActionButton.svelte';

describe('AsyncActionButton', () => {
  it('marks only the invoked action pending until its promise settles', async () => {
    let resolveAction = () => {};
    const action = vi.fn(() => new Promise<void>((resolve) => { resolveAction = resolve; }));
    const { getByRole } = render(AsyncActionButton, {
      props: { action, label: 'REMOVE', pendingLabel: 'REMOVING…' },
    });

    await fireEvent.click(getByRole('button', { name: 'REMOVE' }));
    const pending = getByRole('button', { name: 'REMOVING…' });
    expect(pending.hasAttribute('disabled')).toBe(true);
    expect(pending.getAttribute('aria-busy')).toBe('true');

    resolveAction();
    await waitFor(() => expect(getByRole('button', { name: 'REMOVE' }).hasAttribute('disabled')).toBe(false));
    expect(action).toHaveBeenCalledTimes(1);
  });

  it('restores the idle action and exposes rejection to caller-owned error UI', async () => {
    const error = new Error('raw remove failure');
    const onerror = vi.fn();
    const { getByRole } = render(AsyncActionButton, {
      props: {
        action: async () => { throw error; },
        label: 'DELETE',
        pendingLabel: 'DELETING…',
        onerror,
      },
    });

    await fireEvent.click(getByRole('button', { name: 'DELETE' }));
    await waitFor(() => expect(getByRole('button', { name: 'DELETE' })).toBeTruthy());
    expect(onerror).toHaveBeenCalledWith(error);
  });
});
