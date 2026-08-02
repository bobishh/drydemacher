import assert from 'node:assert/strict';
import { once } from 'node:events';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import { spawn } from 'node:child_process';
import path from 'node:path';

import { resolveModelConfig } from './model_config.js';

const root = path.resolve(import.meta.dirname, '..');
const configFile = path.join(root, 'config.json');

test('Given camelCase /api/generate fields, when config resolves, then request values stay paired', () => {
  assert.deepEqual(
    resolveModelConfig(
      {
        provider: 'openai',
        apiKey: 'request-secret',
        model: 'request-model',
        baseUrl: 'https://request.example/v1',
      },
      {
        MODEL_API_KEY: 'environment-secret',
        MODEL_BASE_URL: 'https://environment.example/v1',
      },
    ),
    {
      provider: 'openai',
      apiKey: 'request-secret',
      model: 'request-model',
      baseUrl: 'https://request.example/v1',
    },
  );
});

test('Given /api/generate baseUrl without request apiKey, when config resolves, then env key cannot leak', () => {
  assert.throws(
    () =>
      resolveModelConfig(
        { provider: 'openai', baseUrl: 'https://attacker.example/collect' },
        { MODEL_API_KEY: 'environment-secret' },
      ),
    /baseUrl requires request apiKey/,
  );
});

test('Given env credentials, when /api/generate config resolves, then only env base URL is used', () => {
  assert.deepEqual(
    resolveModelConfig(
      { provider: 'openai' },
      {
        MODEL_API_KEY: 'environment-secret',
        MODEL_BASE_URL: 'https://environment.example/v1',
        MODEL_NAME: 'environment-model',
      },
    ),
    {
      provider: 'openai',
      apiKey: 'environment-secret',
      model: 'environment-model',
      baseUrl: 'https://environment.example/v1',
    },
  );
});

test('Given snake_case or unsupported /api/generate fields, when config resolves, then input is rejected', () => {
  assert.throws(
    () => resolveModelConfig({ provider: 'openai', api_key: 'secret' }, {}),
    /camelCase/,
  );
  assert.throws(
    () => resolveModelConfig({ provider: 'openai', base_url: 'https:\/\/example.test' }, {}),
    /camelCase/,
  );
  assert.throws(
    () => resolveModelConfig({ provider: 'unknown' }, {}),
    /Unsupported provider: unknown/,
  );
});

test('Given Node API starts, when config endpoints are requested, then config JSON stays retired', async (t) => {
  const source = readFileSync(path.join(root, 'server', 'index.ts'), 'utf8');
  assert.doesNotMatch(source, /config\.json/);
  const configExistedBeforeStart = existsSync(configFile);

  const server = spawn(process.execPath, ['--import', 'tsx', 'server/index.ts'], {
    cwd: root,
    env: { ...process.env, API_PORT: '0' },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  t.after(async () => {
    if (server.exitCode === null && server.signalCode === null) {
      server.kill('SIGTERM');
      await once(server, 'exit');
    }
  });

  let stdout = '';
  let stderr = '';
  server.stdout.setEncoding('utf8');
  server.stderr.setEncoding('utf8');
  server.stdout.on('data', (chunk: string) => {
    stdout += chunk;
  });
  server.stderr.on('data', (chunk: string) => {
    stderr += chunk;
  });

  const port = await new Promise<number>((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Server startup timed out.\nstdout:\n${stdout}\nstderr:\n${stderr}`));
    }, 5_000);
    const inspectOutput = () => {
      const match = stdout.match(/listening on http:\/\/localhost:(\d+)/);
      if (match) {
        clearTimeout(timeout);
        resolve(Number(match[1]));
      }
    };
    server.stdout.on('data', inspectOutput);
    server.once('exit', (code, signal) => {
      clearTimeout(timeout);
      reject(
        new Error(
          `Server exited during startup (${code ?? signal}).\nstdout:\n${stdout}\nstderr:\n${stderr}`,
        ),
      );
    });
  });
  const baseUrl = `http://127.0.0.1:${port}`;
  const healthResponse = await fetch(`${baseUrl}/api/health`);
  assert.equal(healthResponse.ok, true);
  assert.equal((await healthResponse.json() as { port: number }).port, port);
  assert.equal((await fetch(`${baseUrl}/api/config`)).status, 404);
  assert.equal(
    (await fetch(`${baseUrl}/api/config`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ selectedEngineId: 'not-used' }),
    })).status,
    404,
  );
  assert.equal(existsSync(configFile), configExistedBeforeStart);
});
