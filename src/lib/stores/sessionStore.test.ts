import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import { get } from 'svelte/store';
import { session } from './sessionStore';

afterEach(() => {
  session.setError(null);
  session.setGlobalError(null);
});

test('global app errors stay separate from thread session errors', () => {
  session.setError('thread failure');
  session.setGlobalError('config save failure');

  assert.equal(get(session).error, 'thread failure');
  assert.equal(get(session).globalError, 'config save failure');
});

test('setting the same model URL does not reload geometry', () => {
  session.setStlUrl('/tmp/model.stl');
  const loaded = get(session);

  session.setStlUrl('/tmp/model.stl');

  assert.equal(get(session).runtimeRevision, loaded.runtimeRevision);
});

test('explicit runtime reload retries the same model URL', () => {
  session.setStlUrl('/tmp/model.stl');
  const loaded = get(session);

  session.reloadStlUrl('/tmp/model.stl');

  assert.equal(get(session).stlUrl, '/tmp/model.stl');
  assert.equal(get(session).runtimeRevision, loaded.runtimeRevision + 1);
});
