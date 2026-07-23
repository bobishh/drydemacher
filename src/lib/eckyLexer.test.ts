import assert from 'node:assert/strict';
import test from 'node:test';

import { lexEcky } from './eckyLexer';

test('lexes ecky source into the existing highlight kinds without losing source text', () => {
  const source = '(number width 10 :label "Width")\n; shell';
  const tokens = lexEcky(source);

  assert.equal(tokens.map((token) => token.text).join(''), source);
  assert.deepEqual(
    tokens.filter((token) => token.kind).map(({ text, kind }) => ({ text, kind })),
    [
      { text: '(', kind: 'paren1' },
      { text: 'number', kind: 'kind' },
      { text: 'width', kind: 'name' },
      { text: '10', kind: 'number' },
      { text: ':label', kind: 'atom' },
      { text: '"Width"', kind: 'string' },
      { text: ')', kind: 'paren1' },
      { text: '; shell', kind: 'comment' },
    ],
  );
});

test('preserves highlighting state across lines and handles escaped strings', () => {
  const tokens = lexEcky('(model\n  (part body "say \\"hi\\""))');

  assert.deepEqual(
    tokens.filter((token) => token.kind).map((token) => token.kind),
    ['paren1', 'keyword', 'paren2', 'keyword', 'name', 'string', 'paren2', 'paren1'],
  );
});

test('never drops unknown punctuation or whitespace', () => {
  const source = '(custom @ value)\n';
  const tokens = lexEcky(source);

  assert.equal(tokens.map((token) => token.text).join(''), source);
  assert.deepEqual(tokens.find((token) => token.text === '@'), { text: '@', kind: null });
});
