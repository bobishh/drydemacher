#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { spawnSync } from 'node:child_process';
import { buildRenderArgs, root, snapshotFromCli } from './benchmark_direct_occt_fixtures.mjs';

function requestPath(argv) {
  if (argv.length !== 2 || argv[0] !== '--request') throw new Error('usage: benchmark_direct_occt_fixture_worker.mjs --request <request.json>');
  return resolve(argv[1]);
}

function main() {
  const request = JSON.parse(readFileSync(requestPath(process.argv.slice(2)), 'utf8'));
  const bundleDir = join(request.sampleDir, 'bundle');
  mkdirSync(bundleDir, { recursive: true });
  const paramsPath = join(request.sampleDir, 'params.json');
  writeFileSync(paramsPath, `${JSON.stringify(request.params)}\n`);
  const command = buildRenderArgs({ cli: request.cli, fixture: request.fixture, out: bundleDir, params: request.params, driver: request.driver, paramsPath, runtimeRoot: request.runtimeRoot });
  const started = performance.now();
  const execution = spawnSync(command[0], command.slice(1), {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, ECKY_DIRECT_OCCT_WORKERS: String(request.workers), ECKY_DIRECT_OCCT_CACHE_DIR: request.cacheDir },
  });
  const snapshot = snapshotFromCli({
    cliResult: { status: execution.status, signal: execution.signal, stdout: execution.stdout || '', stderr: execution.stderr || '', elapsedMs: performance.now() - started },
    sampleDir: request.sampleDir, fixture: request.fixture, params: request.params, outputDir: bundleDir,
  });
  snapshot.requestedWorkers = request.workers;
  snapshot.command = command;
  writeFileSync(join(request.sampleDir, 'result.json'), `${JSON.stringify(snapshot, null, 2)}\n`);
  process.exitCode = execution.status ?? 1;
}

try { main(); } catch (error) { console.error(error.stack || error.message); process.exitCode = 1; }
