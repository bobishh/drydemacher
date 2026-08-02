import assert from 'node:assert/strict';
import test from 'node:test';

import { createCampaignPreviewCache } from './campaignPreviewCache';
import type { ArtifactBundle } from '../types/domain';

const artifact = (modelId: string): ArtifactBundle => ({
  modelId,
  sourceKind: 'generated',
  contentHash: modelId,
  fcstdPath: '',
  manifestPath: '',
  previewStlPath: `/previews/${modelId}.stl`,
});

test('Given an identical edited source/runtime/backend When preview is warm Then cache returns its immutable artifact', () => {
  const cache = createCampaignPreviewCache();
  const identity = { source: '(model)', runtimeDigest: 'runtime-a', backend: 'mesh' };
  cache.put(identity, artifact('warm'));

  assert.equal(cache.get(identity)?.modelId, 'warm');
});

test('Given a source or runtime change When preview is requested Then cache misses', () => {
  const cache = createCampaignPreviewCache();
  cache.put({ source: '(model)', runtimeDigest: 'runtime-a', backend: 'mesh' }, artifact('warm'));

  assert.equal(cache.get({ source: '(model edited)', runtimeDigest: 'runtime-a', backend: 'mesh' }), null);
  assert.equal(cache.get({ source: '(model)', runtimeDigest: 'runtime-b', backend: 'mesh' }), null);
});
