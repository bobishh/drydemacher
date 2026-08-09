import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import type { CaptureSurfaceAnchor } from '../tauri/contracts';
import {
  addSurfaceTrimAnchor,
  beginSurfaceTrimRegionSelection,
  canApplySurfaceTrim,
  canCloseSurfaceTrimBoundary,
  canRequestSurfaceTrimPreview,
  cancelSurfaceTrim,
  chooseSurfaceTrimKeepSeed,
  closeSurfaceTrimBoundary,
  createSurfaceTrimInteraction,
  markSurfaceTrimApplying,
  markSurfaceTrimPreviewReady,
  moveSurfaceTrimAnchor,
  removeSurfaceTrimAnchor,
  setSurfaceTrimError,
  undoSurfaceTrimAnchor,
} from './surfaceTrimInteraction';

function anchor(
  sourceMeshContentDigest: string,
  triangleIndex: number,
  barycentric: [number, number, number],
): CaptureSurfaceAnchor {
  return {
    sourceMeshContentDigest,
    triangleIndex,
    barycentric,
    sourcePosition: [triangleIndex, triangleIndex + 1, triangleIndex + 2],
    sourceNormal: [0, 0, 1],
  };
}

describe('surface trim interaction', () => {
  it('runs the happy path', () => {
    const a = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);
    const b = anchor('mesh-a', 2, [0.3, 0.3, 0.4]);
    const c = anchor('mesh-a', 3, [0.4, 0.2, 0.4]);
    const seed = anchor('mesh-a', 9, [0.1, 0.2, 0.7]);

    const start = createSurfaceTrimInteraction(17, 'feature', 'flat');
    assert.deepEqual(start, {
      phase: 'placingBoundary',
      anchors: [],
      keepSeed: null,
      pathMode: 'feature',
      capMode: 'flat',
      editingTrimNodeId: 17,
      error: '',
    });

    const withA = addSurfaceTrimAnchor(start, a);
    const withB = addSurfaceTrimAnchor(withA, b);
    const withC = addSurfaceTrimAnchor(withB, c);

    assert.equal(canCloseSurfaceTrimBoundary(withC), true);

    const closed = closeSurfaceTrimBoundary(withC);
    assert.equal(closed.phase, 'boundaryClosed');
    assert.deepEqual(closed.anchors, [a, b, c]);
    assert.equal(closed.error, '');

    const selecting = beginSurfaceTrimRegionSelection(closed);
    assert.equal(selecting.phase, 'selectingRegion');

    const withSeed = chooseSurfaceTrimKeepSeed(selecting, seed);
    assert.equal(withSeed.phase, 'selectingRegion');
    assert.deepEqual(withSeed.keepSeed, seed);
    assert.equal(canRequestSurfaceTrimPreview(withSeed), true);

    const previewReady = markSurfaceTrimPreviewReady(withSeed);
    assert.equal(previewReady.phase, 'previewReady');
    assert.equal(canApplySurfaceTrim(previewReady), true);

    const applying = markSurfaceTrimApplying(previewReady);
    assert.equal(applying.phase, 'applying');
    assert.equal(applying.error, '');
  });

  it('rejects duplicate anchors', () => {
    const a = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);
    const duplicate = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);

    const start = createSurfaceTrimInteraction();
    const withA = addSurfaceTrimAnchor(start, a);
    const rejected = addSurfaceTrimAnchor(withA, duplicate);

    assert.equal(rejected.phase, 'placingBoundary');
    assert.deepEqual(rejected.anchors, [a]);
    assert.equal(rejected.error, 'Duplicate surface trim anchor.');
    assert.equal(withA.error, '');
  });

  it('moves removes and undoes anchors immutably', () => {
    const a = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);
    const b = anchor('mesh-a', 2, [0.3, 0.3, 0.4]);
    const c = anchor('mesh-a', 3, [0.4, 0.2, 0.4]);
    const moved = anchor('mesh-a', 4, [0.1, 0.4, 0.5]);

    const start = createSurfaceTrimInteraction();
    const withAnchors = addSurfaceTrimAnchor(
      addSurfaceTrimAnchor(addSurfaceTrimAnchor(start, a), b),
      c,
    );
    const afterMove = moveSurfaceTrimAnchor(withAnchors, 1, moved);
    const afterRemove = removeSurfaceTrimAnchor(afterMove, 0);
    const afterUndo = undoSurfaceTrimAnchor(afterRemove);

    assert.deepEqual(withAnchors.anchors, [a, b, c]);
    assert.deepEqual(afterMove.anchors, [a, moved, c]);
    assert.deepEqual(afterRemove.anchors, [moved, c]);
    assert.deepEqual(afterUndo.anchors, [moved]);
    assert.equal(afterUndo.error, '');
  });

  it('keeps geometry on invalid transitions', () => {
    const a = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);
    const state = addSurfaceTrimAnchor(createSurfaceTrimInteraction(), a);
    const invalid = beginSurfaceTrimRegionSelection(state);

    assert.equal(invalid.phase, 'placingBoundary');
    assert.deepEqual(invalid.anchors, [a]);
    assert.equal(invalid.error, 'Cannot begin region selection before boundary is closed.');
  });

  it('cancels to a fresh placing boundary state and preserves settings', () => {
    const a = anchor('mesh-a', 1, [0.2, 0.3, 0.5]);
    const dirty = setSurfaceTrimError(
      addSurfaceTrimAnchor(createSurfaceTrimInteraction(44, 'feature', 'surfaceFill'), a),
      'boom',
    );

    const cancelled = cancelSurfaceTrim(dirty);

    assert.deepEqual(cancelled, {
      phase: 'placingBoundary',
      anchors: [],
      keepSeed: null,
      pathMode: 'feature',
      capMode: 'surfaceFill',
      editingTrimNodeId: 44,
      error: '',
    });
  });
});
