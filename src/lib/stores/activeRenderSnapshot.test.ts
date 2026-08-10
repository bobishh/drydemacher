import assert from 'node:assert/strict';
import test from 'node:test';

import { buildActiveRenderSnapshot, RenderSnapshotMismatch } from './activeRenderSnapshot';
import type { ActiveRenderSnapshotInput } from './activeRenderSnapshot';

function input(): ActiveRenderSnapshotInput {
  return {
    threadId: 'thread-1',
    messageId: 'preview-1',
    design: {
      title: 'Model',
      versionName: '',
      response: '',
      interactionMode: 'design',
      macroCode: '(model (part body (box width 1 1)))',
      macroDialect: 'ecky',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
      engineKind: 'ecky',
      uiSpec: { fields: [] },
      initialParams: { width: 10, height: 20 },
      postProcessing: null,
    },
    artifactBundle: {
      modelId: 'model-1',
      sourceKind: 'generated',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
      engineKind: 'ecky',
      contentHash: 'hash-1',
      artifactVersion: 1,
      fcstdPath: '',
      manifestPath: '/model-1.json',
      previewStlPath: '/model-1.stl',
      viewerAssets: [],
      calloutAnchors: [],
      measurementGuides: [],
      edgeTargets: [],
      faceTargets: [],
      exportArtifacts: [],
    },
    modelManifest: {
      modelId: 'model-1',
      sourceKind: 'generated',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
      engineKind: 'ecky',
      document: { documentName: 'Model', documentLabel: 'Model', objectCount: 0, warnings: [] },
      parts: [],
      parameterGroups: [],
      controlPrimitives: [],
      controlRelations: [],
      controlViews: [],
      selectionTargets: [],
      taggedAnchors: {},
      analysisDeclarations: [],
      advisories: [],
      measurementAnnotations: [],
      warnings: [],
      enrichmentState: { status: 'none', proposals: [] },
    },
    selectedPartId: null,
    stlUrl: 'asset://model-1.stl',
    status: 'Preview rendered.',
  };
}

test('Given artifact and manifest identities differ When snapshot builds Then mismatch names both identities', () => {
  const base = input();
  const candidate = {
    ...base,
    modelManifest: { ...base.modelManifest, modelId: 'model-2' },
  };

  assert.throws(
    () => buildActiveRenderSnapshot(candidate),
    (error: unknown) => error instanceof RenderSnapshotMismatch
      && error.message.includes('model-1')
      && error.message.includes('model-2'),
  );
});

test('canonical parameter order produces one frontend projection identity', () => {
  const first = buildActiveRenderSnapshot(input());
  const base = input();
  const reordered = {
    ...base,
    design: {
      ...base.design,
      initialParams: { height: 20, width: 10 },
    },
  };

  assert.equal(buildActiveRenderSnapshot(reordered).snapshotId, first.snapshotId);
});
