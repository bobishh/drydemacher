import assert from 'node:assert/strict';
import test from 'node:test';
import { buildCodeWindowTranspilePrompt } from './cadTranspile';

test('Given a Code buffer When transpile prompt builds Then foreign source is carried verbatim with verify intent', () => {
  const source = '// sentinel μ\r\ncube([10, 20, 30]);\r\n';
  const prompt = buildCodeWindowTranspilePrompt(source);

  assert.ok(prompt.includes(source));
  assert.ok(prompt.includes('Translate the foreign CAD source'));
  assert.ok(prompt.includes('authored `(verify ...)` clauses'));
});
