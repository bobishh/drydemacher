#!/usr/bin/env node
/**
 * Release-only evidence harness for source -> `ecky render` -> Direct OCCT.
 *
 * It deliberately does not call direct-occt-runner with hand-authored plans:
 * every sample starts at a provenance-recorded Ecky source fixture. The small
 * worker writes compact JSON before task_resource_guard removes sample STL/STEP
 * output. Large runner artifacts never become benchmark evidence.
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cpus, tmpdir } from 'node:os';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const guard = join(root, 'scripts/task_resource_guard.mjs');
export const realFixture = join(root, 'model-runtime/examples/film-adapter-golden-6part.ecky');
export const localizedFixture = join(root, 'model-runtime/examples/physical-decision-calibration.ecky');

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function median(values) {
  assert.ok(values.length > 0, 'median needs at least one value');
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.floor(ordered.length / 2)];
}

function commandOutputText(output) {
  return `${output.stdout || ''}${output.stderr || ''}`.trim();
}

function parseCliJson(stdout) {
  for (const line of stdout.split(/\r?\n/).reverse()) {
    const text = line.trim();
    if (!text.startsWith('{')) continue;
    try { return JSON.parse(text); } catch { /* retain raw output below */ }
  }
  return undefined;
}

function numericKey(number) {
  return Number(number).toPrecision(15);
}

function parseStl(path) {
  const bytes = readFileSync(path);
  const triangles = [];
  if (bytes.length >= 84 && 84 + bytes.readUInt32LE(80) * 50 === bytes.length) {
    for (let offset = 84; offset < bytes.length; offset += 50) {
      const triangle = [];
      for (let vertex = 0; vertex < 3; vertex += 1) {
        const start = offset + 12 + vertex * 12;
        triangle.push([bytes.readFloatLE(start), bytes.readFloatLE(start + 4), bytes.readFloatLE(start + 8)]);
      }
      triangles.push(triangle);
    }
  } else {
    const points = [];
    const matcher = /vertex\s+([-+\deE.]+)\s+([-+\deE.]+)\s+([-+\deE.]+)/g;
    for (const match of bytes.toString('utf8').matchAll(matcher)) points.push([Number(match[1]), Number(match[2]), Number(match[3])]);
    for (let index = 0; index + 2 < points.length; index += 3) triangles.push(points.slice(index, index + 3));
  }
  return triangles;
}

function meshMetrics(path) {
  if (!existsSync(path)) return { exists: false, digest: null, bounds: null, signedVolume: null, components: null, triangleCount: 0 };
  const bytes = readFileSync(path);
  const triangles = parseStl(path);
  if (triangles.length === 0) return { exists: true, digest: sha256(bytes), bounds: null, signedVolume: 0, components: 0, triangleCount: 0 };
  const bounds = { xMin: Infinity, yMin: Infinity, zMin: Infinity, xMax: -Infinity, yMax: -Infinity, zMax: -Infinity };
  const vertices = new Map();
  const parents = [];
  function rootOf(index) { while (parents[index] !== index) { parents[index] = parents[parents[index]]; index = parents[index]; } return index; }
  function union(left, right) { left = rootOf(left); right = rootOf(right); if (left !== right) parents[right] = left; }
  function vertexId(point) {
    const key = point.map(numericKey).join(',');
    if (vertices.has(key)) return vertices.get(key);
    const id = parents.length;
    vertices.set(key, id);
    parents.push(id);
    return id;
  }
  let signedVolume = 0;
  for (const triangle of triangles) {
    const ids = triangle.map(point => {
      bounds.xMin = Math.min(bounds.xMin, point[0]); bounds.yMin = Math.min(bounds.yMin, point[1]); bounds.zMin = Math.min(bounds.zMin, point[2]);
      bounds.xMax = Math.max(bounds.xMax, point[0]); bounds.yMax = Math.max(bounds.yMax, point[1]); bounds.zMax = Math.max(bounds.zMax, point[2]);
      return vertexId(point);
    });
    union(ids[0], ids[1]); union(ids[1], ids[2]);
    const [a, b, c] = triangle;
    signedVolume += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6;
  }
  const components = new Set(parents.map((_, index) => rootOf(index))).size;
  return { exists: true, digest: sha256(bytes), bounds, signedVolume, components, triangleCount: triangles.length };
}

