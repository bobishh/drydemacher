import assert from 'node:assert/strict';
import test from 'node:test';

import { LatestTaskGate } from './latestTaskGate';

test('tasks for different actor targets remain current independently', () => {
  const gate = new LatestTaskGate();
  const threadA = gate.reserve('session-1:thread-a');
  const threadB = gate.reserve('session-1:thread-b');

  assert.equal(gate.isCurrent(threadA), true);
  assert.equal(gate.isCurrent(threadB), true);
});

test('new task supersedes only older task for same actor target', () => {
  const gate = new LatestTaskGate();
  const oldThreadA = gate.reserve('session-1:thread-a');
  const threadB = gate.reserve('session-1:thread-b');
  const newThreadA = gate.reserve('session-1:thread-a');

  assert.equal(gate.isCurrent(oldThreadA), false);
  assert.equal(gate.isCurrent(newThreadA), true);
  assert.equal(gate.isCurrent(threadB), true);
});
