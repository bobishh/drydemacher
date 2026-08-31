import assert from 'node:assert/strict';
import test from 'node:test';
import { createCaptureWorkspaceState, reduceCaptureWorkspace } from './captureWorkspaceState';

test('capture workspace runs pairing, capture, reconstruct, preview', () => {
    let state = createCaptureWorkspaceState();
    state = reduceCaptureWorkspace(state, { type: 'start', sessionToken: 'tok', runId: 'run', pairingUrl: 'pair', trustUrl: 'trust' });
    state = reduceCaptureWorkspace(state, { type: 'capture', acceptedFrameCount: 4 });
    state = reduceCaptureWorkspace(state, { type: 'reconstruct' });
    state = reduceCaptureWorkspace(state, { type: 'preview', meshPreview: { stlPath: '/scan.stl', triangleCount: 1, boundsMm: [1, 1, 1], scaleLabel: '1x', warnings: [] } });
    assert.equal(state.phase, 'preview');
    assert.equal(state.error, null);
});

test('capture workspace clears stale preview, progress and error on resume', () => {
    let state = reduceCaptureWorkspace(createCaptureWorkspaceState(), { type: 'start', sessionToken: 'tok', runId: 'run', pairingUrl: 'pair', trustUrl: 'trust' });
    state = reduceCaptureWorkspace(state, { type: 'fail', error: 'camera offline' });
    state = reduceCaptureWorkspace(state, { type: 'resume' });
    assert.deepEqual(state, createCaptureWorkspaceState());
});

test('capture workspace rejects invalid transitions', () => {
  assert.throws(() => reduceCaptureWorkspace(createCaptureWorkspaceState(), { type: 'preview', meshPreview: {} as never }), /Invalid capture transition/);
});

test('Given backend reports preview without mesh When state synchronizes Then impossible preview is rejected', () => {
  assert.throws(() => reduceCaptureWorkspace(createCaptureWorkspaceState(), {
    type: 'patch',
    patch: { phase: 'preview' },
  }), /preview requires mesh/);
});

test('Given prepared preview When reconstruction retries Then stale preview payload is removed', () => {
  let state = reduceCaptureWorkspace(createCaptureWorkspaceState(), {
    type: 'start', sessionToken: 'tok', runId: 'run', pairingUrl: 'pair', trustUrl: 'trust',
  });
  state = reduceCaptureWorkspace(state, { type: 'reconstruct' });
  state = reduceCaptureWorkspace(state, {
    type: 'preview',
    meshPreview: { stlPath: '/scan.stl', triangleCount: 1, boundsMm: [1, 1, 1], scaleLabel: '1x', warnings: [] },
  });
  state = reduceCaptureWorkspace(state, { type: 'patch', patch: { phase: 'reconstructing' } });

  assert.equal(state.phase, 'reconstructing');
  assert.equal(state.meshPreview, null);
  assert.equal(state.preview, null);
  assert.equal(state.error, null);
});