function artifact(path) {
  if (!path || !existsSync(path)) return { exists: false, digest: null };
  return { exists: true, digest: sha256(readFileSync(path)) };
}

function topologyByPart(topology) {
  return Object.fromEntries((topology?.parts || []).map(part => [part.partId, sha256(JSON.stringify(part))]));
}

function partGeometry(bundleDir, manifest, topology) {
  const topologies = topologyByPart(topology);
  return Object.fromEntries((manifest.parts || []).map(part => {
    const path = part.viewerAssetPath ? resolve(bundleDir, part.viewerAssetPath) : join(bundleDir, 'parts', `${part.partId}.stl`);
    const metrics = meshMetrics(path);
    return [part.partId, { ...metrics, topologyDigest: topologies[part.partId] ?? null }];
  }));
}

function removeCliRuntimeBundle(bundleDir) {
  const temporaryRoot = resolve(tmpdir());
  const path = resolve(bundleDir);
  const relation = relative(temporaryRoot, path);
  const segments = relation.split(sep);
  // CliResolver owns `/tmp/ecky-cli-<pid>/...`; only that exact generated
  // runtime bundle is disposable after its compact report is persisted.
  if (relation.startsWith('..') || segments[0]?.startsWith('ecky-cli-') !== true || !segments.includes('model-runtime')) return false;
  rmSync(path, { recursive: true, force: true });
  return true;
}

function removeSampleCache(runDir, cacheDir) {
  const relation = relative(resolve(runDir), resolve(cacheDir));
  if (!relation || relation.startsWith('..') || relation.split(sep).includes('..')) throw new Error(`refusing to remove cache outside benchmark run: ${cacheDir}`);
  rmSync(cacheDir, { recursive: true, force: true });
}

export function buildRenderArgs({ cli, fixture, out, params = {}, driver = 'ecky', paramsPath, runtimeRoot }) {
  if (driver === 'native-occt') {
    assert.ok(paramsPath, 'native-occt driver requires paramsPath');
    assert.ok(runtimeRoot, 'native-occt driver requires runtimeRoot');
    return [cli, fixture, '--out-dir', out, '--params', paramsPath, '--runtime-root', runtimeRoot];
  }
  assert.equal(driver, 'ecky', `unsupported fixture harness driver ${driver}`);
  return [
    cli, 'render', '--backend', 'direct-occt', fixture, '--bundle-dir', out, '--json',
    ...Object.entries(params).sort(([left], [right]) => left.localeCompare(right)).flatMap(([key, value]) => ['--param', `${key}=${value}`]),
  ];
}

export function snapshotFromCli({ cliResult, sampleDir, fixture, params, outputDir }) {
  const cli = parseCliJson(cliResult.stdout || '');
  const rawErrors = [cliResult.stderr || ''].map(value => value.trim()).filter(Boolean);
  const base = {
    fixture: relative(root, fixture),
    params,
    exitCode: cliResult.status ?? 1,
    signal: cliResult.signal ?? null,
    rawErrors,
    elapsedMs: cliResult.elapsedMs,
    validity: false,
    bounds: null,
    signedVolume: null,
    components: null,
    parts: null,
    topologyDigest: null,
    topologyByPart: {},
    step: { exists: false, digest: null },
    stl: { exists: false, digest: null },
    perPart: {},
    stage: null,
  };
  if (cliResult.status !== 0 || (!cli?.manifestPath && !outputDir)) return base;
  const bundleDir = cli?.manifestPath ? dirname(cli.manifestPath) : resolve(outputDir);
  const topologyPath = join(bundleDir, 'topology.json');
  const stagePath = join(bundleDir, 'stage-report.json');
  const topology = existsSync(topologyPath) ? readJson(topologyPath) : null;
  const stage = existsSync(stagePath) ? readJson(stagePath) : null;
  const manifest = cli?.manifestPath && existsSync(cli.manifestPath)
    ? readJson(cli.manifestPath)
    : { parts: (topology?.parts || []).map(part => ({ partId: part.partId, viewerAssetPath: `parts/${part.partId}.stl` })) };
  const stlPath = cli?.previewStlPath || join(bundleDir, 'preview.stl');
  const stepPath = cli?.stepPath || join(bundleDir, 'model.step');
  for (const source of [cli?.manifestPath, topologyPath, stagePath, join(bundleDir, 'plan.json')].filter(Boolean)) {
    if (!existsSync(source)) continue;
    const destination = join(sampleDir, source.endsWith('manifest.json') ? 'runtime-manifest.json' : source.split('/').at(-1));
    writeFileSync(destination, readFileSync(source));
  }
  const stl = meshMetrics(stlPath);
  return {
    ...base,
    validity: Boolean(stage && topology && stl.exists && artifact(stepPath).exists && Number.isFinite(stl.signedVolume)),
    bounds: stl.bounds,
    signedVolume: stl.signedVolume,
    components: stl.components,
    parts: (manifest.parts || []).length,
    topologyDigest: topology ? sha256(JSON.stringify(topology)) : null,
    topologyByPart: topologyByPart(topology),
    step: artifact(stepPath),
    stl,
    perPart: partGeometry(bundleDir, manifest, topology),
    stage,
    runtimeBundleRemoved: cli?.manifestPath ? removeCliRuntimeBundle(bundleDir) : false,
  };
}

