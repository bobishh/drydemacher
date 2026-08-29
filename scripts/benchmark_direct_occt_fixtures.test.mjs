import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { buildRenderArgs, evaluateLocalizedRun, evaluateLocalizedTimingGate, evaluateSpeedupGate, evaluateWorkerParity, guard, localizedFixture, realFixture, root, summarizeResourceEvidence } from './benchmark_direct_occt_fixtures.mjs';

function sample(overrides = {}) {
  return {
    label: 'sample', validity: true,
    bounds: { xMin: 0, yMin: 0, zMin: 0, xMax: 10, yMax: 10, zMax: 10 },
    signedVolume: 100, components: 1, parts: 3, topologyDigest: 'topology-a', rawErrors: [],
    step: { exists: true, digest: 'step-a' }, stl: { exists: true, digest: 'stl-a' },
    perPart: {
      calibration_magnet_coupon: { digest: 'magnet', signedVolume: 10, topologyDigest: 'magnet-topology' },
      calibration_film_clamp_coupon: { digest: 'middle-a', signedVolume: 20, topologyDigest: 'middle-topology-a' },
      calibration_lens_thread_coupon: { digest: 'lens', signedVolume: 30, topologyDigest: 'lens-topology' },
    },
    stage: { parts: [
      { partId: 'calibration_magnet_coupon', cacheHit: true, executedCommandCount: 0 },
      { partId: 'calibration_film_clamp_coupon', cacheHit: false, executedCommandCount: 4 },
      { partId: 'calibration_lens_thread_coupon', cacheHit: true, executedCommandCount: 0 },
    ] },
    ...overrides,
  };
}

test('BDD Given source and unordered params When direct OCCT CLI arguments build Then source route and canonical parameter order remain explicit', () => {
  const args = buildRenderArgs({ cli: '/release/ecky', fixture: '/fixtures/real.ecky', out: '/sample/bundle', params: { zeta: 2, alpha: 1 } });
  assert.deepEqual(args, [
    '/release/ecky', 'render', '--backend', 'direct-occt', '/fixtures/real.ecky', '--bundle-dir', '/sample/bundle', '--json',
    '--param', 'alpha=1', '--param', 'zeta=2',
  ]);
});

