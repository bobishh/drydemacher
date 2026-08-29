import test from 'node:test';
import assert from 'node:assert/strict';

import { createRevisionedSingleflight } from './historyProjectionRefresh';

test('revisioned refresh coalesces a burst and never applies the stale response', async () => {
  const gates: Array<() => void> = [];
  const started: number[] = [];
  const applied: number[] = [];
  const coordinator = createRevisionedSingleflight<number>();
  const fetch = async (revision: number) => {
    started.push(revision);
    await new Promise<void>((resolve) => gates.push(resolve));
    return revision;
  };

  const first = coordinator.request('thread-1', 1, fetch, (value) => applied.push(value));
  for (let revision = 2; revision <= 50; revision += 1) {
    void coordinator.request('thread-1', revision, fetch, (value) => applied.push(value));
  }
  gates.shift()?.();
  await new Promise((resolve) => setTimeout(resolve, 0));
  gates.shift()?.();
  await first;

  assert.deepEqual(started, [1, 50]);
  assert.deepEqual(applied, [50]);
});

test('revisioned refresh keeps different threads independent', async () => {
  const coordinator = createRevisionedSingleflight<string>();
  const applied: string[] = [];
  await Promise.all([
    coordinator.request('a', 1, async () => 'a', (value) => applied.push(value)),
    coordinator.request('b', 1, async () => 'b', (value) => applied.push(value)),
  ]);
  assert.deepEqual(applied.sort(), ['a', 'b']);
});