function parityProjection(sample) {
  return {
    validity: sample.validity,
    bounds: sample.bounds,
    signedVolume: sample.signedVolume,
    components: sample.components,
    parts: sample.parts,
    topologyDigest: sample.topologyDigest,
    stepExists: sample.step.exists,
    stlExists: sample.stl.exists,
    rawErrors: sample.rawErrors,
  };
}

export function evaluateWorkerParity(samples) {
  assert.ok(samples.length > 0, 'need worker samples');
  for (const sample of samples) assert.equal(sample.validity, true, `invalid worker sample ${sample.label}`);
  const reference = parityProjection(samples[0]);
  for (const sample of samples) assert.deepEqual(parityProjection(sample), reference, `worker parity failed for ${sample.label}`);
  return {
    passed: true,
    fields: reference,
    artifactDigests: samples.map(sample => ({ label: sample.label, step: sample.step.digest, stl: sample.stl.digest })),
  };
}

function assertWorkerBudget(samples, expected) {
  for (const sample of samples) {
    assert.equal(sample.stage?.workerBudget, expected, `${sample.label} reported workerBudget ${sample.stage?.workerBudget}, expected ${expected}`);
  }
}

function nativeElapsedMs(sample) {
  const elapsed = sample.stage?.totalElapsedMs;
  assert.ok(Number.isFinite(elapsed) && elapsed >= 0, `${sample.label} missing native stage totalElapsedMs`);
  return elapsed;
}

/** Pure balanced-DAG performance policy. Kept separate from runner execution so
 * retained reports and synthetic timing cases receive identical gate wording. */
export function evaluateSpeedupGate({ serialMedianMs, parallelMedianMs, threshold = 1.8 }) {
  assert.ok(Number.isFinite(serialMedianMs) && serialMedianMs > 0, 'serial median must be a positive finite millisecond value');
  assert.ok(Number.isFinite(parallelMedianMs) && parallelMedianMs > 0, 'parallel median must be a positive finite millisecond value');
  assert.ok(Number.isFinite(threshold) && threshold > 0, 'speedup threshold must be a positive finite number');
  const speedup = serialMedianMs / parallelMedianMs;
  const passed = speedup >= threshold;
  return {
    passed,
    speedup,
    threshold,
    reason: passed ? null : `balanced DAG median speedup ${speedup.toFixed(4)}x is below required ${threshold}x`,
  };
}

/** Pure localized-rerender timing policy. Geometry/cache assertions stay in
 * evaluateLocalizedRun; this owns only the 50-percent timing contract. */
export function evaluateLocalizedTimingGate({ coldMedianMs, warmMedianMs, threshold = 0.5 }) {
  assert.ok(Number.isFinite(coldMedianMs) && coldMedianMs > 0, 'localized cold median must be a positive finite millisecond value');
  assert.ok(Number.isFinite(warmMedianMs) && warmMedianMs >= 0, 'localized warm median must be a non-negative finite millisecond value');
  assert.ok(Number.isFinite(threshold) && threshold > 0, 'localized threshold must be a positive finite number');
  const ratio = warmMedianMs / coldMedianMs;
  const passed = ratio <= threshold;
  return {
    passed,
    ratio,
    threshold,
    reason: passed ? null : `localized warm median ${warmMedianMs}ms is ${(ratio * 100).toFixed(2)}% of cold ${coldMedianMs}ms; required at most ${(threshold * 100).toFixed(2)}%`,
  };
}

