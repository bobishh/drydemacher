import assert from 'node:assert/strict';
import test from 'node:test';
import { buildCaptureDeviationDisplayPoints } from './captureDeviationOverlay';

test('deviation display points keep local positions and classify tolerance without changing geometry', () => {
  const points = buildCaptureDeviationDisplayPoints([
    { sourceVertexIndex: 0, localPositionMm: [0, 0, 0], distanceMm: 0.04 },
    { sourceVertexIndex: 4, localPositionMm: [1, 2, 3], distanceMm: 0.12 },
    { sourceVertexIndex: 8, localPositionMm: [4, 5, 6], distanceMm: 0.4 },
  ], 0.2);

  assert.deepEqual(points.map(point => point.classification), ['within', 'near', 'outlier']);
  assert.deepEqual(points.map(point => point.localPositionMm), [[0, 0, 0], [1, 2, 3], [4, 5, 6]]);
  assert.deepEqual(points.map(point => point.color), ['#52c878', '#e2b24c', '#ef5b5b']);
});
