#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cpus } from 'node:os';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  evaluateWorkerParity,
  makeRequest,
  median,
  nativeElapsedMs,
  root,
  runGuardedSample,
  summarizeResourceEvidence,
} from './benchmark_direct_occt_fixtures.mjs';

export const braceletFixture = join(root, 'model-runtime/examples/daughter-flower-airtag-bracelet.ecky');

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function booleanElapsedMs(sample) {
  const stage = sample.stage?.stages?.find(item => item.name === 'boolean');
  assert.ok(Number.isFinite(stage?.elapsedMs), `${sample.label} lacks Boolean timing`);
  return stage.elapsedMs;
}

function assertCpuEvidence(sample, policy) {
  assert.equal(sample.stage?.parallelPolicy, policy, `${sample.label} policy mismatch`);
  assert.ok(sample.stage?.peakTotalAllocatedCpuUnits <= sample.stage?.workerBudget,
    `${sample.label} oversubscribed CPU budget`);
}

export function evaluateBraceletGate({ outerOnly, adaptive, referenceCpuCount = 18 }) {
  assert.ok(outerOnly.length >= 3 && adaptive.length >= 3, 'bracelet release gate needs three samples per policy');
  for (const sample of outerOnly) assertCpuEvidence(sample, 'outer-only');
  for (const sample of adaptive) assertCpuEvidence(sample, 'adaptive');
  const parity = evaluateWorkerParity([...outerOnly, ...adaptive]);
  assert.equal(parity.fields.components, 3, 'bracelet must preserve three printable components');
  assert.equal(parity.fields.parts, 3, 'bracelet must preserve three authored parts');
  assert.equal(new Set(parity.artifactDigests.map(item => item.stl)).size, 1, 'bracelet STL bytes changed across policies');
  assert.equal(new Set(parity.artifactDigests.map(item => item.step)).size, 1, 'bracelet STEP bytes changed across policies');
  assert.ok(adaptive.some(sample => (sample.stage?.parallelBooleanCount ?? 0) > 0),
    'adaptive policy executed no parallel Boolean');
  const outerNativeMedianMs = median(outerOnly.map(nativeElapsedMs));
  const adaptiveNativeMedianMs = median(adaptive.map(nativeElapsedMs));
  const outerBooleanMedianMs = median(outerOnly.map(booleanElapsedMs));
  const adaptiveBooleanMedianMs = median(adaptive.map(booleanElapsedMs));
  const nativeSpeedup = outerNativeMedianMs / adaptiveNativeMedianMs;
  const booleanSpeedup = outerBooleanMedianMs / adaptiveBooleanMedianMs;
  assert.ok(nativeSpeedup >= 3, `bracelet native speedup ${nativeSpeedup.toFixed(3)}x is below 3x`);
  assert.ok(booleanSpeedup >= 3, `bracelet Boolean speedup ${booleanSpeedup.toFixed(3)}x is below 3x`);
  if (referenceCpuCount === 18) {
    assert.ok(adaptiveNativeMedianMs <= 23_000,
      `bracelet adaptive median ${adaptiveNativeMedianMs}ms exceeds 23s`);
  }
  return {
    passed: true,
    outerNativeMedianMs,
    adaptiveNativeMedianMs,
    outerBooleanMedianMs,
    adaptiveBooleanMedianMs,
    nativeSpeedup,
    booleanSpeedup,
    parity,
  };
}

function partEvidence(sample, partId) {
  return sample.stage?.parts?.find(part => part.partId === partId);
}

function groupEvidence(sample, key) {
  return sample.stage?.partialBooleanGroups?.find(group => group.key === key);
}

export function evaluateBraceletIncremental({ seed, identical, bodyEdit, threadEdit, decorationEdit }) {
  for (const sample of [seed, identical, bodyEdit, threadEdit, decorationEdit]) {
    assert.equal(sample.validity, true, `${sample.label} invalid`);
    assert.equal(sample.components, 3, `${sample.label} changed component count`);
    assert.equal(sample.parts, 3, `${sample.label} changed part count`);
  }
  assert.ok(nativeElapsedMs(identical) <= 3_000, 'identical bracelet rerender exceeds 3s');
  assert.equal((identical.stage?.serialBooleanCount ?? 0) + (identical.stage?.parallelBooleanCount ?? 0), 0,
    'identical bracelet rerender executed Booleans');
  const lid = partEvidence(bodyEdit, 'daughter-flower-center-lid');
  assert.equal(lid?.cacheHit, true, 'body-only edit missed complete lid cache');
  assert.equal(lid?.executedCommandCount, 0, 'body-only edit executed lid commands');
  const decoratedHit = groupEvidence(threadEdit, 'decorated-dome');
  assert.equal(decoratedHit?.cacheHit, true, 'thread edit missed decorated dome');
  assert.equal(decoratedHit?.recomputeCount, 0, 'thread edit recomputed decorated dome');
  assert.ok(nativeElapsedMs(threadEdit) <= 10_000, 'thread edit exceeds 10s');
  const decoratedMiss = groupEvidence(decorationEdit, 'decorated-dome');
  assert.equal(decoratedMiss?.cacheHit, false, 'decoration edit retained stale decorated dome');
  assert.equal(decoratedMiss?.recomputeCount, 1, 'decoration edit did not rebuild decorated dome exactly once');
  const structuralHit = groupEvidence(decorationEdit, 'operand-pair-0');
  assert.equal(structuralHit?.cacheHit, true, 'decoration edit invalidated structural pair');
  assert.equal(structuralHit?.recomputeCount, 0, 'decoration edit recomputed structural pair');
  return { passed: true };
}

