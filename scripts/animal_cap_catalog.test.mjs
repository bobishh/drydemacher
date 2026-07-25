import assert from 'node:assert/strict';
import test from 'node:test';

import { generateLandingModule, validateAnimalCapCatalog } from './animal_cap_catalog.mjs';

const publishedEntry = {
  id: 'pug',
  displayName: 'Pug cap',
  species: 'Pug',
  state: 'published',
  surfaces: { engine: true, landing: true },
  source: {
    author: 'Maker',
    pageUrl: 'https://example.test/pug',
    downloadUrl: 'https://example.test/pug.zip',
    archiveMember: 'Pug.obj',
    license: 'CC0-1.0',
    licenseUrl: 'https://creativecommons.org/publicdomain/zero/1.0/',
    sourceFormat: 'obj',
    sourceSha256: 'a'.repeat(64),
    sourceMeshPath: 'assets/Pug.obj',
    ingestedStlPath: 'assets/Pug.stl',
    ingestedStlSha256: 'b'.repeat(64),
  },
  sourceBounds: { min: [0, 0, 0], max: [1, 1, 1], size: [1, 1, 1] },
  recipe: {
    boreProfileId: 'presta-blind-bomb-v1',
    boreAxis: 'z',
    boreMouthSourceCoordinate: 0,
    boreAxisHeightMm: 8.5,
    uniformScale: 12,
    floorOffsetSourceCoordinate: 0,
  },
  artifact: {
    verificationStatus: 'passed',
    verifiedPartCount: 1,
    verifiedComponentCount: 1,
    verifiedNonManifoldEdgeCount: 0,
    verifiedTriangleCount: 962,
    modelId: 'model',
    threadId: 'thread',
    messageId: 'message',
    sourcePath: 'assets/pug.ecky',
    stlPath: 'assets/pug.stl',
    previewPath: 'assets/pug.png',
    sourceSha256: 'c'.repeat(64),
    stlSha256: 'd'.repeat(64),
  },
};

const baseCatalog = {
  schemaVersion: 1,
  boreProfiles: {
    'presta-blind-bomb-v1': {
      prestaMajorDiameterMm: 5.98,
      threadDepthMm: 0.4,
      baseThreadClearanceMm: 0.15,
      freeBoreClearanceMm: 0.25,
      threadStartMm: 12,
      threadLengthMm: 6,
      innerConeStartMm: 22.2,
      blindDepthMm: 27.8,
      entryLeadMm: 1,
      entryFlareMm: 1.25,
    },
  },
  entries: [publishedEntry],
};

test('published catalog accepts named uniform fit recipe', () => {
  assert.deepEqual(validateAnimalCapCatalog(baseCatalog, { verifyFiles: false }), []);
});

test('published catalog rejects anonymous deformation and missing bore profile', () => {
  const invalid = structuredClone(baseCatalog);
  invalid.entries[0].recipe.scaleX = 12;
  invalid.entries[0].recipe.boreProfileId = 'missing';
  assert.deepEqual(
    validateAnimalCapCatalog(invalid, { verifyFiles: false }),
    [
      'entry pug references unknown bore profile missing',
      'entry pug recipe must use uniformScale only',
    ],
  );
});

test('landing projection contains only complete published landing entries', () => {
  const catalog = structuredClone(baseCatalog);
  catalog.entries.push({
    ...structuredClone(publishedEntry),
    id: 'candidate',
    state: 'candidate',
    surfaces: { engine: false, landing: false },
    recipe: undefined,
    artifact: undefined,
  });
  const moduleSource = generateLandingModule(catalog);
  assert.match(moduleSource, /pug-presta/);
  assert.doesNotMatch(moduleSource, /candidate/);
  assert.match(moduleSource, /GENERATED FILE/);
});
