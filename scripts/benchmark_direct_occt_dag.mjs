import { cpus } from 'node:os';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { evaluateSpeedupGate, summarizeResourceEvidence } from './benchmark_direct_occt_fixtures.mjs';

const root = resolve(new URL('..', import.meta.url).pathname);
const runner = resolve(process.env.ECKY_DIRECT_OCCT_RUNNER ?? join(root, '.dist/runtime/occt/bin/direct-occt-runner'));
const resultRoot = resolve(process.env.ECKY_DAG_BENCH_OUT ?? join(root, 'tmp/direct-occt-dag-bench'));
const parallelWorkers = Math.min(Math.max(cpus().length, 1), 8);
const resourceGuard = join(root, 'scripts/task_resource_guard.mjs');
const sampleCount = (() => {
  const parsed = Number(process.env.ECKY_DAG_BENCH_SAMPLES ?? 5);
  if (!Number.isInteger(parsed) || parsed < 1) throw new Error('ECKY_DAG_BENCH_SAMPLES must be an integer >= 1');
  return parsed;
})();

function arg(kind, value) { return { kind, value }; }
function ref(slot) { return arg('ref', slot); }
function branch(slot, arraySlot, y) {
  return [
    { output: slot, op: 'sphere', args: [arg('number', 10)], keywords: [] },
    {
      output: arraySlot,
      op: 'linear-array',
      args: [arg('number', 12), arg('number', 2.8), arg('number', y), arg('number', 0), ref(slot)],
      keywords: [],
    },
  ];
}
function part(key, offset) {
  return {
    key,
    label: key,
    root: 9,
    commands: [
      ...branch(1, 2, offset + 0), ...branch(3, 4, offset + 28),
      ...branch(5, 6, offset + 56), ...branch(7, 8, offset + 84),
      { output: 9, op: 'compound', args: [ref(2), ref(4), ref(6), ref(8)], keywords: [] },
    ],
  };
}
const plan = { schemaVersion: 1, planId: 'balanced-independent-dag-v1', parts: [part('left', 0), part('right', 128)] };

function run(mode, index, workers) {
  const out = join(resultRoot, `${mode}-${index}`);
  mkdirSync(out, { recursive: true });
  const planPath = join(out, 'plan.json');
  writeFileSync(planPath, JSON.stringify(plan));
  const started = performance.now();
  const resourceReport = join(out, 'resource-report.json');
  let resourceError;
  try {
    execFileSync(process.execPath, [resourceGuard,
      '--report', resourceReport,
      '--exclusive', 'native-occt-benchmark',
      '--cleanup-dir', out,
      '--reservation-mib', process.env.ECKY_DAG_BENCH_RESERVATION_MIB ?? '1024',
      '--', runner, '--plan', planPath, '--out', out,
    ], { encoding: 'utf8', env: { ...process.env, ECKY_DIRECT_OCCT_WORKERS: String(workers) } });
  } catch (error) { resourceError = error; }
  const elapsedMs = performance.now() - started;
  if (resourceError) {
    const failure = (() => { try { return readFileSync(resourceReport, 'utf8'); } catch { return resourceError.stderr || resourceError.message; } })();
    throw new Error(`runner ${mode}/${index} failed: ${failure}`);
  }
  return {
    elapsedMs, workers, output: out,
    stage: JSON.parse(readFileSync(join(out, 'stage-report.json'), 'utf8')),
    topology: readFileSync(join(out, 'topology.json'), 'utf8'),
    resource: JSON.parse(readFileSync(resourceReport, 'utf8')),
  };
}
function median(samples) {
  const sorted = samples.map(sample => sample.elapsedMs).sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

rmSync(resultRoot, { recursive: true, force: true });
mkdirSync(resultRoot, { recursive: true });
const warmup = run('warmup', 0, parallelWorkers);
const serial = Array.from({ length: sampleCount }, (_, i) => run('serial', i + 1, 1));
const parallel = Array.from({ length: sampleCount }, (_, i) => run('parallel', i + 1, parallelWorkers));
const baselineTopology = serial[0].topology;
if (![...serial, ...parallel].every(sample => sample.topology === baselineTopology)) {
  throw new Error('topology parity failed between worker budgets');
}
const report = {
  fixture: 'balanced-independent-dag-v1', cpuCount: cpus().length, parallelWorkers, sampleCount,
  serial, parallel,
  resourceEvidence: summarizeResourceEvidence([warmup, ...serial, ...parallel]),
  serialMedianMs: median(serial), parallelMedianMs: median(parallel),
  speedup: median(serial) / median(parallel),
  peakDagConcurrency: Math.max(...parallel.map(sample => sample.stage.peakDagConcurrency ?? 0)),
  resourcePolicy: {
    taskCapMiB: Number(process.env.ECKY_RESOURCE_TASK_CAP_MIB ?? 6144),
    hostFloorMiB: Number(process.env.ECKY_RESOURCE_HOST_FLOOR_MIB ?? 8192),
    sampleReservationMiB: Number(process.env.ECKY_DAG_BENCH_RESERVATION_MIB ?? 1024),
    sequential: true,
  },
  topologyParity: true,
};
report.performanceGate = evaluateSpeedupGate({ serialMedianMs: report.serialMedianMs, parallelMedianMs: report.parallelMedianMs });
report.gate = report.performanceGate.passed && report.peakDagConcurrency >= 2;
writeFileSync(join(resultRoot, 'report.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
if (!report.gate) process.exitCode = 2;
