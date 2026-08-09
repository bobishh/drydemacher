import type { CaptureSurfaceAnchor } from '../tauri/contracts';

export type SurfaceTrimPathMode = 'shortest' | 'feature';
export type SurfaceTrimCapMode = 'open' | 'flat' | 'surfaceFill';
export type SurfaceTrimPhase =
  | 'idle'
  | 'placingBoundary'
  | 'boundaryClosed'
  | 'selectingRegion'
  | 'previewReady'
  | 'applying';

export type SurfaceTrimInteraction = {
  phase: SurfaceTrimPhase;
  anchors: CaptureSurfaceAnchor[];
  keepSeed: CaptureSurfaceAnchor | null;
  pathMode: SurfaceTrimPathMode;
  capMode: SurfaceTrimCapMode;
  editingTrimNodeId: number | null;
  error: string;
};

const BARYCENTRIC_TOLERANCE = 1e-9;

function cloneAnchor(anchor: CaptureSurfaceAnchor): CaptureSurfaceAnchor {
  return {
    sourceMeshContentDigest: anchor.sourceMeshContentDigest,
    triangleIndex: anchor.triangleIndex,
    barycentric: [...anchor.barycentric] as [number, number, number],
    sourcePosition: [...anchor.sourcePosition] as [number, number, number],
    sourceNormal: [...anchor.sourceNormal] as [number, number, number],
  };
}

function cloneState(state: SurfaceTrimInteraction): SurfaceTrimInteraction {
  return {
    ...state,
    anchors: state.anchors.map(cloneAnchor),
    keepSeed: state.keepSeed ? cloneAnchor(state.keepSeed) : null,
  };
}

function withError(state: SurfaceTrimInteraction, error: string): SurfaceTrimInteraction {
  return {
    ...cloneState(state),
    error,
  };
}

function sameAnchor(
  left: CaptureSurfaceAnchor,
  right: CaptureSurfaceAnchor,
): boolean {
  if (left.sourceMeshContentDigest !== right.sourceMeshContentDigest) {
    return false;
  }

  if (left.triangleIndex !== right.triangleIndex) {
    return false;
  }

  return left.barycentric.every(
    (value, index) => Math.abs(value - right.barycentric[index]) <= BARYCENTRIC_TOLERANCE,
  );
}

function hasDuplicateAnchor(
  anchors: CaptureSurfaceAnchor[],
  anchor: CaptureSurfaceAnchor,
  excludeIndex: number | null = null,
): boolean {
  return anchors.some((existing, index) => {
    if (excludeIndex !== null && index === excludeIndex) {
      return false;
    }

    return sameAnchor(existing, anchor);
  });
}

function uniqueAnchorCount(anchors: CaptureSurfaceAnchor[]): number {
  const unique: CaptureSurfaceAnchor[] = [];
  for (const anchor of anchors) {
    if (!unique.some((existing) => sameAnchor(existing, anchor))) {
      unique.push(anchor);
    }
  }
  return unique.length;
}

function freshState(
  editingTrimNodeId: number | null = null,
  pathMode: SurfaceTrimPathMode = 'feature',
  capMode: SurfaceTrimCapMode = 'open',
): SurfaceTrimInteraction {
  return {
    phase: 'placingBoundary',
    anchors: [],
    keepSeed: null,
    pathMode,
    capMode,
    editingTrimNodeId,
    error: '',
  };
}

export function createSurfaceTrimInteraction(
  editingTrimNodeId?: number | null,
  pathMode?: SurfaceTrimPathMode,
  capMode?: SurfaceTrimCapMode,
): SurfaceTrimInteraction {
  return freshState(editingTrimNodeId ?? null, pathMode ?? 'shortest', capMode ?? 'open');
}

export function addSurfaceTrimAnchor(
  state: SurfaceTrimInteraction,
  anchor: CaptureSurfaceAnchor,
): SurfaceTrimInteraction {
  if (state.phase !== 'placingBoundary') {
    return withError(state, 'Cannot add anchor while not placing boundary.');
  }

  if (hasDuplicateAnchor(state.anchors, anchor)) {
    return withError(state, 'Duplicate surface trim anchor.');
  }

  return {
    ...cloneState(state),
    anchors: [...state.anchors.map(cloneAnchor), cloneAnchor(anchor)],
    keepSeed: null,
    phase: 'placingBoundary',
    error: '',
  };
}

