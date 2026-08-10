import test from 'node:test';
import assert from 'node:assert/strict';

import { materializeViewerTopology } from './viewerTopologyBudget';

test('dense faceted topology stays query-only instead of becoming per-target scene objects', () => {
  assert.deepEqual(materializeViewerTopology(66_648, 44_432), {
    materialize: false,
    reason: 'targetBudgetExceeded',
  });
});

test('bounded exact topology remains directly selectable', () => {
  assert.deepEqual(materializeViewerTopology(48, 24), {
    materialize: true,
    reason: null,
  });
});
