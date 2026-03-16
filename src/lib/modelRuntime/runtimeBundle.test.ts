import assert from 'node:assert/strict';
import test from 'node:test';

import type { ArtifactBundle } from '../types/domain';
import { getRenderableRuntimeBundle, inspectRuntimeBundle } from './runtimeBundle';

function sampleBundle(): ArtifactBundle {
  return {
    schemaVersion: 1,
    modelId: 'generated-test',
    sourceKind: 'generated',
    contentHash: 'hash',
    artifactVersion: 1,
    fcstdPath: '/tmp/model.FCStd',
    manifestPath: '/tmp/manifest.json',
    macroPath: '/tmp/source.FCMacro',
    previewStlPath: '/tmp/preview.stl',
    viewerAssets: [
      {
        partId: 'part-a',
        nodeId: 'node-a',
        objectName: 'Body',
        label: 'Body',
        path: '/tmp/parts/body.stl',
        format: 'stl',
      },
    ],
  };
}

test('inspectRuntimeBundle preserves multipart runtime when all files exist', async () => {
  const bundle = sampleBundle();
  const result = await inspectRuntimeBundle(bundle, async () => true);

  assert.equal(result.previewAvailable, true);
  assert.equal(result.degradedToPreview, false);
  assert.deepEqual(result.bundle, bundle);
});

test('inspectRuntimeBundle degrades to preview-only when any viewer asset is missing', async () => {
  const bundle = sampleBundle();
  const result = await inspectRuntimeBundle(bundle, async (path) => path !== '/tmp/parts/body.stl');

  assert.equal(result.previewAvailable, true);
  assert.equal(result.degradedToPreview, true);
  assert.deepEqual(result.bundle?.viewerAssets, []);
  assert.equal(result.bundle?.previewStlPath, '/tmp/preview.stl');
});

test('inspectRuntimeBundle invalidates the runtime bundle when preview STL is missing', async () => {
  const bundle = sampleBundle();
  const result = await inspectRuntimeBundle(bundle, async (path) => path !== '/tmp/preview.stl');

  assert.equal(result.previewAvailable, false);
  assert.equal(result.degradedToPreview, false);
  assert.equal(result.bundle, null);
});

test('getRenderableRuntimeBundle forces preview STL when displacement post-processing is active', () => {
  const bundle = sampleBundle();
  const result = getRenderableRuntimeBundle(bundle, {
    displacement: {
      imageParam: '__litho_image',
      projection: 'planar',
      depthMm: 3,
      invert: false,
    },
  });

  assert.deepEqual(result?.viewerAssets, []);
  assert.equal(result?.previewStlPath, '/tmp/preview.stl');
});

test('inspectRuntimeBundle degrades to preview-only when displacement post-processing is active', async () => {
  const bundle = sampleBundle();
  const result = await inspectRuntimeBundle(
    bundle,
    async () => true,
    {
      displacement: {
        imageParam: '__litho_image',
        projection: 'planar',
        depthMm: 3,
        invert: false,
      },
    },
  );

  assert.equal(result.previewAvailable, true);
  assert.equal(result.degradedToPreview, true);
  assert.deepEqual(result.bundle?.viewerAssets, []);
  assert.equal(result.bundle?.previewStlPath, '/tmp/preview.stl');
});

test('getRenderableRuntimeBundle forces preview STL when lithophane attachments are active', () => {
  const bundle = sampleBundle();
  const result = getRenderableRuntimeBundle(bundle, {
    displacement: null,
    lithophaneAttachments: [
      {
        id: 'panel',
        enabled: true,
        source: { kind: 'file', imagePath: '/tmp/panel.png' },
        targetPartId: 'part-a',
        placement: {
          mode: 'partSidePatch',
          side: 'front',
          projection: 'auto',
        },
        relief: {
          depthMm: 2,
          invert: false,
        },
        color: {
          mode: 'mono',
          channelThicknessMm: 0.4,
        },
      },
    ],
  });

  assert.deepEqual(result?.viewerAssets, []);
  assert.equal(result?.previewStlPath, '/tmp/preview.stl');
});
