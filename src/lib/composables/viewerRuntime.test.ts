import test from 'node:test';
import assert from 'node:assert/strict';
import { canRepairSavedVersionRuntime, createViewerLoadRuntime } from './viewerRuntime';

test('Given a runtime target When repair is considered Then only the exact saved version is eligible', () => {
  assert.equal(canRepairSavedVersionRuntime(
    { kind: 'savedVersion', threadId: 'thread-1', messageId: 'version-1' },
    'thread-1',
    'version-1',
  ), true);
  assert.equal(canRepairSavedVersionRuntime(
    null,
    'thread-1',
    'version-1',
  ), true);
  assert.equal(canRepairSavedVersionRuntime(
    { kind: 'savedVersion', threadId: 'other-thread', messageId: 'version-x' },
    'thread-1',
    'version-1',
  ), true);
  assert.equal(canRepairSavedVersionRuntime(
    { kind: 'draft', threadId: 'thread-1', previewId: 'version-1', sessionId: 'session-1' },
    'thread-1',
    'version-1',
  ), false);
});

test('Given a pending visible viewer load When the viewer reports loaded Then the waiter resolves', async () => {
  const runtime = createViewerLoadRuntime();
  const pending = runtime.waitForLoad('visible', 0, 50);

  runtime.markLoaded('visible');

  await pending;
  assert.equal(runtime.loadNonce('visible'), 1);
});

test('Given a pending hidden viewer load When it reports an error Then the waiter rejects with the raw error', async () => {
  const runtime = createViewerLoadRuntime();
  const pending = runtime.waitForLoad('hidden', 0, 50);

  runtime.markFailed('hidden', 'asset responded with 404');

  await assert.rejects(pending, /Hidden viewer failed to load model\. asset responded with 404/);
});

test('Given a wait exceeds its timeout When no model loads Then it rejects and clears the pending waiter', async () => {
  const runtime = createViewerLoadRuntime();

  await assert.rejects(runtime.waitForLoad('visible', 0, 1), /Timed out waiting for the visible viewer to load/);
  assert.equal(runtime.pendingCount('visible'), 0);
});
