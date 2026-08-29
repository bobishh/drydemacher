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
