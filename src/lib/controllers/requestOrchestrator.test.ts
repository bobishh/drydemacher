import assert from 'node:assert/strict';
import test from 'node:test';

import {
  canPublishGenerationProjection,
  projectExplorationRunProgress,
  projectExplorationRunTerminal,
} from './requestOrchestrator';

test('progress maps backend phases and queue counts into existing request projection', () => {
  assert.deepEqual(projectExplorationRunProgress({
    requestId: 'request-1', threadId: 'thread-1', phase: 'building', attempt: 2,
    maxAttempts: 3, runningBuilds: 1, pendingBuilds: 2, summary: 'Repairing exact version.',
  }), {
    requestPhase: 'repairing', buildQueueState: 'running', attempt: 2, maxAttempts: 3,
    copy: 'Repairing exact version. · RUNNING 1 · PENDING 2',
  });
});

test('terminal projection keeps stopped and superseded distinct from provider failure', () => {
  assert.deepEqual(projectExplorationRunTerminal('stopped', null), {
    requestPhase: 'canceled', copy: 'EXPLORATION STOPPED', error: null,
  });
  assert.deepEqual(projectExplorationRunTerminal('superseded', null), {
    requestPhase: 'canceled', copy: 'EXPLORATION SUPERSEDED BY NEWER INPUT', error: null,
  });
  assert.deepEqual(projectExplorationRunTerminal('failed', 'raw provider body'), {
    requestPhase: 'error', copy: 'EXPLORATION FAILED · raw provider body', error: 'raw provider body',
  });
});

test('Given PLAN asks a question, when the Rust run yields, then the request finishes without a frontend error', () => {
  assert.deepEqual(projectExplorationRunTerminal('awaitingInput', null, 'Which clearance?'), {
    requestPhase: 'success', copy: 'Which clearance?', error: null,
  });
});

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
