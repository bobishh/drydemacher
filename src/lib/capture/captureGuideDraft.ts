import type {
  CaptureAuthoredSelector,
  CaptureExpectedGeometryKind,
  CaptureFeatureExpectation,
  CaptureGuideSourceMesh,
  CaptureLandmark,
  CaptureLandmarkRole,
  CaptureOrderedProfile,
  CaptureProfileKind,
  CaptureProfileOperationHint,
  CaptureReconstructionGuide,
  CaptureRequiredBrepTopologyKind,
  CaptureSelectorCardinality,
  CaptureSurfaceAnchor,
} from '../tauri/contracts';

export type MechanicalGuideReadiness = {
  ready: boolean;
  reasons: string[];
};

export type CaptureGuideDraftHistory = {
  past: CaptureReconstructionGuide[];
  present: CaptureReconstructionGuide;
};

export type CaptureLandmarkEdit = Pick<CaptureLandmark, 'label' | 'role'>;

export type CaptureProfileEdit = Partial<Pick<
  CaptureOrderedProfile,
  'label' | 'kind' | 'operationHint' | 'supportPlaneId' | 'featureLabel' | 'fitRole'
>>;

export type CaptureFeatureExpectationEdit = Partial<{
  label: string;
  expectedGeometryKind: CaptureExpectedGeometryKind;
  requiredBrepTopologyKind: CaptureRequiredBrepTopologyKind;
  cardinality: CaptureSelectorCardinality;
  partId: string;
  instancePath: string | null;
  expectedAuthoredSelector: CaptureAuthoredSelector;
  requiredForAcceptance: boolean;
  positionToleranceMm: number | null;
  normalToleranceDeg: number | null;
  radialToleranceMm: number | null;
}>;

function cloneGuide(guide: CaptureReconstructionGuide): CaptureReconstructionGuide {
  return JSON.parse(JSON.stringify(guide)) as CaptureReconstructionGuide;
}

function invalidateComputedEvidence(guide: CaptureReconstructionGuide) {
  guide.surfaceNeighborhoods = [];
  guide.primitiveCandidates = [];
  guide.primitiveHypotheses = [];
  guide.surfaceRegions = [];
  guide.regionAdjacency = [];
  guide.reconstructedProfiles = [];
  guide.constraintGraph = { dimensions: [], relations: [], contentDigest: '' };
  guide.featurePlanCandidates = [];
  guide.selectedFeaturePlanId = null;
  guide.reconstructionReadiness = {
    ready: false,
    stages: [],
    missingStages: [],
    ambiguousStages: [],
    selectedFeaturePlanId: null,
    detail: '',
  };
  guide.landmarks = guide.landmarks.map(landmark => ({ ...landmark, uncertaintyMm: null }));
}

export function createCaptureGuideDraft(
  captureRunId: string,
  targetThreadId: string,
  targetMessageId: string | null,
  targetSourceDigest: string,
  targetVersionId: string | null,
  sourceMesh: CaptureGuideSourceMesh,
): CaptureReconstructionGuide {
  return {
    schemaVersion: 1,
    guideId: crypto.randomUUID(),
    revision: 0,
    captureRunId,
    targetThreadId,
    targetMessageId,
    targetSourceDigest,
    targetVersionId,
    sourceMesh,
    calibration: {
      sourceUnits: 'sourceUnit',
      millimetresPerSourceUnit: 1,
      method: { kind: 'knownDistance' },
      measurements: [],
      residualMm: 0,
    },
    reconstructionFrame: {
      originMm: [0, 0, 0],
      xAxis: [1, 0, 0],
      yAxis: [0, 1, 0],
      zAxis: [0, 0, 1],
      sourceLandmarkIds: [],
    },
    landmarks: [],
    evidenceComputationPolicy: {
      neighborhoodRadiusMm: 2,
      maxNeighborhoodTriangles: 64,
    },
    surfaceNeighborhoods: [],
    primitiveCandidates: [],
    primitiveHypotheses: [],
    surfaceRegions: [],
    regionAdjacency: [],
    reconstructedProfiles: [],
    authoredConstraints: [],
    constraintGraph: { dimensions: [], relations: [], contentDigest: '' },
    featurePlanCandidates: [],
    selectedFeaturePlanId: null,
    stageBypasses: [],
    reconstructionReadiness: {
      ready: false,
      stages: [],
      missingStages: [],
      ambiguousStages: [],
      selectedFeaturePlanId: null,
      detail: '',
    },
    featureExpectations: [],
    measurements: [],
    axes: [],
    planes: [],
    profiles: [],
    ignoredRegions: [],
    remapProposals: [],
    symmetryCompletion: { kind: 'none' },
    instruction: '',
    evidenceViews: [],
    canonicalDigest: '',
  };
}

