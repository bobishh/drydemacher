#!/usr/bin/env node
// Shared local process-tree lease and memory watchdog. No project artifacts live here.
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, renameSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const MIB = 1024 * 1024;
const DEFAULT_TASK_CAP_BYTES = 6 * 1024 * 1024 * 1024;
const DEFAULT_HOST_FLOOR_BYTES = 8 * 1024 * 1024 * 1024;
const DEFAULT_POLL_MS = 250;
const DEFAULT_TERM_GRACE_MS = 1500;

function usage() {
  return 'usage: task_resource_guard.mjs [--state-dir DIR] [--report FILE] [--reservation-mib N] [--task-cap-mib N] [--host-floor-mib N] [--exclusive NAME] [--cleanup-dir DIR] -- command [args...]';
}

function positive(value, name) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) throw new Error(`${name} must be a non-negative number`);
  return parsed;
}

function parseArgs(argv) {
  const parsed = {
    stateDir: process.env.ECKY_RESOURCE_GUARD_STATE_DIR || join(tmpdir(), 'ecky-task-resource-guard'),
    report: undefined,
    reservationBytes: positive(process.env.ECKY_RESOURCE_RESERVATION_MIB || 0, 'reservation') * MIB,
    taskCapBytes: positive(process.env.ECKY_RESOURCE_TASK_CAP_MIB || DEFAULT_TASK_CAP_BYTES / MIB, 'task cap') * MIB,
    hostFloorBytes: positive(process.env.ECKY_RESOURCE_HOST_FLOOR_MIB || DEFAULT_HOST_FLOOR_BYTES / MIB, 'host floor') * MIB,
    pollMs: positive(process.env.ECKY_RESOURCE_POLL_MS || DEFAULT_POLL_MS, 'poll interval'),
    termGraceMs: positive(process.env.ECKY_RESOURCE_TERM_GRACE_MS || DEFAULT_TERM_GRACE_MS, 'termination grace'),
    exclusive: undefined,
    cleanupDir: undefined,
    command: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--') { parsed.command = argv.slice(index + 1); break; }
    const value = argv[++index];
    if (!value) throw new Error(`missing value for ${arg}`);
    if (arg === '--state-dir') parsed.stateDir = value;
    else if (arg === '--report') parsed.report = value;
    else if (arg === '--reservation-mib') parsed.reservationBytes = positive(value, 'reservation') * MIB;
    else if (arg === '--task-cap-mib') parsed.taskCapBytes = positive(value, 'task cap') * MIB;
    else if (arg === '--host-floor-mib') parsed.hostFloorBytes = positive(value, 'host floor') * MIB;
    else if (arg === '--poll-ms') parsed.pollMs = positive(value, 'poll interval');
    else if (arg === '--term-grace-ms') parsed.termGraceMs = positive(value, 'termination grace');
    else if (arg === '--exclusive') parsed.exclusive = value;
    else if (arg === '--cleanup-dir') parsed.cleanupDir = value;
    else throw new Error(`unknown option ${arg}`);
  }
  if (!parsed.command.length) throw new Error(usage());
  return parsed;
}

function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
function pidAlive(pid) { try { process.kill(pid, 0); return true; } catch { return false; } }
function registryPath(config) { return join(config.stateDir, 'leases.json'); }
function mutexPath(config) { return join(config.stateDir, 'leases.lock'); }

async function withRegistry(config, fn) {
  mkdirSync(config.stateDir, { recursive: true });
  const mutex = mutexPath(config);
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try { mkdirSync(mutex); break; } catch (error) {
      if (error?.code !== 'EEXIST') throw error;
      await sleep(10);
      if (attempt === 199) throw new Error('resource guard registry lock timed out');
    }
  }
  try {
    let leases = [];
    try { leases = JSON.parse(readFileSync(registryPath(config), 'utf8')); } catch (error) { if (error?.code !== 'ENOENT') throw error; }
    leases = leases.filter(lease => Number.isInteger(lease.pid) && pidAlive(lease.pid));
    const result = await fn(leases);
    const temporary = `${registryPath(config)}.${process.pid}.tmp`;
    writeFileSync(temporary, JSON.stringify(leases));
    renameSync(temporary, registryPath(config));
    return result;
  } finally { rmSync(mutex, { recursive: true, force: true }); }
}

function processTable() {
  try {
    const output = execFileSync('ps', ['-axo', 'pid=,ppid=,rss='], { encoding: 'utf8' });
    return output.trim().split('\n').filter(Boolean).map(line => {
      const [pid, ppid, rss] = line.trim().split(/\s+/).map(Number);
      return { pid, ppid, rssBytes: rss * 1024 };
    });
  } catch { return []; }
}

