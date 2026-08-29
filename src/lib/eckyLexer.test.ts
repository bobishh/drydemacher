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

test('highlights import-stl preparation keywords without losing source text', () => {
  const source = '(model (part body (import-stl "/tmp/part.stl" :target-triangles 4000 :max-error 0.05 :preserve-boundaries #t)))';
  const tokens = lexEcky(source);

  assert.equal(tokens.map((token) => token.text).join(''), source);
  assert.deepEqual(
    tokens.filter((token) => token.kind).map(({ text, kind }) => ({ text, kind })),
    [
      { text: '(', kind: 'paren1' },
      { text: 'model', kind: 'keyword' },
      { text: '(', kind: 'paren2' },
      { text: 'part', kind: 'keyword' },
      { text: 'body', kind: 'name' },
      { text: '(', kind: 'paren3' },
      { text: 'import-stl', kind: 'op' },
      { text: '"/tmp/part.stl"', kind: 'string' },
      { text: ':target-triangles', kind: 'atom' },
      { text: '4000', kind: 'number' },
      { text: ':max-error', kind: 'atom' },
      { text: '0.05', kind: 'number' },
      { text: ':preserve-boundaries', kind: 'atom' },
      { text: '#t', kind: 'atom' },
      { text: ')', kind: 'paren3' },
      { text: ')', kind: 'paren2' },
      { text: ')', kind: 'paren1' },
    ],
  );
});

test('never drops unknown punctuation or whitespace', () => {
  const source = '(custom @ value)\n';
  const tokens = lexEcky(source);

  assert.equal(tokens.map((token) => token.text).join(''), source);
  assert.deepEqual(tokens.find((token) => token.text === '@'), { text: '@', kind: null });
});