export function addCaptureLandmark(
  guide: CaptureReconstructionGuide,
  role: CaptureLandmarkRole,
  anchor: CaptureSurfaceAnchor,
): CaptureReconstructionGuide {
  if (anchor.sourceMeshContentDigest !== guide.sourceMesh.contentDigest) {
    throw new Error('Capture anchor mesh digest differs from guide source mesh.');
  }
  const next = cloneGuide(guide);
  const nextOrdinal = next.landmarks.reduce((maximum, landmark) => {
    const match = /^landmark-(\d+)$/.exec(landmark.landmarkId);
    return Math.max(maximum, match ? Number(match[1]) : 0);
  }, 0) + 1;
  next.landmarks.push({
    landmarkId: `landmark-${nextOrdinal}`,
    label: `${role} ${nextOrdinal}`,
    role,
    anchor,
    localPositionMm: [...anchor.sourcePosition],
    localNormal: [...anchor.sourceNormal],
    uncertaintyMm: null,
  });
  invalidateComputedEvidence(next);
  next.canonicalDigest = '';
  return next;
}

export function removeLastCaptureLandmark(
  guide: CaptureReconstructionGuide,
): CaptureReconstructionGuide {
  const last = guide.landmarks.at(-1);
  return last ? removeCaptureLandmark(guide, last.landmarkId) : cloneGuide(guide);
}

export function updateCaptureLandmark(
  guide: CaptureReconstructionGuide,
  landmarkId: string,
  edit: CaptureLandmarkEdit,
): CaptureReconstructionGuide {
  const next = cloneGuide(guide);
  const landmark = next.landmarks.find(item => item.landmarkId === landmarkId);
  if (!landmark) throw new Error(`Capture landmark '${landmarkId}' does not exist.`);
  const label = edit.label.trim();
  if (!label) throw new Error('Capture landmark label is required.');
  landmark.label = label;
  landmark.role = edit.role;
  invalidateComputedEvidence(next);
  next.canonicalDigest = '';
  return next;
}

export function removeCaptureLandmark(
  guide: CaptureReconstructionGuide,
  landmarkId: string,
): CaptureReconstructionGuide {
  if (!guide.landmarks.some(item => item.landmarkId === landmarkId)) {
    throw new Error(`Capture landmark '${landmarkId}' does not exist.`);
  }
  const next = cloneGuide(guide);
  next.landmarks = next.landmarks.filter(item => item.landmarkId !== landmarkId);
  next.surfaceNeighborhoods = (next.surfaceNeighborhoods ?? [])
    .filter(item => item.landmarkId !== landmarkId);
  const survivingNeighborhoodIds = new Set(
    next.surfaceNeighborhoods.map(item => item.neighborhoodId),
  );
  next.primitiveCandidates = (next.primitiveCandidates ?? []).filter(candidate =>
    candidate.neighborhoodIds.every(id => survivingNeighborhoodIds.has(id)));
  const ids = new Set(next.landmarks.map(landmark => landmark.landmarkId));
  next.calibration.measurements = next.calibration.measurements.filter(measurement =>
    ids.has(measurement.firstLandmarkId) && ids.has(measurement.secondLandmarkId));
  next.reconstructionFrame.sourceLandmarkIds = next.reconstructionFrame.sourceLandmarkIds.filter(id => ids.has(id));
  next.measurements = next.measurements.filter(item => item.landmarkIds.every(id => ids.has(id)));
  next.axes = next.axes.filter(item => item.landmarkIds.every(id => ids.has(id)));
  next.planes = next.planes.filter(item => item.landmarkIds.every(id => ids.has(id)));
  next.profiles = next.profiles.filter(item => item.landmarkIds.every(id => ids.has(id)));
  next.ignoredRegions = next.ignoredRegions.filter(item => item.landmarkIds.every(id => ids.has(id)));
  const guideItemIds = new Set([
    ...next.landmarks.map(item => item.landmarkId),
    ...next.measurements.map(item => item.measurementId),
    ...next.axes.map(item => item.axisId),
    ...next.planes.map(item => item.planeId),
    ...next.profiles.map(item => item.profileId),
    ...next.ignoredRegions.map(item => item.regionId),
  ]);
  next.featureExpectations = next.featureExpectations.filter(expectation =>
    expectation.guideItemIds.every(id => guideItemIds.has(id)));
  const symmetryCompletion = next.symmetryCompletion;
  if (symmetryCompletion.kind === 'half'
    && !next.planes.some(item => item.planeId === symmetryCompletion.planeId)) {
    next.symmetryCompletion = { kind: 'none' };
  }
  if (symmetryCompletion.kind === 'quarter'
    && (!next.planes.some(item => item.planeId === symmetryCompletion.firstPlaneId)
      || !next.planes.some(item => item.planeId === symmetryCompletion.secondPlaneId))) {
    next.symmetryCompletion = { kind: 'none' };
  }
  invalidateComputedEvidence(next);
  next.canonicalDigest = '';
  return next;
}

