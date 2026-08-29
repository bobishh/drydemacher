import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveRestartTargetRef } from './sessionSnapshot';

test('saved version snapshot infers durable restart target from thread and message ids', () => {
  assert.deepEqual(
    resolveRestartTargetRef(undefined, null, 'thread-1', 'message-1'),
    {
      kind: 'savedVersion',
      threadId: 'thread-1',
      messageId: 'message-1',
    },
  );
});

test('explicit draft target remains authoritative over saved-version inference', () => {
  const draft = {
    kind: 'draft' as const,
    threadId: 'thread-1',
    previewId: 'preview-1',
    sessionId: 'session-1',
  };

  assert.deepEqual(
    resolveRestartTargetRef(draft, null, 'thread-1', 'preview-1'),
    draft,
  );
});

test('explicit null target prevents accidental restart authority', () => {
  assert.equal(
    resolveRestartTargetRef(null, null, 'thread-1', 'message-1'),
    null,
  );
});

test('stale active target from another thread cannot replace current saved version', () => {
  assert.deepEqual(
    resolveRestartTargetRef(
      undefined,
      {
        kind: 'draft',
        threadId: 'thread-old',
        previewId: 'preview-old',
        sessionId: 'session-old',
      },
      'thread-current',
      'message-current',
    ),
    {
      kind: 'savedVersion',
      threadId: 'thread-current',
      messageId: 'message-current',
    },
  );
});