export function moveSurfaceTrimAnchor(
  state: SurfaceTrimInteraction,
  index: number,
  anchor: CaptureSurfaceAnchor,
): SurfaceTrimInteraction {
  if (state.phase !== 'placingBoundary') {
    return withError(state, 'Cannot move anchor while not placing boundary.');
  }

  if (index < 0 || index >= state.anchors.length) {
    return withError(state, 'Invalid anchor index.');
  }

  if (hasDuplicateAnchor(state.anchors, anchor, index)) {
    return withError(state, 'Duplicate surface trim anchor.');
  }

  const anchors = state.anchors.map(cloneAnchor);
  anchors[index] = cloneAnchor(anchor);

  return {
    ...cloneState(state),
    anchors,
    keepSeed: null,
    phase: 'placingBoundary',
    error: '',
  };
}

export function removeSurfaceTrimAnchor(
  state: SurfaceTrimInteraction,
  index: number,
): SurfaceTrimInteraction {
  if (state.phase !== 'placingBoundary') {
    return withError(state, 'Cannot remove anchor while not placing boundary.');
  }

  if (index < 0 || index >= state.anchors.length) {
    return withError(state, 'Invalid anchor index.');
  }

  return {
    ...cloneState(state),
    anchors: state.anchors
      .filter((_, anchorIndex) => anchorIndex !== index)
      .map(cloneAnchor),
    keepSeed: null,
    phase: 'placingBoundary',
    error: '',
  };
}

export function undoSurfaceTrimAnchor(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  if (state.phase !== 'placingBoundary') {
    return withError(state, 'Cannot undo anchor while not placing boundary.');
  }

  if (state.anchors.length === 0) {
    return withError(state, 'No surface trim anchor to undo.');
  }

  return {
    ...cloneState(state),
    anchors: state.anchors.slice(0, -1).map(cloneAnchor),
    keepSeed: null,
    phase: 'placingBoundary',
    error: '',
  };
}

export function closeSurfaceTrimBoundary(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  if (state.phase !== 'placingBoundary') {
    return withError(state, 'Cannot close boundary while not placing boundary.');
  }

  if (uniqueAnchorCount(state.anchors) < 3) {
    return withError(state, 'Need at least 3 unique anchors to close boundary.');
  }

  return {
    ...cloneState(state),
    phase: 'boundaryClosed',
    error: '',
  };
}

export function beginSurfaceTrimRegionSelection(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  if (state.phase !== 'boundaryClosed') {
    return withError(state, 'Cannot begin region selection before boundary is closed.');
  }

  return {
    ...cloneState(state),
    phase: 'selectingRegion',
    error: '',
  };
}

export function chooseSurfaceTrimKeepSeed(
  state: SurfaceTrimInteraction,
  keepSeed: CaptureSurfaceAnchor,
): SurfaceTrimInteraction {
  if (state.phase !== 'selectingRegion') {
    return withError(state, 'Cannot choose keep seed before region selection.');
  }

  return {
    ...cloneState(state),
    keepSeed: cloneAnchor(keepSeed),
    phase: 'selectingRegion',
    error: '',
  };
}

export function markSurfaceTrimPreviewReady(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  if (state.phase !== 'selectingRegion') {
    return withError(state, 'Cannot mark preview ready before region selection.');
  }

  if (state.keepSeed === null) {
    return withError(state, 'Cannot mark preview ready without a keep seed.');
  }

  return {
    ...cloneState(state),
    phase: 'previewReady',
    error: '',
  };
}

export function markSurfaceTrimApplying(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  if (state.phase !== 'previewReady') {
    return withError(state, 'Cannot mark applying before preview is ready.');
  }

  return {
    ...cloneState(state),
    phase: 'applying',
    error: '',
  };
}

export function setSurfaceTrimError(
  state: SurfaceTrimInteraction,
  error: string,
): SurfaceTrimInteraction {
  return {
    ...cloneState(state),
    error,
  };
}

export function cancelSurfaceTrim(
  state: SurfaceTrimInteraction,
): SurfaceTrimInteraction {
  return freshState(state.editingTrimNodeId, state.pathMode, state.capMode);
}

export function canCloseSurfaceTrimBoundary(state: SurfaceTrimInteraction): boolean {
  return state.phase === 'placingBoundary' && uniqueAnchorCount(state.anchors) >= 3;
}

export function canRequestSurfaceTrimPreview(state: SurfaceTrimInteraction): boolean {
  return state.phase === 'selectingRegion' && state.keepSeed !== null;
}

export function canApplySurfaceTrim(state: SurfaceTrimInteraction): boolean {
  return state.phase === 'previewReady';
}
