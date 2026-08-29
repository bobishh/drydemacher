import assert from 'node:assert/strict';
import test from 'node:test';

import { deriveViewerBusyState } from './viewerBusyState';

test('backend render lock activity is the only viewport preloader signal', () => {
  assert.deepEqual(deriveViewerBusyState({ geometryRenderActive: true }), {
    showViewerBusyMask: true,
    viewerBusyPhase: 'rendering',
    viewerBusyText: 'Rendering geometry.',
  });
});

test('viewport preloader stays absent without backend render lock activity', () => {
  assert.deepEqual(deriveViewerBusyState({ geometryRenderActive: false }), {
    showViewerBusyMask: false,
    viewerBusyPhase: null,
    viewerBusyText: null,
  });
});
