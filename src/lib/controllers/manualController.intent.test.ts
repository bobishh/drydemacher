import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('./manualController.ts', import.meta.url), 'utf8');

test('manual controller never manufactures thread identity', () => {
  assert.doesNotMatch(source, /crypto\.randomUUID\s*\(/);
});

test('Given edited code fork When confirmed Then frontend sends one Rust-owned thread intent', () => {
  const fork = source.slice(source.indexOf('export async function forkManualVersion'));
  assert.match(fork, /createDesignThreadIntent\s*\(/);
  assert.match(fork, /activateWorkspaceProjection\s*\(/);
  assert.doesNotMatch(fork, /crypto\.randomUUID\s*\(/);
  assert.doesNotMatch(fork, /commitManualVersion\s*\(/);
});
