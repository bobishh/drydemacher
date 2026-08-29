import assert from 'node:assert/strict';
import test from 'node:test';

import type { ModelManifest } from '../types/domain';
import { buildImportedEvidence, isForeignCadEvidence } from './importedEvidence';

const manifest: ModelManifest = {
  modelId: 'imported-step-bearing',
  sourceKind: 'importedStep',
  sourceDigest: 'sha256-source',
  document: {
    documentName: 'Bearing608',
    documentLabel: '608 Bearing',
    sourcePath: '/library/Mechanical Parts/Bearings/608.step',
    objectCount: 1,
    warnings: ['Inspect-only import.'],
  },
  parts: [{
    partId: 'bearing-body',
    freecadObjectName: 'Body',
    label: 'Bearing body',
    kind: 'PartDesign::Body',
    viewerNodeIds: ['bearing-body'],
    parameterKeys: [],
    editable: false,
    bounds: { xMin: -11, xMax: 11, yMin: -11, yMax: 11, zMin: 0, zMax: 7 },
    volume: 1800,
    area: 900,
  }],
  taggedAnchors: {},
  analysisDeclarations: [],
  warnings: ['Inspect-only import.'],
};

test('foreign CAD report is stable, readable, and contains printable dimensions', () => {
  const evidence = buildImportedEvidence(manifest);

  assert.match(evidence, /^IMPORTED STEP — READ ONLY/m);
  assert.match(evidence, /^FILE  \/library\/Mechanical Parts\/Bearings\/608\.step$/m);
  assert.match(evidence, /^PRINT SIZE  22 × 22 × 7 mm$/m);
  assert.match(evidence, /^PARTS  1$/m);
  assert.match(evidence, /^1\. Bearing body$/m);
  assert.match(evidence, /^   SIZE  22 × 22 × 7 mm$/m);
  assert.doesNotMatch(evidence, /format|modelId|sourceDigest|confidence|0\.42|[{}]/);
  assert.equal(buildImportedEvidence(manifest), evidence);
});

test('foreign evidence mode applies only to FCStd and STEP imports', () => {
  assert.equal(isForeignCadEvidence(manifest), true);
  assert.equal(isForeignCadEvidence({ ...manifest, sourceKind: 'importedFcstd' }), true);
  assert.equal(isForeignCadEvidence({ ...manifest, sourceKind: 'importedMesh' }), false);
  assert.equal(isForeignCadEvidence({ ...manifest, sourceKind: 'generated' }), false);
});

test('foreign CAD report includes every imported part without UI truncation', () => {
  const parts = Array.from({ length: 620 }, (_, index) => ({
    ...manifest.parts![0],
    partId: `part-${index + 1}`,
    label: `Assembly part ${index + 1}`,
  }));
  const report = buildImportedEvidence({
    ...manifest,
    document: { ...manifest.document, objectCount: parts.length },
    parts,
  });

  assert.match(report, /^620\. Assembly part 620$/m);
  assert.doesNotMatch(report, /MORE PARTS|omitted|truncat/i);
});
