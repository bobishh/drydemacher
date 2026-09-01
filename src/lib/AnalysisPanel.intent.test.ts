import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('./AnalysisPanel.svelte', import.meta.url), 'utf8');

test('Given FEM Run When submitted Then frontend sends one Rust-owned intent', () => {
  assert.match(source, /runFemStudyIntent\s*\(/);
  const runStudy = source.slice(source.indexOf('async function runStudy()'), source.indexOf('async function previewMesh()'));
  assert.doesNotMatch(runStudy, /validateFemStudy\s*\(/);
  assert.doesNotMatch(runStudy, /runFemStudy\s*\(/);
  assert.doesNotMatch(runStudy, /nextJobId\s*\(/);
});

test('Given FEM panel actions When invoked Then frontend never authors jobs or compute policy', () => {
  assert.doesNotMatch(source, /function request\s*\(/);
  assert.doesNotMatch(source, /function nextJobId\s*\(/);
  assert.doesNotMatch(source, /boundaryTriangles:\s*250_000/);
  assert.doesNotMatch(source, /maximumRuntimeMs:\s*10 \* 60 \* 1000/);
});

test('Given validate, preview, and convergence actions When submitted Then each uses one Rust-owned intent', () => {
  const validate = source.slice(source.indexOf('async function validateStudy()'), source.indexOf('async function runStudy()'));
  const preview = source.slice(source.indexOf('async function previewMesh()'), source.indexOf('async function runConvergenceStudy()'));
  const convergence = source.slice(source.indexOf('async function runConvergenceStudy()'), source.indexOf('function statusLabel'));
  assert.match(validate, /validateFemStudyIntent\s*\(/);
  assert.doesNotMatch(validate, /validateFemStudy\s*\(/);
  assert.match(preview, /previewFemMeshIntent\s*\(/);
  assert.doesNotMatch(preview, /validateFemStudy\s*\(/);
  assert.doesNotMatch(preview, /previewFemMesh\s*\(/);
  assert.match(convergence, /runFemConvergenceIntent\s*\(/);
  assert.doesNotMatch(convergence, /runFemConvergence\s*\(/);
  assert.match(source, /getCachedFemConvergenceIntent\s*\(/);
  assert.doesNotMatch(source, /getCachedFemConvergence\s*\(/);
});

test('Given immutable result export When target path is chosen Then Rust owns byte safety policy', () => {
  const exportAction = source.slice(source.indexOf('async function exportVtu()'), source.indexOf('</script>'));
  assert.match(exportAction, /exportFemResultVtuIntent\s*\(/);
  assert.doesNotMatch(exportAction, /maximumResultBytes/);
});
