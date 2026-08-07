import assert from 'node:assert/strict';
import test from 'node:test';

import { buildCaptureSolidifySource } from './captureSolidifySource';

test('empty capture target becomes a minimal solidified STL model', () => {
  assert.equal(
    buildCaptureSolidifySource('', [], '/tmp/capture mesh.stl', 'abc123', 0.05),
    '(model\n  (params (number capture_scale_abc123 0.05 :label "Capture scale" :min 0.001 :max 2 :step 0.001))\n  (part capture_abc123 (scale capture_scale_abc123 capture_scale_abc123 capture_scale_abc123 (solidify (import-stl "/tmp/capture mesh.stl")))))',
  );
});

test('existing model receives a solidified capture part at parser model boundary', () => {
  const source = '(model\n  (part body (box 10 20 30)))';
  const result = buildCaptureSolidifySource(
    source,
    [{ id: 'model', kind: 'model', label: 'model', startByte: 0, endByte: source.length }],
    '/tmp/scan.stl',
    'xyz789',
    0.05,
  );

  assert.equal(
    result,
    '(model\n  (part body (box 10 20 30))\n  (params (number capture_scale_xyz789 0.05 :label "Capture scale" :min 0.001 :max 2 :step 0.001))\n  (part capture_xyz789 (scale capture_scale_xyz789 capture_scale_xyz789 capture_scale_xyz789 (solidify (import-stl "/tmp/scan.stl")))))',
  );
});

test('non-empty source without parser model range is rejected', () => {
  assert.throws(
    () => buildCaptureSolidifySource('(model)', [], '/tmp/scan.stl', 'abc', 0.05),
    /model AST range/i,
  );
});
