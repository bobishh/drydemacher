import type { CaptureLandmarkRole, CaptureReconstructionGuide } from '../tauri/contracts';

export type CaptureGuideOverlayLandmark = {
  landmarkId: string;
  ordinal: number;
  label: string;
  role: CaptureLandmarkRole;
  sourcePosition: [number, number, number];
};

export type CaptureGuideOverlaySegment = {
  key: string;
  fromLandmarkId: string;
  toLandmarkId: string;
  kind: 'profile' | 'axis';
};

export type CaptureGuideOverlayPlaneLoop = {
  planeId: string;
  label: string;
  landmarkIds: string[];
};

export type CaptureGuideOverlayPrimitives = {
  landmarks: CaptureGuideOverlayLandmark[];
  profileSegments: CaptureGuideOverlaySegment[];
  axisSegments: CaptureGuideOverlaySegment[];
  planeLoops: CaptureGuideOverlayPlaneLoop[];
  evidenceScopeLabel: 'OBSERVED REGION ONLY';
  inferredRegionLabel: string | null;
};

export function buildCaptureGuideOverlayPrimitives(
  guide: CaptureReconstructionGuide,
): CaptureGuideOverlayPrimitives {
  const landmarkIds = new Set(guide.landmarks.map(item => item.landmarkId));
  const profileSegments = guide.profiles.flatMap(profile => {
    const ids = profile.landmarkIds.filter(id => landmarkIds.has(id));
    const pairs = ids.slice(1).map((toLandmarkId, index) => [ids[index], toLandmarkId] as const);
    if (profile.kind === 'closed' && ids.length >= 3) {
      pairs.push([ids.at(-1)!, ids[0]]);
    }
    return pairs.map(([fromLandmarkId, toLandmarkId], index) => ({
      key: `${profile.profileId}:${index}:${fromLandmarkId}:${toLandmarkId}`,
      fromLandmarkId,
      toLandmarkId,
      kind: 'profile' as const,
    }));
  });
  const axisSegments = guide.axes.flatMap(axis => {
    const ids = axis.landmarkIds.filter(id => landmarkIds.has(id));
    if (ids.length < 2) return [];
    return [{
      key: `${axis.axisId}:${ids[0]}:${ids.at(-1)}`,
      fromLandmarkId: ids[0],
      toLandmarkId: ids.at(-1)!,
      kind: 'axis' as const,
    }];
  });
  const inferredRegionLabel = guide.symmetryCompletion.kind === 'half'
    ? 'INFERRED HALF · UNVERIFIED'
    : guide.symmetryCompletion.kind === 'quarter'
      ? 'INFERRED THREE QUARTERS · UNVERIFIED'
      : null;
  return {
    landmarks: guide.landmarks.map((landmark, index) => ({
      landmarkId: landmark.landmarkId,
      ordinal: index + 1,
      label: landmark.label,
      role: landmark.role,
      sourcePosition: [...landmark.anchor.sourcePosition],
    })),
    profileSegments,
    axisSegments,
    planeLoops: guide.planes
      .map(plane => ({
        planeId: plane.planeId,
        label: plane.label,
        landmarkIds: plane.landmarkIds.filter(id => landmarkIds.has(id)),
      }))
      .filter(plane => plane.landmarkIds.length >= 3),
    evidenceScopeLabel: 'OBSERVED REGION ONLY',
    inferredRegionLabel,
  };
}
