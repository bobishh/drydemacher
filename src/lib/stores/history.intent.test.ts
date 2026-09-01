import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./history.ts', import.meta.url), 'utf8');

function between(start: string, end: string): string {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from + start.length);
  assert.notEqual(from, -1, `missing ${start}`);
  assert.notEqual(to, -1, `missing ${end}`);
  return source.slice(from, to);
}

test('delete version is one Rust intent and applies only its canonical projection', () => {
  const body = between('export async function deleteVersion', 'export async function restoreVersion');
  assert.match(body, /deleteVersionIntent\s*\(/);
  assert.doesNotMatch(body, /deleteVersionCommand\s*\(/);
  assert.doesNotMatch(body, /refreshHistory\s*\(/);
  assert.doesNotMatch(body, /getThreadLatestVersion\s*\(/);
  assert.doesNotMatch(body, /getThreadMessagesPage\s*\(/);
  assert.doesNotMatch(body, /persistLastSessionSnapshot\s*\(/);
});

test('restore version is one Rust intent and applies only its canonical projection', () => {
  const body = between('export async function restoreVersion', 'export async function createNewThread');
  assert.match(body, /restoreVersionIntent\s*\(/);
  assert.doesNotMatch(body, /restoreVersionCommand\s*\(/);
  assert.doesNotMatch(body, /refreshHistory\s*\(/);
  assert.doesNotMatch(body, /getThreadLatestVersion\s*\(/);
  assert.doesNotMatch(body, /getThreadMessagesPage\s*\(/);
  assert.doesNotMatch(body, /persistLastSessionSnapshot\s*\(/);
});

test('backend-owned workspace projection hydration never writes a duplicate snapshot', () => {
  const body = between('export async function projectWorkspaceProjection', '/** Activate one canonical workspace');
  assert.match(body, /persistSnapshot:\s*false/);
  const create = between('export async function createNewThread', 'async function resolveActiveVersionMessage');
  assert.doesNotMatch(create, /clearLastSessionSnapshot\s*\(/);
});

test('rename projects its exact accepted title without a global history refresh', () => {
  const body = between('export async function renameThread', 'export async function deleteVersion');
  assert.match(body, /renameThreadCommand\s*\(/);
  assert.doesNotMatch(body, /refreshHistory\s*\(/);
});

test('thread delete, finalize, reopen, and inventory open each use one Rust projection intent', () => {
  const deletion = between('export async function deleteThread', 'export async function renameThread');
  assert.match(deletion, /deleteThreadIntent\s*\(/);
  assert.doesNotMatch(deletion, /getHistory\s*\(|clearLastSessionSnapshot\s*\(/);

  const finalize = between('export async function finalizeThread', 'export async function reopenThread');
  assert.match(finalize, /finalizeThreadIntent\s*\(/);
  assert.doesNotMatch(finalize, /refreshHistory\s*\(|clearLastSessionSnapshot\s*\(/);

  const reopen = between('export async function reopenThread', 'export async function openInventoryThread');
  assert.match(reopen, /reopenThreadIntent\s*\(/);
  assert.doesNotMatch(reopen, /refreshHistory\s*\(/);

  const inventory = between('export async function openInventoryThread', 'export async function loadOlderThreadMessages');
  assert.match(inventory, /openInventoryThreadIntent\s*\(/);
  assert.doesNotMatch(inventory, /getInventoryCommand\s*\(|loadFromHistory\s*\(/);
});
