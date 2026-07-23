import test from 'node:test';
import assert from 'node:assert/strict';

import {
  seedCodeModalDraftField,
  shouldReseedCodeModalDraftFields,
} from './codeModalDraftFields';

test('shouldReseedCodeModalDraftFields resets stale commit metadata when modal scope changes', () => {
  assert.equal(
    shouldReseedCodeModalDraftFields('thread-a:msg-1', 'thread-b:msg-9', 'idle'),
    true,
  );
});

test('shouldReseedCodeModalDraftFields keeps edited fields inside the same modal scope', () => {
  assert.equal(
    shouldReseedCodeModalDraftFields('thread-a:msg-1', 'thread-a:msg-1', 'idle'),
    false,
  );
});

test('shouldReseedCodeModalDraftFields does not reset fields mid-apply or mid-commit', () => {
  assert.equal(
    shouldReseedCodeModalDraftFields('thread-a:msg-1', 'thread-b:msg-9', 'applying'),
    false,
  );
  assert.equal(
    shouldReseedCodeModalDraftFields('thread-a:msg-1', 'thread-b:msg-9', 'committing'),
    false,
  );
});

test('seedCodeModalDraftField trims valid metadata and falls back for empty strings', () => {
  assert.equal(seedCodeModalDraftField('  Razor  ', 'Manual Edit'), 'Razor');
  assert.equal(seedCodeModalDraftField('   ', 'Manual Edit'), 'Manual Edit');
});
