import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createSourceActions,
  type SourceActionDeps,
  type SourceLink,
} from './sourceActions';

// thread-source-binding §3.2/§3.3 — the source-action seam must (a) resolve the
// bound source link through the existing open_project_in_editor backend, (b)
// surface RAW backend errors verbatim (never invent generic messages), and
// (c) expose a reveal-folder seam whose native action is a tracked backend
// dependency. All backend/touch surfaces are injected so this stays a pure unit.

type OpenCall = { threadId: string | null; messageId: string | null };

function makeDeps(overrides: Partial<SourceActionDeps> = {}): {
  deps: SourceActionDeps;
  openCalls: OpenCall[];
  revealCalls: OpenCall[];
} {
  const openCalls: OpenCall[] = [];
  const revealCalls: OpenCall[] = [];
  const link = {
    slug: 'pug-cap',
    folder: '/tmp/projects/pug-cap',
    file: '/tmp/projects/pug-cap/model.ecky',
  };
  const deps: SourceActionDeps = {
    openInEditor: async (threadId, messageId) => {
      openCalls.push({ threadId, messageId });
      return link;
    },
    revealInFileManager: async (threadId, messageId) => {
      revealCalls.push({ threadId, messageId });
      return link;
    },
    ...overrides,
  };
  return { deps, openCalls, revealCalls };
}

const knownLink: SourceLink = {
  slug: 'pug-cap',
  folder: '/tmp/projects/pug-cap',
  file: '/tmp/projects/pug-cap/model.ecky',
};

test('openSourceFile resolves the bound file/folder link via open_project_in_editor', async () => {
  const { deps, openCalls } = makeDeps();
  const actions = createSourceActions(deps);

  const outcome = await actions.openSourceFile('thread-1', 'msg-1');

  assert.equal(outcome.kind, 'open');
  assert.equal(outcome.ok, true);
  assert.deepEqual(outcome.link, {
    slug: 'pug-cap',
    folder: '/tmp/projects/pug-cap',
    file: '/tmp/projects/pug-cap/model.ecky',
  });
  assert.deepEqual(openCalls, [{ threadId: 'thread-1', messageId: 'msg-1' }]);
});

test('openSourceFile surfaces the RAW backend error instead of a generic message', async () => {
  const { deps } = makeDeps({
    openInEditor: async () => {
      // Faithful AppError shape: backend open_project_in_editor returns these.
      throw {
        code: 'internal' as const,
        message:
          "Failed to open '/tmp/x/model.ecky' in the system editor: No such file or directory",
        details: 'os error 2',
      };
    },
  });
  const actions = createSourceActions(deps);

  const outcome = await actions.openSourceFile('thread-1', null);

  assert.equal(outcome.ok, false);
  assert.ok(!('link' in outcome));
  assert.ok('error' in outcome);
  // raw backend text preserved verbatim — no fabricated "Check API Key"-style message
  assert.ok(
    outcome.error.includes("Failed to open '/tmp/x/model.ecky'"),
    `expected raw error text, got: ${outcome.error}`,
  );
  assert.ok(outcome.error.includes('os error 2'));
});

test('revealSourceFolder calls native reveal without opening the editor file', async () => {
  const { deps, openCalls, revealCalls } = makeDeps();
  const actions = createSourceActions(deps);

  const outcome = await actions.revealSourceFolder('thread-1', 'msg-1', knownLink);

  assert.equal(outcome.kind, 'reveal');
  assert.equal(outcome.ok, true);
  assert.deepEqual(revealCalls, [{ threadId: 'thread-1', messageId: 'msg-1' }]);
  assert.deepEqual(openCalls, []);
});

test('revealSourceFolder surfaces the raw reveal failure (never invents a generic error)', async () => {
  const { deps } = makeDeps({
    revealInFileManager: async () => {
      throw new Error('plugin:shell|open rejected: scope disallowed');
    },
  });
  const actions = createSourceActions(deps);

  const outcome = await actions.revealSourceFolder('thread-1', null, knownLink);

  assert.equal(outcome.ok, false);
  assert.ok(outcome.error && outcome.error.includes('plugin:shell|open rejected'));
});