export function createCaptureGuideDraftHistory(
  guide: CaptureReconstructionGuide,
): CaptureGuideDraftHistory {
  return { past: [], present: cloneGuide(guide) };
}

export function applyCaptureGuideDraftEdit(
  history: CaptureGuideDraftHistory,
  guide: CaptureReconstructionGuide,
): CaptureGuideDraftHistory {
  return {
    past: [...history.past.slice(-99), cloneGuide(history.present)],
    present: cloneGuide(guide),
  };
}

export function undoCaptureGuideDraftEdit(
  history: CaptureGuideDraftHistory,
): CaptureGuideDraftHistory {
  const previous = history.past.at(-1);
  if (!previous) return { past: [], present: cloneGuide(history.present) };
  return {
    past: history.past.slice(0, -1).map(cloneGuide),
    present: cloneGuide(previous),
  };
}

export function moveCaptureProfileLandmark(
  guide: CaptureReconstructionGuide,
  profileId: string,
  landmarkId: string,
  targetIndex: number,
): CaptureReconstructionGuide {
  const next = cloneGuide(guide);
  const profile = next.profiles.find(item => item.profileId === profileId);
  if (!profile) throw new Error(`Capture profile '${profileId}' does not exist.`);
  const sourceIndex = profile.landmarkIds.indexOf(landmarkId);
  if (sourceIndex < 0) {
    throw new Error(`Capture profile '${profileId}' does not contain landmark '${landmarkId}'.`);
  }
  if (!Number.isInteger(targetIndex)) throw new Error('Capture profile target index must be an integer.');
  const [moved] = profile.landmarkIds.splice(sourceIndex, 1);
  profile.landmarkIds.splice(Math.max(0, Math.min(targetIndex, profile.landmarkIds.length)), 0, moved);
  next.canonicalDigest = '';
  return next;
}

export function configureCaptureProfile(
  guide: CaptureReconstructionGuide,
  profileId: string,
  edit: CaptureProfileEdit,
): CaptureReconstructionGuide {
  const next = cloneGuide(guide);
  const profile = next.profiles.find(item => item.profileId === profileId);
  if (!profile) throw new Error(`Capture profile '${profileId}' does not exist.`);
  if (edit.label !== undefined) {
    const label = edit.label.trim();
    if (!label) throw new Error('Capture profile label is required.');
    profile.label = label;
  }
  if (edit.kind !== undefined) profile.kind = edit.kind as CaptureProfileKind;
  if (edit.operationHint !== undefined) {
    profile.operationHint = edit.operationHint as CaptureProfileOperationHint;
  }
  if (edit.supportPlaneId !== undefined) profile.supportPlaneId = edit.supportPlaneId;
  if (edit.featureLabel !== undefined) profile.featureLabel = edit.featureLabel?.trim() || null;
  if (edit.fitRole !== undefined) profile.fitRole = edit.fitRole?.trim() || null;
  next.canonicalDigest = '';
  return next;
}

