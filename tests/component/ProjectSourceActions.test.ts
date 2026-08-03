import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import type { SourceActionOutcome } from '../../src/lib/sourceActions';

let openSourceFile = vi.fn<(threadId: string | null, messageId: string | null) => Promise<SourceActionOutcome>>();
let revealSourceFolder = vi.fn<(threadId: string | null, messageId: string | null) => Promise<SourceActionOutcome>>();

vi.mock('../../src/lib/sourceActions', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    createSourceActions: () => ({
      openSourceFile,
      revealSourceFolder,
    }),
  };
});

import CodeSourceActions from '../../src/lib/components/CodeSourceActions.svelte';

// thread-source-binding §3.1 — discoverable OPEN FILE / REVEAL FOLDER on the
// active project card. Contract: both actions are visible, OPEN FILE invokes
// the bound source path, the exact absolute file + folder paths are shown once
// resolved, and RAW backend errors render in-UI (never console-only). REVEAL
// FOLDER is the frontend seam for a native reveal that needs a backend command.

describe('ProjectSourceActions', () => {
  it('renders both OPEN FILE and REVEAL FOLDER controls for a bound thread', () => {
    openSourceFile = vi.fn(async () => ({
      kind: 'open',
      ok: true,
      link: {
        slug: 'project',
        folder: '/tmp/projects/project',
        file: '/tmp/projects/project/model.ecky',
      },
    }));
    const { getByTestId } = render(CodeSourceActions, {
      props: { threadId: 'thread-1' },
    });

    expect(getByTestId('source-open-file')).toBeTruthy();
    expect(getByTestId('source-reveal-folder')).toBeTruthy();
  });

  it('OPEN FILE invokes the bound source and shows the exact absolute path', async () => {
    openSourceFile = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'open',
      ok: true,
      link: {
        slug: 'pug-cap',
        folder: '/tmp/projects/pug-cap',
        file: '/tmp/projects/pug-cap/model.ecky',
      },
    }));
    const { getByTestId } = render(CodeSourceActions, {
      props: { threadId: 'thread-1' },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-path-file').textContent).toContain(
        '/tmp/projects/pug-cap/model.ecky',
      );
    });

    expect(openSourceFile).toHaveBeenCalledWith('thread-1', null);
    expect(getByTestId('source-path-folder').textContent).toContain('/tmp/projects/pug-cap');
  });

  it('renders the RAW open error in-UI instead of console-only', async () => {
    openSourceFile = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'open',
      ok: false,
      error: "Failed to open '/x/model.ecky' in the system editor: No such file or directory",
    }));
    const { getByTestId } = render(CodeSourceActions, {
      props: { threadId: 'thread-1' },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-error').textContent).toContain(
        "Failed to open '/x/model.ecky'",
      );
    });
  });

  it('REVEAL FOLDER surfaces the raw reveal error when the native reveal is not wired', async () => {
    revealSourceFolder = vi.fn(async (): Promise<SourceActionOutcome> => ({
      kind: 'reveal',
      ok: false,
      error: 'Revealing requires a backend reveal command (not yet wired).',
    }));
    const { getByTestId } = render(CodeSourceActions, {
      props: {
        threadId: 'thread-1',
      },
    });

    await fireEvent.click(getByTestId('source-reveal-folder'));
    await waitFor(() => {
      expect(getByTestId('source-error').textContent).toContain('backend reveal command');
    });
  });

  it('shows a pending state on the action while it is in flight', async () => {
    let resolveOpen: (value: SourceActionOutcome) => void = () => {};
    openSourceFile = vi.fn(
      () =>
        new Promise<SourceActionOutcome>((resolve) => {
          resolveOpen = resolve;
        }),
    );
    const { getByTestId } = render(CodeSourceActions, {
      props: { threadId: 'thread-1' },
    });

    await fireEvent.click(getByTestId('source-open-file'));
    await waitFor(() => {
      expect(getByTestId('source-open-file').hasAttribute('disabled')).toBe(true);
    });

    resolveOpen({
      kind: 'open',
      ok: true,
      link: {
        slug: 'p',
        folder: '/f',
        file: '/f/model.ecky',
      },
    });
    await waitFor(() => {
      expect(getByTestId('source-open-file').hasAttribute('disabled')).toBe(false);
    });
  });
});
