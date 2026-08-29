import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveVersionLoupeRuntime } from './versionLoupeRuntime';
import type { ArtifactBundle, Message } from './types/domain';

function bundle(): ArtifactBundle {
  return {
    schemaVersion: 2,
    modelId: 'model-1',
    sourceKind: 'generated',
    contentHash: 'hash-1',
    artifactVersion: 1,
    fcstdPath: '/tmp/model.FCStd',
    manifestPath: '/tmp/model.json',
    modelStlPath: '/tmp/model.stl',
    viewerAssets: [
      {
        partId: 'body',
        nodeId: 'body-node',
        objectName: 'Body001',
        label: 'Body',
        path: '/tmp/body.stl',
        format: 'stl',
      },
    ],
    edgeTargets: [],
    calloutAnchors: [],
    measurementGuides: [],
  };
}

function message(): Pick<Message, 'id' | 'artifactBundle' | 'modelManifest' | 'output'> {
  return {
    id: 'version-1',
    artifactBundle: bundle(),
    modelManifest: {
      modelId: 'model-1',
      sourceKind: 'generated',
      sourceLanguage: 'legacyPython',
      geometryBackend: 'freecad',
      document: {
        documentName: 'Test',
        documentLabel: 'Test',
        objectCount: 1,
        warnings: [],
      },
      taggedAnchors: {},
      analysisDeclarations: [],
      parts: [],
      parameterGroups: [],
      controlPrimitives: [],
      controlRelations: [],
      controlViews: [],
      selectionTargets: [],
      advisories: [],
      measurementAnnotations: [],
      warnings: [],
      enrichmentState: { status: 'none', proposals: [] },
    },
    output: {
      title: 'Test',
      versionName: 'V1',
      response: '',
      interactionMode: 'design',
      macroCode: 'cube()',
      sourceLanguage: 'legacyPython',
      geometryBackend: 'freecad',
      uiSpec: { fields: [] },
      initialParams: {},
      postProcessing: null,
    },
  };
}

test('resolveVersionLoupeRuntime returns renderable preview urls when runtime exists', async () => {
  const resolved = await resolveVersionLoupeRuntime(
    message(),
    'thread-1',
    (path) => `asset:${path ?? ''}`,
    {
      materializePreview: async () => ({
        artifactBundle: bundle(),
        modelManifest: message().modelManifest!,
        leaseId: null,
        ephemeral: false,
      }),
    },
  );

  assert.equal(resolved.available, true);
  assert.equal(resolved.previewUrl, 'asset:/tmp/model.stl');
  assert.equal(resolved.viewerAssets[0]?.path, 'asset:/tmp/body.stl');
});

test('resolveVersionLoupeRuntime hides the viewer when runtime is gone', async () => {
  const resolved = await resolveVersionLoupeRuntime(
    {
      ...message(),
      output: null,
      artifactBundle: null,
    },
    'thread-1',
    (path) => `asset:${path ?? ''}`,
    {
      getThreadMessageVersion: async () => null,
      materializePreview: async () => assert.fail('runtime must not materialize without a version'),
    },
  );

  assert.equal(resolved.available, false);
  assert.equal(resolved.previewUrl, null);
  assert.deepEqual(resolved.viewerAssets, []);
  assert.equal(resolved.leaseId, null);
});

test('resolveVersionLoupeRuntime materializes an ephemeral lease for an old version', async () => {
  const calls: string[] = [];
  const resolved = await resolveVersionLoupeRuntime(
    message(),
    'thread-1',
    (path) => `asset:${path ?? ''}`,
    {
      materializePreview: async (threadId, messageId) => {
        calls.push(`materialize:${threadId}:${messageId}`);
        return {
          artifactBundle: {
            ...bundle(),
            modelStlPath: '/tmp/history-preview/lease-1/model.stl',
          },
          modelManifest: message().modelManifest!,
          leaseId: 'lease-1',
          ephemeral: true,
        };
      },
    },
  );

  assert.equal(resolved.available, true);
  assert.equal(resolved.previewUrl, 'asset:/tmp/history-preview/lease-1/model.stl');
  assert.equal(resolved.leaseId, 'lease-1');
  assert.deepEqual(calls, ['materialize:thread-1:version-1']);
});
