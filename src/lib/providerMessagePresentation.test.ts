import assert from 'node:assert/strict';
import test from 'node:test';
import {
  providerMessagePresentation,
  providerMessageText,
} from './providerMessagePresentation';

test('providerMessagePresentation exposes current-model markdown links and removes standalone debug ids', () => {
  const presentation = providerMessagePresentation([
    'Высота секции: 18 → 80 мм.',
    '',
    'Параметр: [model.ecky](/Users/bogdan/Library/Application%20Support/ecky/model.ecky:110)',
    '',
    '`messageId: dc1fd5aa-a024-4d4e-97fe-a261845806d9`\\',
    '`modelId: generated-direct-occt-58b8b28e2cc2`',
  ].join('\n'));

  assert.equal(presentation.text.includes('messageId'), false);
  assert.equal(presentation.text.includes('modelId'), false);
  assert.deepEqual(presentation.segments, [
    { kind: 'text', text: 'Высота секции: 18 → 80 мм.\n\nПараметр: ' },
    {
      kind: 'codeReference',
      label: 'model.ecky',
      path: '/Users/bogdan/Library/Application Support/ecky/model.ecky',
      line: 110,
    },
  ]);
});

test('providerMessageText keeps diagnostic prose that merely mentions modelId', () => {
  assert.equal(
    providerMessageText('Validation failed: modelId mismatch.\nmodelId: debug-only'),
    'Validation failed: modelId mismatch.',
  );
});

test('providerMessagePresentation segments supported LaTeX delimiters and leaves an incomplete formula as text', () => {
  const presentation = providerMessagePresentation([
    'Inline $50^\\circ$ and \\(30\\text{ см}\\).',
    'Display: $$A/B \\approx 0.884$$ and \\[x^2 + y^2\\].',
    'Incomplete $formula',
  ].join('\n'));

  assert.deepEqual(presentation.segments, [
    { kind: 'text', text: 'Inline ' },
    { kind: 'math', latex: '50^\\circ', displayMode: false },
    { kind: 'text', text: ' and ' },
    { kind: 'math', latex: '30\\text{ см}', displayMode: false },
    { kind: 'text', text: '.\nDisplay: ' },
    { kind: 'math', latex: 'A/B \\approx 0.884', displayMode: true },
    { kind: 'text', text: ' and ' },
    { kind: 'math', latex: 'x^2 + y^2', displayMode: true },
    { kind: 'text', text: '.\nIncomplete $formula' },
  ]);
});