test('BDD Given recorded benchmark fixtures When harness loads them Then real model has six parts and localized middle owns film_gap', () => {
  const realSource = readFileSync(realFixture, 'utf8');
  const localizedSource = readFileSync(localizedFixture, 'utf8');
  assert.equal((realSource.match(/\(part\s+/g) || []).length, 6);
  assert.match(localizedSource, /\(number film_gap 0\.30\b/);
  const partText = localizedSource.slice(localizedSource.indexOf('(part '));
  const filmGapParts = partText.split(/\n\s*(?=\(part )/).filter(part => /film_gap/.test(part));
  assert.equal(filmGapParts.length, 1);
  assert.match(filmGapParts[0], /^\(part calibration_film_clamp_coupon/);
});

test('BDD Given worker outputs with distinct artifact bytes When parity evaluates Then semantic fields pass and both digests stay in evidence', () => {
  const first = sample({ label: 'worker1', step: { exists: true, digest: 'step-serial' }, stl: { exists: true, digest: 'stl-serial' } });
  const second = sample({ label: 'production', step: { exists: true, digest: 'step-parallel' }, stl: { exists: true, digest: 'stl-parallel' } });
  const report = evaluateWorkerParity([first, second]);
  assert.equal(report.passed, true);
  assert.deepEqual(report.artifactDigests, [
    { label: 'worker1', step: 'step-serial', stl: 'stl-serial' },
    { label: 'production', step: 'step-parallel', stl: 'stl-parallel' },
  ]);
});

test('BDD Given retained pre-DAG balanced report When performance gate evaluates Then 0.9174x fails the 1.8x gate with its exact reason', () => {
  const baseline = JSON.parse(readFileSync(join(root, 'tmp/direct-occt-dag-bench/report.json'), 'utf8'));
  const gate = evaluateSpeedupGate({ serialMedianMs: baseline.serialMedianMs, parallelMedianMs: baseline.parallelMedianMs });
  assert.deepEqual(gate, {
    passed: false,
    speedup: baseline.speedup,
    threshold: 1.8,
    reason: 'balanced DAG median speedup 0.9174x is below required 1.8x',
  });
});

test('BDD Given balanced timings above threshold When performance gate evaluates Then it passes', () => {
  assert.deepEqual(evaluateSpeedupGate({ serialMedianMs: 180, parallelMedianMs: 100 }), {
    passed: true,
    speedup: 1.8,
    threshold: 1.8,
    reason: null,
  });
});

test('BDD Given localized warm median above half cold median When timing gate evaluates Then it fails with its exact reason', () => {
  assert.deepEqual(evaluateLocalizedTimingGate({ coldMedianMs: 100, warmMedianMs: 51 }), {
    passed: false,
    ratio: 0.51,
    threshold: 0.5,
    reason: 'localized warm median 51ms is 51.00% of cold 100ms; required at most 50.00%',
  });
});

test('BDD Given localized warm median at half cold median When timing gate evaluates Then it passes', () => {
  assert.deepEqual(evaluateLocalizedTimingGate({ coldMedianMs: 100, warmMedianMs: 50 }), {
    passed: true,
    ratio: 0.5,
    threshold: 0.5,
    reason: null,
  });
});

test('BDD Given successful guarded benchmark samples When compact evidence assembles Then every resource bound and no-overlap invariant is explicit', () => {
  const resource = {
    outcome: 'success', peakTaskRssBytes: 64, hostAvailableMinBytes: 512,
    limits: { taskCapBytes: 128, hostFloorBytes: 256 },
    swap: { beforeBytes: 10, afterBytes: 10, growthBytes: 0, didNotGrow: true },
    lease: { exclusiveRequested: true, exclusiveAcquired: true, sampleOverlapDetected: false },
    terminated: false,
  };
  const evidence = summarizeResourceEvidence([sample({ label: 'serial-1', resource }), sample({ label: 'parallel-1', resource })]);
  assert.equal(evidence.passed, true);
  assert.equal(evidence.samples.length, 2);
  assert.equal(evidence.maxPeakTaskRssBytes, 64);
  assert.equal(evidence.minHostAvailableBytes, 512);
  assert.equal(evidence.totalSwapGrowthBytes, 0);
});

test('BDD Given film_gap changes only middle part When localized report evaluates Then clean cache hits and dirty closure are required', () => {
  const cold = sample({ stage: { parts: [] } });
  const warm = sample({
    signedVolume: 99.7,
    perPart: {
      calibration_magnet_coupon: { digest: 'magnet', signedVolume: 10, topologyDigest: 'magnet-topology' },
      calibration_film_clamp_coupon: { digest: 'middle-b', signedVolume: 19.7, topologyDigest: 'middle-topology-b' },
      calibration_lens_thread_coupon: { digest: 'lens', signedVolume: 30, topologyDigest: 'lens-topology' },
    },
  });
  const result = evaluateLocalizedRun(cold, warm);
  assert.equal(result.geometryChanged, true);
  assert.equal(result.topologyChanged, true);
});

test('BDD Given a clean part executes kernel work When localized report evaluates Then it fails', () => {
  const cold = sample({ stage: { parts: [] } });
  const warm = sample({ stage: { parts: [
    { partId: 'calibration_magnet_coupon', cacheHit: true, executedCommandCount: 1 },
    { partId: 'calibration_film_clamp_coupon', cacheHit: false, executedCommandCount: 4 },
    { partId: 'calibration_lens_thread_coupon', cacheHit: true, executedCommandCount: 0 },
  ] } });
  assert.throws(() => evaluateLocalizedRun(cold, warm), /execute zero commands/);
});

test('BDD Given guarded source CLI sample When worker snapshots output Then compact evidence survives recursive geometry cleanup', () => {
  const stateDir = mkdtempSync(join(tmpdir(), 'ecky-fixture-worker-state-'));
  const sampleDir = join(stateDir, 'sample');
  mkdirSync(sampleDir);
  const fakeCli = join(stateDir, 'fake-ecky.mjs');
  writeFileSync(fakeCli, `#!/usr/bin/env node
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
const args = process.argv.slice(2);
const out = args[args.indexOf('--bundle-dir') + 1];
mkdirSync(join(out, 'parts'), { recursive: true });
writeFileSync(join(out, 'manifest.json'), JSON.stringify({ parts: [{ partId: 'sample', viewerAssetPath: 'parts/sample.stl' }] }));
writeFileSync(join(out, 'topology.json'), JSON.stringify({ parts: [{ partId: 'sample', edges: [], faces: [] }] }));
writeFileSync(join(out, 'plan.json'), JSON.stringify({ schemaVersion: 1, parts: [] }));
writeFileSync(join(out, 'stage-report.json'), JSON.stringify({ schemaVersion: 1, stages: [] }));
const stl = 'solid tiny\\nfacet normal 0 0 1\\n outer loop\\n  vertex 0 0 0\\n  vertex 1 0 0\\n  vertex 0 1 0\\n endloop\\nendfacet\\nendsolid tiny\\n';
writeFileSync(join(out, 'model.stl'), stl); writeFileSync(join(out, 'parts/sample.stl'), stl); writeFileSync(join(out, 'model.step'), 'ISO-10303-21;');
process.stdout.write(JSON.stringify({ manifestPath: join(out, 'manifest.json'), modelStlPath: join(out, 'model.stl'), stepPath: join(out, 'model.step') }) + '\\n');
`);
  chmodSync(fakeCli, 0o755);
  const requestPath = join(sampleDir, 'request.json');
  writeFileSync(requestPath, JSON.stringify({ label: 'fake', sampleDir, fixture: join(root, 'model-runtime/examples/physical-decision-calibration.ecky'), cli: fakeCli, params: {}, workers: 1, cacheDir: join(stateDir, 'cache') }));
  const output = spawnSync(process.execPath, [
    guard, '--state-dir', join(stateDir, 'guard'), '--report', join(sampleDir, 'resource-failure.json'),
    '--task-cap-mib', '512', '--host-floor-mib', '0', '--reservation-mib', '16', '--cleanup-dir', sampleDir,
    '--', process.execPath, join(root, 'scripts/benchmark_direct_occt_fixture_worker.mjs'), '--request', requestPath,
  ], { cwd: root, encoding: 'utf8' });
  assert.equal(output.status, 0, output.stderr);
  const result = JSON.parse(readFileSync(join(sampleDir, 'result.json'), 'utf8'));
  assert.equal(result.validity, true);
  for (const name of ['runtime-manifest.json', 'topology.json', 'stage-report.json', 'plan.json', 'result.json']) assert.equal(existsSync(join(sampleDir, name)), true, name);
  for (const name of ['model.step', 'model.stl', 'parts/sample.stl']) assert.equal(existsSync(join(sampleDir, 'bundle', name)), false, name);
});