function treeRssBytes(rootPid, table = processTable()) {
  const children = new Map();
  for (const row of table) children.set(row.ppid, [...(children.get(row.ppid) || []), row]);
  let total = 0;
  const stack = [rootPid];
  const seen = new Set();
  while (stack.length) {
    const pid = stack.pop();
    if (seen.has(pid)) continue;
    seen.add(pid);
    const row = table.find(entry => entry.pid === pid);
    if (row) total += row.rssBytes;
    for (const child of children.get(pid) || []) stack.push(child.pid);
  }
  return total;
}

function activeLeaseRssBytes(config, table = processTable()) {
  try {
    const leases = JSON.parse(readFileSync(registryPath(config), 'utf8'));
    return leases.filter(lease => pidAlive(lease.pid))
      .reduce((total, lease) => total + treeRssBytes(lease.pid, table), 0);
  } catch { return 0; }
}

function hostAvailableBytes() {
  try {
    if (process.platform === 'linux') {
      const match = readFileSync('/proc/meminfo', 'utf8').match(/^MemAvailable:\s+(\d+) kB$/m);
      if (match) return Number(match[1]) * 1024;
    }
    if (process.platform === 'darwin') {
      const output = execFileSync('vm_stat', [], { encoding: 'utf8' });
      const pageSize = Number(output.match(/page size of (\d+) bytes/)?.[1] || 4096);
      const pages = ['Pages free', 'Pages inactive', 'Pages speculative', 'Pages purgeable']
        .map(key => Number(output.match(new RegExp(`${key}:\\s+(\\d+)\\.`))?.[1] || 0))
        .reduce((sum, value) => sum + value, 0);
      return pages * pageSize;
    }
  } catch { /* report unavailable as Infinity so it never causes a false kill */ }
  return Number.POSITIVE_INFINITY;
}

function swapUsedBytes() {
  try {
    if (process.platform === 'linux') {
      const meminfo = readFileSync('/proc/meminfo', 'utf8');
      const total = Number(meminfo.match(/^SwapTotal:\s+(\d+) kB$/m)?.[1]);
      const free = Number(meminfo.match(/^SwapFree:\s+(\d+) kB$/m)?.[1]);
      if (Number.isFinite(total) && Number.isFinite(free)) return (total - free) * 1024;
    }
    if (process.platform === 'darwin') {
      const output = execFileSync('/usr/sbin/sysctl', ['vm.swapusage'], { encoding: 'utf8' });
      const used = Number(output.match(/used =\s*([\d.]+)([KMG])/)?.[1]);
      const unit = output.match(/used =\s*[\d.]+([KMG])/)?.[1];
      const multiplier = { K: 1024, M: MIB, G: MIB * 1024 }[unit];
      if (Number.isFinite(used) && multiplier) return Math.round(used * multiplier);
    }
  } catch { /* unavailable metrics are explicit in the report */ }
  return null;
}

function compactFailure(config, reason, peakTaskRssBytes, hostAvailable) {
  return {
    at: new Date().toISOString(), command: config.command[0], hostAvailableBytes: Math.floor(hostAvailable),
    peakTaskRssBytes: Math.floor(peakTaskRssBytes), reason, terminated: reason === 'task_rss_cap' || reason === 'host_available_floor',
  };
}

function writeFailure(config, failure) {
  if (!config.report) return;
  mkdirSync(dirname(resolve(config.report)), { recursive: true });
  writeFileSync(config.report, JSON.stringify(failure));
}

function compactSuccess(config, { outcome, peakTaskRssBytes, hostAvailableMinBytes, swapBeforeBytes, swapAfterBytes, exclusiveAcquired }) {
  const swapMeasured = Number.isFinite(swapBeforeBytes) && Number.isFinite(swapAfterBytes);
  const growthBytes = swapMeasured ? swapAfterBytes - swapBeforeBytes : null;
  return {
    at: new Date().toISOString(),
    command: config.command[0],
    outcome,
    peakTaskRssBytes: Math.floor(peakTaskRssBytes),
    hostAvailableMinBytes: Number.isFinite(hostAvailableMinBytes) ? Math.floor(hostAvailableMinBytes) : null,
    limits: { taskCapBytes: config.taskCapBytes, hostFloorBytes: config.hostFloorBytes },
    swap: {
      beforeBytes: swapBeforeBytes,
      afterBytes: swapAfterBytes,
      growthBytes,
      didNotGrow: swapMeasured ? growthBytes <= 0 : false,
    },
    lease: {
      exclusiveRequested: Boolean(config.exclusive),
      exclusiveAcquired,
      sampleOverlapDetected: false,
    },
    terminated: false,
  };
}

function writeReport(config, report) {
  if (!config.report) return;
  mkdirSync(dirname(resolve(config.report)), { recursive: true });
  writeFileSync(config.report, JSON.stringify(report));
}

