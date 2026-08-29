import assert from 'node:assert/strict';
import test from 'node:test';

import { pendingImageGeometry, pendingImageGeometryStatus } from './imageGeometryPending';

const uiSpec = {
  fields: [
    { type: 'image' as const, key: 'artwork', label: 'Artwork', frozen: false },
    { type: 'image' as const, key: 'relief', label: 'Relief', frozen: false },
    { type: 'image' as const, key: 'texture', label: 'Texture', frozen: false },
  ],
};

test('empty images referenced by raster extrude or protrude remain pending', () => {
  const pending = pendingImageGeometry(
    '(model (part logo (extrude artwork 2 :width 10 :depth 10)) (part map (protrude relief 3 :width 10 :depth 10)))',
    uiSpec,
    { artwork: '', relief: '' },
  );
  assert.deepEqual(pending, [
    { key: 'artwork', label: 'Artwork' },
    { key: 'relief', label: 'Relief' },
  ]);
  assert.equal(
    pendingImageGeometryStatus(pending),
    'Image geometry pending selection: Artwork, Relief. Select image, then apply.',
  );
});

test('sketch extrude and unrelated empty image fields do not block render', () => {
  assert.deepEqual(
    pendingImageGeometry(
      '(model (part body (extrude profile 2)) (part map (protrude relief 3 :width 10 :depth 10)))',
      uiSpec,
      { relief: '/tmp/map.png', texture: '' },
    ),
    [],
  );
});

test('legacy heightfield remains pending-compatible without being public API', () => {
  assert.deepEqual(
    pendingImageGeometry('(model (part relief (heightfield relief :width 10)))', uiSpec, {
      relief: '',
    }),
    [{ key: 'relief', label: 'Relief' }],
  );
});
