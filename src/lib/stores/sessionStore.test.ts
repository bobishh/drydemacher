import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import { get } from 'svelte/store';
import { session } from './sessionStore';
import type { ArtifactBundle, ModelManifest } from '../types/domain';

function runtimeFixture(modelId = 'model-1'): {
  artifactBundle: ArtifactBundle;
  modelManifest: ModelManifest;
} {
  return {
    artifactBundle: {
      modelId,
      sourceKind: 'generated',
      sourceLanguage: 'ecky',
      geometryBackend: 'mesh',
      engineKind: 'ecky',
      contentHash: `hash-${modelId}`,
      artifactVersion: 1,
      fcstdPath: '',
      manifestPath: `/${modelId}.json`,
      modelStlPath: `/${modelId}.stl`,
      viewerAssets: [],
      calloutAnchors: [],
      measurementGuides: [],
      edgeTargets: [],
      faceTargets: [],
      exportArtifacts: [],
    },
    modelManifest: {
      modelId,
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
  };
}

afterEach(() => {
  session.setError(null);
  session.setGlobalError(null);
  session.clearRuntime();
});

test('global app errors stay separate from thread session errors', () => {
  session.setError('thread failure');
  session.setGlobalError('config save failure');

  assert.equal(get(session).error, 'thread failure');
  assert.equal(get(session).globalError, 'config save failure');
});

test('setting the same model URL does not reload geometry', () => {
  session.setStlUrl('/tmp/model.stl');
  const loaded = get(session);

  session.setStlUrl('/tmp/model.stl');

  assert.equal(get(session).runtimeRevision, loaded.runtimeRevision);
});

test('explicit runtime reload retries the same model URL', () => {
  session.setStlUrl('/tmp/model.stl');
  const loaded = get(session);

  session.reloadStlUrl('/tmp/model.stl');

  assert.equal(get(session).stlUrl, '/tmp/model.stl');
  assert.equal(get(session).runtimeRevision, loaded.runtimeRevision + 1);
});

test('Given a rendered model When runtime is published Then URL, bundle, manifest, and selection change atomically', () => {
  const { artifactBundle, modelManifest } = runtimeFixture();

  session.setRuntime({
    kind: 'model',
    stlUrl: 'asset://model-1.stl',
    artifactBundle,
    modelManifest,
    selectedPartId: null,
  });

  const loaded = get(session);
  assert.equal(loaded.runtime.kind, 'model');
  assert.equal(loaded.stlUrl, 'asset://model-1.stl');
  assert.equal(loaded.artifactBundle?.modelId, 'model-1');
  assert.equal(loaded.modelManifest?.modelId, 'model-1');
});

test('Given mismatched model identities When runtime is published Then session rejects the invalid state', () => {
  const { artifactBundle } = runtimeFixture('artifact-model');
  const { modelManifest } = runtimeFixture('manifest-model');

  assert.throws(() => session.setRuntime({
    kind: 'model',
    stlUrl: 'asset://artifact-model.stl',
    artifactBundle,
    modelManifest,
    selectedPartId: null,
  }), /modelId mismatch/);
});

test('Given a loaded model When geometry URL changes Then stale model metadata cannot survive', () => {
  const { artifactBundle, modelManifest } = runtimeFixture();
  session.setRuntime({
    kind: 'model',
    stlUrl: 'asset://model-1.stl',
    artifactBundle,
    modelManifest,
    selectedPartId: null,
  });

  session.setStlUrl('asset://other.stl');

  const loaded = get(session);
  assert.equal(loaded.runtime.kind, 'geometryOnly');
  assert.equal(loaded.artifactBundle, null);
  assert.equal(loaded.modelManifest, null);
  assert.equal(loaded.selectedPartId, null);
});
