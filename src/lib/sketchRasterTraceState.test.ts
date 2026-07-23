import assert from 'node:assert/strict';
import test from 'node:test';

import { sketchDocumentToStrokes } from './sketchDocumentReplay';
import { buildSketchDraftRequest, type SketchStroke } from './sketchWorkspaceState';
import type { RasterTraceProvenance, SketchDocument } from './tauri/contracts';

const provenance: RasterTraceProvenance = {
  kind: 'rasterTrace',
  asset: {
    imagePath: '/tmp/front.png',
    digest: 'sha256:front',
    widthPixels: 800,
    heightPixels: 600,
  },
  view: 'front',
  calibration: { physicalWidth: 120, physicalHeight: 80 },
  threshold: 143,
  invert: true,
  contourId: 'raster-front-0',
  extractorVersion: 'raster-trace-v1',
};

test('reviewed raster stroke keeps provenance in draft primitive', () => {
  const stroke: SketchStroke = {
    primitiveId: 'raster-front',
    sketchId: 'sketch-front',
    view: 'front',
    kind: 'polyline',
    points: [
      [0, 0],
      [120, 0],
      [120, 80],
      [0, 80],
      [0, 0],
    ],
    closed: true,
    provenance,
  };

  const request = buildSketchDraftRequest([stroke]);
  assert.ok(!('error' in request));
  assert.deepEqual(request.sketch.primitives?.[0]?.provenance, provenance);
});

test('restored sketch document keeps raster asset, settings, contour, and extractor identity', () => {
  const document: SketchDocument = {
    documentId: 'raster-document',
    activeSketchId: 'sketch-front',
    units: 'mm',
    sketches: [
      {
        sketchId: 'sketch-front',
        view: 'front',
        primitives: [
          {
            primitiveId: 'raster-front',
            kind: 'polyline',
            points: [
              [0, 0],
              [120, 0],
              [120, 80],
              [0, 80],
              [0, 0],
            ],
            closed: true,
            provenance,
          },
        ],
        constraints: [{ constraintId: 'raster-front-closed', kind: 'closed', targetIds: ['raster-front'] }],
      },
    ],
  };

  const replay = sketchDocumentToStrokes(document);
  assert.ok(!('error' in replay));
  assert.deepEqual(replay.strokes[0]?.provenance, provenance);
});
