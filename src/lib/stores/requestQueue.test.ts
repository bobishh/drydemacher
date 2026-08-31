import assert from 'node:assert/strict';
import test from 'node:test';
import { applyRequestPatch } from './requestQueue';
import type { Request } from '../types/domain';

function request(overrides: Partial<Request> = {}): Request {
  return {
    id: 'req-test', prompt: 'test', attachments: [], createdAt: 0,
    phase: 'rendering', attempt: 1, maxAttempts: 3, maxVerifyAttempts: 2,
    isQuestion: false, lightResponse: '', screenshot: null, threadId: null,
    baseMessageId: null, baseModelId: null, result: null, error: null,
    cookingStartTime: null, cookingElapsed: 0, ...overrides,
    buildMode: overrides.buildMode ?? 'interactive',
    buildQueueState: overrides.buildQueueState ?? 'running',
  };
}

const result = {
  design: null, threadId: 'thread', messageId: 'message', stlUrl: '',
  artifactBundle: null, modelManifest: null,
};

test('request lifecycle rejects success with an error payload', () => {
  assert.throws(() => applyRequestPatch(request(), {
    phase: 'success', result, error: 'stale error',
  } as never), /cannot carry error/);
});

test('request lifecycle rejects error with a result payload', () => {
  assert.throws(() => applyRequestPatch(request(), {
    phase: 'error', error: 'render failed', result,
  } as never), /cannot carry result/);
});

test('request lifecycle rejects arbitrary terminal to active patch', () => {
  assert.throws(() => applyRequestPatch(request({ phase: 'success', result }), {
    phase: 'rendering',
  }), /terminal request/);
});

test('Given an in-flight provisional result When request fails Then stale result is removed', () => {
  const failed = applyRequestPatch(request({ result }), {
    phase: 'error',
    error: 'render failed',
  });

  assert.equal(failed.phase, 'error');
  assert.equal(failed.result, null);
  assert.equal(failed.error, 'render failed');
});

test('Given an in-flight provisional result When request is canceled Then terminal payload is empty', () => {
  const canceled = applyRequestPatch(request({ result }), { phase: 'canceled' });

  assert.equal(canceled.phase, 'canceled');
  assert.equal(canceled.result, null);
  assert.equal(canceled.error, null);
});

test('Given a failed request When metadata tries to attach a result Then invalid terminal state is rejected', () => {
  assert.throws(() => applyRequestPatch(request({ phase: 'error', error: 'failed' }), {
    result,
  }), /Failed request cannot carry result/);
});
