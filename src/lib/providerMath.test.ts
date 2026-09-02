import assert from 'node:assert/strict';
import test from 'node:test';
import { renderProviderMath } from './providerMath';

test('renderProviderMath produces KaTeX markup for inline and display formulas', () => {
  const inline = renderProviderMath('50^\\circ', false);
  const display = renderProviderMath('A/B \\approx 0.884', true);

  assert.match(inline, /class="katex"/);
  assert.match(inline, /50/);
  assert.match(display, /class="katex-display"/);
});

test('renderProviderMath keeps invalid LaTeX visible without trusted HTML', () => {
  const rendered = renderProviderMath('\\href{javascript:alert(1)}{unsafe}', false);

  assert.doesNotMatch(rendered, /href="javascript:/);
  assert.match(rendered, /unsafe/);
});
