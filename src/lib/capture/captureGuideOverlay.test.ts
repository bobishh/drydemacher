import assert from 'node:assert/strict';
import test from 'node:test';
import type { CaptureGuideSourceMesh, CaptureLandmarkRole, CaptureSurfaceAnchor } from '../tauri/contracts';
import { addCaptureLandmark, createCaptureGuideDraft, finalizeMechanicalGuideDraft } from './captureGuideDraft';
import { buildCaptureGuideOverlayPrimitives } from './captureGuideOverlay';

const sourceMesh: CaptureGuideSourceMesh = {
  artifactDigest: 'sha256:artifact',
  contentDigest: 'sha256:mesh',
  selection: 'raw',
  cropDigest: null,
  triangleCount: 20,
  sourceBounds: { min: [0, 0, 0], max: [10, 10, 2] },
};

function anchor(index: number, position: [number, number, number]): CaptureSurfaceAnchor {
  return {
    sourceMeshContentDigest: sourceMesh.contentDigest,
    triangleIndex: index,
    barycentric: [1, 0, 0],
    sourcePosition: position,
    sourceNormal: [0, 0, 1],
  };
}

test('guide overlay preserves landmark/profile order and labels inferred regions as unverified', () => {
  let guide = createCaptureGuideDraft('run-1', 'thread-1', null, 'sha256:target', null, sourceMesh);
  const add = (role: CaptureLandmarkRole, position: [number, number, number]) => {
    guide = addCaptureLandmark(guide, role, anchor(guide.landmarks.length, position));
  };
  add('calibrationEndpoint', [0, 0, 0]);
  add('calibrationEndpoint', [4, 0, 0]);
  add('frameOrigin', [0, 0, 0]);
  add('frameDirection', [4, 0, 0]);
  add('frameDirection', [0, 4, 0]);
  add('symmetrySample', [0, 0, 0]);
  add('symmetrySample', [0, 4, 0]);
  add('symmetrySample', [0, 0, 2]);
  add('profileVertex', [0, 0, 0]);
  add('profileVertex', [4, 0, 0]);
  add('profileVertex', [4, 4, 0]);
  add('rotationAxisEndpoint', [2, 2, 0]);
  add('rotationAxisEndpoint', [2, 2, 2]);
  guide = finalizeMechanicalGuideDraft(guide, 40, 'Build symmetric insert.');

  const overlay = buildCaptureGuideOverlayPrimitives(guide);
  assert.equal(overlay.landmarks.length, 13);
  assert.deepEqual(
    overlay.profileSegments.map(segment => [segment.fromLandmarkId, segment.toLandmarkId]),
    [
      ['landmark-9', 'landmark-10'],
      ['landmark-10', 'landmark-11'],
      ['landmark-11', 'landmark-9'],
    ],
  );
  assert.deepEqual(
    overlay.axisSegments.map(segment => [segment.fromLandmarkId, segment.toLandmarkId]),
    [['landmark-12', 'landmark-13']],
  );
  assert.equal(overlay.planeLoops[0].landmarkIds.length, 3);
  assert.equal(overlay.evidenceScopeLabel, 'OBSERVED REGION ONLY');
  assert.equal(overlay.inferredRegionLabel, 'INFERRED HALF · UNVERIFIED');
});