export function summarizeResourceEvidence(samples) {
  assert.ok(samples.length > 0, 'need guarded samples for resource evidence');
  const compact = samples.map(sample => {
    const resource = sample.resource;
    assert.equal(resource?.outcome, 'success', `${sample.label} missing successful resource report`);
    assert.ok(resource.peakTaskRssBytes <= resource.limits?.taskCapBytes, `${sample.label} exceeded task RSS cap`);
    assert.ok(resource.hostAvailableMinBytes >= resource.limits?.hostFloorBytes, `${sample.label} crossed host memory floor`);
    assert.equal(resource.swap?.didNotGrow, true, `${sample.label} grew swap by ${resource.swap?.growthBytes}`);
    assert.equal(resource.lease?.exclusiveRequested, true, `${sample.label} did not request the benchmark lease`);
    assert.equal(resource.lease?.exclusiveAcquired, true, `${sample.label} did not acquire the benchmark lease`);
    assert.equal(resource.lease?.sampleOverlapDetected, false, `${sample.label} overlapped another benchmark sample`);
    assert.equal(resource.terminated, false, `${sample.label} was watchdog terminated`);
    return {
      label: sample.label,
      peakTaskRssBytes: resource.peakTaskRssBytes,
      hostAvailableMinBytes: resource.hostAvailableMinBytes,
      swap: resource.swap,
      lease: resource.lease,
      terminated: resource.terminated,
    };
  });
  return {
    passed: true,
    samples: compact,
    maxPeakTaskRssBytes: Math.max(...compact.map(sample => sample.peakTaskRssBytes)),
    minHostAvailableBytes: Math.min(...compact.map(sample => sample.hostAvailableMinBytes)),
    totalSwapGrowthBytes: compact.reduce((total, sample) => total + sample.swap.growthBytes, 0),
  };
}

export function evaluateLocalizedRun(cold, warm) {
  const partIds = ['calibration_magnet_coupon', 'calibration_film_clamp_coupon', 'calibration_lens_thread_coupon'];
  const [first, middle, third] = partIds;
  assert.ok(cold.validity && warm.validity, 'cold and warm localized samples must be valid');
  assert.deepEqual(warm.bounds, cold.bounds, 'film_gap must not change assembly bounds');
  assert.equal(warm.components, cold.components, 'film_gap must not change assembly component count');
  assert.equal(warm.parts, cold.parts, 'film_gap must not change part count');
  assert.equal(warm.step.exists, cold.step.exists, 'STEP existence changed across localized render');
  assert.equal(warm.stl.exists, cold.stl.exists, 'STL existence changed across localized render');
  assert.deepEqual(warm.rawErrors, cold.rawErrors, 'raw errors changed across localized render');
  for (const partId of [first, third]) {
    const evidence = warm.stage?.parts?.find(part => part.partId === partId);
    assert.equal(evidence?.cacheHit, true, `${partId} must be a cache hit`);
    assert.equal(evidence?.executedCommandCount, 0, `${partId} must execute zero commands`);
    assert.deepEqual(warm.perPart[partId], cold.perPart[partId], `${partId} geometry/topology changed`);
  }
  const dirty = warm.stage?.parts?.find(part => part.partId === middle);
  assert.equal(dirty?.cacheHit, false, `${middle} must be a cache miss`);
  assert.ok((dirty?.executedCommandCount ?? 0) > 0, `${middle} must execute its dirty closure`);
  const topologyChanged = cold.perPart[middle]?.topologyDigest !== warm.perPart[middle]?.topologyDigest;
  const geometryChanged = cold.perPart[middle]?.digest !== warm.perPart[middle]?.digest || cold.perPart[middle]?.signedVolume !== warm.perPart[middle]?.signedVolume;
  assert.ok(topologyChanged || geometryChanged, 'film_gap .30 -> .31 did not change middle output');
  return { first, middle, third, topologyChanged, geometryChanged };
}

export function criticalStages(samples) {
  const totals = new Map();
  for (const sample of samples) for (const stage of sample.stage?.stages || []) {
    totals.set(stage.name, [...(totals.get(stage.name) || []), stage.elapsedMs]);
  }
  return [...totals].map(([name, elapsed]) => ({ name, medianMs: median(elapsed) })).sort((left, right) => right.medianMs - left.medianMs);
}

