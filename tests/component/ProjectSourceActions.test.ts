import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import ProjectSourceActions from '../../src/lib/components/ProjectSourceActions.svelte';
import type { SourceActionOutcome } from '../../src/lib/sourceActions';

// thread-source-binding §3.1 — discoverable OPEN FILE / REVEAL FOLDER on the
// active project card. Contract: both actions are visible, OPEN FILE invokes
// the bound source path, the exact absolute file + folder paths are shown once
// resolved, and RAW backend errors render in-UI (never console-only). REVEAL
// FOLDER is the frontend seam for a native reveal that needs a backend command.

describe('ProjectSourceActions', () => {
  it('renders both OPEN FILE and REVEAL FOLDER controls for a bound thread', () => {
    const openFile = vi.fn();
    const { getByTestId } = render(ProjectSourceActions, {
      props: { threadId: 'thread-1', threadTitle: 'Pug Cap', openFile, revealFolder: vi.fn() },
    });

    expect(getByTestId('source-open-file')).toBeTruthy();
    expect(getByTestId('source-reveal-folder')).toBeTruthy();
  });

  it('OPEN FILE invokes the bound source and shows the exact absolute path', async () => {
    const openFile = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'open',
      ok: true,
      link: {
        slug: 'pug-cap',
        folder: '/tmp/projects/pug-cap',
        file: '/tmp/projects/pug-cap/model.ecky',
      },
    }));
    const { getByTestId } = render(ProjectSourceActions, {
      props: { threadId: 'thread-1', threadTitle: 'Pug Cap', openFile, revealFolder: vi.fn() },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-path-file').textContent).toContain(
        '/tmp/projects/pug-cap/model.ecky',
      );
    });

    expect(openFile).toHaveBeenCalledWith('thread-1');
    expect(getByTestId('source-path-folder').textContent).toContain('/tmp/projects/pug-cap');
  });

  it('renders the RAW open error in-UI instead of console-only', async () => {
    const openFile = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'open',
      ok: false,
      error: "Failed to open '/x/model.ecky' in the system editor: No such file or directory",
    }));
    const { getByTestId } = render(ProjectSourceActions, {
      props: { threadId: 'thread-1', threadTitle: 'Pug Cap', openFile, revealFolder: vi.fn() },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-error').textContent).toContain(
        "Failed to open '/x/model.ecky'",
      );
    });
  });

  it('REVEAL FOLDER surfaces the raw reveal error when the native reveal is not wired', async () => {
    const revealFolder = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'reveal',
      ok: false,
      error: 'Revealing requires a backend reveal command (not yet wired).',
    }));
    const { getByTestId } = render(ProjectSourceActions, {
      props: {
        threadId: 'thread-1',
        threadTitle: 'Pug Cap',
        openFile: vi.fn(),
        revealFolder,
      },
    });

    await fireEvent.click(getByTestId('source-reveal-folder'));
    await waitFor(() => {
      expect(getByTestId('source-error').textContent).toContain('backend reveal command');
    });
  });

  it('shows a pending state on the action while it is in flight', async () => {
    let resolveOpen: (value: SourceActionOutcome) => void = () => {};
    const openFile = vi.fn(
      () =>
        new Promise<SourceActionOutcome>((resolve) => {
          resolveOpen = resolve;
        }),
    );
    const { getByTestId } = render(ProjectSourceActions, {
      props: { threadId: 'thread-1', threadTitle: 'Pug Cap', openFile, revealFolder: vi.fn() },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-open-file').hasAttribute('disabled')).toBe(true);
    });

    resolveOpen({ kind: 'open', ok: true, link: {
      slug: 'p', folder: '/f', file: '/f/model.ecky',
    } });
    await waitFor(() => {
      expect(getByTestId('source-open-file').hasAttribute('disabled')).toBe(false);
    });
  });
});