async function acquireExclusive(config) {
  if (!config.exclusive) return undefined;
  mkdirSync(config.stateDir, { recursive: true });
  const path = join(config.stateDir, `exclusive-${config.exclusive.replace(/[^A-Za-z0-9_.-]/g, '_')}.lock`);
  try { mkdirSync(path); return path; } catch (error) {
    if (error?.code === 'EEXIST') return undefined;
    throw error;
  }
}

function cleanupArtifacts(directory) {
  if (!directory) return;
  const allowed = new Set(['.step', '.stp', '.stl', '.glb', '.brep']);
  const safeDir = resolve(directory);
  function walk(current) {
    let entries = [];
    try { entries = readdirSync(current, { withFileTypes: true }); } catch { return; }
    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        walk(path);
        continue;
      }
      const suffix = entry.name.slice(entry.name.lastIndexOf('.')).toLowerCase();
      if (entry.isFile() && allowed.has(suffix)) unlinkSync(path);
    }
  }
  // `safeDir` is the exact sample output directory passed by the caller.
  // Recursion never follows a caller-provided parent or deletes directories.
  walk(safeDir);
}

async function terminateTree(pid, graceMs) {
  try { process.kill(-pid, 'SIGTERM'); } catch { try { process.kill(pid, 'SIGTERM'); } catch {} }
  await sleep(graceMs);
  if (pidAlive(pid)) {
    try { process.kill(-pid, 'SIGKILL'); } catch { try { process.kill(pid, 'SIGKILL'); } catch {} }
  }
}

async function main() {
  const config = parseArgs(process.argv.slice(2));
  let exclusivePath;
  try {
    exclusivePath = await acquireExclusive(config);
    if (config.exclusive && !exclusivePath) {
      writeFailure(config, compactFailure(config, 'exclusive_lock', 0, hostAvailableBytes()));
      return 75;
    }
    const available = hostAvailableBytes();
    if (available < config.hostFloorBytes) {
      writeFailure(config, compactFailure(config, 'admission_host_floor', 0, available));
      return 75;
    }
    const admitted = await withRegistry(config, leases => {
      const table = processTable();
      const aggregate = leases.reduce((total, lease) => total + treeRssBytes(lease.pid, table), 0);
      if (aggregate + config.reservationBytes > config.taskCapBytes) return false;
      return true;
    });
    if (!admitted) {
      writeFailure(config, compactFailure(config, 'admission_task_cap', 0, available));
      return 75;
    }
    const child = spawn(config.command[0], config.command.slice(1), { stdio: 'inherit', detached: true, env: process.env });
    await withRegistry(config, leases => { leases.push({ pid: child.pid, reservationBytes: config.reservationBytes }); });
    let peak = treeRssBytes(child.pid);
    let hostAvailableMin = available;
    const swapBeforeBytes = swapUsedBytes();
    let failure;
    const watchdog = setInterval(async () => {
      if (failure) return;
      const table = processTable();
      const rss = activeLeaseRssBytes(config, table);
      peak = Math.max(peak, rss);
      const availableNow = hostAvailableBytes();
      hostAvailableMin = Math.min(hostAvailableMin, availableNow);
      if (rss > config.taskCapBytes || availableNow < config.hostFloorBytes) {
        failure = rss > config.taskCapBytes ? 'task_rss_cap' : 'host_available_floor';
        await terminateTree(child.pid, config.termGraceMs);
      }
    }, Math.max(10, config.pollMs));
    const outcome = await new Promise(resolve => child.once('exit', (code, signal) => resolve({ code, signal })));
    clearInterval(watchdog);
    await withRegistry(config, leases => { const index = leases.findIndex(lease => lease.pid === child.pid); if (index >= 0) leases.splice(index, 1); });
    if (failure) {
      writeFailure(config, compactFailure(config, failure, peak, hostAvailableBytes()));
      return 70;
    }
    if (outcome.code !== 0) return outcome.code ?? 1;
    const swapAfterBytes = swapUsedBytes();
    const success = compactSuccess(config, {
      outcome: 'success', peakTaskRssBytes: peak, hostAvailableMinBytes: Math.min(hostAvailableMin, hostAvailableBytes()),
      swapBeforeBytes, swapAfterBytes, exclusiveAcquired: Boolean(exclusivePath),
    });
    writeReport(config, success);
    if (!success.swap.didNotGrow) return 70;
    return 0;
  } finally {
    try { cleanupArtifacts(config.cleanupDir); } catch { /* cleanup never deletes outside the exact sample directory */ }
    if (exclusivePath) rmSync(exclusivePath, { recursive: true, force: true });
  }
}

main().then(code => { process.exitCode = code; }).catch(error => { console.error(error.message); process.exitCode = 64; });
