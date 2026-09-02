import { renderToString } from 'katex';

export function renderProviderMath(latex: string, displayMode: boolean): string {
  return renderToString(latex, {
    displayMode,
    throwOnError: false,
    strict: 'ignore',
    trust: false,
    output: 'htmlAndMathml',
    maxExpand: 1000,
  });
}
