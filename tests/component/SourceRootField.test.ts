import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import SourceRootField from '../../src/lib/components/SourceRootField.svelte';

// thread-source-binding §3.3 — Settings source-root picker.
// Presentational contract: shows the configured projectsRoot (or the default
// fallback), lets the user pick a directory, persists via the existing save
// command, and renders RAW filesystem/persist errors in-UI (never console-only).
// The directory picker is dependency-injected so this runs under jsdom.

describe('SourceRootField', () => {
  it('renders the configured absolute source root', () => {
    const { getByTestId } = render(SourceRootField, {
      props: {
        projectsRoot: '/Users/me/ecky-projects',
        pickDirectory: async () => null,
      },
    });

    expect(getByTestId('source-root-path').textContent).toContain('/Users/me/ecky-projects');
  });

  it('shows the default-root hint when projectsRoot is blank', () => {
    const { getByTestId } = render(SourceRootField, {
      props: { projectsRoot: '', pickDirectory: async () => null },
    });

    expect(getByTestId('source-root-path').textContent).toMatch(/default/i);
  });

  it('persists the picked directory through the existing save command', async () => {
    const onsave = vi.fn(async () => {});
    const pickDirectory = vi.fn(async () => '/Volumes/Design/source-root');
    const { getByTestId } = render(SourceRootField, {
      props: { projectsRoot: '', onsave, pickDirectory },
    });

    await fireEvent.click(getByTestId('source-root-pick'));
    await waitFor(() => {
      expect(pickDirectory).toHaveBeenCalled();
    });
    // The picked value re-renders. Svelte 5 runes props are not readable off
    // the component instance, so the DOM + the save callback are the
    // observable contract: the bindable reassigned locally and onsave ran.
    await waitFor(() => {
      expect(getByTestId('source-root-path').textContent).toContain(
        '/Volumes/Design/source-root',
      );
    });
    expect(onsave).toHaveBeenCalled();
  });

  it('shows a pending state on the picker while the directory dialog is in flight', async () => {
    let resolvePick: (value: string | null) => void = () => {};
    const pickDirectory = vi.fn(
      () =>
        new Promise<string | null>((resolve) => {
          resolvePick = resolve;
        }),
    );
    const { getByTestId } = render(SourceRootField, {
      props: { projectsRoot: '', onsave: vi.fn(), pickDirectory },
    });

    await fireEvent.click(getByTestId('source-root-pick'));
    await waitFor(() => {
      expect(getByTestId('source-root-pick').hasAttribute('disabled')).toBe(true);
    });

    resolvePick('/Volumes/Design/source-root');
    await waitFor(() => {
      expect(getByTestId('source-root-pick').hasAttribute('disabled')).toBe(false);
    });
    await waitFor(() => {
      expect(getByTestId('source-root-path').textContent).toContain(
        '/Volumes/Design/source-root',
      );
    });
  });

  it('renders the RAW picker/persist error in-UI instead of swallowing it', async () => {
    const onsave = vi.fn(async () => {
      throw new Error('save_config: projects_root not writable (os error 13)');
    });
    const pickDirectory = vi.fn(async () => '/Volumes/Design/source-root');
    const { getByTestId } = render(SourceRootField, {
      props: { projectsRoot: '', onsave, pickDirectory },
    });

    await fireEvent.click(getByTestId('source-root-pick'));

    const errorEl = await waitFor(() => getByTestId('source-root-error'));
    expect(errorEl.textContent).toContain('save_config: projects_root not writable');
  });

  it('ignores a cancelled picker without erroring', async () => {
    const onsave = vi.fn(async () => {});
    const pickDirectory = vi.fn(async () => null);
    const { getByTestId, queryByTestId } = render(SourceRootField, {
      props: { projectsRoot: '/keep/me', onsave, pickDirectory },
    });

    await fireEvent.click(getByTestId('source-root-pick'));
    await Promise.resolve();

    expect(onsave).not.toHaveBeenCalled();
    expect(queryByTestId('source-root-error')).toBeNull();
    expect(getByTestId('source-root-path').textContent).toContain('/keep/me');
  });

  it('persists clearing the configured root back to the default', async () => {
    const onsave = vi.fn(async () => {});
    const { getByRole, getByTestId } = render(SourceRootField, {
      props: {
        projectsRoot: '/Volumes/Design/source-root',
        onsave,
        pickDirectory: async () => null,
      },
    });

    await fireEvent.click(getByRole('button', { name: /clear/i }));

    await waitFor(() => expect(onsave).toHaveBeenCalledTimes(1));
    expect(getByTestId('source-root-path').textContent).toMatch(/default/i);
  });
});