function sampleCount() {
  const count = Number(process.env.ECKY_BRACELET_BENCH_SAMPLES ?? 3);
  if (!Number.isInteger(count) || count < 1) throw new Error('ECKY_BRACELET_BENCH_SAMPLES must be an integer >= 1');
  return count;
}

function removeCache(runDir, cacheDir) {
  const resolvedRun = `${resolve(runDir)}/`;
  const resolvedCache = resolve(cacheDir);
  if (!resolvedCache.startsWith(resolvedRun)) throw new Error(`cache outside run dir: ${resolvedCache}`);
  rmSync(resolvedCache, { recursive: true, force: true });
}

function request(runDir, cli, workers, label, policy, cacheDir, params = {}) {
  return makeRequest({
    runDir, cli, workers, label, parallelPolicy: policy, cacheDir, params,
    fixture: braceletFixture, driver: 'ecky', runtimeRoot: join(root, '.dist/runtime/occt'),
  });
}

function runColdPolicy({ runDir, cli, workers, policy, samples }) {
  return Array.from({ length: samples }, (_, index) => {
    const cacheDir = join(runDir, 'cache', `${policy}-${index + 1}`);
    const sample = runGuardedSample(request(
      runDir, cli, workers, `${policy}-${index + 1}`, policy, cacheDir));
    removeCache(runDir, cacheDir);
    return sample;
  });
}

function runIncremental({ runDir, cli, workers }) {
  const cacheDir = join(runDir, 'cache', 'incremental');
  const run = (label, params) => runGuardedSample(request(
    runDir, cli, workers, label, 'adaptive', cacheDir, params));
  const result = {
    seed: run('incremental-seed', {}),
    identical: run('incremental-identical', {}),
    bodyEdit: run('incremental-body-edit', { 'petal-radius': 6.3 }),
    threadEdit: run('incremental-thread-edit', { 'thread-pitch': 2.1 }),
    decorationEdit: run('incremental-decoration-edit', { 'center-decoration-width': 24.5 }),
  };
  result.gate = evaluateBraceletIncremental(result);
  removeCache(runDir, cacheDir);
  return result;
}

export function main() {
  const cli = resolve(process.env.ECKY_DIRECT_OCCT_CLI ?? join(root, 'src-tauri/target/release/ecky'));
  if (!existsSync(cli)) throw new Error(`release Ecky CLI missing: ${cli}`);
  const workers = Number(process.env.ECKY_DIRECT_OCCT_WORKERS ?? cpus().length);
  const samples = sampleCount();
  const runDir = resolve(process.env.ECKY_BRACELET_BENCH_OUT ??
    join(root, 'tmp/direct-occt-bracelet-bench', `run-${new Date().toISOString().replace(/[:.]/g, '-')}-${process.pid}`));
  mkdirSync(runDir, { recursive: true });
  const characterization = process.env.ECKY_BRACELET_CHARACTERIZATION === '1';
  const warmupCache = join(runDir, 'cache', 'warmup');
  const warmup = characterization
    ? null
    : runGuardedSample(request(runDir, cli, workers, 'warmup', 'adaptive', warmupCache));
  if (warmup) removeCache(runDir, warmupCache);
  const outerOnly = characterization
    ? []
    : runColdPolicy({ runDir, cli, workers, policy: 'outer-only', samples });
  const adaptive = runColdPolicy({ runDir, cli, workers, policy: 'adaptive', samples });
  const gate = !characterization && samples >= 3
    ? evaluateBraceletGate({ outerOnly, adaptive, referenceCpuCount: cpus().length })
    : {
        passed: null,
        reason: 'characterization-only; release gate needs three samples per policy',
        historicalOuterOnly: { nativeMs: 69_669, booleanMs: 64_133 },
        adaptiveNativeMs: nativeElapsedMs(adaptive[0]),
        adaptiveBooleanMs: booleanElapsedMs(adaptive[0]),
      };
  const incremental = process.env.ECKY_BRACELET_SKIP_INCREMENTAL === '1'
    ? null
    : runIncremental({ runDir, cli, workers });
  const resourceSamples = [warmup, ...outerOnly, ...adaptive,
    ...(incremental ? Object.values(incremental).filter(value => value?.resource) : [])];
  const report = {
    schemaVersion: 1,
    provenance: {
      harness: relative(root, fileURLToPath(import.meta.url)),
      fixture: relative(root, braceletFixture),
      fixtureDigest: sha256(readFileSync(braceletFixture)),
      cli: relative(root, cli),
      cliDigest: sha256(readFileSync(cli)),
    },
    cpuCount: cpus().length,
    workers,
    samples,
    warmup,
    outerOnly,
    adaptive,
    gate,
    incremental,
    resourceEvidence: summarizeResourceEvidence(resourceSamples.filter(Boolean)),
  };
  writeFileSync(join(runDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ report: join(runDir, 'report.json'), gate, incremental: incremental?.gate ?? null }, null, 2)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(error.stack || error.message); process.exitCode = 1; }
}