function sampleCount() {
  const count = Number(process.env.ECKY_FIXTURE_BENCH_SAMPLES ?? 5);
  if (!Number.isInteger(count) || count < 5) throw new Error('ECKY_FIXTURE_BENCH_SAMPLES must be an integer >= 5');
  return count;
}

function productionWorkers() {
  const requested = Number(process.env.ECKY_DIRECT_OCCT_WORKERS ?? Math.min(cpus().length, 8));
  if (!Number.isInteger(requested) || requested < 1) throw new Error('ECKY_DIRECT_OCCT_WORKERS must be an integer >= 1');
  return requested;
}

function fixtureDriver() {
  const driver = process.env.ECKY_DIRECT_OCCT_DRIVER ?? 'ecky';
  if (driver !== 'ecky' && driver !== 'native-occt') throw new Error('ECKY_DIRECT_OCCT_DRIVER must be ecky or native-occt');
  return driver;
}

function releaseCli() {
  return resolve(process.env.ECKY_DIRECT_OCCT_CLI ?? join(root, 'src-tauri/target/release/ecky'));
}

function writeRequest(sampleDir, request) {
  const path = join(sampleDir, 'request.json');
  writeJson(path, request);
  return path;
}

function runGuardedSample(request) {
  mkdirSync(request.sampleDir, { recursive: true });
  const requestPath = writeRequest(request.sampleDir, request);
  const guardArgs = [
    guard, '--report', join(request.sampleDir, 'resource-report.json'), '--exclusive', 'native-occt-fixture-benchmark',
    '--cleanup-dir', request.sampleDir, '--reservation-mib', process.env.ECKY_FIXTURE_BENCH_RESERVATION_MIB ?? '1024',
    '--', process.execPath, join(root, 'scripts/benchmark_direct_occt_fixture_worker.mjs'), '--request', requestPath,
  ];
  const guardOutput = spawnSync(process.execPath, guardArgs, { cwd: root, encoding: 'utf8', env: process.env });
  const resultPath = join(request.sampleDir, 'result.json');
  if (!existsSync(resultPath)) throw new Error(`sample ${request.label} did not write result.json\n${commandOutputText(guardOutput)}`);
  const result = readJson(resultPath);
  result.label = request.label;
  if (guardOutput.status !== 0 || result.exitCode !== 0) {
    throw new Error(`sample ${request.label} failed\n${JSON.stringify(result.rawErrors)}\n${commandOutputText(guardOutput)}`);
  }
  const resourcePath = join(request.sampleDir, 'resource-report.json');
  if (!existsSync(resourcePath)) throw new Error(`sample ${request.label} did not write resource report`);
  result.resource = readJson(resourcePath);
  return result;
}

function makeRequest({ runDir, label, fixture, cli, driver, runtimeRoot, params, workers, cacheDir }) {
  const sampleDir = join(runDir, label);
  return { label, sampleDir, fixture, cli, driver, runtimeRoot, params, workers, cacheDir };
}

function runReal({ runDir, cli, driver, runtimeRoot, samples, workers }) {
  const warmup = runGuardedSample(makeRequest({ runDir, label: 'real-warmup', fixture: realFixture, cli, driver, runtimeRoot, workers, cacheDir: join(runDir, 'cache', 'real-warmup'), params: {} }));
  removeSampleCache(runDir, join(runDir, 'cache', 'real-warmup'));
  const serial = Array.from({ length: samples }, (_, index) => {
    const cacheDir = join(runDir, 'cache', `real-worker1-${index + 1}`);
    const sample = runGuardedSample(makeRequest({ runDir, label: `real-worker1-${index + 1}`, fixture: realFixture, cli, driver, runtimeRoot, workers: 1, cacheDir, params: {} }));
    removeSampleCache(runDir, cacheDir);
    return sample;
  });
  const production = Array.from({ length: samples }, (_, index) => {
    const cacheDir = join(runDir, 'cache', `real-production-${index + 1}`);
    const sample = runGuardedSample(makeRequest({ runDir, label: `real-production-${index + 1}`, fixture: realFixture, cli, driver, runtimeRoot, workers, cacheDir, params: {} }));
    removeSampleCache(runDir, cacheDir);
    return sample;
  });
  assertWorkerBudget(serial, 1);
  assertWorkerBudget(production, workers);
  const parity = evaluateWorkerParity([...serial, ...production]);
  const serialMedianMs = median(serial.map(nativeElapsedMs));
  const productionMedianMs = median(production.map(nativeElapsedMs));
  return { warmup, serial, production, serialMedianMs, productionMedianMs, naturalSpeedup: serialMedianMs / productionMedianMs, timingSource: 'stage-report.totalElapsedMs', criticalStages: criticalStages(production), parity };
}

