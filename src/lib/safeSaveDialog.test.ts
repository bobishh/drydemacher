import assert from 'node:assert/strict';
import test from 'node:test';
import { createSaveDialogGate, type SaveDialogOptions } from './safeSaveDialog';

const options: SaveDialogOptions = {
  defaultPath: 'part.step',
  filters: [{ name: 'STEP', extensions: ['step'] }],
};

test('save dialog gate rejects overlap and releases after completion', async () => {
  let release!: (path: string) => void;
  const gate = createSaveDialogGate(
    () => new Promise<string>((resolve) => { release = resolve; }),
  );
  const first = gate(options);
  await assert.rejects(gate(options), /already open/);
  release('/tmp/part.step');
  assert.equal(await first, '/tmp/part.step');
});

test('save dialog gate releases after backend error', async () => {
  let calls = 0;
  const gate = createSaveDialogGate(async () => {
    calls += 1;
    if (calls === 1) throw new Error('native panel unavailable');
    return '/tmp/recovered.step';
  });
  await assert.rejects(gate(options), /native panel unavailable/);
  assert.equal(await gate(options), '/tmp/recovered.step');
});
