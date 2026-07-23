import assert from 'node:assert/strict';
import test from 'node:test';

import { pendingHeightfieldImages, pendingHeightfieldStatus } from './heightfieldPending';

const uiSpec = {
  fields: [
    { type: 'image' as const, key: 'heightmap', label: 'Height Map', frozen: false },
    { type: 'image' as const, key: 'texture', label: 'Texture', frozen: false },
  ],
};

test('direct empty heightfield image parameter remains pending', () => {
  const pending = pendingHeightfieldImages(
    '(model (params (image heightmap "")) (part relief (heightfield heightmap :width 10)))',
    uiSpec,
    { heightmap: '' },
  );
  assert.deepEqual(pending, [{ key: 'heightmap', label: 'Height Map' }]);
  assert.equal(
    pendingHeightfieldStatus(pending),
    'Heightfield pending image selection: Height Map. Select image, then apply.',
  );
});

test('selected heightfield path clears pending state', () => {
  assert.deepEqual(
    pendingHeightfieldImages(
      '(model (part relief (heightfield heightmap :width 10)))',
      uiSpec,
      { heightmap: '/tmp/map.png' },
    ),
    [],
  );
});

test('unrelated empty image field does not block heightfield render', () => {
  assert.deepEqual(
    pendingHeightfieldImages(
      '(model (part relief (heightfield heightmap :width 10)))',
      uiSpec,
      { heightmap: '/tmp/map.png', texture: '' },
    ),
    [],
  );
});