export function updateCaptureFeatureExpectation(
  guide: CaptureReconstructionGuide,
  expectationId: string,
  edit: CaptureFeatureExpectationEdit,
): CaptureReconstructionGuide {
  const next = cloneGuide(guide);
  const expectation = next.featureExpectations.find(item => item.expectationId === expectationId);
  if (!expectation) throw new Error(`Capture expectation '${expectationId}' does not exist.`);
  Object.assign(expectation, edit as Partial<CaptureFeatureExpectation>);
  expectation.label = expectation.label.trim();
  expectation.partId = expectation.partId.trim();
  expectation.instancePath = expectation.instancePath?.trim() || null;
  expectation.expectedAuthoredSelector.name = expectation.expectedAuthoredSelector.name.trim();
  if (!expectation.label || !expectation.partId || !expectation.expectedAuthoredSelector.name) {
    throw new Error('Capture expectation label, part, and authored selector are required.');
  }
  next.canonicalDigest = '';
  return next;
}

export function mechanicalGuideReadiness(
  guide: CaptureReconstructionGuide,
): MechanicalGuideReadiness {
  const count = (role: CaptureLandmarkRole) => guide.landmarks.filter(landmark => landmark.role === role).length;
  const reasons: string[] = [];
  if (count('calibrationEndpoint') < 2) reasons.push('Pick two calibration endpoints.');
  if (count('frameOrigin') < 1) reasons.push('Pick one frame origin.');
  if (count('frameDirection') < 2) reasons.push('Pick X and XY frame directions.');
  if (count('symmetrySample') < 3) reasons.push('Pick three symmetry-plane samples.');
  if (count('profileVertex') < 3) reasons.push('Pick at least three ordered profile vertices.');
  return { ready: reasons.length === 0, reasons };
}

