import assert from 'node:assert/strict';
import test from 'node:test';
import type { CaptureGuideSourceMesh, CaptureLandmarkRole, CaptureSurfaceAnchor } from '../tauri/contracts';
import {
  addCaptureLandmark,
  applyCaptureGuideDraftEdit,
  configureCaptureProfile,
  createCaptureGuideDraft,
  createCaptureGuideDraftHistory,
  finalizeMechanicalGuideDraft,
  mechanicalGuideReadiness,
  moveCaptureProfileLandmark,
  removeCaptureLandmark,
  removeLastCaptureLandmark,
  undoCaptureGuideDraftEdit,
  updateCaptureFeatureExpectation,
  updateCaptureLandmark,
} from './captureGuideDraft';

const sourceMesh: CaptureGuideSourceMesh = {
  artifactDigest: 'sha256:artifact',
  contentDigest: 'sha256:mesh',
  selection: 'raw',
  cropDigest: null,
  triangleCount: 10,
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

test('draft collects explicit mechanical roles, preserves profile order, and undo stays draft-only', () => {
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

  assert.equal(mechanicalGuideReadiness(guide).ready, true);
  const evidenceOnly = finalizeMechanicalGuideDraft(guide, 40, '');
  assert.equal(evidenceOnly.instruction, '');
  const finalized = finalizeMechanicalGuideDraft(guide, 40, 'Build insert around named mating geometry.');
  assert.equal(finalized.calibration.measurements[0].knownDistanceMm, 40);
  assert.deepEqual(finalized.reconstructionFrame.sourceLandmarkIds.map(id => finalized.landmarks.find(item => item.landmarkId === id)?.role), ['frameOrigin', 'frameDirection', 'frameDirection']);
  assert.deepEqual(
    finalized.profiles[0].landmarkIds,
    guide.landmarks.filter(item => item.role === 'profileVertex').map(item => item.landmarkId),
  );
  assert.equal(finalized.symmetryCompletion.kind, 'half');

  const undone = removeLastCaptureLandmark(guide);
  assert.equal(undone.landmarks.length, guide.landmarks.length - 1);
  assert.equal(guide.landmarks.length, 11);
  assert.equal(mechanicalGuideReadiness(undone).ready, false);
});

test('landmark edit/delete and generic undo preserve draft identity without model history', () => {
  let guide = createCaptureGuideDraft('run-1', 'thread-1', null, 'sha256:target', null, sourceMesh);
  guide = addCaptureLandmark(guide, 'namedReference', anchor(0, [1, 2, 0]));
  guide = addCaptureLandmark(guide, 'outerExtent', anchor(1, [3, 2, 0]));
  const originalGuideId = guide.guideId;
  let history = createCaptureGuideDraftHistory(guide);

  history = applyCaptureGuideDraftEdit(
    history,
    updateCaptureLandmark(history.present, 'landmark-1', {
      label: 'datum A',
      role: 'matingSurfaceSample',
    }),
  );
  assert.equal(history.present.landmarks[0].label, 'datum A');
  assert.equal(history.present.landmarks[0].role, 'matingSurfaceSample');

  history = applyCaptureGuideDraftEdit(
    history,
    removeCaptureLandmark(history.present, 'landmark-2'),
  );
  assert.deepEqual(history.present.landmarks.map(item => item.landmarkId), ['landmark-1']);
  history = undoCaptureGuideDraftEdit(history);
  assert.deepEqual(history.present.landmarks.map(item => item.landmarkId), ['landmark-1', 'landmark-2']);
  assert.equal(history.present.guideId, originalGuideId);
  assert.equal(history.present.revision, guide.revision);
});

test('profile order/kind and exact target contract remain explicit editable guide data', () => {
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
  guide = finalizeMechanicalGuideDraft(guide, 40, 'Build exact insert.');

  guide = moveCaptureProfileLandmark(guide, 'profile-1', 'landmark-11', 0);
  guide = configureCaptureProfile(guide, 'profile-1', {
    kind: 'open',
    operationHint: 'sweep',
    label: 'mating rail',
  });
  assert.deepEqual(guide.profiles[0].landmarkIds, ['landmark-11', 'landmark-9', 'landmark-10']);
  assert.equal(guide.profiles[0].kind, 'open');
  assert.equal(guide.profiles[0].operationHint, 'sweep');

  guide = updateCaptureFeatureExpectation(guide, 'expectation-profile-edges', {
    expectedGeometryKind: 'profile',
    requiredBrepTopologyKind: 'orderedEdges',
    cardinality: 'oneOrMore',
    partId: 'insert-body',
    instancePath: 'mirror-y/instance-2',
    expectedAuthoredSelector: { kind: 'binding', name: 'mating_rail_edges' },
  });
  assert.deepEqual(guide.featureExpectations[1].expectedAuthoredSelector, {
    kind: 'binding',
    name: 'mating_rail_edges',
  });
  assert.equal(guide.featureExpectations[1].instancePath, 'mirror-y/instance-2');
  assert.equal(guide.canonicalDigest, '');
});
