import assert from 'node:assert/strict';
import test from 'node:test';
import { visibleManifestWarnings } from './manifestWarningPresentation';

test('visibleManifestWarnings hides internal feature graph carry-forward diagnostics', () => {
  assert.deepEqual(
    visibleManifestWarnings([
      'Feature graph was not carried forward because rendered topology no longer validates old feature bindings.',
      'Manufacturing clearance could not be verified.',
    ]),
    ['Manufacturing clearance could not be verified.'],
  );
});

test('visibleManifestWarnings trims and deduplicates user-actionable warnings', () => {
  assert.deepEqual(
    visibleManifestWarnings([' Thin wall. ', 'Thin wall.', '', 'Hybrid poly BRep bridge: internal']),
    ['Thin wall.'],
  );
});