export function finalizeMechanicalGuideDraft(
  guide: CaptureReconstructionGuide,
  knownDistanceMm: number,
  instruction: string,
  featureDepthMm = 18,
): CaptureReconstructionGuide {
  const readiness = mechanicalGuideReadiness(guide);
  if (!readiness.ready) throw new Error(readiness.reasons.join(' '));
  if (!Number.isFinite(knownDistanceMm) || knownDistanceMm <= 0) {
    throw new Error('Known calibration distance must be finite and positive.');
  }
  if (!Number.isFinite(featureDepthMm) || featureDepthMm <= 0) {
    throw new Error('Feature depth must be finite and positive.');
  }
  const next = cloneGuide(guide);
  const byRole = (role: CaptureLandmarkRole) => next.landmarks.filter(landmark => landmark.role === role);
  const calibration = byRole('calibrationEndpoint').slice(0, 2);
  const frameOrigin = byRole('frameOrigin')[0];
  const frameDirections = byRole('frameDirection').slice(0, 2);
  const symmetrySamples = byRole('symmetrySample');
  const profileVertices = byRole('profileVertex');
  const rotationAxisEndpoints = byRole('rotationAxisEndpoint');
  const ignoredDamage = byRole('ignoredDamagedRegion');
  const existingProfile = next.profiles.find(item => item.profileId === 'profile-1');
  const profileIds = profileVertices.map(item => item.landmarkId);
  const orderedProfileIds = [
    ...(existingProfile?.landmarkIds ?? []).filter(id => profileIds.includes(id)),
    ...profileIds.filter(id => !existingProfile?.landmarkIds.includes(id)),
  ];
  next.calibration = {
    sourceUnits: 'sourceUnit',
    millimetresPerSourceUnit: next.calibration.millimetresPerSourceUnit || 1,
    method: { kind: 'knownDistance' },
    measurements: [{
      measurementId: 'calibration-1',
      label: 'Known calibration distance',
      firstLandmarkId: calibration[0].landmarkId,
      secondLandmarkId: calibration[1].landmarkId,
      knownDistanceMm,
      fittedDistanceMm: 0,
      residualMm: 0,
      acceptedToleranceMm: Math.max(0.01, knownDistanceMm * 0.0025),
    }],
    residualMm: 0,
  };
  next.reconstructionFrame.sourceLandmarkIds = [
    frameOrigin.landmarkId,
    frameDirections[0].landmarkId,
    frameDirections[1].landmarkId,
  ];
  next.planes = [{
    planeId: 'symmetry-plane-1',
    label: 'Primary symmetry plane',
    role: 'symmetry',
    landmarkIds: symmetrySamples.map(landmark => landmark.landmarkId),
    originMm: [0, 0, 0],
    normal: [1, 0, 0],
    fit: { rmsMm: 0, maxMm: 0, toleranceMm: 0.25 },
  }];
  next.axes = rotationAxisEndpoints.length >= 2 ? [{
    axisId: 'axis-1',
    label: 'Named rotation axis',
    landmarkIds: rotationAxisEndpoints.map(item => item.landmarkId),
    originMm: [...rotationAxisEndpoints[0].localPositionMm],
    direction: [0, 0, 1],
    fit: { rmsMm: 0, maxMm: 0, toleranceMm: 0.25 },
  }] : [];
  next.ignoredRegions = ignoredDamage.length > 0 ? [{
    regionId: 'ignored-damage-1',
    label: 'Observed damaged region',
    landmarkIds: ignoredDamage.map(item => item.landmarkId),
    reason: 'Excluded from fit evidence by explicit user role.',
  }] : [];
  next.profiles = [{
    profileId: 'profile-1',
    label: existingProfile?.label ?? 'Ordered reconstruction profile',
    kind: existingProfile?.kind ?? 'closed',
    supportPlaneId: existingProfile?.supportPlaneId ?? 'symmetry-plane-1',
    landmarkIds: orderedProfileIds,
    operationHint: existingProfile?.operationHint ?? 'extrude',
    featureLabel: existingProfile?.featureLabel ?? 'insert-body',
    fitRole: existingProfile?.fitRole ?? 'outer-envelope',
  }];
  const existingExpectation = (expectationId: string) =>
    next.featureExpectations.find(item => item.expectationId === expectationId);
  const symmetryExpectation: CaptureFeatureExpectation =
    existingExpectation('expectation-symmetry-face') ?? {
      expectationId: 'expectation-symmetry-face',
      guideItemIds: ['symmetry-plane-1'],
      label: 'Exact symmetry support face',
      expectedGeometryKind: 'plane',
      requiredBrepTopologyKind: 'face',
      cardinality: 'one',
      partId: 'insert-body',
      instancePath: null,
      expectedAuthoredSelector: { kind: 'tag', name: 'symmetry-face' },
      requiredForAcceptance: true,
      positionToleranceMm: 0.25,
      normalToleranceDeg: 1,
      radialToleranceMm: null,
    };
  const profileExpectation: CaptureFeatureExpectation =
    existingExpectation('expectation-profile-edges') ?? {
      expectationId: 'expectation-profile-edges',
      guideItemIds: ['profile-1'],
      label: 'Exact ordered profile edges',
      expectedGeometryKind: 'profile',
      requiredBrepTopologyKind: 'orderedEdges',
      cardinality: 'oneOrMore',
      partId: 'insert-body',
      instancePath: null,
      expectedAuthoredSelector: { kind: 'tag', name: 'profile-edges' },
      requiredForAcceptance: true,
      positionToleranceMm: 0.25,
      normalToleranceDeg: null,
      radialToleranceMm: null,
    };
  next.featureExpectations = [
    { ...symmetryExpectation, guideItemIds: ['symmetry-plane-1'] },
    { ...profileExpectation, guideItemIds: ['profile-1'] },
  ];
  next.symmetryCompletion = { kind: 'half', planeId: 'symmetry-plane-1' };
  next.measurements = [{
    measurementId: 'feature-depth',
    label: 'Feature depth',
    landmarkIds: [],
    value: featureDepthMm,
    unit: 'mm',
    fitCritical: true,
    authoredParameterName: 'feature-depth',
    constraintKind: 'extent',
  }];
  next.instruction = instruction.trim();
  next.evidenceViews = [];
  next.canonicalDigest = '';
  return next;
}
