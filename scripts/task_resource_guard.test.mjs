import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('..', import.meta.url).pathname;
const guard = join(root, 'scripts/task_resource_guard.mjs');

function run(args, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [guard, ...args], {
      cwd: root,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', chunk => { stdout += chunk; });
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', reject);
    child.on('exit', (code, signal) => resolve({ code, signal, stdout, stderr }));
  });
}

function guardArgs(stateDir, reportPath, command) {
  return [
    '--state-dir', stateDir,
    '--report', reportPath,
    '--task-cap-mib', '64',
    '--host-floor-mib', '0',
    '--poll-ms', '25',
    '--term-grace-ms', '50',
    '--reservation-mib', '1',
    '--', ...command,
  ];
}

test('BDD Given a synthetic memory hog When cap crossed Then tree terminated and compact report written', async () => {
  const stateDir = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const report = join(stateDir, 'hog.json');
  const hog = "const blocks=[]; setInterval(()=>{ const b=Buffer.alloc(8*1024*1024); b.fill(1); blocks.push(b); }, 5);";
  const result = await run(guardArgs(stateDir, report, [process.execPath, '-e', hog]));
  assert.notEqual(result.code, 0);
  const failure = JSON.parse(readFileSync(report, 'utf8'));
  assert.equal(failure.reason, 'task_rss_cap');
  assert.equal(failure.terminated, true);
  assert.ok(failure.peakTaskRssBytes > 64 * 1024 * 1024);
  assert.deepEqual(Object.keys(failure).sort(), ['at', 'command', 'hostAvailableBytes', 'peakTaskRssBytes', 'reason', 'terminated']);
});

test('BDD Given a successful guarded sample When it exits Then compact resource evidence proves no overlap, no termination, and zero swap growth', async () => {
  const parent = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const stateDir = join(parent, 'fresh-state');
  const report = join(parent, 'success.json');
  const args = guardArgs(stateDir, report, [process.execPath, '-e', 'setTimeout(() => {}, 80)']);
  args.splice(args.indexOf('--'), 0, '--exclusive', 'success-evidence');
  const result = await run(args);
  assert.equal(result.code, 0, result.stderr);
  const success = JSON.parse(readFileSync(report, 'utf8'));
  assert.equal(success.outcome, 'success');
  assert.ok(success.peakTaskRssBytes > 0);
  assert.ok(success.hostAvailableMinBytes >= 0);
  assert.equal(success.swap.growthBytes, 0);
  assert.equal(success.swap.didNotGrow, true);
  assert.equal(success.lease.exclusiveRequested, true);
  assert.equal(success.lease.exclusiveAcquired, true);
  assert.equal(success.lease.sampleOverlapDetected, false);
  assert.ok(success.peakTaskRssBytes <= success.limits.taskCapBytes);
  assert.ok(success.hostAvailableMinBytes >= success.limits.hostFloorBytes);
  assert.equal(success.terminated, false);
});

test('BDD Given a restricted macOS PATH When swap is sampled Then the guard still records real swap evidence', {
  skip: process.platform !== 'darwin',
}, async () => {
  const parent = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const stateDir = join(parent, 'fresh-state');
  const report = join(parent, 'success.json');
  const args = guardArgs(stateDir, report, [process.execPath, '-e', 'setTimeout(() => {}, 80)']);
  const result = await run(args, { PATH: '/usr/bin:/bin' });
  assert.equal(result.code, 0, result.stderr);
  const success = JSON.parse(readFileSync(report, 'utf8'));
  assert.equal(typeof success.swap.beforeBytes, 'number');
  assert.equal(typeof success.swap.afterBytes, 'number');
  assert.equal(success.swap.didNotGrow, true);
});

test('BDD Given leased heavy task When next heavy task requests cap Then refused until release', async () => {
  const stateDir = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const reportA = join(stateDir, 'a.json');
  const reportB = join(stateDir, 'b.json');
  const argsA = guardArgs(stateDir, reportA, [process.execPath, '-e', 'setTimeout(() => {}, 700)']);
  argsA.splice(argsA.indexOf('--reservation-mib') + 1, 1, '48');
  const first = run(argsA);
  await new Promise(resolve => setTimeout(resolve, 100));
  const argsB = guardArgs(stateDir, reportB, [process.execPath, '-e', 'process.exit(0)']);
  argsB.splice(argsB.indexOf('--reservation-mib') + 1, 1, '48');
  const refused = await run(argsB);
  assert.notEqual(refused.code, 0);
  assert.equal(JSON.parse(readFileSync(reportB, 'utf8')).reason, 'admission_task_cap');
  await first;
  const admitted = await run(argsB);
  assert.equal(admitted.code, 0, admitted.stderr);
});

test('BDD Given exclusive benchmark lock When second sample starts Then it refuses', async () => {
  const stateDir = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const firstArgs = guardArgs(stateDir, join(stateDir, 'a.json'), [process.execPath, '-e', 'setTimeout(() => {}, 500)']);
  firstArgs.splice(firstArgs.indexOf('--'), 0, '--exclusive', 'native-benchmark');
  const first = run(firstArgs);
  await new Promise(resolve => setTimeout(resolve, 100));
  const secondArgs = guardArgs(stateDir, join(stateDir, 'b.json'), [process.execPath, '-e', 'process.exit(0)']);
  secondArgs.splice(secondArgs.indexOf('--'), 0, '--exclusive', 'native-benchmark');
  const second = await run(secondArgs);
  assert.notEqual(second.code, 0);
  assert.equal(JSON.parse(readFileSync(join(stateDir, 'b.json'), 'utf8')).reason, 'exclusive_lock');
  await first;
});

test('BDD Given sample artifacts When command completes Then only generated geometry is removed', async () => {
  const stateDir = mkdtempSync(join(tmpdir(), 'ecky-guard-state-'));
  const out = join(stateDir, 'sample');
  mkdirSync(out);
  mkdirSync(join(out, 'parts'));
  for (const name of ['model.step', 'mesh.stl', 'preview.glb', 'shape.brep', 'keep.json']) writeFileSync(join(out, name), name);
  writeFileSync(join(out, 'parts', 'left.stl'), 'left');
  writeFileSync(join(out, 'parts', 'right.STP'), 'right');
  mkdirSync(join(out, 'parts', 'nested'));
  writeFileSync(join(out, 'parts', 'nested', 'deep.brep'), 'deep');
  writeFileSync(join(out, 'parts', 'nested', 'keep.txt'), 'nested keep');
  writeFileSync(join(out, 'parts', 'keep.json'), 'nested keep');
  const args = guardArgs(stateDir, join(stateDir, 'cleanup.json'), [process.execPath, '-e', 'process.exit(0)']);
  args.splice(args.indexOf('--'), 0, '--cleanup-dir', out);
  const result = await run(args);
  assert.equal(result.code, 0, result.stderr);
  for (const name of ['model.step', 'mesh.stl', 'preview.glb', 'shape.brep']) assert.equal(existsSync(join(out, name)), false);
  assert.equal(readFileSync(join(out, 'keep.json'), 'utf8'), 'keep.json');
  assert.equal(existsSync(join(out, 'parts', 'left.stl')), false);
  assert.equal(existsSync(join(out, 'parts', 'right.STP')), false);
  assert.equal(existsSync(join(out, 'parts', 'nested', 'deep.brep')), false);
  assert.equal(readFileSync(join(out, 'parts', 'nested', 'keep.txt'), 'utf8'), 'nested keep');
  assert.equal(readFileSync(join(out, 'parts', 'keep.json'), 'utf8'), 'nested keep');
});
