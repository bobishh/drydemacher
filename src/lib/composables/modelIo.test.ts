import test from 'node:test';
import assert from 'node:assert/strict';
import { createModelIo } from './modelIo';

test('Given a STEP artifact When STEP export runs Then it saves and exports that artifact', async () => {
  const calls: string[] = [];
  const io = createModelIo({
    save: async () => '/tmp/model.step',
    exportFile: async (source, target) => { calls.push(`${source}:${target}`); },
    exportMultipart3mf: async () => {}, exportMultipartStlZip: async () => {},
    setStatus: () => {}, setError: () => {}, formatError: String,
  });
  await io.exportModel('step', { modelId: 'm', previewStlPath: '', fcstdPath: '', manifestPath: '', contentHash: '', sourceKind: 'generated', exportArtifacts: [{ format: 'step', path: '/tmp/model.step', label: 'STEP', role: 'primary' }] }, { threeMf: '', multipartStlZip: '', stl: '', step: 'model.step', fcstd: '' }, [], false, 'model');
  assert.deepEqual(calls, ['/tmp/model.step:/tmp/model.step']);
});
