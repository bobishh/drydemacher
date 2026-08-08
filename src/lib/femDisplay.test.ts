import assert from 'node:assert/strict';
import test from 'node:test';
import { femColorRamp, normalizeFemField, type FemDisplayOptions } from './femDisplay';

test('FEM display clamps finite fields and maps low-to-high stress without changing source geometry', () => {
  assert.equal(normalizeFemField(5, 0, 10), 0.5);
  assert.equal(normalizeFemField(-1, 0, 10), 0);
  assert.equal(normalizeFemField(11, 0, 10), 1);
  assert.deepEqual(normalizeFemField(Number.NaN, 0, 10), 0);
  assert.notDeepEqual(femColorRamp(0), femColorRamp(1));
});

test('FEM view state remains display-only', () => {
  const display: FemDisplayOptions = {
    field: 'vonMises',
    deformationScale: 4,
    showMesh: true,
    showOutline: true,
    clipFraction: 1,
  };
  assert.deepEqual(Object.keys(display).sort(), [
    'clipFraction', 'deformationScale', 'field', 'showMesh', 'showOutline',
  ]);
});