function runLocalized({ runDir, cli, driver, runtimeRoot, samples, workers }) {
  const runs = Array.from({ length: samples }, (_, index) => {
    const cacheDir = join(runDir, 'cache', `localized-${index + 1}`);
    const cold = runGuardedSample(makeRequest({ runDir, label: `localized-cold-${index + 1}`, fixture: localizedFixture, cli, driver, runtimeRoot, workers, cacheDir, params: { film_gap: 0.30 } }));
    const warm = runGuardedSample(makeRequest({ runDir, label: `localized-warm-${index + 1}`, fixture: localizedFixture, cli, driver, runtimeRoot, workers, cacheDir, params: { film_gap: 0.31 } }));
    const validation = evaluateLocalizedRun(cold, warm);
    removeSampleCache(runDir, cacheDir);
    return { cold, warm, validation };
  });
  const coldMedianMs = median(runs.map(run => nativeElapsedMs(run.cold)));
  const warmMedianMs = median(runs.map(run => nativeElapsedMs(run.warm)));
  const timingGate = evaluateLocalizedTimingGate({ coldMedianMs, warmMedianMs });
  assert.ok(timingGate.passed, timingGate.reason);
  return { runs, coldMedianMs, warmMedianMs, ratio: timingGate.ratio, gate: true, timingGate };
}

export function buildReport({ runDir, cli, driver, runtimeRoot, workers, samples, real, localized }) {
  const resourceSamples = [
    real.warmup, ...real.serial, ...real.production,
    ...localized.runs.flatMap(run => [run.cold, run.warm]),
  ];
  return {
    schemaVersion: 2,
    provenance: {
      harness: relative(root, fileURLToPath(import.meta.url)),
      driver,
      cli: relative(root, cli),
      cliDigest: sha256(readFileSync(cli)),
      runtimeRoot: relative(root, runtimeRoot),
      realFixture: { path: relative(root, realFixture), digest: sha256(readFileSync(realFixture)) },
      localizedFixture: { path: relative(root, localizedFixture), digest: sha256(readFileSync(localizedFixture)), baseline: { film_gap: 0.30 }, changed: { film_gap: 0.31 } },
    },
    runDir: relative(root, runDir), cpuCount: cpus().length, productionWorkers: workers, samples,
    real: { ...real, gate: 'informational-natural-speedup-only' },
    localized,
    resourceEvidence: summarizeResourceEvidence(resourceSamples),
  };
}

export function main() {
  const cli = releaseCli();
  if (!existsSync(cli)) throw new Error(`release Ecky CLI missing: ${cli}\nBuild first: cd src-tauri && cargo build --release --bin ecky`);
  const samples = sampleCount();
  const workers = productionWorkers();
  const driver = fixtureDriver();
  const runtimeRoot = resolve(process.env.ECKY_DIRECT_OCCT_RUNTIME_ROOT ?? join(root, '.dist/runtime/occt'));
  if (driver === 'native-occt' && !existsSync(runtimeRoot)) throw new Error(`native OCCT runtime root missing: ${runtimeRoot}`);
  const outputRoot = resolve(process.env.ECKY_FIXTURE_BENCH_OUT ?? join(root, 'tmp/direct-occt-fixture-bench'));
  const runDir = join(outputRoot, `run-${new Date().toISOString().replace(/[:.]/g, '-')}-${process.pid}`);
  mkdirSync(runDir, { recursive: true });
  const real = runReal({ runDir, cli, driver, runtimeRoot, samples, workers });
  const localized = runLocalized({ runDir, cli, driver, runtimeRoot, samples, workers });
  const report = buildReport({ runDir, cli, driver, runtimeRoot, workers, samples, real, localized });
  writeJson(join(runDir, 'report.json'), report);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(error.stack || error.message); process.exitCode = 1; }
}
