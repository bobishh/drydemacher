import assert from 'node:assert/strict';
import test from 'node:test';

import { selectProjectFolderWatchEvent } from './projectFolderWatchEvents';

test('active thread event wins over a later background folder event', () => {
  const selected = selectProjectFolderWatchEvent([
    { kind: 'detected', threadId: 'active' },
    { kind: 'applied', threadId: 'background' },
  ], 'active');

  assert.deepEqual(selected, { kind: 'detected', threadId: 'active' });
});

test('background event is ignored before an active thread exists', () => {
  const selected = selectProjectFolderWatchEvent([
    { kind: 'detected', threadId: 'first' },
    { kind: 'applied', threadId: 'second' },
  ], null);

  assert.equal(selected, undefined);
});

test('background event is ignored when it does not belong to the active thread', () => {
  const selected = selectProjectFolderWatchEvent([
    { kind: 'detected', threadId: 'background' },
  ], 'active');

  assert.equal(selected, undefined);
});
