import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveCodeModalSource } from './codeModalSource';

test('active viewport render source wins over bound project source', () => {
  assert.deepEqual(
    resolveCodeModalSource({
      activeRenderSource: 'print("agent draft")',
      boundSource: 'print("committed")',
      activeRenderMatchesViewport: true,
    }),
    {
      source: 'print("agent draft")',
      authority: 'draft',
    },
  );
});

test('bound project source remains authoritative without an active render draft', () => {
  assert.deepEqual(
    resolveCodeModalSource({
      activeRenderSource: 'print("working copy")',
      boundSource: 'print("committed")',
      activeRenderMatchesViewport: false,
    }),
    {
      source: 'print("committed")',
      authority: 'bound',
    },
  );
});
