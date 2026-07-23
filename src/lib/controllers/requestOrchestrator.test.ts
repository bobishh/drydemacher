import assert from 'node:assert/strict';
import test from 'node:test';

import { canPublishGenerationProjection } from './requestOrchestrator';

test('Given a newer same-thread request is active, when an older request finishes, then it cannot publish the active version or session snapshot', () => {
  assert.equal(
    canPublishGenerationProjection({
      requestId: 'older-request',
      requestThreadId: 'thread-1',
      latestThreadRequestId: 'newer-request',
      activeThreadId: 'thread-1',
    }),
    false,
  );
});

test('Given the finishing request remains active for its thread, when it finishes, then it may publish the active projection', () => {
  assert.equal(
    canPublishGenerationProjection({
      requestId: 'newer-request',
      requestThreadId: 'thread-1',
      latestThreadRequestId: 'newer-request',
      activeThreadId: 'thread-1',
    }),
    true,
  );
});
