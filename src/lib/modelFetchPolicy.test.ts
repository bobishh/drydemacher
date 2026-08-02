import assert from 'node:assert/strict';
import test from 'node:test';
import { modelSourceFingerprint } from './modelFetchPolicy';

const engine = {
  id: 'nim',
  name: 'NVIDIA NIM',
  provider: 'openai',
  apiKey: 'nvapi-original',
  model: 'meta/llama-3.1-70b-instruct',
  lightModel: '',
  baseUrl: 'https://integrate.api.nvidia.com/v1',
};

test('model source fingerprint changes only for fields used to list models', () => {
  const original = modelSourceFingerprint(engine);

  assert.equal(modelSourceFingerprint({ ...engine, name: 'Renamed' }), original);
  assert.equal(modelSourceFingerprint({ ...engine, model: 'other-model' }), original);
  assert.notEqual(modelSourceFingerprint({ ...engine, provider: 'gemini' }), original);
  assert.notEqual(modelSourceFingerprint({ ...engine, apiKey: 'changed' }), original);
  assert.notEqual(modelSourceFingerprint({ ...engine, baseUrl: 'http://localhost:11434/v1' }), original);
});
