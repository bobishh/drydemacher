import assert from 'node:assert/strict';
import test from 'node:test';
import { formatProviderMermaidError, providerMermaidConfig } from './providerMermaid';

test('provider Mermaid config disables autostart and keeps strict untrusted-content security', () => {
  assert.equal(providerMermaidConfig.startOnLoad, false);
  assert.equal(providerMermaidConfig.securityLevel, 'strict');
  assert.equal(providerMermaidConfig.suppressErrorRendering, true);
});

test('formatProviderMermaidError preserves the parser error body', () => {
  assert.equal(
    formatProviderMermaidError(new Error('Parse error on line 2: BROKEN -->')),
    'Parse error on line 2: BROKEN -->',
  );
});
